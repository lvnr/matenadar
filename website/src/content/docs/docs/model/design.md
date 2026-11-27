---
title: The design
description: Colours, font stacks, paragraph and character styles, hyphenation exceptions, figure numbering, masters and components.
---

`design` holds everything a document looks like, apart from the content: named colours and font stacks, text styles, the masters pages are cut from, reusable components, hyphenation exceptions and figure numbering.

```json
"design": {
  "colors": {"ink": "#1d1b19", "accent": "#c2410c"},
  "fonts": {"serif": ["IBM Plex Serif", "Noto Serif"]},
  "base": {"font": "serif", "size": 10.5, "line_height": 1.4, "color": "ink"},
  "styles": {"body": {"align": "justify", "indent": "1.2em"}},
  "spans": {"smallcaps": {"case": "small-caps", "tracking": "0.04em"}},
  "masters": {"body": {"width": "6in", "height": "9in", "margins": {"top": "0.8in", "bottom": "0.9in", "inside": "0.8in", "outside": "0.7in"}}},
  "components": {},
  "hyphenation": {"exceptions": ["Mate-na-dar"]},
  "figures": {"label": "Fig.", "per": "chapter"}
}
```

## Colours and font stacks

- `colors` maps names to [colours](/docs/model/overview/#colours). Use the names anywhere a colour goes.
- `fonts` maps names to a family or a list of families, such as `"serif": ["IBM Plex Serif", "Noto Serif"]`. A style’s `font` may name a stack, and later families supply glyphs the first one lacks.

A family may be named as font menus and fontconfig name it (its typographic name, `"Garamond Premier Pro"`) where Typst knows it by a shorter legacy name (`"Garamond Premr Pro"`).

## Styles

`base` is the default for all text. `styles` holds **paragraph styles**; `spans` holds **character styles**. A paragraph style has every character setting plus the paragraph settings, and may `inherit` another.

### Character settings

These work in styles and spans, inline on [runs](/docs/model/content/#inline-content-runs), and as `set` overrides on blocks and frames.

| Field | Meaning |
|---|---|
| `font` | family, stack name, or list |
| `size` | length; `em` or `%` are of the surrounding size |
| `weight` | 100–900, or `thin` … `regular` `medium` `semibold` `bold` … `black` |
| `italic` | bool |
| `stretch` | width in % for fonts with widths |
| `tracking` | letter spacing (`"0.12em"`). Every letter is followed by it, the last letter of a run included (so splitting a word into runs never changes its width), except the paragraph’s last. Letter-spaced text drops ligatures unless `ligatures` is set. |
| `word_spacing` | extra space between words |
| `color` | colour |
| `case` | `upper`, `lower`, `small-caps`, `all-small-caps` |
| `features` | OpenType features: `["onum", "ss01", "-liga", "salt=2"]` |
| `baseline` | raises (positive) or lowers the text |
| `underline`, `strike` | bool |
| `lang` | BCP 47, for hyphenation and shaping |
| `outline` | a stroke around each glyph (also fakes a bold) |
| `hyphenate`, `ligatures` | bool |
| `overhang` | lets punctuation and hyphens hang slightly into the margin at line ends (default true) |
| `line_height` | see the paragraph settings; on a run, `0` keeps a large letter or mark from opening its line |
| `halo` | a halo behind each glyph, for type over pictures: a stroke, `{"width": 2, "color": "#fff"}` |
| `variations` | variable-font axes, such as `{"opsz": 12, "SOFT": 50}`. `weight`, `stretch` and `size` already drive `wght`, `wdth` and `opsz`, so a variable font needs no static instances. |

### Paragraph settings

| Field | Meaning |
|---|---|
| `inherits` | another style to start from |
| `line_height` | baseline to baseline. A bare number is a multiple of the size; a length is absolute. Lines sit as in CSS: half the extra leading above, half below, with the font’s ascent and descent rounded to whole CSS pixels as browsers do. A run in a larger size opens its line in proportion. |
| `align` | `left`, `right`, `center`, `justify`, `outside`, `inside` |
| `indent` | first-line indent. Not applied after a heading, a break or anything but a paragraph, unless `indent_first`. |
| `hanging` | indent of all lines but the first |
| `margin_left`, `margin_right` | indent of the whole paragraph |
| `space_before`, `space_after` | adjacent spaces collapse to the larger, as in CSS |
| `keep_with_next`, `keep_together` | bool |
| `allow_widows`, `allow_orphans` | widows and orphans are prevented by default |
| `page_break_before` | bool |
| `balance` | lines of equal length (CSS `text-wrap: balance`) |
| `rule_above`, `rule_below` | `{"stroke", "width", "gap"}` |
| `fill`, `inset` | a background box behind the paragraph |
| `linebreaks` | `simple` (one line at a time, as browsers do) or `optimized` (the whole paragraph at once, Knuth–Plass, for evener lines). By default justified text is optimized and ragged text simple. |
| `wrap` | `pretty`: the paragraph never ends on a lone word (its last two words stay together) |
| `justification` | how far justified lines may stretch and shrink: `{"spacing": {"min": "100%", "max": "150%"}, "tracking": {"min": "-0.01em", "max": "0.02em"}}`. `min: "100%"` never tightens word spaces. |
| `drop_cap` | `{"lines": 3, "font", "color", "weight", "gap"}`. The cap’s top lines up with the first line’s capitals and its foot with the baseline of line `lines`. It applies to paragraphs of plain text, and an opening quote goes with the cap. |

A chapter opening, as styles:

```json
"styles": {
  "body":  {"align": "justify", "indent": "1.2em", "hyphenate": true, "linebreaks": "optimized"},
  "lede":  {"inherits": "body", "indent": 0, "drop_cap": {"lines": 3, "color": "accent"}},
  "h1":    {"size": 22, "space_before": "2.4em", "space_after": "1em", "case": "upper",
            "tracking": "0.08em", "align": "center", "keep_with_next": true},
  "note":  {"size": 8, "line_height": 1.3, "italic": true, "color": "accent"}
}
```

Headings use the styles `h1`, `h2`… when the design has them; the contents use `toc-1`, `toc-2`…, and the list of figures `lof`.

## Hyphenation

Typst hyphenates by the language’s patterns. `hyphenation.exceptions` lists words whose breaks you choose instead: hyphens mark the only places a word may break, and a word written without hyphens never breaks.

```json
"hyphenation": {"exceptions": ["Mate-na-dar", "pro-gram", "Levon"]}
```

Matching ignores case. The exceptions apply in paragraphs that hyphenate: justified ones, unless `hyphenate` is false, and any with `hyphenate: true`.

## Figures

`figures` turns numbering on. Every figure with a caption then gets a number (`numbered: false` leaves one out; `numbered: true` numbers one without a caption), set before the caption as a label:

```json
"figures": {"label": "Fig.", "per": "chapter", "separator": ". ", "span": "figlabel"}
```

| Field | Meaning |
|---|---|
| `label` | the word before the number (default `"Figure"`). A no-break space joins it to the number. |
| `per` | `"book"` (default: 1, 2, 3… through the book) or `"chapter"`: the count restarts in each section, after the section’s `chapter` var when it has one (`3.1`, `3.2`) |
| `separator` | between the number and the caption’s words (default `". "`) |
| `span` | a character style for the label and number |

Figures are numbered in the order they stand in the document, wherever a float or plate puts them; print and EPUB number them alike. The list of figures (`lof`) and references (`{"ref": "fig-id", "show": "label"}`) find them by id.

## Masters

A master is a page size, a flow area and what is drawn on every page:

```json
"masters": {
  "body": {
    "width": "6in", "height": "9in", "bleed": "0.125in",
    "margins": {"top": "0.8in", "bottom": "0.9in", "inside": "0.8in", "outside": "0.7in"},
    "columns": 1, "gutter": "12pt",
    "fill": "paper",
    "style": "body",
    "notes": {"width": "1.1in", "gap": "0.15in", "side": "outside", "style": "note", "spacing": "6pt"},
    "background": [],
    "foreground": [],
    "inherits": "another master"
  }
}
```

- **`preset`** fills in standard trim dimensions when `width` or `height` are omitted (explicit `width`/`height` override it): `a4` (210 × 297 mm), `a4-landscape`, `a5` (148 × 210 mm), `a5-landscape`, `a3` (297 × 420 mm), `letter` (8.5 × 11 in), `letter-landscape`, `legal` (8.5 × 14 in), `16:9` (960 × 540 pt), `4:3` (720 × 540 pt), `social` (1080 × 1080 pt), `social-portrait` (1080 × 1350 pt), and `social-story` (1080 × 1920 pt).
- **Margins** mirror (`inside` is the spine side) unless `left` or `right` is given: `{"top", "bottom", "left", "right"}` is the same on every page.
- **`columns`** and **`gutter`** divide the flow area.
- **`style`** is the default paragraph style of flows on this master.
- **`notes`** makes a column for [margin notes](/docs/model/content/#blocks): its `width`, its `gap` from the text, its `side` (`outside`, the default, `inside`, `left` or `right`), its `style` and the `spacing` between notes.
- **`background`** and **`foreground`** are [placed elements](/docs/model/content/#elements) drawn under and over every page’s content: folios, running heads, rules, art.
- **`inherits`** starts from another master.
- **`grow: true`** makes the page as tall as what it holds: a section on it is one page, however long its flow (its `height` is not used). It is for pages read on a screen, such as a store’s module proofs and long strips.

### Conditional layers

Elements in `background` and `foreground` can carry `when`, a list of conditions that must all hold: `recto`, `verso`, `first`, `blank`, and their `not-` forms (`not-first`, `not-blank`). A running head that skips chapter openings and blank pages, mirrored on each side:

```json
"foreground": [
  {"type": "text", "x": "0.8in", "y": "0.45in", "w": "4.4in", "style": "running",
   "content": [{"field": "section.title"}], "when": "not-first not-blank recto", "set": {"align": "right"}},
  {"type": "text", "x": "0.8in", "y": "0.45in", "w": "4.4in", "style": "running",
   "content": [{"field": "doc.title"}], "when": "not-first not-blank verso"},
  {"type": "text", "x": "0.8in", "y": "8.45in", "w": "4.4in", "style": "folio",
   "content": [{"field": "page"}], "when": "not-blank", "set": {"align": "center"}}
]
```

To leave a master element off one particular page (a full-bleed plate, a part title), use [`on_page`](/docs/model/content/#on-the-page-of-a-node) instead: it follows the node when the text reflows.

## Components

A component is a named, reusable composition with parameters:

```json
"components": {
  "plate": {
    "params": {"src": null, "caption": "", "tint": "#eee"},
    "w": "4in", "h": "3in",
    "elements": [
      {"type": "rect", "x": 0, "y": 0, "w": "4in", "h": "3in", "fill": "{{tint}}"},
      {"type": "image", "src": "{{src}}", "x": 6, "y": 6, "w": "3.8in", "h": "2.6in"},
      {"type": "text", "x": 6, "y": "2.7in", "w": "3.8in", "style": "caption", "text": "{{caption}}"}
    ]
  }
}
```

An instance is a `use`, as an element or a block:

```json
{"type": "use", "id": "plate-3", "component": "plate", "x": "1in", "y": "2in",
 "props": {"src": "art/harbour.jpg", "caption": "The harbour at dawn."}}
```

- An instance’s `props` replace the `{{name}}` placeholders.
- A string that is exactly `"{{name}}"` takes the prop’s value as it is: a number, a list of elements, an object. A placeholder inside a longer string takes the prop’s text.
- `null` marks a required parameter.
- A component made of `blocks` instead of `elements` is used in flows.
