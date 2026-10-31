//! Pictures: tone filters, spreads across two pages, and figure numbers.

use matenadar_model::block::Figure;
use matenadar_model::element::{Crop, Filter};
use matenadar_model::style::Master;
use matenadar_model::units::num;
use matenadar_model::{Length, Spread};

use super::Gen;
use super::typ::str_lit;

/// Filtered pictures, kept across layouts (the generator runs on every
/// edit): source path, its version, the filter and opacity → new bytes.
type Toned = ((String, u128, String), Vec<u8>, &'static str);

/// What the cache of filtered pictures may hold, in bytes.
const TONED_BYTES: usize = 192 << 20;

impl Gen<'_> {
    /// The filter a raster picture is drawn with: its own, or the book's
    /// (`print.filter`), unless that changes nothing.
    pub fn filter_for(&self, own: Option<&Filter>, path: &str) -> Option<Filter> {
        if path.to_lowercase().ends_with(".svg") {
            return None;
        }
        let f = own.or(self.doc.print.as_ref().and_then(|p| p.filter.as_ref()))?;
        (!f.is_identity()).then(|| f.clone())
    }

    /// `path` (an asset) with `filter` applied and its alpha scaled by
    /// `opacity`, as a file the compiler serves; `None` when it can't be read.
    pub fn toned(&mut self, path: &str, filter: &Filter, opacity: f64) -> Option<String> {
        static CACHE: std::sync::OnceLock<std::sync::Mutex<Vec<Toned>>> = std::sync::OnceLock::new();
        let spec = format!("{}|{}", serde_json::to_string(filter).unwrap_or_default(), (opacity * 1000.0).round());
        let key = (path.to_string(), self.assets.version(path)?, spec);
        let cache = CACHE.get_or_init(Default::default);
        if let Some((_, data, ext)) = cache.lock().ok()?.iter().find(|(k, _, _)| *k == key).cloned() {
            return Some(self.file(ext, data));
        }
        let bytes = self.assets.read(path)?;
        let (mut data, mut ext) = crate::tone::apply(&bytes, filter)?;
        if opacity < 1.0 {
            let mut img = image::load_from_memory(&data).ok()?.to_luma_alpha8();
            for px in img.pixels_mut() {
                px.0[1] = (px.0[1] as f64 * opacity).round() as u8;
            }
            data.clear();
            image::DynamicImage::ImageLumaA8(img).write_to(&mut std::io::Cursor::new(&mut data), image::ImageFormat::Png).ok()?;
            ext = "png";
        }
        if let Ok(mut c) = cache.lock() {
            c.push((key, data.clone(), ext));
            let mut total: usize = c.iter().map(|e| e.1.len()).sum();
            while total > TONED_BYTES && c.len() > 1 {
                total -= c.remove(0).1.len();
            }
        }
        Some(self.file(ext, data))
    }

    /// A picture's path as the compiler should draw it: the asset, or its
    /// filtered copy.
    pub fn picture(&mut self, src: &str, own: Option<&Filter>) -> Option<String> {
        let path = self.asset(src)?;
        match self.filter_for(own, &path) {
            Some(f) => Some(self.toned(&path, &f, 1.0).unwrap_or(path)),
            None => Some(path),
        }
    }

    /// One page of a spread: its part of the picture, from the bleed's
    /// corner. The verso shows the left of the picture, the recto the right;
    /// a side with its own picture shows all of that.
    pub fn spread_part(&mut self, s: &Spread, m: &Master, recto: bool) {
        let side = if recto { &s.right } else { &s.left };
        let pt = |l: Option<Length>, d: f64| l.and_then(|v| v.pt()).unwrap_or(d);
        let (pw, ph) = (pt(m.width, 432.0), pt(m.height, 648.0));
        let b = pt(m.bleed, 0.0);
        let own = side.src.is_some();
        let Some(src) = side.src.as_deref().or(s.src.as_deref()) else { return };
        let filter = side.filter.as_ref().or(s.filter.as_ref());
        let Some(path) = self.picture(src, filter) else { return };
        let crop = side.crop.or(if own { None } else { s.crop }).unwrap_or(Crop { x: 0.0, y: 0.0, w: 1.0, h: 1.0 });
        let (fx, fy) = side.focus.or(s.focus).unwrap_or((0.5, 0.5));
        // What the picture covers, and where this page's window starts in it.
        let half = b + pw;
        let (total, from) = if own {
            (half, 0.0)
        } else {
            let g = pt(s.gutter, 0.0).clamp(0.0, pw / 2.0);
            let total = 2.0 * (half - g);
            (total, if recto { total - half } else { 0.0 })
        };
        self.raw(&format!(
            "#place(top + left, dx: {}pt, dy: {}pt, mt-spread-part({p}, ({}, {}, {}, {}), ({}, {}), ({}pt, {}pt), {}pt, ({}pt, {}pt), (w, h) => image({p}, width: w, height: h, fit: \"stretch\")))\n",
            num(if recto { 0.0 } else { -b }),
            num(-b),
            num(crop.x),
            num(crop.y),
            num(crop.w),
            num(crop.h),
            num(fx.clamp(0.0, 1.0)),
            num(fy.clamp(0.0, 1.0)),
            num(total),
            num(ph + 2.0 * b),
            num(from),
            num(half),
            num(ph + 2.0 * b),
            p = str_lit(&path),
        ));
    }

    /// The number and label of a figure the design numbers:
    /// `("3.2", "Figure 3.2")`.
    pub fn figure_number(&self, f: &Figure) -> Option<(String, String)> {
        self.fig_numbers.get(&matenadar_model::figures::key(f)).cloned()
    }

    /// The marker the list of figures and references find a numbered
    /// figure by.
    pub fn figure_marker(&mut self, f: &Figure, number: &str, label: &str) {
        let caption = matenadar_model::figures::list_caption(f);
        self.raw(&format!(
            "#metadata((id: {}, number: {}, label: {}, caption: {})) <mt-fig>",
            str_lit(f.id.as_deref().unwrap_or("")),
            str_lit(number),
            str_lit(label),
            str_lit(caption.trim())
        ));
    }
}
