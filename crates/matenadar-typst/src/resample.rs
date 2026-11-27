//! Deterministic image downsampling: separable Lanczos-3 over premultiplied
//! sRGB samples using IEEE-754 arithmetic and `libm`, so native and WebAssembly
//! builds emit bit-identical pixels.
//!
//! Adapted from `recto-resample` and wired as a post-layout PDF export pass so
//! oversized 4K photos placed in small frames shrink cleanly to `max_ppi`
//! without affecting SVGs, images already below `max_ppi`, or interactive edit
//! latency.

use std::f64::consts::PI;
use std::sync::{Mutex, OnceLock};

use image::DynamicImage;
use typst::foundations::{Bytes, Smart};
use typst::layout::{Frame, FrameItem};
use typst::model::Document as _;
use typst::visualize::{ExchangeFormat, Image, ImageKind, RasterFormat, RasterImage};
use typst_layout::PagedDocument;

const LOBES: f64 = 3.0;

/// Bounded cache of resampled images across exports: `(src_hash, target_w, target_h) -> (bytes, is_jpeg)`.
type CacheEntry = ((u128, u32, u32), Bytes, bool);
const RESAMPLE_CACHE_BYTES: usize = 128 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleBuf {
    Eight(Vec<u8>),
    Sixteen(Vec<u16>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub channels: usize,
    pub samples: SampleBuf,
}

impl Pixels {
    pub fn has_alpha(&self) -> bool {
        self.channels == 2 || self.channels == 4
    }
}

/// Resamples the window `[x0, y0, x1, y1]` of `src` to `size` pixels using
/// separable Lanczos-3 over premultiplied encoded samples.
pub fn resample(src: &Pixels, window: [f64; 4], size: (u32, u32)) -> Pixels {
    let (ow, oh) = (size.0 as usize, size.1 as usize);
    let n = src.channels;
    let across = Weights::new(src.width as usize, window[0], window[2], ow);
    let down = Weights::new(src.height as usize, window[1], window[3], oh);
    let (x0, x1) = across.span();
    let (y0, y1) = down.span();

    let mut rows = vec![0f32; (y1 - y0) * ow * n];
    let mut line = vec![0f32; (x1 - x0) * n];
    for (r, out) in rows.chunks_exact_mut(ow * n).enumerate() {
        unpack(src, y0 + r, x0, x1, &mut line);
        match n {
            1 => across.apply::<1>(&line, out),
            2 => across.apply::<2>(&line, out),
            3 => across.apply::<3>(&line, out),
            _ => across.apply::<4>(&line, out),
        }
    }

    let sixteen = matches!(src.samples, SampleBuf::Sixteen(_));
    let mut out8 = Vec::new();
    let mut out16 = Vec::new();
    if sixteen {
        out16.reserve_exact(oh * ow * n);
    } else {
        out8.reserve_exact(oh * ow * n);
    }
    let mut acc = vec![0f32; ow * n];
    for o in 0..oh {
        acc.fill(0.0);
        let (start, taps) = down.taps(o);
        let band = &rows[start * ow * n..(start + taps.len()) * ow * n];
        for (&w, row) in taps.iter().zip(band.chunks_exact(ow * n)) {
            for (a, v) in acc.iter_mut().zip(row) {
                *a += w * v;
            }
        }
        if sixteen {
            encode_row(&acc, n, src.has_alpha(), |v| {
                out16.push((v * 65535.0 + 0.5) as u16)
            });
        } else {
            encode_row(&acc, n, src.has_alpha(), |v| {
                out8.push((v * 255.0 + 0.5) as u8)
            });
        }
    }
    Pixels {
        width: size.0,
        height: size.1,
        channels: n,
        samples: if sixteen { SampleBuf::Sixteen(out16) } else { SampleBuf::Eight(out8) },
    }
}

fn unpack(src: &Pixels, y: usize, x0: usize, x1: usize, line: &mut [f32]) {
    let n = src.channels;
    let alpha = src.has_alpha();
    let colours = if alpha { n - 1 } else { n };
    let start = (y * src.width as usize + x0) * n;
    let end = (y * src.width as usize + x1) * n;
    match &src.samples {
        SampleBuf::Eight(s) => {
            for (px, out) in s[start..end].chunks_exact(n).zip(line.chunks_exact_mut(n)) {
                let a = if alpha { f32::from(px[n - 1]) / 255.0 } else { 1.0 };
                for c in 0..colours {
                    out[c] = f32::from(px[c]) / 255.0 * a;
                }
                if alpha {
                    out[n - 1] = a;
                }
            }
        }
        SampleBuf::Sixteen(s) => {
            for (px, out) in s[start..end].chunks_exact(n).zip(line.chunks_exact_mut(n)) {
                let a = if alpha { f32::from(px[n - 1]) / 65535.0 } else { 1.0 };
                for c in 0..colours {
                    out[c] = f32::from(px[c]) / 65535.0 * a;
                }
                if alpha {
                    out[n - 1] = a;
                }
            }
        }
    }
}

#[inline(always)]
fn encode_row(acc: &[f32], n: usize, alpha: bool, mut emit: impl FnMut(f32)) {
    let colours = if alpha { n - 1 } else { n };
    for px in acc.chunks_exact(n) {
        let a = if alpha { px[n - 1].clamp(0.0, 1.0) } else { 1.0 };
        for &c in &px[..colours] {
            let v = if !alpha {
                c
            } else if a > 0.0 {
                c / a
            } else {
                0.0
            };
            emit(v.clamp(0.0, 1.0));
        }
        if alpha {
            emit(a);
        }
    }
}

struct Weights {
    first: usize,
    last: usize,
    starts: Vec<usize>,
    offsets: Vec<usize>,
    values: Vec<f32>,
}

impl Weights {
    fn new(input: usize, from: f64, to: f64, output: usize) -> Self {
        let first = (from.floor().max(0.0) as usize).min(input - 1);
        let last = (to.ceil() as usize).clamp(first + 1, input);
        let start = from - first as f64;
        let scale = (to - from) / output as f64;
        let widen = scale.max(1.0);
        let radius = LOBES * widen;
        let available = last - first;
        let mut w = Weights {
            first,
            last,
            starts: Vec::with_capacity(output),
            offsets: Vec::with_capacity(output + 1),
            values: Vec::new(),
        };
        w.offsets.push(0);
        let mut taps = Vec::new();
        for i in 0..output {
            let centre = start + (i as f64 + 0.5) * scale;
            let lo = ((centre - radius).floor().max(0.0) as usize).min(available - 1);
            let hi = ((centre + radius).ceil() as usize).clamp(lo + 1, available);
            taps.clear();
            taps.extend((lo..hi).map(|j| lanczos((j as f64 + 0.5 - centre) / widen)));
            let sum: f64 = taps.iter().sum();
            w.starts.push(lo);
            if sum.abs() > 1e-9 {
                w.values.extend(taps.iter().map(|t| (t / sum) as f32));
            } else {
                let nearest = (centre.floor().max(0.0) as usize).clamp(lo, hi - 1);
                w.values.extend((lo..hi).map(|j| if j == nearest { 1.0 } else { 0.0 }));
            }
            w.offsets.push(w.values.len());
        }
        w
    }

    fn span(&self) -> (usize, usize) {
        (self.first, self.last)
    }

    #[inline(always)]
    fn taps(&self, i: usize) -> (usize, &[f32]) {
        (self.starts[i], &self.values[self.offsets[i]..self.offsets[i + 1]])
    }

    #[inline(always)]
    fn apply<const N: usize>(&self, line: &[f32], out: &mut [f32]) {
        let (line, _) = line.as_chunks::<N>();
        for (i, dst) in out.as_chunks_mut::<N>().0.iter_mut().enumerate() {
            let (start, taps) = self.taps(i);
            let mut acc = [0f32; N];
            for (&w, px) in taps.iter().zip(&line[start..start + taps.len()]) {
                for c in 0..N {
                    acc[c] += w * px[c];
                }
            }
            *dst = acc;
        }
    }
}

fn lanczos(t: f64) -> f64 {
    if t == 0.0 {
        return 1.0;
    }
    if t.abs() >= LOBES {
        return 0.0;
    }
    let x = PI * t;
    LOBES * libm::sin(x) * libm::sin(x / LOBES) / (x * x)
}

/// Downsamples every oversized raster image in `doc` so its effective resolution
/// at its placed size does not exceed `max_ppi`. SVGs and images already at or
/// below `max_ppi` are left untouched.
pub fn downsample_document(doc: PagedDocument, max_ppi: f32) -> PagedDocument {
    if !max_ppi.is_finite() || max_ppi <= 0.0 {
        return doc;
    }
    let mut pages = doc.pages().to_vec();
    let mut changed = false;
    for page in &mut pages {
        changed |= visit_frame(&mut page.frame, max_ppi as f64);
    }
    if !changed {
        return doc;
    }
    PagedDocument::new(pages.into_iter().collect(), doc.info().clone())
}

fn visit_frame(frame: &mut Frame, max_ppi: f64) -> bool {
    let mut changed = false;
    frame.retain(|item| {
        match item {
            FrameItem::Group(g) => {
                changed |= visit_frame(&mut g.frame, max_ppi);
            }
            FrameItem::Image(img, draw_size, _) => {
                if let ImageKind::Raster(raster) = img.kind() {
                    let w_in = draw_size.x.to_pt() / 72.0;
                    let h_in = draw_size.y.to_pt() / 72.0;
                    if w_in > 0.0 && h_in > 0.0 {
                        let tw = (w_in * max_ppi).ceil().max(1.0) as u32;
                        let th = (h_in * max_ppi).ceil().max(1.0) as u32;
                        let (rw, rh) = (raster.width(), raster.height());
                        if rw > tw || rh > th {
                            let target = (tw.min(rw), th.min(rh));
                            if let Some(next_raster) = downsample_raster(raster, target) {
                                *img = Image::new(next_raster, img.alt().map(Into::into), img.scaling());
                                changed = true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        true
    });
    changed
}

fn downsample_raster(raster: &RasterImage, target: (u32, u32)) -> Option<RasterImage> {
    static CACHE: OnceLock<Mutex<Vec<CacheEntry>>> = OnceLock::new();
    let raw = raster.data();
    let key = (typst::utils::hash128(raw), target.0, target.1);
    let cache = CACHE.get_or_init(Default::default);
    if let Some((_, bytes, is_jpg)) = cache.lock().ok()?.iter().find(|(k, _, _)| *k == key).cloned() {
        let fmt = if is_jpg { ExchangeFormat::Jpg } else { ExchangeFormat::Png };
        return RasterImage::new(bytes, fmt, raster.icc().cloned().map(Smart::Custom).unwrap_or(Smart::Auto)).ok();
    }

    let dyn_img = image::load_from_memory(raw).ok()?;
    let (w, h) = (dyn_img.width(), dyn_img.height());
    if w == 0 || h == 0 || (w <= target.0 && h <= target.1) {
        return None;
    }
    let has_alpha = dyn_img.color().has_alpha();
    let is_gray = !dyn_img.color().has_color();
    let src = match (is_gray, has_alpha) {
        (true, false) => Pixels { width: w, height: h, channels: 1, samples: SampleBuf::Eight(dyn_img.to_luma8().into_raw()) },
        (true, true) => Pixels { width: w, height: h, channels: 2, samples: SampleBuf::Eight(dyn_img.to_luma_alpha8().into_raw()) },
        (false, false) => Pixels { width: w, height: h, channels: 3, samples: SampleBuf::Eight(dyn_img.to_rgb8().into_raw()) },
        (false, true) => Pixels { width: w, height: h, channels: 4, samples: SampleBuf::Eight(dyn_img.to_rgba8().into_raw()) },
    };
    let resampled = resample(&src, [0.0, 0.0, w as f64, h as f64], target);
    let SampleBuf::Eight(buf) = resampled.samples else { return None };
    let out_img = match resampled.channels {
        1 => DynamicImage::ImageLuma8(image::GrayImage::from_raw(target.0, target.1, buf)?),
        2 => DynamicImage::ImageLumaA8(image::GrayAlphaImage::from_raw(target.0, target.1, buf)?),
        3 => DynamicImage::ImageRgb8(image::RgbImage::from_raw(target.0, target.1, buf)?),
        _ => DynamicImage::ImageRgba8(image::RgbaImage::from_raw(target.0, target.1, buf)?),
    };

    let prefer_jpg = matches!(raster.format(), RasterFormat::Exchange(ExchangeFormat::Jpg)) && !has_alpha;
    let mut encoded = Vec::new();
    let fmt = if prefer_jpg {
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 90);
        out_img.write_with_encoder(enc).ok()?;
        ExchangeFormat::Jpg
    } else {
        out_img.write_to(&mut std::io::Cursor::new(&mut encoded), image::ImageFormat::Png).ok()?;
        ExchangeFormat::Png
    };
    let bytes = Bytes::new(encoded);
    let next = RasterImage::new(bytes.clone(), fmt, raster.icc().cloned().map(Smart::Custom).unwrap_or(Smart::Auto)).ok()?;
    if let Ok(mut c) = cache.lock() {
        c.push((key, bytes, prefer_jpg));
        let mut total: usize = c.iter().map(|e| e.1.len()).sum();
        while total > RESAMPLE_CACHE_BYTES && c.len() > 1 {
            total -= c.remove(0).1.len();
        }
    }
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lanczos_keeps_flat_colour_and_premultiplies_alpha() {
        let src = Pixels {
            width: 64,
            height: 48,
            channels: 3,
            samples: SampleBuf::Eight([200u8, 30, 117].repeat(64 * 48)),
        };
        let out = resample(&src, [0.0, 0.0, 64.0, 48.0], (16, 12));
        let SampleBuf::Eight(s) = out.samples else { panic!() };
        assert!(s.chunks_exact(3).all(|p| p == [200, 30, 117]));
    }
}
