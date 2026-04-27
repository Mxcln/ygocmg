use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageReader};

use crate::domain::common::error::{AppError, AppResult};

pub fn import_main_image(source_path: &Path) -> AppResult<Vec<u8>> {
    let image = read_image(source_path)?;
    let resized = image.resize_exact(400, 580, FilterType::Triangle);
    encode_jpeg(&resized, source_path)
}

pub fn import_field_image(source_path: &Path) -> AppResult<Vec<u8>> {
    let image = read_image(source_path)?;
    encode_jpeg(&image, source_path)
}

fn read_image(source_path: &Path) -> AppResult<DynamicImage> {
    let reader = ImageReader::open(source_path)
        .map_err(|source| {
            AppError::from_io("resource.image_open_failed", source)
                .with_detail("path", source_path.display().to_string())
        })?
        .with_guessed_format()
        .map_err(|source| {
            AppError::from_io("resource.image_format_detection_failed", source)
                .with_detail("path", source_path.display().to_string())
        })?;
    reader.decode().map_err(|source| {
        AppError::new("resource.image_decode_failed", source.to_string())
            .with_detail("path", source_path.display().to_string())
    })
}

fn encode_jpeg(image: &DynamicImage, source_path: &Path) -> AppResult<Vec<u8>> {
    let mut encoded = Vec::new();
    let rgb = image.to_rgb8();
    let (width, height) = image.dimensions();
    let mut encoder = JpegEncoder::new_with_quality(&mut encoded, 90);
    encoder
        .encode(&rgb, width, height, image::ColorType::Rgb8.into())
        .map_err(|source| {
            AppError::new("resource.image_encode_failed", source.to_string())
                .with_detail("path", source_path.display().to_string())
        })?;
    Ok(encoded)
}
