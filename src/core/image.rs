// SPDX-License-Identifier: GPL-2.0-or-later

//! Bounded, GTK-independent profile-image normalisation.

use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;

use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageEncoder, ImageReader, Limits};
use thiserror::Error;

pub(super) const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const SUPPORTED_AVATAR_MIME_TYPES: [&str; 3] = ["image/png", "image/jpeg", "image/webp"];
pub(crate) const SUPPORTED_AVATAR_SUFFIXES: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
const MAX_SOURCE_DIMENSION: u32 = 16_384;
const MAX_SOURCE_PIXELS: u64 = 32_000_000;
const MAX_DECODER_ALLOCATION: u64 = 128 * 1024 * 1024;
const NORMALISED_SIZE: u32 = 512;

#[derive(Debug, Error)]
pub(crate) enum ImageNormalisationError {
    #[error("image file could not be opened: {0}")]
    Open(#[source] std::io::Error),
    #[error("image file metadata could not be read: {0}")]
    Metadata(#[source] std::io::Error),
    #[error("image path is not a regular file")]
    NotAFile,
    #[error("image file could not be read: {0}")]
    Read(#[source] std::io::Error),
    #[error("image data is empty")]
    Empty,
    #[error("image data exceeds the {MAX_SOURCE_BYTES}-byte safety limit")]
    SourceTooLarge,
    #[error(
        "image dimensions {width} by {height} exceed the safety limit of {MAX_SOURCE_DIMENSION} pixels per side and {MAX_SOURCE_PIXELS} pixels total"
    )]
    Dimensions { width: u32, height: u32 },
    #[error("image could not be decoded: {0}")]
    Decode(#[source] image::ImageError),
    #[error("normalised PNG could not be encoded: {0}")]
    Encode(#[source] image::ImageError),
}

/// Reads a local regular file with a hard byte bound, then normalises it.
pub(crate) fn normalise_avatar_file(path: &Path) -> Result<Vec<u8>, ImageNormalisationError> {
    let file = File::open(path).map_err(ImageNormalisationError::Open)?;
    let metadata = file.metadata().map_err(ImageNormalisationError::Metadata)?;
    if !metadata.is_file() {
        return Err(ImageNormalisationError::NotAFile);
    }
    if metadata.len() > MAX_SOURCE_BYTES as u64 {
        return Err(ImageNormalisationError::SourceTooLarge);
    }

    let mut source = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut source)
        .map_err(ImageNormalisationError::Read)?;
    normalise_avatar(&source)
}

/// Decodes an image, applies its embedded orientation, and returns a square PNG.
pub(crate) fn normalise_avatar(source: &[u8]) -> Result<Vec<u8>, ImageNormalisationError> {
    if source.is_empty() {
        return Err(ImageNormalisationError::Empty);
    }
    if source.len() > MAX_SOURCE_BYTES {
        return Err(ImageNormalisationError::SourceTooLarge);
    }

    let reader = ImageReader::new(Cursor::new(source))
        .with_guessed_format()
        .map_err(|error| ImageNormalisationError::Decode(image::ImageError::IoError(error)))?;
    let mut decoder = reader
        .into_decoder()
        .map_err(ImageNormalisationError::Decode)?;
    let (width, height) = decoder.dimensions();
    validate_dimensions(width, height)?;

    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_DIMENSION);
    limits.max_image_height = Some(MAX_SOURCE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODER_ALLOCATION);
    decoder
        .set_limits(limits)
        .map_err(ImageNormalisationError::Decode)?;

    let orientation = decoder
        .orientation()
        .map_err(ImageNormalisationError::Decode)?;
    let mut decoded =
        DynamicImage::from_decoder(decoder).map_err(ImageNormalisationError::Decode)?;
    decoded.apply_orientation(orientation);

    let (width, height) = decoded.dimensions();
    let side = width.min(height);
    let crop = decoded.view((width - side) / 2, (height - side) / 2, side, side);
    let normalised = image::imageops::resize(
        &*crop,
        NORMALISED_SIZE,
        NORMALISED_SIZE,
        FilterType::Lanczos3,
    );

    let mut output = Vec::new();
    PngEncoder::new(&mut output)
        .write_image(
            normalised.as_raw(),
            NORMALISED_SIZE,
            NORMALISED_SIZE,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(ImageNormalisationError::Encode)?;
    Ok(output)
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), ImageNormalisationError> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0
        || height == 0
        || width > MAX_SOURCE_DIMENSION
        || height > MAX_SOURCE_DIMENSION
        || pixels > MAX_SOURCE_PIXELS
    {
        return Err(ImageNormalisationError::Dimensions { width, height });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use image::codecs::jpeg::JpegEncoder;
    use image::codecs::webp::WebPEncoder;
    use image::{ColorType, ImageBuffer, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};

    use super::*;

    static NEXT_TEMPORARY_FILE: AtomicU64 = AtomicU64::new(0);

    fn temporary_file(name: &str) -> std::path::PathBuf {
        let sequence = NEXT_TEMPORARY_FILE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "xfce4-profile-editor-{}-{sequence}-{name}",
            std::process::id()
        ))
    }

    fn png_bytes(image: &RgbaImage) -> Vec<u8> {
        let mut output = Vec::new();
        PngEncoder::new(&mut output)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        output
    }

