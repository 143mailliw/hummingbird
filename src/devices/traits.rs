use crate::media::playback::{GetInnerSamples, PlaybackFrame};

use super::{
    errors::{
        CloseError, FindError, InfoError, InitializationError, ListError, OpenError,
        SubmissionError,
    },
    format::{FormatInfo, SupportedFormat},
};

/// The DeviceProvider trait defines the methods used to interact with a device provider. A device
/// provider is responsible for providing a list of devices available to the system, as well as
/// opening and closing streams on those devices.
///
/// The current audio pipeline is as follows:
pub trait DeviceProvider {
    /// Requests the device provider prepare itself for use.
    fn initialize(&mut self) -> Result<(), InitializationError>;
    /// Returns a list of devices available to the device provider.
    fn get_devices(&mut self) -> Result<Vec<Box<dyn Device>>, ListError>;
    /// Returns the default device of the device provider.
    fn get_default_device(&mut self) -> Result<Box<dyn Device>, FindError>;
    /// Requests the device provider find and return a device by its UID.
    fn get_device_by_uid(&mut self, id: &String) -> Result<Box<dyn Device>, FindError>;
}

pub trait Device {
    /// Requests the device open a stream with the given format.
    fn open_device(&mut self, format: FormatInfo) -> Result<Box<dyn OutputStream>, OpenError>;

    /// Returns the supported formats of the device.
    fn get_supported_formats(&self) -> Result<Vec<SupportedFormat>, InfoError>;
    /// Returns the device's default format.
    fn get_default_format(&self) -> Result<FormatInfo, InfoError>;
    /// Returns the name of the device.
    fn get_name(&self) -> Result<String, InfoError>;
    /// Returns the UID of the device. If the provider is unable to provide a UID, it should return
    /// the name of the device.
    fn get_uid(&self) -> Result<String, InfoError>;
    /// This function returns true if resampling and bit-depth matching is required to play audio
    /// on this device. If the device supports playing arbitrary bit-depths and sample-rates
    /// without advanced notice, this function should return false. If the device requires a
    /// matching and consistent format and rate, this function should return true.
    fn requires_matching_format(&self) -> bool;
}

pub trait OutputStream {
    /// Submits a playback frame to the device for playback. If Device::requires_matching_format is
    /// true, the audio *must* be in the same format as the current stream (can be retrieved with
    /// OutputStream::get_current_format). If requires_matching_format is false, the audio can be
    /// in any format.
    fn submit_frame(&mut self, frame: PlaybackFrame) -> Result<(), SubmissionError>;
    /// Closes the stream and releases any resources associated with it.
    fn close_stream(&mut self) -> Result<(), CloseError>;
    /// Returns true if the stream requires input (e.g. the buffer is empty).
    fn needs_input(&self) -> bool;
    /// Returns the current format of the stream.
    fn get_current_format(&self) -> Result<&FormatInfo, InfoError>;
}
