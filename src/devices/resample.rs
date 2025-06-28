use std::collections::VecDeque;

use intx::{I24, U24};
use rubato::{FftFixedIn, VecResampler};
use tracing::info;

use crate::media::playback::{PlaybackFrame, Samples};

use super::format::{FormatInfo, SampleFormat};

fn scale<T, U>(target: Vec<Vec<T>>) -> Vec<Vec<U>>
where
    T: Copy,
    U: SampleFrom<T>,
{
    target
        .iter()
        .map(|v| v.iter().map(|v| U::sample_from(*v)).collect())
        .collect()
}

pub fn convert_samples<T>(target_frame: Samples) -> Vec<Vec<T>>
where
    T: Copy + SampleInto<f64> + SampleFrom<f64>,
{
    match target_frame {
        Samples::Float64(v) => scale(v),
        Samples::Float32(v) => scale(v),
        Samples::Signed32(v) => scale(v),
        Samples::Unsigned32(v) => scale(v),
        Samples::Signed24(v) => scale(v),
        Samples::Unsigned24(v) => scale(v),
        Samples::Signed16(v) => scale(v),
        Samples::Unsigned16(v) => scale(v),
        Samples::Signed8(v) => scale(v),
        Samples::Unsigned8(v) => scale(v),
        Samples::Dsd(_) => unimplemented!(),
    }
}

pub trait SampleInto<T> {
    fn sample_into(self) -> T;
}

impl SampleInto<f64> for U24 {
    fn sample_into(self) -> f64 {
        f64::from(u32::from(self)) / f64::from(i32::from(I24::MAX)) - 1.0
    }
}

impl SampleInto<f64> for I24 {
    fn sample_into(self) -> f64 {
        f64::from(i32::from(self)) / f64::from(i32::from(I24::MAX))
    }
}

impl SampleInto<f64> for f32 {
    fn sample_into(self) -> f64 {
        self as f64
    }
}

macro_rules! f64_to {
    ($t:ty, $max_type:ty, $offset:expr) => {
        impl SampleInto<f64> for $t {
            fn sample_into(self) -> f64 {
                f64::from(self) / (f64::from(<$max_type>::MAX)) + $offset
            }
        }
    };
}

f64_to!(u32, i32, -1.0);
f64_to!(u16, i16, -1.0);
f64_to!(u8, i8, -1.0);
f64_to!(i32, i32, 0.0);
f64_to!(i16, i16, 0.0);
f64_to!(i8, i8, 0.0);

pub trait SampleFrom<T> {
    fn sample_from(value: T) -> Self;
}

impl SampleFrom<f64> for U24 {
    fn sample_from(value: f64) -> Self {
        U24::try_from(((value + 1.0) * f64::from(i32::from(I24::MAX))) as u32)
            .expect("out of U24 bounds")
    }
}

impl SampleFrom<f64> for I24 {
    fn sample_from(value: f64) -> Self {
        I24::try_from((value * f64::from(i32::from(I24::MAX))) as i32).expect("out of I24 bounds")
    }
}

impl SampleFrom<f64> for f32 {
    fn sample_from(value: f64) -> Self {
        value as f32
    }
}

macro_rules! f64_from {
    ($t:ty, $max_type:ty, $offset:expr) => {
        impl SampleFrom<f64> for $t {
            fn sample_from(value: f64) -> $t {
                ((value - $offset) * f64::from(<$max_type>::MAX)) as $t
            }
        }
    };
}

f64_from!(u32, i32, -1.0);
f64_from!(u16, i16, -1.0);
f64_from!(u8, i8, -1.0);
f64_from!(i32, i32, 0.0);
f64_from!(i16, i16, 0.0);
f64_from!(i8, i8, 0.0);

impl<T, U> SampleFrom<T> for U
where
    T: SampleInto<f64>,
    U: SampleFrom<f64>,
{
    fn sample_from(value: T) -> Self {
        let a: f64 = T::sample_into(value);
        U::sample_from(a)
    }
}

impl SampleFrom<f64> for f64 {
    fn sample_from(value: f64) -> Self {
        value
    }
}

