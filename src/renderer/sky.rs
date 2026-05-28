use half::f16;

#[derive(Debug)]
pub enum SkyError {
    Fetch(String),
    Decode(String),
}

impl std::fmt::Display for SkyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkyError::Fetch(message) => write!(f, "récupération du fond: {message}"),
            SkyError::Decode(message) => write!(f, "décodage du fond: {message}"),
        }
    }
}

impl std::error::Error for SkyError {}

pub struct SkyImage {
    pub width: u32,
    pub height: u32,
    pub rgba_f16: Vec<u16>,
}

impl SkyImage {
    pub fn fallback() -> Self {
        let dark = f16::from_f32(0.02).to_bits();
        let opaque = f16::from_f32(1.0).to_bits();
        Self {
            width: 1,
            height: 1,
            rgba_f16: vec![dark, dark, dark, opaque],
        }
    }

    pub fn bytes_per_row(&self) -> u32 {
        self.width * 4 * std::mem::size_of::<u16>() as u32
    }
}

pub async fn load(source: &str) -> Result<SkyImage, SkyError> {
    let bytes = load_bytes(source).await?;
    decode(&bytes)
}

fn decode(bytes: &[u8]) -> Result<SkyImage, SkyError> {
    let format = image::guess_format(bytes).map_err(|e| SkyError::Decode(e.to_string()))?;
    let decoded = image::load_from_memory_with_format(bytes, format)
        .map_err(|e| SkyError::Decode(e.to_string()))?;

    let rgba = decoded.to_rgba32f();
    let (width, height) = rgba.dimensions();
    let already_linear = format == image::ImageFormat::Hdr;

    Ok(SkyImage {
        width,
        height,
        rgba_f16: to_rgba_f16(rgba.as_raw(), already_linear),
    })
}

fn to_rgba_f16(rgba32: &[f32], already_linear: bool) -> Vec<u16> {
    let opaque = f16::from_f32(1.0).to_bits();
    let mut out = Vec::with_capacity(rgba32.len());
    for pixel in rgba32.chunks_exact(4) {
        out.push(encode_channel(pixel[0], already_linear));
        out.push(encode_channel(pixel[1], already_linear));
        out.push(encode_channel(pixel[2], already_linear));
        out.push(opaque);
    }
    out
}

fn encode_channel(value: f32, already_linear: bool) -> u16 {
    let linear = if already_linear {
        value
    } else {
        srgb_to_linear(value)
    };
    f16::from_f32(linear).to_bits()
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn load_bytes(path: &str) -> Result<Vec<u8>, SkyError> {
    std::fs::read(path).map_err(|e| SkyError::Fetch(e.to_string()))
}

#[cfg(target_arch = "wasm32")]
async fn load_bytes(url: &str) -> Result<Vec<u8>, SkyError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let to_err = |label: &'static str| move |_| SkyError::Fetch(label.to_string());

    let window = web_sys::window().ok_or_else(|| SkyError::Fetch("aucune window".into()))?;
    let response_value = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(to_err("fetch échoué"))?;
    let response: web_sys::Response = response_value
        .dyn_into()
        .map_err(to_err("réponse invalide"))?;
    if !response.ok() {
        return Err(SkyError::Fetch(format!("HTTP {}", response.status())));
    }

    let buffer = JsFuture::from(response.array_buffer().map_err(to_err("array_buffer"))?)
        .await
        .map_err(to_err("lecture du corps échouée"))?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}
