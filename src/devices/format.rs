use std::ops::Range;

use bitflags::bitflags;

pub enum SampleFormat {
    Float64,
    Float32,
    Signed32,
    Unsigned32,
    Signed24,
    Unsigned24,
    Signed16,
    Unsigned16,
    Signed8,
    Unsigned8,
    DSD,
    Unsupported,
}

pub enum ChannelSpec {
    Bitmask(Channels),
    Count(u16),
}

pub enum BufferSize {
    Range(Range<u32>),
    Fixed(u32),
    Unknown,
}

pub struct FormatInfo {
    pub originating_provider: &'static str,
    pub sample_type: SampleFormat,
    pub sample_rate: u32,
    pub buffer_size: BufferSize,
    pub channels: ChannelSpec,
}
pub struct SupportedFormat {
    pub originating_provider: &'static str,
    pub sample_type: SampleFormat,
    pub sample_rates: Range<u32>,
    pub buffer_size: BufferSize,
    pub channels: ChannelSpec,
}

bitflags! {
    #[derive(Default)]
    pub struct Channels: u32 {
        const FRONT_LEFT            = 0x1;
        const FRONT_RIGHT           = 0x2;
        const FRONT_CENTER          = 0x4;
        const LOW_FREQUENCY         = 0x8;
        const BACK_LEFT             = 0x10;
        const BACK_RIGHT            = 0x20;
        const FRONT_LEFT_OF_CENTER  = 0x40;
        const FRONT_RIGHT_OF_CENTER = 0x80;
        const BACK_CENTER           = 0x100;
        const SIDE_LEFT             = 0x200;
        const SIDE_RIGHT            = 0x400;
        const TOP_CENTER            = 0x800;
        const TOP_FRONT_LEFT        = 0x1000;
        const TOP_FRONT_CENTER      = 0x2000;
        const TOP_FRONT_RIGHT       = 0x4000;
        const TOP_BACK_LEFT         = 0x8000;
        const TOP_BACK_CENTER       = 0x10000;
        const TOP_BACK_RIGHT        = 0x20000;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Mono,
    Stereo,
    TwoOne,
    FiveOne,
    SevenOne,
}

impl Layout {
    pub fn channels(&self) -> Channels {
        match self {
            Layout::Mono => Channels::FRONT_LEFT,
            Layout::Stereo => Channels::FRONT_LEFT | Channels::FRONT_RIGHT,
            Layout::TwoOne => {
                Channels::FRONT_LEFT | Channels::FRONT_RIGHT | Channels::LOW_FREQUENCY
            }
            Layout::FiveOne => {
                Channels::FRONT_LEFT
                    | Channels::FRONT_RIGHT
                    | Channels::BACK_LEFT
                    | Channels::BACK_RIGHT
                    | Channels::LOW_FREQUENCY
            }
            Layout::SevenOne => {
                Channels::FRONT_LEFT
                    | Channels::FRONT_RIGHT
                    | Channels::SIDE_LEFT
                    | Channels::SIDE_RIGHT
                    | Channels::BACK_LEFT
                    | Channels::BACK_RIGHT
                    | Channels::LOW_FREQUENCY
            }
        }
    }
}
