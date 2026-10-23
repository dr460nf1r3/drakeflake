use std::io::Cursor;
use std::path::{Path, PathBuf};

use tray_icon::Icon;

pub struct Icons {
    pub normal: Option<Icon>,
    pub update: Option<Icon>,
}

impl Icons {
    pub fn load() -> Self {
        let dir = icon_dir();
        let load = |name: &str| {
            let file = dir.as_deref()?.join(name);
            decode(&file)
                .inspect_err(|error| {
                    drakeflake_log::warn!("menu bar icon {}: {error}", file.display());
                })
                .ok()
        };

        Self {
            normal: load("drakeflake-tray.png"),
            update: load("drakeflake-tray-update.png"),
        }
    }
}

fn icon_dir() -> Option<PathBuf> {
    drakeflake_tray_core::icon_dir("share/drakeflake/tray").filter(|dir| dir.is_dir())
}

fn decode(file: &Path) -> Result<Icon, String> {
    let data = std::fs::read(file).map_err(|error| error.to_string())?;
    let mut decoder = png::Decoder::new(Cursor::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("the image is too large")?;

    let mut rgba = vec![0; size];
    let frame = reader
        .next_frame(&mut rgba)
        .map_err(|error| error.to_string())?;

    if frame.color_type != png::ColorType::Rgba || frame.bit_depth != png::BitDepth::Eight {
        return Err(format!(
            "expected 8-bit RGBA, found {}-bit {}",
            bits(frame.bit_depth),
            color_name(frame.color_type)
        ));
    }

    rgba.truncate(frame.buffer_size());

    Icon::from_rgba(rgba, frame.width, frame.height).map_err(|error| error.to_string())
}

const fn bits(depth: png::BitDepth) -> u8 {
    match depth {
        png::BitDepth::One => 1,
        png::BitDepth::Two => 2,
        png::BitDepth::Four => 4,
        png::BitDepth::Eight => 8,
        png::BitDepth::Sixteen => 16,
    }
}

const fn color_name(color: png::ColorType) -> &'static str {
    match color {
        png::ColorType::Grayscale => "grayscale",
        png::ColorType::Rgb => "RGB",
        png::ColorType::Indexed => "indexed",
        png::ColorType::GrayscaleAlpha => "grayscale with alpha",
        png::ColorType::Rgba => "RGBA",
    }
}