    fn jpeg_with_orientation(image: &RgbImage, orientation: u8) -> Vec<u8> {
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 95)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ColorType::Rgb8.into(),
            )
            .unwrap();

        let mut exif = b"Exif\0\0II*\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0".to_vec();
        exif.extend_from_slice(&[orientation, 0, 0, 0, 0, 0, 0, 0]);
        let segment_length = u16::try_from(exif.len() + 2).unwrap().to_be_bytes();

        let mut result = Vec::with_capacity(jpeg.len() + exif.len() + 4);
        result.extend_from_slice(&jpeg[..2]);
        result.extend_from_slice(&[0xff, 0xe1]);
        result.extend_from_slice(&segment_length);
        result.extend_from_slice(&exif);
        result.extend_from_slice(&jpeg[2..]);
        result
    }

    fn decode_normalised(bytes: &[u8]) -> DynamicImage {
        image::load_from_memory_with_format(bytes, ImageFormat::Png).unwrap()
    }

    fn webp_bytes(image: &RgbaImage) -> Vec<u8> {
        let mut output = Vec::new();
        WebPEncoder::new_lossless(&mut output)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        output
    }

    #[test]
    fn centre_crops_and_encodes_a_512_pixel_rgba_png() {
        let source = ImageBuffer::from_fn(4, 2, |x, _| match x {
            1 => Rgba([255, 0, 0, 255]),
            2 => Rgba([0, 255, 0, 255]),
            _ => Rgba([0, 0, 0, 255]),
        });

        let output = normalise_avatar(&png_bytes(&source)).unwrap();
        let decoded = decode_normalised(&output);

        assert_eq!(&output[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(decoded.dimensions(), (NORMALISED_SIZE, NORMALISED_SIZE));
        assert_eq!(decoded.color(), ColorType::Rgba8);
        assert!(decoded.get_pixel(64, 256).0[0] > 240);
        assert!(decoded.get_pixel(448, 256).0[1] > 240);
    }

    #[test]
    fn reads_and_normalises_a_bounded_local_file() {
        let path = temporary_file("valid.png");
        let source = RgbaImage::from_pixel(2, 2, Rgba([20, 40, 60, 255]));
        fs::write(&path, png_bytes(&source)).unwrap();

        let output = normalise_avatar_file(&path).unwrap();

        fs::remove_file(&path).unwrap();
        assert_eq!(
            decode_normalised(&output).dimensions(),
            (NORMALISED_SIZE, NORMALISED_SIZE)
        );
    }

    #[test]
    fn applies_embedded_exif_orientation_before_cropping() {
        let source = ImageBuffer::from_fn(40, 20, |x, _| {
            if x < 20 {
                Rgb([255, 0, 0])
            } else {
                Rgb([0, 255, 0])
            }
        });

        let output = normalise_avatar(&jpeg_with_orientation(&source, 6)).unwrap();
        let decoded = decode_normalised(&output);
        let top = decoded.get_pixel(256, 64).0;
        let bottom = decoded.get_pixel(256, 448).0;

        assert!(top[0] > 200 && top[1] < 50, "unexpected top pixel: {top:?}");
        assert!(
            bottom[1] > 200 && bottom[0] < 50,
            "unexpected bottom pixel: {bottom:?}"
        );
    }

    #[test]
    fn accepts_webp_and_normalises_it_to_png() {
        let source = RgbaImage::from_pixel(2, 2, Rgba([20, 40, 60, 128]));

        let output = normalise_avatar(&webp_bytes(&source)).unwrap();
        let decoded = decode_normalised(&output);

        assert_eq!(decoded.dimensions(), (NORMALISED_SIZE, NORMALISED_SIZE));
        assert_eq!(decoded.get_pixel(256, 256), Rgba([20, 40, 60, 128]));
    }

    #[test]
    fn rejects_empty_oversized_and_invalid_input() {
        assert!(matches!(
            normalise_avatar(&[]),
            Err(ImageNormalisationError::Empty)
        ));
        assert!(matches!(
            normalise_avatar(&vec![0; MAX_SOURCE_BYTES + 1]),
            Err(ImageNormalisationError::SourceTooLarge)
        ));
        assert!(matches!(
            normalise_avatar(b"not an image"),
            Err(ImageNormalisationError::Decode(_))
        ));
    }

    #[test]
    fn rejects_non_file_and_oversized_file_paths() {
        assert!(matches!(
            normalise_avatar_file(&std::env::temp_dir()),
            Err(ImageNormalisationError::NotAFile)
        ));

        let path = temporary_file("oversized.png");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_SOURCE_BYTES as u64 + 1).unwrap();
        drop(file);

        assert!(matches!(
            normalise_avatar_file(&path),
            Err(ImageNormalisationError::SourceTooLarge)
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_dimensions_beyond_the_decode_limit() {
        let source = RgbaImage::new(MAX_SOURCE_DIMENSION + 1, 1);

        assert!(matches!(
            normalise_avatar(&png_bytes(&source)),
            Err(ImageNormalisationError::Dimensions {
                width,
                height: 1
            }) if width == MAX_SOURCE_DIMENSION + 1
        ));
    }

    #[test]
    fn rejects_excessive_total_pixel_count() {
        assert!(matches!(
            validate_dimensions(8_000, 4_001),
            Err(ImageNormalisationError::Dimensions {
                width: 8_000,
                height: 4_001
            })
        ));
    }
}
