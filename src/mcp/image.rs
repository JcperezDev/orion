use anyhow::Result;
use std::path::Path;
use image::{DynamicImage, ImageFormat, GenericImageView};
use std::io::Cursor;

#[allow(dead_code)]
pub struct ImageProcessor {
    max_dimension: u32,
    quality: u8,
}

#[allow(dead_code)]
impl ImageProcessor {
    pub fn new() -> Self {
        Self {
            max_dimension: 1024,
            quality: 85,
        }
    }

    pub fn process(&self, path: &Path) -> Result<String> {
        let img = image::open(path)?;
        let resized = self.resize_if_needed(img);
        let base64 = self.encode_base64(&resized)?;
        Ok(base64)
    }

    fn resize_if_needed(&self, img: DynamicImage) -> DynamicImage {
        let (width, height) = img.dimensions();
        if width > self.max_dimension || height > self.max_dimension {
            img.resize(self.max_dimension, self.max_dimension, image::imageops::FilterType::Lanczos3)
        } else {
            img
        }
    }

    fn encode_base64(&self, img: &DynamicImage) -> Result<String> {
        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, ImageFormat::Png)?;
        let base64_str = base64_encode(buffer.into_inner());
        Ok(base64_str)
    }

    pub fn detect_format(&self, path: &Path) -> Option<&'static str> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "png" => Some("image/png"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            "gif" => Some("image/gif"),
            "webp" => Some("image/webp"),
            "bmp" => Some("image/bmp"),
            _ => None,
        }
    }
}

impl Default for ImageProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
fn base64_encode(data: Vec<u8>) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}
