//! Tones of raster pictures: the grey filter for black-ink interiors, and
//! how much colour a picture holds (for the black-ink check).

use image::{DynamicImage, GenericImageView};
use matenadar_model::element::Filter;

/// The filter's tone curve as a lookup table: input levels, then gamma,
/// then a sigmoid about the middle grey that keeps black black and white
/// white (as ImageMagick's `-sigmoidal-contrast` does).
pub fn curve(f: &Filter) -> [u8; 256] {
    let (b, w) = (f.black.unwrap_or(0.0), f.white.unwrap_or(1.0));
    let gamma = f.gamma.unwrap_or(1.0);
    let c = f.contrast.unwrap_or(0.0);
    let sig = |x: f64| 1.0 / (1.0 + (-c * (x - 0.5)).exp());
    let (s0, s1) = (sig(0.0), sig(1.0));
    let mut lut = [0u8; 256];
    for (i, v) in lut.iter_mut().enumerate() {
        let mut t = ((i as f64 / 255.0 - b) / (w - b).max(1e-6)).clamp(0.0, 1.0);
        if gamma != 1.0 {
            t = t.powf(1.0 / gamma);
        }
        if c > 1e-6 {
            t = (sig(t) - s0) / (s1 - s0);
        }
        *v = (t * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    lut
}

/// Rec. 709 luma of gamma-encoded RGB.
fn luma(r: u8, g: u8, b: u8) -> u8 {
    (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64).round().clamp(0.0, 255.0) as u8
}

/// The picture in `data` (PNG, JPEG, GIF or WebP) with the filter applied:
/// its new bytes and their extension. Photographs stay JPEG; anything
/// else becomes PNG (with its transparency kept).
pub fn apply(data: &[u8], f: &Filter) -> Option<(Vec<u8>, &'static str)> {
    let format = image::guess_format(data).ok()?;
    let img = image::load_from_memory(data).ok()?;
    let lut = curve(f);
    let alpha = img.color().has_alpha();
    let out: DynamicImage = match (f.gray, alpha) {
        (true, false) => {
            let rgb = img.to_rgb8();
            let mut gray = image::GrayImage::new(rgb.width(), rgb.height());
            for (o, p) in gray.pixels_mut().zip(rgb.pixels()) {
                o.0[0] = lut[luma(p.0[0], p.0[1], p.0[2]) as usize];
            }
            DynamicImage::ImageLuma8(gray)
        }
        (true, true) => {
            let rgba = img.to_rgba8();
            let mut gray = image::GrayAlphaImage::new(rgba.width(), rgba.height());
            for (o, p) in gray.pixels_mut().zip(rgba.pixels()) {
                o.0 = [lut[luma(p.0[0], p.0[1], p.0[2]) as usize], p.0[3]];
            }
            DynamicImage::ImageLumaA8(gray)
        }
        (false, _) => {
            let mut rgba = img.to_rgba8();
            for p in rgba.pixels_mut() {
                for c in &mut p.0[..3] {
                    *c = lut[*c as usize];
                }
            }
            if alpha { DynamicImage::ImageRgba8(rgba) } else { DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(rgba).to_rgb8()) }
        }
    };
    let mut bytes = Vec::new();
    if format == image::ImageFormat::Jpeg && !alpha {
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 92);
        out.write_with_encoder(enc).ok()?;
        Some((bytes, "jpg"))
    } else {
        out.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png).ok()?;
        Some((bytes, "png"))
    }
}

/// How much colour a picture holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colour {
    /// The share of the picture's visible pixels that are clearly coloured.
    pub share: f64,
    /// The mean chroma of the visible pixels (0–255).
    pub mean: f64,
}

impl Colour {
    /// Coloured enough to matter on a black-ink page: more than a few
    /// pixels of clear colour, or an overall cast (sepia). JPEG noise in a
    /// grey photograph stays below both.
    pub fn is_colour(&self) -> bool {
        self.share > 0.002 || self.mean > 6.0
    }
}

/// Chroma (max − min of R, G, B) above this is clearly a colour.
const CLEAR: u8 = 24;

/// How much colour the pixels of `img` hold, from a sample of at most
/// about 250,000 of them. Fully transparent pixels don't count.
pub fn colour(img: &DynamicImage) -> Colour {
    if !img.color().has_color() {
        return Colour { share: 0.0, mean: 0.0 };
    }
    let (w, h) = img.dimensions();
    let step = ((w as f64 * h as f64 / 250_000.0).sqrt().ceil() as u32).max(1);
    let (mut n, mut coloured, mut sum) = (0u64, 0u64, 0u64);
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let p = img.get_pixel(x, y).0;
            if p[3] > 0 {
                let (hi, lo) = (p[0].max(p[1]).max(p[2]), p[0].min(p[1]).min(p[2]));
                let chroma = hi - lo;
                n += 1;
                sum += chroma as u64;
                if chroma > CLEAR {
                    coloured += 1;
                }
            }
            x += step;
        }
        y += step;
    }
    if n == 0 {
        return Colour { share: 0.0, mean: 0.0 };
    }
    Colour { share: coloured as f64 / n as f64, mean: sum as f64 / n as f64 }
}

/// Whether a flat colour (red, green, blue in 0–1) is a colour rather than
/// a grey.
pub fn is_chromatic(r: f32, g: f32, b: f32) -> bool {
    let (hi, lo) = (r.max(g).max(b), r.min(g).min(b));
    hi - lo > 0.02
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identity_curve_keeps_every_tone() {
        let lut = curve(&Filter::default());
        assert!(lut.iter().enumerate().all(|(i, v)| *v as usize == i));
    }

    #[test]
    fn contrast_steepens_the_middle_and_keeps_the_ends() {
        let lut = curve(&Filter { gray: true, contrast: Some(5.0), ..Filter::default() });
        assert_eq!((lut[0], lut[255]), (0, 255));
        assert!(lut[64] < 64 && lut[192] > 192, "{} {}", lut[64], lut[192]);
        assert!((lut[128] as i32 - 128).abs() <= 1);
    }

    #[test]
    fn levels_clip_to_black_and_white() {
        let lut = curve(&Filter { black: Some(0.1), white: Some(0.9), ..Filter::default() });
        assert_eq!(lut[20], 0);
        assert_eq!(lut[240], 255);
    }

    #[test]
    fn a_colour_picture_turns_grey_and_measures_as_grey() {
        let mut img = image::RgbImage::new(40, 20);
        for (x, _, p) in img.enumerate_pixels_mut() {
            p.0 = if x < 20 { [200, 30, 30] } else { [30, 160, 220] };
        }
        let mut png = Vec::new();
        DynamicImage::ImageRgb8(img.clone()).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        assert!(colour(&DynamicImage::ImageRgb8(img)).is_colour());
        let (out, ext) = apply(&png, &Filter { gray: true, ..Filter::default() }).unwrap();
        assert_eq!(ext, "png");
        let back = image::load_from_memory(&out).unwrap();
        assert!(!back.color().has_color());
        assert!(!colour(&back).is_colour());
        // Rec. 709: the red half is darker than the blue one.
        let g = back.to_luma8();
        assert!(g.get_pixel(5, 5).0[0] < g.get_pixel(30, 5).0[0]);
    }
}
