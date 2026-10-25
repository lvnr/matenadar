//! Elements: things placed at a position.

use matenadar_model::Length;
use matenadar_model::block::{Block, Para};
use matenadar_model::element::{Element, GridCell, GridRow, LetterGrid, Lettering, Place, TextFrame};
use matenadar_model::style::{Stroke, TextStyle};
use matenadar_model::units::num;

use super::Gen;
use super::typ::{anchor, insets, insets_pt, radius, str_lit, when};
use crate::glyphs::{LetterSpec, letter, pick, qr_svg, weight_number};

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
            self.node(l.id.as_deref(), "line", start);
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
                self.text_frame(t, &place, &x, &y, fx, fy, rot, scaled);
                self.node(place.id.as_deref(), if place.decorative == Some(true) { "decorative" } else { "text" }, start);
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
                let focused = match (&asset, i.focus, place.w.and_then(|w| w.pt()), place.h.and_then(|h| h.pt())) {
                    (Some(p), Some((fx, fy)), Some(bw), Some(bh)) if i.fit.as_deref().unwrap_or("cover") == "cover" => {
                        crate::glyphs::image_size(&self.root.join(p.trim_start_matches('/'))).map(|(iw, ih)| {
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
                let mut body = match (focused, asset) {
                    (Some(f), _) => f,
                    (None, Some(p)) => format!("image({}, {})", str_lit(&p), args.join(", ")),
                    (None, None) => format!(
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
                let svg = format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="{vb}" preserveAspectRatio="none"><path d="{d}" fill="{fill}"{stroke}/></svg>"#,
                    w = num(w),
                    h = num(h),
                    d = xml_escape(&p.d)
                );
                let path = self.file("svg", svg.into_bytes());
                format!("image({}, width: {}pt, height: {}pt)", str_lit(&path), num(w), num(h))
            }
            Element::Svg(s) => {
                let path = match (&s.src, &s.markup) {
                    (Some(src), _) => self.asset(src),
                    (None, Some(m)) => Some(self.file("svg", m.clone().into_bytes())),
                    _ => None,
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
                if g.clip == Some(true) {
                    args.push("clip: true".into());
                }
                if let Some(f) = &g.fill {
                    args.push(format!("fill: {}", self.pal.fill(f)));
                }
                if let Some(s) = &g.stroke {
                    args.push(format!("stroke: {}", self.pal.border(s, scaled)));
                }
                if let Some(r) = &g.radius {
                    args.push(format!("radius: {}", radius(r, scaled)));
                }
                let saved = std::mem::take(&mut self.out);
                for c in &g.children {
                    self.element(c, scaled);
                }
                let inner = std::mem::replace(&mut self.out, saved);
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
                        let saved = std::mem::take(&mut self.out);
                        if x.blocks.is_empty() {
                            for c in &x.elements {
                                self.element(c, scaled);
                            }
                        } else {
                            self.flow(&x.blocks, "mt-base", scaled);
                        }
                        let inner = std::mem::replace(&mut self.out, saved);
                        self.depth -= 1;
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
                let dark = q.color.as_ref().map(|c| self.pal.svg(c)).unwrap_or_else(|| "#000000".into());
                let light = q.background.as_ref().map(|c| self.pal.svg(c));
                match qr_svg(&q.data, q.ecc.as_deref().unwrap_or("M"), q.quiet.unwrap_or(0), &dark, light.as_deref()) {
                    Ok((svg, _)) => {
                        let path = self.file("svg", svg.into_bytes());
                        let w = place.w.map(|w| w.typst()).unwrap_or_else(|| "1in".into());
                        format!("image({}, width: {w}, height: {w})", str_lit(&path))
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
        if rot != 0.0 {
            body = format!("mt-rot({}deg, {body})", num(rot));
        }
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
            if rot != 0.0 {
                shadow = format!("mt-rot({}deg, {shadow})", num(rot));
            }
            let code = format!("mt-at({x} + {}, {y} + {}, {}, {}, {shadow})", sh.dx.typst(), sh.dy.typst(), num(fx), num(fy));
            self.wrap_when(place.when.as_deref(), &code);
        }
        let placed = format!("mt-at({x}, {y}, {}, {}, {body})", num(fx), num(fy));
        if let Some(id) = place.id.as_deref().filter(|id| self.labels && super::label_safe(id)) {
            // A marker at the element's position, for locating it.
            self.raw(&format!("#place(top + left, dx: {x}, dy: {y}, [#metadata({}) <{id}>])\n", str_lit(id)));
        }
        self.wrap_when(place.when.as_deref(), &placed);
        self.node(place.id.as_deref(), if place.decorative == Some(true) { "decorative" } else { e.kind() }, start);
    }

    fn skew_of(&self, p: &Place) -> Option<f64> {
        p.skew.filter(|s| *s != 0.0).map(|s| -s)
    }

    fn wrap_when(&mut self, cond: Option<&str>, body: &str) {
        match cond {
            Some(c) if !c.trim().is_empty() => self.raw(&format!("#context if {} {{ {body} }}\n", when(c))),
            _ => self.raw(&format!("#{body}\n")),
        }
    }

    /// A file made by the compiler, served under `/@mt/`.
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
        let w = place.w.unwrap_or(Length::Pct(100.0));
        let wi = match w {
            Length::Pt(v) => format!("{}pt", num(v - il - ir)),
            other => other.typst(),
        };
        let shrink = t.fit.as_deref() == Some("shrink");
        let hi = place.h.and_then(|h| h.pt()).map(|h| h - it - ib);

        // The frame's paragraphs.
        let blocks: Vec<Block> = if !t.blocks.is_empty() {
            t.blocks.clone()
        } else if let Some(text) = &t.text {
            text.split("\n\n")
                .enumerate()
                .map(|(i, p)| {
                    Block::P(Para {
                        id: if i == 0 { place.id.as_ref().map(|id| format!("{id}.p")) } else { None },
                        text: Some(p.to_string()),
                        ..Para::default()
                    })
                })
                .collect()
        } else {
            vec![Block::P(Para { content: t.content.clone(), ..Para::default() })]
        };

        let saved = std::mem::take(&mut self.out);
        let saved_notes = self.notes.take();
        self.flow(&blocks, &st, shrink || scaled);
        self.notes = saved_notes;
        let mut content = std::mem::replace(&mut self.out, saved);
        if let Some(cols) = t.columns.filter(|c| *c > 1) {
            content = format!("#columns({cols}, gutter: {})[{content}]", t.gutter.map(|g| g.typst()).unwrap_or_else(|| "12pt".into()));
        }
        let inner = if shrink {
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
        let inner = match t.valign.as_deref() {
            Some("middle") | Some("center") if hi.is_some() => format!("align(horizon, {inner})"),
            Some("bottom") if hi.is_some() => format!("align(bottom, {inner})"),
            _ => inner,
        };
        let mut args = vec![format!("width: {}", w.typst())];
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
        }
        if let Some(r) = &t.radius {
            args.push(format!("radius: {}", radius(r, false)));
        }
        if t.clip == Some(true) {
            args.push("clip: true".into());
        }
        let mut frame = format!("block({}, c)", args.join(", "));
        if rot != 0.0 {
            frame = format!("mt-rot({}deg, {frame})", num(rot));
        }
        let check = match (&hi, shrink, &place.id) {
            (Some(h), false, Some(id)) if self.labels => format!("; mt-overflow({}, {wi}, {}pt, c)", str_lit(id), num(*h)),
            _ => String::new(),
        };
        let shadow = match &place.shadow {
            Some(sh) => {
                let r = t.radius.as_ref().map(|r| format!(", radius: {}", radius(r, false))).unwrap_or_default();
                let mut box_ = format!("rect(width: s.width, height: s.height, fill: {}{r})", self.pal.color(&sh.color));
                if rot != 0.0 {
                    box_ = format!("mt-rot({}deg, {box_})", num(rot));
                }
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
        let spec = LetterSpec {
            text: &text,
            font: &font,
            size_pt: size,
            tracking_pt: tracking,
            features: &features,
            fill: &fill,
            stroke: stroke.as_ref().map(|(c, w)| (c.as_str(), *w)),
            line_height: l.line_height.unwrap_or(1.2),
            align,
            width_pt: place.w.and_then(|w| w.pt()),
        };
        match letter(&spec) {
            Ok(out) => {
                let path = self.file("svg", out.svg.into_bytes());
                format!("image({}, width: {}pt, height: {}pt)", str_lit(&path), num(out.width_pt), num(out.height_pt))
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