pub fn match_bit_depth(target_frame: PlaybackFrame, target_depth: SampleFormat) -> PlaybackFrame {
    let rate = target_frame.rate;

    let samples = if !target_frame.samples.is_format(target_depth) {
        match target_depth {
            SampleFormat::Float64 => todo!(),
            SampleFormat::Float32 => Samples::Float32(convert_samples(target_frame.samples)),
            SampleFormat::Signed32 => Samples::Signed32(convert_samples(target_frame.samples)),
            SampleFormat::Unsigned32 => Samples::Unsigned32(convert_samples(target_frame.samples)),
            SampleFormat::Signed24 => Samples::Signed24(convert_samples(target_frame.samples)),
            SampleFormat::Unsigned24 => Samples::Unsigned24(convert_samples(target_frame.samples)),
            SampleFormat::Signed24Packed => {
                Samples::Signed24(convert_samples(target_frame.samples))
            }
            SampleFormat::Unsigned24Packed => {
                Samples::Unsigned24(convert_samples(target_frame.samples))
            }
            SampleFormat::Signed16 => Samples::Signed16(convert_samples(target_frame.samples)),
            SampleFormat::Unsigned16 => Samples::Unsigned16(convert_samples(target_frame.samples)),
            SampleFormat::Signed8 => Samples::Signed8(convert_samples(target_frame.samples)),
            SampleFormat::Unsigned8 => Samples::Unsigned8(convert_samples(target_frame.samples)),
            SampleFormat::Dsd => unimplemented!(),
            SampleFormat::Unsupported => panic!("target depth is unsupported"),
        }
    } else {
        target_frame.samples
    };

    PlaybackFrame { samples, rate }
}

pub struct Resampler {
    resampler: FftFixedIn<f32>,
    duration: u64,
    input_buffer: Vec<VecDeque<f32>>,
    eof: bool,
}

impl Resampler {
    pub fn new(orig_rate: u32, target_rate: u32, duration: u64, channels: u16) -> Self {
        if orig_rate != target_rate {
            info!(
                "Resampling required, resampling from {:?} to {:?} (duration {:?})",
                orig_rate, target_rate, duration
            );
        }

        let resampler = FftFixedIn::<f32>::new(
            orig_rate as usize,
            target_rate as usize,
            duration as usize,
            2,
            channels as usize,
        )
        .unwrap();

        Resampler {
            resampler,
            duration,
            input_buffer: (0..channels)
                .map(|_| VecDeque::with_capacity(duration as usize * 2))
                .collect(),
            eof: false,
        }
    }

    pub fn convert_formats(
        &mut self,
        frame: PlaybackFrame,
        target_format: &FormatInfo,
    ) -> PlaybackFrame {
        if target_format.sample_rate == frame.rate {
            return match_bit_depth(frame, target_format.sample_type);
        }
        let source: Vec<Vec<f32>> = convert_samples(frame.samples);

        self.input_buffer
            .iter_mut()
            .zip(source.into_iter().map(VecDeque::from))
            .for_each(|(buffer, mut src)| {
                buffer.append(&mut src);
            });

        if self.input_buffer[0].len() < self.duration as usize {
            // if source[0].len() == 0 {
            //     warn!("Zero length PlaybackFrame presented to convert_formats!");
            //     warn!("This is a decoding bug: please report it (with logs)");
            //     Vec::new()
            // } else {
            //     self.resampler
            //         .process_partial(Some(&source), None)
            //         .expect("resampler error")
            // }

            match_bit_depth(
                PlaybackFrame {
                    samples: Samples::Float32(Vec::with_capacity(0)),
                    rate: target_format.sample_rate,
                },
                target_format.sample_type,
            )
        } else {
            let split = self
                .input_buffer
                .iter_mut()
                .map(|v| v.drain(0..self.duration as usize).collect::<Vec<_>>())
                .collect::<Vec<_>>();

            let resampled = self
                .resampler
                .process(&split, None)
                .expect("resampler error");

            match_bit_depth(
                PlaybackFrame {
                    samples: Samples::Float32(resampled),
                    rate: target_format.sample_rate,
                },
                target_format.sample_type,
            )
        }
    }

    pub fn eof(&mut self) {
        self.eof = true;
    }
}
