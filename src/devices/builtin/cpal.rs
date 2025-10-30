use std::ops::Range;

use crate::{
    devices::{
        errors::{
            CloseError, FindError, InfoError, InitializationError, ListError, OpenError,
            ResetError, StateError, SubmissionError,
        },
        format::{BufferSize, ChannelSpec, FormatInfo, SampleFormat, SupportedFormat},
        traits::{Device, DeviceProvider, OutputStream},
        util::{Scale, interleave},
    },
    media::playback::{GetInnerSamples, Mute, PlaybackFrame},
    util::make_unknown_error,
};
use cpal::{
    Host, SizedSample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rb::{Producer, RB, RbConsumer, RbProducer, SpscRb};

pub struct CpalProvider {
    host: Host,
}

impl Default for CpalProvider {
    fn default() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }
}

impl DeviceProvider for CpalProvider {
    fn initialize(&mut self) -> Result<(), InitializationError> {
        self.host = cpal::default_host();
        Ok(())
    }

    fn get_devices(&mut self) -> Result<Vec<Box<dyn Device>>, ListError> {
        Ok(self
            .host
            .devices()?
            .map(|dev| Box::new(CpalDevice::from(dev)) as Box<dyn Device>)
            .collect())
    }

    fn get_default_device(&mut self) -> Result<Box<dyn Device>, FindError> {
        self.host
            .default_output_device()
            .ok_or(FindError::DeviceDoesNotExist)
            .map(|dev| Box::new(CpalDevice::from(dev)) as Box<dyn Device>)
    }

    fn get_device_by_uid(&mut self, id: &str) -> Result<Box<dyn Device>, FindError> {
        self.host
            .devices()?
            .find(|dev| dev.name().unwrap_or("NULL".into()) == *id)
            .ok_or(FindError::DeviceDoesNotExist)
            .map(|dev| Box::new(CpalDevice::from(dev)) as Box<dyn Device>)
    }
}

struct CpalDevice {
    device: cpal::Device,
}

impl From<cpal::Device> for CpalDevice {
    fn from(value: cpal::Device) -> Self {
        CpalDevice { device: value }
    }
}

fn format_from_cpal(format: &cpal::SampleFormat) -> SampleFormat {
    match format {
        cpal::SampleFormat::I8 => SampleFormat::Signed8,
        cpal::SampleFormat::I16 => SampleFormat::Signed16,
        cpal::SampleFormat::I32 => SampleFormat::Signed32,
        cpal::SampleFormat::U8 => SampleFormat::Unsigned8,
        cpal::SampleFormat::U16 => SampleFormat::Unsigned16,
        cpal::SampleFormat::U32 => SampleFormat::Unsigned32,
        cpal::SampleFormat::F32 => SampleFormat::Float32,
        cpal::SampleFormat::F64 => SampleFormat::Float64,
        _ => SampleFormat::Unsupported, // should never happen
    }
}

fn cpal_config_from_info(format: &FormatInfo) -> Result<cpal::StreamConfig, ()> {
    if format.originating_provider != "cpal" {
        Err(())
    } else {
        Ok(cpal::StreamConfig {
            channels: 2,
            sample_rate: cpal::SampleRate(format.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        })
    }
}

fn create_stream_internal<
    T: SizedSample + GetInnerSamples + Default + Send + Sized + 'static + Mute,
>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer_size: usize,
) -> Result<(cpal::Stream, Producer<T>), OpenError> {
    let rb: SpscRb<T> = SpscRb::new(buffer_size);
    let cons = rb.consumer();
    let prod = rb.producer();

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let written = cons.read(data).unwrap_or(0);

            data[written..].iter_mut().for_each(|v| *v = T::muted())
        },
        move |_| {},
        None,
    )?;

    Ok((stream, prod))
}

trait CpalSample: SizedSample + GetInnerSamples + Default + Send + Sized + 'static + Mute {}

impl<T> CpalSample for T where
    T: SizedSample + GetInnerSamples + Default + Send + Sized + 'static + Mute
{
}

impl CpalDevice {
    fn create_stream<T>(&mut self, format: FormatInfo) -> Result<Box<dyn OutputStream>, OpenError>
    where
        T: CpalSample,
        Vec<Vec<T>>: Scale,
    {
        let config =
            cpal_config_from_info(&format).map_err(|_| OpenError::InvalidConfigProvider)?;

        let channels = match format.channels {
            ChannelSpec::Count(v) => v,
            _ => panic!("non cpal device"),
        };

        let buffer_size = ((200 * config.sample_rate.0 as usize) / 1000) * channels as usize;

        let (stream, prod) = create_stream_internal::<T>(&self.device, &config, buffer_size)?;

        Ok(Box::new(CpalStream {
            ring_buf: prod,
            stream,
            format,
            config,
            buffer_size,
            device: self.device.clone(),
            volume: 1.0,
        }))
    }
}

