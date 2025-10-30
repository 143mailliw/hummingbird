#![allow(dead_code)]

use thiserror::Error;

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum OpenError {
    #[error("File is corrupt")]
    FileCorrupt,
    #[error("Format not supported by decoder")]
    UnsupportedFormat,
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum CloseError {
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum PlaybackStartError {
    #[error("No media is open")]
    NothingOpen,
    #[error("Media is open but has no audio")]
    NothingToPlay,
    #[error("Media is undecodable")]
    Undecodable,
    #[error("Media container is broken")]
    BrokenContainer,
    #[error("Container is supported but not codec")]
    ContainerSupportedButNotCodec,
    #[error("Failed to process media: {0}")]
    MediaError(String),
    #[error("Audio stream error: {0}")]
    StreamError(String),
    #[error("Channel configuration error: {0}")]
    ChannelError(String),
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum PlaybackStopError {
    #[error("No media is open")]
    NothingOpen,
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum PlaybackReadError {
    #[error("No media is open")]
    NothingOpen,
    #[error("Media is open but was never started")]
    NeverStarted,
    #[error("End of file reached")]
    Eof,
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
    #[error("Decode error: `{0}`")]
    DecodeFatal(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum MetadataError {
    #[error("No media is open")]
    NothingOpen,
    #[error("The selected MediaProvider does not support metadata")]
    OperationUnsupported,
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum FrameDurationError {
    #[error("No media is open")]
    NothingOpen,
    #[error("Frame length requested before decoding")]
    NeverDecoded,
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum TrackDurationError {
    #[error("No media is open")]
    NothingOpen,
    #[error("Media is open but was never started")]
    NeverStarted,
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum SeekError {
    #[error("No media is open")]
    NothingOpen,
    #[error("Seek position out of bounds")]
    OutOfBounds,
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}

#[derive(PartialEq, Eq, Debug, Clone, Error)]
pub enum ChannelRetrievalError {
    #[error("No media is open")]
    NothingOpen,
    #[error("Media is open but has no audio")]
    NothingToPlay,
    #[error("Unknown media provider error: `{0}`")]
    Unknown(String),
}
