use anyhow::Result;
use image::{DynamicImage, ImageFormat, GenericImageView};
use std::io::Cursor;
use std::path::Path;

#[allow(dead_code)]
pub struct ImageEncoder {
    max_size: u32,
    quality: u8,
}

#[allow(dead_code)]
impl ImageEncoder {
    pub fn new() -> Self {
        Self {
            max_size: 1024,
            quality: 85,
        }
    }

    pub fn encode_file(&self, path: &Path) -> Result<String> {
        let img = image::open(path)?;
        let resized = self.resize(img);
        self.to_base64(&resized)
    }

    pub fn encode_bytes(&self, data: &[u8]) -> Result<String> {
        let img = image::load_from_memory(data)?;
        let resized = self.resize(img);
        self.to_base64(&resized)
    }

    fn resize(&self, img: DynamicImage) -> DynamicImage {
        let (w, h) = img.dimensions();
        let max = self.max_size;

        if w > max || h > max {
            let ratio = (max as f32 / w.max(h) as f32).min(1.0);
            let new_w = (w as f32 * ratio) as u32;
            let new_h = (h as f32 * ratio) as u32;
            img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
        } else {
            img
        }
    }

    fn to_base64(&self, img: &DynamicImage) -> Result<String> {
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Png)?;
        Ok(base64::encode(buf.into_inner()))
    }

    pub fn get_dimensions(&self, path: &Path) -> Result<(u32, u32)> {
        let img = image::open(path)?;
        Ok(img.dimensions())
    }
}

impl Default for ImageEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
mod base64 {
    pub fn encode(data: Vec<u8>) -> String {
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
}
