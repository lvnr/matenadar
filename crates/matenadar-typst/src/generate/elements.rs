//! Elements: things placed at a position.

use matenadar_model::Length;
use matenadar_model::block::{Block, Inline, Para, Run, content_of};
use matenadar_model::element::{Clip, Element, GridCell, GridRow, LetterGrid, Lettering, Origin, Place, TextFrame};
use matenadar_model::style::{Stroke, TextStyle};
use matenadar_model::units::num;

use super::Gen;
use super::typ::{anchor, insets, insets_pt, radius, str_lit, stroke_outset, when};
use crate::effects::Effect;
use crate::glyphs::{LetterSpec, letter, pick, qr_runs, svg_group, tint_svg, weight_number};

impl Gen<'_> {
    pub fn element(&mut self, e: &Element, scaled: bool) {
        let start = self.out.len();
        if let Element::Line(l) = e {
            let p = |v: &Option<Length>| v.map(|l| l.typst()).unwrap_or_else(|| "0pt".into());
            let stroke = l.stroke.as_ref().map(|s| self.pal.stroke(s, scaled)).unwrap_or_else(|| "1pt".into());
            let body = format!(
                "place(top + left, line(start: ({}, {}), end: ({}, {}), stroke: {stroke}))",
                p(&l.x1),
                p(&l.y1),
                p(&l.x2),
                p(&l.y2)
            );
            self.wrap_when(l.when.as_deref(), &body);
            self.placed_node(l.id.as_deref(), "line", start);
            return;
        }
        let place = e.place().cloned().unwrap_or_default();
        let x = place.x.map(|v| v.typst()).unwrap_or_else(|| "0pt".into());
        let y = place.y.map(|v| v.typst()).unwrap_or_else(|| "0pt".into());
        let (fx, fy) = anchor(place.anchor.as_deref());
        let rot = place.rotate.unwrap_or(0.0);

        // The element's body as a Typst expression (code mode).
        let body = match e {
            Element::Text(t) => {
                if let Some(id) = place.id.as_deref().filter(|id| self.labels && super::label_safe(id)) {
                    self.raw(&format!("#place(top + left, dx: {x}, dy: {y}, [#metadata({}) <{id}>])\n", str_lit(id)));
                }
                self.text_frame(t, &place, &x, &y, fx, fy, rot, scaled);
                self.placed_node(place.id.as_deref(), if place.decorative == Some(true) { "decorative" } else { "text" }, start);
                return;
            }
            Element::Image(i) => {
                let mut args = Vec::new();
                if let Some(w) = &place.w {
                    args.push(format!("width: {}", w.typst()));
                }
                if let Some(h) = &place.h {
                    args.push(format!("height: {}", h.typst()));
                }
                args.push(format!("fit: {}", str_lit(i.fit.as_deref().unwrap_or("cover"))));
                let asset = self.asset(&i.src);
                let op = place.opacity.unwrap_or(1.0) * self.opacity;
                // Faded, the picture is drawn from a copy with its alpha scaled.
                let shown = match &asset {
                    Some(p) if op < 1.0 => self.faded(p, op).or_else(|| Some(p.clone())),
                    other => other.clone(),
                };
                let focused = match (&asset, &shown, i.focus, place.w.and_then(|w| w.pt()), place.h.and_then(|h| h.pt())) {
                    (Some(orig), Some(p), Some((fx, fy)), Some(bw), Some(bh)) if i.fit.as_deref().unwrap_or("cover") == "cover" => {
                        self.assets.read(orig).and_then(|d| crate::glyphs::image_size(&d)).map(|(iw, ih)| {
                            let scale = (bw / iw as f64).max(bh / ih as f64);
                            let (sw, sh) = (iw as f64 * scale, ih as f64 * scale);
                            format!(
                                "box(width: {}pt, height: {}pt, clip: true, place(top + left, dx: {}pt, dy: {}pt, image({}, width: {}pt, height: {}pt)))",
                                num(bw),
                                num(bh),
                                num(-(sw - bw) * fx.clamp(0.0, 1.0)),
                                num(-(sh - bh) * fy.clamp(0.0, 1.0)),
                                str_lit(p),
                                num(sw),
                                num(sh)
                            )
                        })
                    }
                    _ => None,
                };
                let cropped = match (i.crop, &shown) {
                    (Some(c), Some(p)) => Some(crop_call(
                        p,
                        &c,
                        place.w.map(|w| w.typst()),
                        place.h.map(|h| h.typst()),
                        i.fit.as_deref().unwrap_or("cover"),
                        i.focus,
                    )),
                    _ => None,
                };
                let mut body = match (cropped, focused, shown) {
                    (Some(c), _, _) => c,
                    (None, Some(f), _) => f,
                    (None, None, Some(p)) => format!("image({}, {})", str_lit(&p), args.join(", ")),
                    (None, None, None) => format!(
                        "rect({}, fill: luma(88%), stroke: 0.5pt + red)",
                        args.iter().filter(|a| !a.starts_with("fit")).cloned().collect::<Vec<_>>().join(", ")
                    ),
                };
                match i.flip.as_deref() {
                    Some("x") | Some("horizontal") => body = format!("scale(x: -100%, reflow: false, {body})"),
                    Some("y") | Some("vertical") => body = format!("scale(y: -100%, reflow: false, {body})"),
                    Some("both") => body = format!("scale(x: -100%, y: -100%, reflow: false, {body})"),
                    _ => {}
                }
                if i.radius.is_some() || i.stroke.is_some() {
                    body = format!(
                        "box(clip: true{}{}, {body})",
                        i.radius.as_ref().map(|r| format!(", radius: {}", radius(r, scaled))).unwrap_or_default(),
                        i.stroke.as_ref().map(|s| format!(", stroke: {}", self.pal.border(s, scaled))).unwrap_or_default()
                    );
                }
                body
            }
            Element::Rect(s) | Element::Ellipse(s) => {
                let shape = if matches!(e, Element::Rect(_)) { "rect" } else { "ellipse" };
                let mut args = vec![
                    format!("width: {}", place.w.map(|w| w.typst()).unwrap_or_else(|| "0pt".into())),
                    format!("height: {}", place.h.map(|h| h.typst()).unwrap_or_else(|| "0pt".into())),
                ];
                args.push(format!("fill: {}", s.fill.as_ref().map(|f| self.pal.fill(f)).unwrap_or_else(|| "none".into())));
                args.push(format!("stroke: {}", s.stroke.as_ref().map(|b| self.pal.border(b, scaled)).unwrap_or_else(|| "none".into())));
                if let Some(o) = s.stroke.as_ref().and_then(|b| stroke_outset(b, place.stroke_align.as_deref(), scaled)) {
                    args.push(format!("outset: {o}"));
                }
                if let (Some(r), "rect") = (&s.radius, shape) {
                    args.push(format!("radius: {}", radius(r, scaled)));
                }
                format!("{shape}({})", args.join(", "))
            }
            Element::Path(p) => {
                let (w, h) = (pt_of(place.w), pt_of(place.h));
                let vb = p.viewbox.map(|v| format!("{} {} {} {}", v[0], v[1], v[2], v[3])).unwrap_or_else(|| format!("0 0 {} {}", num(w), num(h)));
                let fill = p.fill.as_ref().map(|c| self.pal.svg(c)).unwrap_or_else(|| "none".into());
                let stroke = svg_stroke(self, p.stroke.as_ref());
                let op = place.opacity.unwrap_or(1.0) * self.opacity;
                let opacity = if op < 1.0 { format!(r#" opacity="{}""#, num(op)) } else { String::new() };
                let svg = format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="{vb}" preserveAspectRatio="none"><path d="{d}" fill="{fill}"{stroke}{opacity}/></svg>"#,
                    w = num(w),
                    h = num(h),
                    d = xml_escape(&p.d)
                );
                let path = self.file("svg", svg.into_bytes());
                format!("image({}, width: {}pt, height: {}pt)", str_lit(&path), num(w), num(h))
            }
            Element::Svg(s) => {
                let op = place.opacity.unwrap_or(1.0) * self.opacity;
                let changed = s.viewbox.is_some() || s.tint.is_some() || op < 1.0;
                let path = if changed {
                    let text = match (&s.src, &s.markup) {
                        (Some(src), _) => self.asset(src).and_then(|p| String::from_utf8(self.assets.read(&p)?.to_vec()).ok()),
                        (None, Some(m)) => Some(m.clone()),
                        _ => None,
                    };
                    text.map(|mut t| {
                        if let Some(vb) = &s.viewbox {
                            t = with_viewbox(&t, vb);
                        }
                        if let Some(tint) = &s.tint {
                            t = tint_svg(&t, &self.pal.svg(&tint.color), tint.amount);
                        }
                        if op < 1.0 {
                            t = svg_group(&t, &format!(r#"opacity="{}""#, num(op)));
                        }
                        self.file("svg", t.into_bytes())
                    })
                } else {
                    match (&s.src, &s.markup) {
                        (Some(src), _) => self.asset(src),
                        (None, Some(m)) => Some(self.file("svg", m.clone().into_bytes())),
                        _ => None,
                    }
                };
                let mut args = Vec::new();
                if let Some(w) = &place.w {
                    args.push(format!("width: {}", w.typst()));
                }
                if let Some(h) = &place.h {
                    args.push(format!("height: {}", h.typst()));
                }
                args.push(format!("fit: {}", str_lit(s.fit.as_deref().unwrap_or("contain"))));
                match path {
                    Some(p) => format!("image({}, {})", str_lit(&p), args.join(", ")),
                    None => "none".into(),
                }
            }
            Element::Group(g) => {
                let mut args = Vec::new();
                if let Some(w) = &place.w {
                    args.push(format!("width: {}", w.typst()));
                }
                if let Some(h) = &place.h {
                    args.push(format!("height: {}", h.typst()));
                }
                if g.clip.as_ref().is_some_and(Clip::is_box) {
                    args.push("clip: true".into());
                }
                if let Some(f) = &g.fill {
                    args.push(format!("fill: {}", self.pal.fill(f)));
                }
                if let Some(s) = &g.stroke {
                    args.push(format!("stroke: {}", self.pal.border(s, scaled)));
                    if let Some(o) = stroke_outset(s, place.stroke_align.as_deref(), scaled) {
                        args.push(format!("outset: {o}"));
                    }
                }
                if let Some(r) = &g.radius {
                    args.push(format!("radius: {}", radius(r, scaled)));
                }
                // Drawings inside fade themselves; text and shapes fade after layout.
                let outer = self.opacity;
                self.opacity *= place.opacity.unwrap_or(1.0);
                let inner = self.capture(|g2| {
                    for c in &g.children {
                        g2.element(c, scaled);
                    }
                });
                self.opacity = outer;
                let mut body = format!("box({})[{inner}]", args.join(", "));
                if let Some(s) = g.scale.filter(|s| *s != 1.0) {
                    body = format!("scale({}%, origin: top + left, reflow: false, {body})", num(s * 100.0));
                }
                body
            }
            Element::Use(u) => {
                let comp = self.doc.design.components.get(&u.component).cloned();
                match comp.map(|c| c.expand(&u.component, &u.props)) {
                    Some(Ok(x)) if self.depth < 8 => {
                        self.depth += 1;
                        let w = place.w.or(x.w).map(|w| format!("width: {}", w.typst())).unwrap_or_default();
                        let h = place.h.or(x.h).map(|h| format!(", height: {}", h.typst())).unwrap_or_default();
                        let outer = self.opacity;
                        self.opacity *= place.opacity.unwrap_or(1.0);
                        let inner = self.capture(|g2| {
                            if x.blocks.is_empty() {
                                for c in &x.elements {
                                    g2.element(c, scaled);
                                }
                            } else {
                                g2.flow(&x.blocks, "mt-base", scaled);
                            }
                        });
                        self.depth -= 1;
                        self.opacity = outer;
                        format!("box({w}{h})[{inner}]")
                    }
                    Some(Err(err)) => {
                        self.problem(err);
                        "none".into()
                    }
                    _ => {
                        self.problem(format!("component \"{}\" is missing or nested too deeply", u.component));
                        "none".into()
                    }
                }
            }
            Element::Qr(q) => {
                let dark = q.color.as_ref().map(|c| self.pal.color(c)).unwrap_or_else(|| "black".into());
                let light = q.background.as_ref().map(|c| self.pal.color(c)).unwrap_or_else(|| "none".into());
                match qr_runs(&q.data, q.ecc.as_deref().unwrap_or("M"), q.quiet.unwrap_or(0)) {
                    Ok((runs, n)) => {
                        // Drawn as one vector shape, so every renderer keeps the modules square.
                        let w = place.w.map(|w| w.typst()).unwrap_or_else(|| "1in".into());
                        let runs: Vec<String> = runs.iter().map(|(x, y, l)| format!("({x},{y},{l})")).collect();
                        format!("mt-qr({w}, {n}, ({},), {dark}, {light})", runs.join(","))
                    }
                    Err(err) => {
                        self.problem(err);
                        "none".into()
                    }
                }
            }
            Element::Lettering(l) => self.lettering(l, &place),
            Element::LetterGrid(g) => self.letter_grid(g, &place, scaled),
            Element::Line(_) => unreachable!(),
        };
        let mut body = body;
        // Text and shapes fade, and decorative text becomes outlines, after
        // layout; drawings have faded themselves above.
        let op = place.opacity.unwrap_or(1.0) as f32;
        let decorative = place.decorative == Some(true);
        let drawing = matches!(e, Element::Svg(_) | Element::Path(_) | Element::Lettering(_) | Element::Image(_));
        let clip = match e {
            Element::Group(g) => g.clip.as_ref().and_then(Clip::shape),
            Element::Image(i) => i.clip.as_ref().and_then(Clip::shape),
            _ => None,
        };
        let clip = clip.and_then(|c| self.clip_curve(c, &place));
        if (!drawing && (op < 1.0 || decorative)) || clip.is_some() {
            // Drawings have faded themselves; the box only clips them.
            let mut effect = if drawing { Effect::new(1.0, false) } else { Effect::new(op, decorative) };
            effect.clip = clip;
            body = self.with_effect(&body, effect);
        }
        body = self.rotated(&place, rot, body);
        if let Some(skew) = self.skew_of(&place) {
            body = format!("skew(ax: {}deg, reflow: false, {body})", num(skew));
        }
        if let Some(sh) = &place.shadow
            && let (Some(w), Some(h)) = (place.w, place.h)
        {
            let r = match e {
                Element::Rect(s) => s.radius.as_ref().map(|r| radius(r, scaled)),
                Element::Image(i) => i.radius.as_ref().map(|r| radius(r, scaled)),
                Element::Group(g) => g.radius.as_ref().map(|r| radius(r, scaled)),
                _ => None,
            };
            let shape = if matches!(e, Element::Ellipse(_)) { "ellipse" } else { "rect" };
            let mut shadow = format!(
                "{shape}(width: {}, height: {}, fill: {}{})",
                w.typst(),
                h.typst(),
                self.pal.color(&sh.color),
                r.map(|r| format!(", radius: {r}")).filter(|_| shape == "rect").unwrap_or_default()
            );
            shadow = self.rotated(&place, rot, shadow);
            let code = format!("mt-at({x} + {}, {y} + {}, {}, {}, {shadow})", sh.dx.typst(), sh.dy.typst(), num(fx), num(fy));
            self.wrap_when(place.when.as_deref(), &code);
        }
        let placed = format!("mt-at({x}, {y}, {}, {}, {body})", num(fx), num(fy));
        if let Some(id) = place.id.as_deref().filter(|id| self.labels && super::label_safe(id)) {
            // A marker at the element's position, for locating it.
            self.raw(&format!("#place(top + left, dx: {x}, dy: {y}, [#metadata({}) <{id}>])\n", str_lit(id)));
        }
        self.wrap_when(place.when.as_deref(), &placed);
        self.placed_node(place.id.as_deref(), if place.decorative == Some(true) { "decorative" } else { e.kind() }, start);
    }

    /// `body` turned by `rot` degrees about the element's `origin` (its centre
    /// by default), keeping its place in the layout.
    fn rotated(&self, place: &Place, rot: f64, body: String) -> String {
        if rot == 0.0 {
            return body;
        }
        match &place.origin {
            Some(Origin::Point([px, py])) if px.pt().is_some() && py.pt().is_some() => {
                // Turn about the top-left corner, then move the pivot back.
                let (px, py) = (px.pt().unwrap_or(0.0), py.pt().unwrap_or(0.0));
                let (sin, cos) = rot.to_radians().sin_cos();
                let (dx, dy) = (px - (px * cos - py * sin), py - (px * sin + py * cos));
                format!("move(dx: {}pt, dy: {}pt, rotate({}deg, origin: top + left, reflow: false, {body}))", num(dx), num(dy), num(rot))
            }
            Some(Origin::Word(w)) => format!("rotate({}deg, origin: {}, reflow: false, {body})", num(rot), origin_align(w)),
            _ => format!("mt-rot({}deg, {body})", num(rot)),
        }
    }

    fn skew_of(&self, p: &Place) -> Option<f64> {
        p.skew.filter(|s| *s != 0.0).map(|s| -s)
    }

    fn wrap_when(&mut self, cond: Option<&str>, body: &str) {
        match cond {
            Some(c) if !c.trim().is_empty() => self.emit(&format!("#context if {} {{ {body} }}\n", when(c))),
            _ => self.emit(&format!("#{body}\n")),
        }
    }

    /// A file made by the compiler, served under `/@mt/`.
    /// The curve for a clip shape, in the element's coordinates.
    fn clip_curve(&mut self, shape: &str, place: &Place) -> Option<typst::visualize::Curve> {
        let (w, h) = (place.w.and_then(|w| w.pt()), place.h.and_then(|h| h.pt()));
        if shape.trim() == "ellipse" && (w.is_none() || h.is_none()) {
            self.problem(format!("{}: an ellipse clip needs w and h in points", place.id.as_deref().unwrap_or("an element")));
            return None;
        }
        match crate::effects::clip_curve(shape, w.unwrap_or(0.0), h.unwrap_or(0.0)) {
            Ok(c) => Some(c),
            Err(e) => {
                self.problem(format!("{}: {e}", place.id.as_deref().unwrap_or("an element")));
                None
            }
        }
    }

    /// A copy of the picture at `path` with its opacity scaled: an SVG
    /// wrapped in a translucent group, a raster image with its alpha
    /// multiplied. Kept between builds, so edits stay fast.
    fn faded(&mut self, path: &str, opacity: f64) -> Option<String> {
        type Entry = ((String, u64, u128), Vec<u8>, &'static str);
        static CACHE: std::sync::OnceLock<std::sync::Mutex<Vec<Entry>>> = std::sync::OnceLock::new();
        let key = (path.to_string(), (opacity * 1000.0).round() as u64, self.assets.version(path)?);
        let cache = CACHE.get_or_init(Default::default);
        if let Some((_, data, ext)) = cache.lock().ok()?.iter().find(|(k, _, _)| *k == key).cloned() {
            return Some(self.file(ext, data));
        }
        let bytes = self.assets.read(path)?;
        let (data, ext) = if path.to_lowercase().ends_with(".svg") {
            let svg = std::str::from_utf8(&bytes).ok()?;
            (svg_group(svg, &format!(r#"opacity="{}""#, num(opacity))).into_bytes(), "svg")
        } else {
            let mut img = image::load_from_memory(&bytes).ok()?.to_rgba8();
            for px in img.pixels_mut() {
                px.0[3] = (px.0[3] as f64 * opacity).round() as u8;
            }
            let mut out = Vec::new();
            image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).ok()?;
            (out, "png")
        };
        if let Ok(mut c) = cache.lock() {
            if c.len() >= 32 {
                c.remove(0);
            }
            c.push((key, data.clone(), ext));
        }
        Some(self.file(ext, data))
    }

    pub fn file(&mut self, ext: &str, data: Vec<u8>) -> String {
        self.files += 1;
        self.virt.insert(&format!("f{}.{ext}", self.files), data)
    }

    #[allow(clippy::too_many_arguments)]
    fn text_frame(&mut self, t: &TextFrame, place: &Place, x: &str, y: &str, fx: f64, fy: f64, rot: f64, scaled: bool) {
        let style_name = t.style.clone();
        let st = match (&style_name, &t.set) {
            (Some(n), s) => self.style_with(n, s.as_ref()),
            (None, Some(s)) => self.style_with("", Some(s)),
            (None, None) => "mt-base".to_string(),
        };
        let size = style_name.as_deref().map(|n| self.doc.design.style(n)).unwrap_or_else(|| self.doc.design.style("")).text.size;
        let size_pt = size.map_or(11.0, |s| s.resolve(11.0, 0.0));
        let (it, ir, ib, il) = insets_pt(t.inset.as_ref(), size_pt);
        let auto_w = place.w.is_none();
        let w = place.w.unwrap_or(Length::Pct(100.0));
        let wi = match w {
            _ if auto_w => "auto".to_string(),
            Length::Pt(v) => format!("{}pt", num(v - il - ir)),
            other => other.typst(),
        };
        let shrink = t.fit.as_deref() == Some("shrink");
        let fit_width = t.fit.as_deref() == Some("width");
        let hi = place.h.and_then(|h| h.pt()).map(|h| h - it - ib);

        // The frame's paragraphs: its blocks, or its words with a blank line
        // between paragraphs. Those stay the frame's words, for the caret.
        let blocks: Vec<Block> = if !t.blocks.is_empty() {
            t.blocks.clone()
        } else {
            let frame = place.id.clone();
            paragraphs(&content_of(&t.text, &t.content))
                .into_iter()
                .enumerate()
                .map(|(i, (base, content))| {
                    let id = frame.as_ref().map(|f| if i == 0 { format!("{f}.p") } else { format!("{f}.p{}", i + 1) });
                    if let (Some(f), Some(id)) = (&frame, &id) {
                        // Only a `text` frame's first paragraph has been labelled (as `<id>.p`).
                        self.owners.insert(id.clone(), (f.clone(), base, i == 0 && t.text.is_some()));
                    }
                    Block::P(Para { id, content, ..Para::default() })
                })
                .collect()
        };

        let saved_notes = self.notes.take();
        let saved_hug = std::mem::replace(&mut self.hug, fit_width || auto_w);
        let mut content = self.capture(|g2| g2.flow(&blocks, &st, shrink || fit_width || scaled));
        self.hug = saved_hug;
        self.notes = saved_notes;
        if let Some(cols) = t.columns.filter(|c| *c > 1) {
            content = format!("#columns({cols}, gutter: {})[{content}]", t.gutter.map(|g| g.typst()).unwrap_or_else(|| "12pt".into()));
        }
        let inner = if fit_width {
            let kmin = t.min_size.map(|m| (m.resolve(size_pt, 0.0) / size_pt).max(0.01)).unwrap_or(0.05);
            let kmax = t.max_size.map(|m| m.resolve(size_pt, 0.0) / size_pt).unwrap_or(100.0);
            format!("mt-fitw({wi}, {}, {}, k => [{content}])", num(kmin), num(kmax))
        } else if shrink {
            let kmin = t
                .min_size
                .map(|m| (m.resolve(size_pt, 0.0) / size_pt).clamp(0.05, 1.0))
                .unwrap_or(0.5);
            match &hi {
                Some(h) => format!("mt-shrink({wi}, {}pt, {}, k => [{content}])", num(*h), num(kmin)),
                None => format!("[{content}]"),
            }
        } else {
            format!("[{content}]")
        };
        // Aligning the frame's content must not align what is inside it.
        let inner = match t.valign.as_deref() {
            Some("middle") | Some("center") if hi.is_some() => format!("align(horizon, block(width: 100%, {{ set align(top); {inner} }}))"),
            Some("bottom") if hi.is_some() => format!("align(bottom, block(width: 100%, {{ set align(top); {inner} }}))"),
            _ => inner,
        };
        let mut args = vec![format!("width: {}", if auto_w { "auto".to_string() } else { w.typst() })];
        if let Some(h) = &place.h {
            args.push(format!("height: {}", h.typst()));
        }
        if let Some(i) = &t.inset {
            args.push(format!("inset: {}", insets(i, false)));
        }
        if let Some(f) = &t.fill {
            args.push(format!("fill: {}", self.pal.fill(f)));
        }
        if let Some(s) = &t.stroke {
            args.push(format!("stroke: {}", self.pal.border(s, false)));
            if let Some(o) = stroke_outset(s, place.stroke_align.as_deref(), false) {
                args.push(format!("outset: {o}"));
            }
        }
        if let Some(r) = &t.radius {
            args.push(format!("radius: {}", radius(r, false)));
        }
        if t.clip.as_ref().is_some_and(Clip::is_box) {
            args.push("clip: true".into());
        }
        let mut frame = format!("block({}, c)", args.join(", "));
        let op = place.opacity.unwrap_or(1.0) as f32;
        let clip = t.clip.as_ref().and_then(Clip::shape).and_then(|c| self.clip_curve(c, place));
        if op < 1.0 || place.decorative == Some(true) || clip.is_some() {
            let mut effect = Effect::new(op, place.decorative == Some(true));
            effect.clip = clip;
            frame = self.with_effect(&frame, effect);
        }
        frame = self.rotated(place, rot, frame);
        let check = match (&hi, shrink, &place.id) {
            (Some(h), false, Some(id)) if self.labels => format!("; mt-overflow({}, {wi}, {}pt, c)", str_lit(id), num(*h)),
            _ => String::new(),
        };
        let shadow = match &place.shadow {
            Some(sh) => {
                let r = t.radius.as_ref().map(|r| format!(", radius: {}", radius(r, false))).unwrap_or_default();
                let box_ = format!("rect(width: s.width, height: s.height, fill: {}{r})", self.pal.color(&sh.color));
                let box_ = self.rotated(place, rot, box_);
                format!(
                    "let s = measure(block({}, c)); mt-at({x} + {}, {y} + {}, {}, {}, {box_}); ",
                    args.iter().filter(|a| a.starts_with("width") || a.starts_with("height") || a.starts_with("inset")).cloned().collect::<Vec<_>>().join(", "),
                    sh.dx.typst(),
                    sh.dy.typst(),
                    num(fx),
                    num(fy)
                )
            }
            None => String::new(),
        };
        // Anchored by the first baseline (as TikZ places text): shift the frame
        // up by the inset and the first line's top edge.
        let (y, fy) = match place.anchor.as_deref() {
            Some(a) if a.starts_with("baseline") => {
                let resolved = style_name.as_deref().map(|n| self.doc.design.style(n)).unwrap_or_else(|| self.doc.design.style(""));
                let resolved = match &t.set {
                    Some(over) => resolved.with(over),
                    None => resolved,
                };
                let first = self.first_baseline(&resolved);
                (format!("({y} - {}pt)", num(it + first)), 0.0)
            }
            _ => (y.to_string(), fy),
        };
        let y = y.as_str();
        let code = if shadow.is_empty() {
            format!("{{ let c = {inner}; mt-at({x}, {y}, {}, {}, {frame}){check} }}", num(fx), num(fy))
        } else {
            format!("context {{ let c = {inner}; {shadow}mt-at({x}, {y}, {}, {}, {frame}){check} }}", num(fx), num(fy))
        };
        self.wrap_when(place.when.as_deref(), &code);
    }

    fn lettering(&mut self, l: &Lettering, place: &Place) -> String {
        let base = l.span.as_ref().and_then(|s| self.doc.design.spans.get(s).cloned()).unwrap_or_default();
        let t: TextStyle = base.with(&l.style);
        let fams = {
            let f = self.families(&t);
            if f.is_empty() { self.families(&self.doc.design.style("").text) } else { f }
        };
        let weight = t.weight.as_ref().map(weight_number).unwrap_or(400);
        let font = match pick(self.fonts, &fams, weight, t.italic.unwrap_or(false)) {
            Some(f) => f,
            None => {
                self.problem(format!("lettering: no font found for {}", fams.join(", ")));
                return "none".into();
            }
        };
        let size = t.size.map_or(12.0, |s| s.resolve(12.0, 0.0));
        let tracking = t.tracking.map_or(0.0, |tr| tr.resolve(size, 0.0));
        let fill = t.color.as_ref().map(|c| self.pal.svg(c)).unwrap_or_else(|| "#000000".into());
        let stroke = t.outline.as_ref().map(|s| match s {
            Stroke::Color(c) => (self.pal.svg(c), 1.0),
            Stroke::Full(spec) => (
                spec.color.as_ref().map(|c| self.pal.svg(c)).unwrap_or_else(|| fill.clone()),
                spec.width.map_or(1.0, |w| w.resolve(size, 0.0)),
            ),
        });
        let text = match t.case {
            Some(matenadar_model::style::Case::Upper) => l.text.to_uppercase(),
            Some(matenadar_model::style::Case::Lower) => l.text.to_lowercase(),
            _ => l.text.clone(),
        };
        let features = t.features.clone().unwrap_or_default();
        let align = match l.align.as_deref() {
            Some("center") => 0.5,
            Some("right") => 1.0,
            _ => 0.0,
        };
        // Variable fonts: the same axes Typst sets for text.
        let mut variations: Vec<([u8; 4], f32)> = vec![(*b"wght", weight as f32), (*b"opsz", size as f32)];
        if let Some(st) = t.stretch {
            variations.push((*b"wdth", st as f32));
        }
        for (tag, v) in t.variations.iter().flatten() {
            if let Ok(tag) = <[u8; 4]>::try_from(tag.as_bytes()) {
                variations.retain(|(t, _)| *t != tag);
                variations.push((tag, *v as f32));
            }
        }
        let spec = LetterSpec {
            text: &text,
            font: &font,
            size_pt: size,
            tracking_pt: tracking,
            features: &features,
            fill: &fill,
            stroke: stroke.as_ref().map(|(c, w)| (c.as_str(), *w)),
            line_height: match t.line_height {
                Some(matenadar_model::style::LineHeight::Factor(f)) => f,
                Some(matenadar_model::style::LineHeight::Length(len)) if size > 0.0 => len.resolve(size, size * 100.0) / size,
                _ => 1.2,
            },
            align,
            width_pt: place.w.and_then(|w| w.pt()),
            variations,
            path: l.path.as_deref(),
            start_pt: None,
        };
        // A share of the path needs its length: measure it first.
        let spec = match (&l.path, l.start) {
            (Some(_), Some(start)) => {
                let length = crate::glyphs::path_length(l.path.as_deref().unwrap_or("")).unwrap_or(0.0);
                LetterSpec { start_pt: Some(start.resolve(size, length)), ..spec }
            }
            _ => spec,
        };
        match letter(&spec) {
            Ok(out) => {
                let op = place.opacity.unwrap_or(1.0) * self.opacity;
                let svg = if op < 1.0 { svg_group(&out.svg, &format!(r#"opacity="{}""#, num(op))) } else { out.svg };
                let path = self.file("svg", svg.into_bytes());
                let image = format!("image({}, width: {}pt, height: {}pt)", str_lit(&path), num(out.width_pt), num(out.height_pt));
                if l.path.is_some() {
                    // Placed by the path's own coordinates, in a box that ends where the drawing does.
                    let (ox, oy) = out.offset;
                    let w = place.w.map(|w| w.typst()).unwrap_or_else(|| format!("{}pt", num((ox + out.width_pt).max(0.0))));
                    let h = place.h.map(|h| h.typst()).unwrap_or_else(|| format!("{}pt", num((oy + out.height_pt).max(0.0))));
                    format!("box(width: {w}, height: {h}, place(top + left, dx: {}pt, dy: {}pt, {image}))", num(ox), num(oy))
                } else {
                    image
                }
            }
            Err(err) => {
                self.problem(format!("lettering: {err}"));
                "none".into()
            }
        }
    }

    fn letter_grid(&mut self, g: &LetterGrid, place: &Place, scaled: bool) -> String {
        let rows: Vec<Vec<GridCell>> = g
            .rows
            .iter()
            .map(|r| match r {
                GridRow::Letters(s) => s.chars().filter(|c| !c.is_whitespace()).map(|c| GridCell::Text(c.to_string())).collect(),
                GridRow::Cells(cells) => cells.clone(),
            })
            .collect();
        let ncols = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
        let nrows = rows.len().max(1);
        let cw = match (g.cell, place.w) {
            (Some(c), _) => c.resolve(11.0, 0.0),
            (None, Some(w)) => w.resolve(11.0, 0.0) / ncols as f64,
            _ => 18.0,
        };
        let ch = g.cell_h.map_or(cw, |c| c.resolve(11.0, 0.0));
        let span = {
            let mut t = match &g.style {
                Some(n) if self.doc.design.styles.contains_key(n) => self.doc.design.style(n).text,
                Some(n) => self.doc.design.spans.get(n).cloned().unwrap_or_default(),
                None => TextStyle::default(),
            };
            if let Some(s) = &g.set {
                t = t.with(s);
            }
            self.text_dict(&t, None)
        };
        let mut out = format!(
            "box(width: {}pt, height: {}pt{}{}{}, {{ ",
            num(cw * ncols as f64),
            num(ch * nrows as f64),
            g.fill.as_ref().map(|f| format!(", fill: {}", self.pal.fill(f))).unwrap_or_default(),
            g.frame.as_ref().map(|s| format!(", stroke: {}", self.pal.stroke(s, scaled))).unwrap_or_default(),
            g.radius.as_ref().map(|r| format!(", radius: {}", radius(r, scaled))).unwrap_or_default(),
        );
        out.push_str(&format!("let sp = {span}; "));
        for hl in &g.highlights {
            let fill = hl.fill.as_ref().map(|c| self.pal.color(c)).unwrap_or_else(|| "none".into());
            let stroke = hl.stroke.as_ref().map(|s| self.pal.stroke(s, scaled)).unwrap_or_else(|| "none".into());
            let t = hl.thickness.unwrap_or(0.78) * ch.min(cw);
            if hl.shape.as_deref() == Some("cells") {
                let (r0, c0) = hl.from;
                let (r1, c1) = hl.to;
                let steps = (r1 as i64 - r0 as i64).abs().max((c1 as i64 - c0 as i64).abs());
                for i in 0..=steps {
                    let r = r0 as f64 + (r1 as f64 - r0 as f64) * i as f64 / steps.max(1) as f64;
                    let c = c0 as f64 + (c1 as f64 - c0 as f64) * i as f64 / steps.max(1) as f64;
                    out.push_str(&format!(
                        "place(top + left, dx: {}pt, dy: {}pt, rect(width: {}pt, height: {}pt, fill: {fill}, stroke: {stroke})); ",
                        num(c * cw),
                        num(r * ch),
                        num(cw),
                        num(ch)
                    ));
                }
                continue;
            }
            let (x0, y0) = ((hl.from.1 as f64 + 0.5) * cw, (hl.from.0 as f64 + 0.5) * ch);
            let (x1, y1) = ((hl.to.1 as f64 + 0.5) * cw, (hl.to.0 as f64 + 0.5) * ch);
            let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt() + t;
            let angle = (y1 - y0).atan2(x1 - x0).to_degrees();
            let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            out.push_str(&format!(
                "place(top + left, dx: {}pt, dy: {}pt, rotate({}deg, origin: center + horizon, reflow: false, rect(width: {}pt, height: {}pt, radius: {}pt, fill: {fill}, stroke: {stroke}))); ",
                num(mx - len / 2.0),
                num(my - t / 2.0),
                num(angle),
                num(len),
                num(t),
                num(t / 2.0)
            ));
        }
        let lines = g.lines.as_ref().map(|s| self.pal.stroke(s, scaled)).unwrap_or_else(|| "none".into());
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                let (text, fill, number, color, block, weight, cspan) = match cell {
                    GridCell::Text(t) => (t.clone(), None, None, None, false, None, None),
                    GridCell::Full(s) => (
                        s.text.clone().unwrap_or_default(),
                        s.fill.as_ref().map(|f| self.pal.fill(f)),
                        s.number.clone(),
                        s.color.as_ref().map(|c| self.pal.color(c)),
                        s.block == Some(true),
                        s.weight.as_ref().map(weight_number),
                        s.span.as_ref().map(|n| self.span_var(n)),
                    ),
                };
                let fill = if block { Some(fill.unwrap_or_else(|| "black".into())) } else { fill };
                let mut letter = format!("[{}]", super::escape::escape(&text).0);
                if let Some(cs) = cspan {
                    letter = format!("mt-span({cs}, {letter})");
                }
                let mut targs = Vec::new();
                if let Some(c) = color {
                    targs.push(format!("fill: {c}"));
                }
                if let Some(w) = weight {
                    targs.push(format!("weight: {w}"));
                }
                if !targs.is_empty() {
                    letter = format!("text({}, {letter})", targs.join(", "));
                }
                let num_part = number
                    .map(|n| format!("place(top + left, dx: 1.5pt, dy: 1pt, text(size: {}pt, [{}])); ", num(ch * 0.28), super::escape::escape(&n).0))
                    .unwrap_or_default();
                out.push_str(&format!(
                    "place(top + left, dx: {}pt, dy: {}pt, box(width: {}pt, height: {}pt, stroke: {lines}{}, {{ {num_part}{} }})); ",
                    num(c as f64 * cw),
                    num(r as f64 * ch),
                    num(cw),
                    num(ch),
                    fill.map(|f| format!(", fill: {f}")).unwrap_or_default(),
                    if block || text.is_empty() {
                        "none".to_string()
                    } else {
                        format!("align(center + horizon, text(top-edge: \"cap-height\", bottom-edge: \"baseline\", mt-span(sp, {letter})))")
                    }
                ));
            }
        }
        out.push_str("})");
        out
    }
}

/// The SVG with its root `viewBox` replaced (and its fixed size dropped, so
/// the placed box decides it).
fn with_viewbox(svg: &str, vb: &[f64; 4]) -> String {
    let Some(start) = svg.find("<svg") else { return svg.to_string() };
    let Some(end) = svg[start..].find('>').map(|e| start + e) else { return svg.to_string() };
    let mut tag = svg[start..end].to_string();
    for attr in ["viewBox", "width", "height"] {
        if let Some(i) = tag.find(&format!(" {attr}=")) {
            let q = tag[i + attr.len() + 2..].chars().next().unwrap_or('"');
            if let Some(j) = tag[i + attr.len() + 3..].find(q) {
                tag.replace_range(i..i + attr.len() + 4 + j, "");
            }
        }
    }
    tag.push_str(&format!(r#" viewBox="{} {} {} {}""#, num(vb[0]), num(vb[1]), num(vb[2]), num(vb[3])));
    format!("{}{}{}", &svg[..start], tag, &svg[end..])
}

fn pt_of(l: Option<Length>) -> f64 {
    l.map_or(0.0, |l| l.resolve(11.0, 0.0))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

fn svg_stroke(g: &Gen, s: Option<&Stroke>) -> String {
    match s {
        None => String::new(),
        Some(Stroke::Color(c)) => format!(r#" stroke="{}" stroke-width="1""#, g.pal.svg(c)),
        Some(Stroke::Full(spec)) => {
            let mut out = format!(
                r#" stroke="{}" stroke-width="{}""#,
                spec.color.as_ref().map(|c| g.pal.svg(c)).unwrap_or_else(|| "#000000".into()),
                num(spec.width.map_or(1.0, |w| w.resolve(11.0, 0.0)))
            );
            if let Some(cap) = &spec.cap {
                out.push_str(&format!(r#" stroke-linecap="{cap}""#));
            }
            if let Some(join) = &spec.join {
                out.push_str(&format!(r#" stroke-linejoin="{join}""#));
            }
            if let Some(serde_json::Value::Array(d)) = &spec.dash {
                let parts: Vec<String> = d
                    .iter()
                    .filter_map(|v| serde_json::from_value::<Length>(v.clone()).ok())
                    .map(|l| num(l.resolve(11.0, 0.0)))
                    .collect();
                out.push_str(&format!(r#" stroke-dasharray="{}""#, parts.join(" ")));
            }
            out
        }
    }
}

/// A rotation origin word as a Typst alignment.
fn origin_align(word: &str) -> &'static str {
    match word {
        "top-left" => "top + left",
        "top" => "top + center",
        "top-right" => "top + right",
        "left" => "horizon + left",
        "right" => "horizon + right",
        "bottom-left" => "bottom + left",
        "bottom" => "bottom + center",
        "bottom-right" => "bottom + right",
        _ => "center + horizon",
    }
}

/// A picture cut to `crop` and fitted into its box (`mt-crop` in the prelude).
pub(crate) fn crop_call(path: &str, c: &matenadar_model::element::Crop, w: Option<String>, h: Option<String>, fit: &str, focus: Option<(f64, f64)>) -> String {
    let (fx, fy) = focus.unwrap_or((0.5, 0.5));
    format!(
        "mt-crop({}, ({}, {}, {}, {}), width: {}, height: {}, fit: {}, focus: ({}, {}), draw: (w, h) => image({}, width: w, height: h, fit: \"stretch\"))",
        str_lit(path),
        num(c.x),
        num(c.y),
        num(c.w),
        num(c.h),
        w.unwrap_or_else(|| "auto".into()),
        h.unwrap_or_else(|| "auto".into()),
        str_lit(fit),
        num(fx.clamp(0.0, 1.0)),
        num(fy.clamp(0.0, 1.0)),
        str_lit(path)
    )
}

/// Inline content cut into paragraphs at each blank line in its words, each
/// with where its words start in the whole.
fn paragraphs(content: &[Inline]) -> Vec<(usize, Vec<Inline>)> {
    let mut out: Vec<(usize, Vec<Inline>)> = vec![(0, Vec::new())];
    let mut at = 0;
    for x in content {
        let text = match x {
            Inline::Plain(s) => s.as_str(),
            Inline::Run(r) => r.text.as_str(),
            other => {
                out.last_mut().expect("one paragraph at least").1.push(other.clone());
                continue;
            }
        };
        for (i, piece) in text.split("\n\n").enumerate() {
            if i > 0 {
                at += 2;
                out.push((at, Vec::new()));
            }
            let words = match x {
                Inline::Run(r) => Inline::Run(Run { text: piece.to_string(), ..r.clone() }),
                _ => Inline::Plain(piece.to_string()),
            };
            out.last_mut().expect("one paragraph at least").1.push(words);
            at += piece.len();
        }
    }
    out
}
