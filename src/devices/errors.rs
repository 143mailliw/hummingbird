#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum InitializationError {
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum SubmissionError {
    RequiresOpenDevice,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ListError {
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum FindError {
    DeviceDoesNotExist,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum InfoError {
    RequiresOpenDevice,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum OpenError {
    InvalidConfigProvider,
    InvalidSampleFormat,
    Unknown,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum CloseError {
    Unknown,
}
