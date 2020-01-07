use image::{DynamicImage, ImageError};
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{self, BlendMode, TextureCreator, TextureValueError, UpdateTextureError};
use std::fmt::{self, Display, Formatter};

pub struct Texture<'r> {
    texture: render::Texture<'r>,
    width: u32,
    height: u32,
}

impl<'r> Texture<'r> {
    pub fn new<T>(creator: &'r TextureCreator<T>, data: &[u8]) -> Result<Texture<'r>, Error> {
        let image = image::load_from_memory(data).map_err(Error::Image)?;
        let (format, samples, width, height) = match image {
            DynamicImage::ImageRgba8(ref image) => (
                PixelFormatEnum::RGBA32,
                image.as_flat_samples().samples,
                image.width(),
                image.height(),
            ),
            DynamicImage::ImageRgb8(ref image) => (
                PixelFormatEnum::RGB24,
                image.as_flat_samples().samples,
                image.width(),
                image.height(),
            ),
            _ => return Err(Error::ImageFormat),
        };

        let mut texture = creator
            .create_texture_static(format, width, height)
            .map_err(Error::TextureValue)?;
        texture
            .update(None, samples, width as usize * format.byte_size_per_pixel())
            .map_err(Error::TextureUpdate)?;
        texture.set_blend_mode(BlendMode::Blend);

        Ok(Texture {
            texture,
            width,
            height,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn inner(&self) -> &render::Texture<'r> {
        &self.texture
    }
}

#[derive(Debug)]
pub enum Error {
    ImageFormat,
    Image(ImageError),
    TextureValue(TextureValueError),
    TextureUpdate(UpdateTextureError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::ImageFormat => write!(f, "Invalid image format"),
            Error::Image(err) => err.fmt(f),
            Error::TextureValue(err) => err.fmt(f),
            Error::TextureUpdate(err) => err.fmt(f),
        }
    }
}