impl Device for CpalDevice {
    fn open_device(&mut self, format: FormatInfo) -> Result<Box<dyn OutputStream>, OpenError> {
        if format.originating_provider != "cpal" {
            Err(OpenError::InvalidConfigProvider)
        } else {
            match format.sample_type {
                SampleFormat::Signed8 => self.create_stream::<i8>(format),
                SampleFormat::Signed16 => self.create_stream::<i16>(format),
                SampleFormat::Signed32 => self.create_stream::<i32>(format),
                SampleFormat::Unsigned8 => self.create_stream::<u8>(format),
                SampleFormat::Unsigned16 => self.create_stream::<u16>(format),
                SampleFormat::Unsigned32 => self.create_stream::<u32>(format),
                SampleFormat::Float32 => self.create_stream::<f32>(format),
                SampleFormat::Float64 => self.create_stream::<f64>(format),
                _ => Err(OpenError::InvalidSampleFormat),
            }
        }
    }

    fn get_supported_formats(&self) -> Result<Vec<SupportedFormat>, InfoError> {
        Ok(self
            .device
            .supported_output_configs()?
            .filter(|c| {
                let format = c.sample_format();
                format != cpal::SampleFormat::I64 && format != cpal::SampleFormat::U64
            })
            .map(|c| SupportedFormat {
                originating_provider: "cpal",
                sample_type: format_from_cpal(&c.sample_format()),
                sample_rates: Range {
                    start: c.min_sample_rate().0,
                    end: c.max_sample_rate().0,
                },
                buffer_size: match c.buffer_size() {
                    cpal::SupportedBufferSize::Range { min, max } => BufferSize::Range(Range {
                        start: *min,
                        end: *max,
                    }),
                    cpal::SupportedBufferSize::Unknown => BufferSize::Unknown,
                },
                channels: ChannelSpec::Count(c.channels()),
            })
            .collect())
    }

    fn get_default_format(&self) -> Result<FormatInfo, InfoError> {
        let format = self.device.default_output_config()?;
        Ok(FormatInfo {
            originating_provider: "cpal",
            sample_type: format_from_cpal(&format.sample_format()),
            sample_rate: format.sample_rate().0,
            buffer_size: match format.buffer_size() {
                cpal::SupportedBufferSize::Range { min, max } => BufferSize::Range(Range {
                    start: *min,
                    end: *max,
                }),
                cpal::SupportedBufferSize::Unknown => BufferSize::Unknown,
            },
            channels: ChannelSpec::Count(format.channels()),
            rate_channel_ratio: if cfg!(target_os = "windows") {
                2
            } else {
                format.channels()
            },
            rate_channel_ratio_fixed: cfg!(target_os = "windows"),
        })
    }

    fn get_name(&self) -> Result<String, InfoError> {
        self.device.name().map_err(|v| v.into())
    }

    fn get_uid(&self) -> Result<String, InfoError> {
        self.device.name().map_err(|v| v.into())
    }

    fn requires_matching_format(&self) -> bool {
        false
    }
}

struct CpalStream<T>
where
    T: GetInnerSamples + SizedSample + Default,
{
    pub ring_buf: Producer<T>,
    pub stream: cpal::Stream,
    pub config: cpal::StreamConfig,
    pub device: cpal::Device,
    pub format: FormatInfo,
    pub buffer_size: usize,
    pub volume: f64,
}

impl<T> OutputStream for CpalStream<T>
where
    T: CpalSample,
    Vec<Vec<T>>: Scale,
{
    fn submit_frame(&mut self, frame: PlaybackFrame) -> Result<(), SubmissionError> {
        let samples = if self.volume > 0.98 {
            // don't scale if the volume is close to 1, it could lead to (negligable) quality loss
            T::inner(frame.samples)
        } else {
            T::inner(frame.samples).scale(self.volume)
        };

        let interleaved = interleave(samples);
        let mut slice: &[T] = &interleaved;

        while let Some(written) = self.ring_buf.write_blocking(slice) {
            slice = &slice[written..];
        }

        Ok(())
    }

    fn close_stream(&mut self) -> Result<(), CloseError> {
        Ok(())
    }

    fn needs_input(&self) -> bool {
        true // will always be true as long as the submitting thread is not blocked by submit_frame
    }

    fn get_current_format(&self) -> Result<&FormatInfo, InfoError> {
        Ok(&self.format)
    }

    fn play(&mut self) -> Result<(), StateError> {
        self.stream.play().map_err(|v| v.into())
    }

    fn pause(&mut self) -> Result<(), StateError> {
        self.stream.pause().map_err(|v| v.into())
    }

    fn reset(&mut self) -> Result<(), ResetError> {
        let (stream, prod) =
            create_stream_internal::<T>(&self.device, &self.config, self.buffer_size)?;

        self.stream = stream;
        self.ring_buf = prod;

        Ok(())
    }

    fn set_volume(&mut self, volume: f64) -> Result<(), StateError> {
        self.volume = volume;
        Ok(())
    }
}

make_unknown_error!(OpenError, ResetError);
make_unknown_error!(cpal::PlayStreamError, StateError);
make_unknown_error!(cpal::PauseStreamError, StateError);
make_unknown_error!(cpal::DeviceNameError, InfoError);
make_unknown_error!(cpal::DefaultStreamConfigError, InfoError);
make_unknown_error!(cpal::SupportedStreamConfigsError, InfoError);
make_unknown_error!(cpal::BuildStreamError, OpenError);
make_unknown_error!(cpal::DevicesError, ListError);
make_unknown_error!(cpal::DevicesError, FindError);
