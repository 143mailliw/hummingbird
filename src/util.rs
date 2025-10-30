use image::{Pixel, RgbaImage};

pub fn rgb_to_bgr(image: &mut RgbaImage) {
    image.pixels_mut().for_each(|v| {
        let slice = v.channels();
        *v = *image::Rgba::from_slice(&[slice[2], slice[1], slice[0], slice[3]]);
    });
}

macro_rules! make_unknown_error {
    ($from:ty, $to:ty) => {
        impl From<$from> for $to {
            fn from(value: $from) -> Self {
                <$to>::Unknown(value.to_string())
            }
        }
    };
}

macro_rules! make_unknown_error_unwrap {
    ($from:ty, $to:ty) => {
        impl From<$from> for $to {
            fn from(value: $from) -> Self {
                <$to>::Unknown(value.to_string().unwrap())
            }
        }
    };
}

pub(crate) use make_unknown_error;
