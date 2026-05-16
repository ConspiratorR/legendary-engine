use image::DynamicImage;

pub fn load_image(path: &str) -> Result<DynamicImage, String> {
    image::open(path).map_err(|e| format!("Failed to load image '{}': {}", path, e))
}

pub struct ImageData {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
}

pub enum PixelFormat {
    Rgba8,
}

impl ImageData {
    pub fn from_dynamic(img: &DynamicImage) -> Self {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        Self {
            pixels: rgba.into_raw(),
            width: w,
            height: h,
            format: PixelFormat::Rgba8,
        }
    }
}
