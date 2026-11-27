//! Pure-Rust 1D vector barcode encoder: Code 128, GS1-128, EAN-13, ISBN-13,
//! UPC-A, and Code 39. Zero external or C dependencies; identical bar geometry
//! on native and WebAssembly targets.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarcodeFormat {
    Code128,
    Gs1128,
    Ean13,
    Isbn13,
    UpcA,
    Code39,
}

impl BarcodeFormat {
    pub fn parse(s: Option<&str>) -> Result<Self, String> {
        match s.unwrap_or("code128").trim().to_ascii_lowercase().as_str() {
            "code128" | "code-128" => Ok(Self::Code128),
            "gs1-128" | "gs1128" | "ean128" => Ok(Self::Gs1128),
            "ean13" | "ean-13" => Ok(Self::Ean13),
            "isbn13" | "isbn-13" => Ok(Self::Isbn13),
            "upca" | "upc-a" => Ok(Self::UpcA),
            "code39" | "code-39" => Ok(Self::Code39),
            other => Err(format!("unknown barcode format \"{other}\" (use code128, gs1-128, ean13, isbn13, upca, code39)")),
        }
    }

    pub fn default_quiet(self) -> u32 {
        match self {
            Self::Ean13 | Self::Isbn13 => 11,
            Self::UpcA => 9,
            Self::Code128 | Self::Gs1128 | Self::Code39 => 10,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bar {
    /// Module offset from the start of the symbol (excluding left quiet zone).
    pub x: u32,
    /// Bar width in modules.
    pub w: u32,
    /// Guard bar that extends deeper into the human-readable zone (EAN-13 / UPC-A).
    pub tall: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedBarcode {
    pub format: BarcodeFormat,
    /// Total symbol width in modules (excluding quiet zones).
    pub symbol_modules: u32,
    pub quiet: u32,
    pub bars: Vec<Bar>,
    /// Canonical human-readable string (e.g. with computed check digit).
    pub label: String,
}

pub fn encode(data: &str, format: Option<&str>, quiet: Option<u32>) -> Result<EncodedBarcode, String> {
    let fmt = BarcodeFormat::parse(format)?;
    let quiet = quiet.unwrap_or_else(|| fmt.default_quiet());
    match fmt {
        BarcodeFormat::Code128 => encode_code128(data, false, quiet),
        BarcodeFormat::Gs1128 => encode_code128(data, true, quiet),
        BarcodeFormat::Ean13 => encode_ean13(data, BarcodeFormat::Ean13, quiet),
        BarcodeFormat::Isbn13 => encode_ean13(data, BarcodeFormat::Isbn13, quiet),
        BarcodeFormat::UpcA => encode_upca(data, quiet),
        BarcodeFormat::Code39 => encode_code39(data, quiet),
    }
}

/// Renders the bars of `bc` as a crisp SVG document spanning `(total_modules)` wide.
pub fn to_svg(bc: &EncodedBarcode, w_pt: f64, h_pt: f64, bar_color: &str, bg_color: Option<&str>, extend_guards: bool) -> String {
    use matenadar_model::units::num;
    let total = (bc.symbol_modules + 2 * bc.quiet).max(1) as f64;
    let unit = w_pt / total;
    let short_h = if extend_guards && bc.bars.iter().any(|b| b.tall) {
        h_pt * 0.88
    } else {
        h_pt
    };
    let mut body = String::with_capacity(bc.bars.len() * 48 + 128);
    if let Some(bg) = bg_color
        && bg != "none"
    {
        body.push_str(&format!(
            r#"<rect x="0" y="0" width="{}" height="{}" fill="{bg}"/>"#,
            num(w_pt),
            num(h_pt)
        ));
    }
    let mut path = String::with_capacity(bc.bars.len() * 28);
    for bar in &bc.bars {
        let x = (bc.quiet + bar.x) as f64 * unit;
        let bw = bar.w as f64 * unit;
        let bh = if bar.tall && extend_guards { h_pt } else { short_h };
        path.push_str(&format!("M{} 0h{}v{}h-{}Z", num(x), num(bw), num(bh), num(bw)));
    }
    body.push_str(&format!(r#"<path d="{path}" fill="{bar_color}"/>"#));
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="0 0 {w} {h}" shape-rendering="crispEdges">{body}</svg>"#,
        w = num(w_pt),
        h = num(h_pt)
    )
}

// =========================================================================
// Code 128 & GS1-128 (ISO/IEC 15417)
// =========================================================================

const C128_FNC1: u8 = 102;
const C128_CODE_C: u8 = 99;
const C128_CODE_B: u8 = 100;
const C128_START_B: u8 = 104;
const C128_START_C: u8 = 105;
const C128_STOP: u8 = 106;

/// Bar/space module widths for Code 128 symbols 0..=106 (6 widths for 0..105, 7 for 106).
const C128_PATTERNS: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312",
    "132212", "221213", "221312", "231212", "112232", "122132", "122231", "113222",
    "123122", "123221", "223211", "221132", "221231", "213212", "223112", "312131",
    "311222", "321122", "321221", "312212", "322112", "322211", "212123", "212321",
    "232121", "111323", "131123", "131321", "112313", "132113", "132311", "211313",
    "231113", "231311", "112133", "112331", "132131", "113123", "113321", "133121",
    "313121", "211331", "231131", "213113", "213311", "213131", "311123", "311321",
    "331121", "312113", "312311", "332111", "314111", "221411", "431111", "111224",
    "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111",
    "111242", "121142", "121241", "114212", "124112", "124211", "411212", "421112",
    "421211", "212141", "214121", "412121", "111143", "111341", "131141", "114113",
    "114311", "411113", "411311", "113141", "114131", "311141", "411131", "211412",
    "211214", "211232", "2331112",
];

fn encode_code128(data: &str, gs1: bool, quiet: u32) -> Result<EncodedBarcode, String> {
    if data.is_empty() {
        return Err("barcode data is empty".into());
    }
    let payload: String = if gs1 {
        // Strip visual AI parentheses, e.g. "(01)01234567890128(17)261231".
        data.chars().filter(|&c| c != '(' && c != ')').collect()
    } else {
        data.to_string()
    };
    if payload.is_empty() {
        return Err("GS1-128 barcode payload is empty".into());
    }
    let bytes = payload.as_bytes();
    for &b in bytes {
        if b != 0x1d && !(32..=126).contains(&b) {
            return Err(format!("Code 128 supports printable ASCII (32..=126); got byte 0x{b:02x}"));
        }
    }

    let digit_run = |pos: usize| -> usize {
        bytes[pos..].iter().take_while(|b| b.is_ascii_digit()).count()
    };

    let mut codes: Vec<u8> = Vec::with_capacity(bytes.len() + 6);
    let mut pos = 0usize;
    let mut in_c = digit_run(0) >= 4 || (digit_run(0) == bytes.len() && bytes.len() >= 2 && bytes.len() % 2 == 0);
    if gs1 {
        codes.push(if in_c { C128_START_C } else { C128_START_B });
        codes.push(C128_FNC1);
    } else {
        codes.push(if in_c { C128_START_C } else { C128_START_B });
    }

    while pos < bytes.len() {
        if bytes[pos] == 0x1d {
            codes.push(C128_FNC1);
            pos += 1;
            continue;
        }
        if in_c {
            let run = digit_run(pos);
            if run >= 2 {
                let pair = (bytes[pos] - b'0') * 10 + (bytes[pos + 1] - b'0');
                codes.push(pair);
                pos += 2;
            } else {
                codes.push(C128_CODE_B);
                in_c = false;
            }
        } else {
            let run = digit_run(pos);
            if run >= 4 || (run >= 2 && pos + run == bytes.len() && run % 2 == 0) {
                if run % 2 == 1 {
                    codes.push(bytes[pos] - 32);
                    pos += 1;
                }
                codes.push(C128_CODE_C);
                in_c = true;
            } else {
                codes.push(bytes[pos] - 32);
                pos += 1;
            }
        }
    }

    let mut sum: u32 = codes[0] as u32;
    for (i, &c) in codes.iter().enumerate().skip(1) {
        sum += (i as u32) * (c as u32);
    }
    codes.push((sum % 103) as u8);
    codes.push(C128_STOP);

    let mut bars = Vec::new();
    let mut x = 0u32;
    for code in codes {
        let pat = C128_PATTERNS[code as usize];
        for (idx, ch) in pat.bytes().enumerate() {
            let w = (ch - b'0') as u32;
            if idx % 2 == 0 {
                bars.push(Bar { x, w, tall: false });
            }
            x += w;
        }
    }
    Ok(EncodedBarcode {
        format: if gs1 { BarcodeFormat::Gs1128 } else { BarcodeFormat::Code128 },
        symbol_modules: x,
        quiet,
        bars,
        label: data.to_string(),
    })
}

// =========================================================================
// EAN-13 / ISBN-13 and UPC-A
// =========================================================================

const EAN_L: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011",
    "0110001", "0101111", "0111011", "0110111", "0001011",
];
const EAN_G: [&str; 10] = [
    "0100111", "0110011", "0011011", "0100001", "0011101",
    "0111001", "0000101", "0010001", "0001001", "0010111",
];
const EAN_R: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100",
    "1001110", "1010000", "1000100", "1001000", "1110100",
];
const EAN13_PARITY: [&str; 10] = [
    "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG",
    "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL",
];

fn digits_only(data: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(data.len());
    for c in data.chars() {
        if c.is_ascii_digit() {
            out.push((c as u8) - b'0');
        } else if c == '-' || c == ' ' {
            continue;
        } else {
            return Err(format!("expected digits (and optional hyphens/spaces), found '{c}'"));
        }
    }
    Ok(out)
}

fn ean13_check(d: &[u8]) -> u8 {
    let sum: u32 = d.iter().enumerate().map(|(i, &v)| if i % 2 == 0 { v as u32 } else { 3 * v as u32 }).sum();
    ((10 - (sum % 10)) % 10) as u8
}

fn push_bits(bars: &mut Vec<Bar>, x: &mut u32, bits: &str, tall: bool) {
    let bytes = bits.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'1' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'1' {
                i += 1;
            }
            let w = (i - start) as u32;
            bars.push(Bar { x: *x, w, tall });
            *x += w;
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] == b'0' {
                i += 1;
            }
            *x += (i - start) as u32;
        }
    }
}

fn encode_ean13(data: &str, format: BarcodeFormat, quiet: u32) -> Result<EncodedBarcode, String> {
    let mut d = digits_only(data)?;
    match d.len() {
        12 => d.push(ean13_check(&d)),
        13 => {
            let expected = ean13_check(&d[..12]);
            if d[12] != expected {
                return Err(format!("EAN-13/ISBN-13 check digit mismatch: expected {expected}, got {}", d[12]));
            }
        }
        n => return Err(format!("EAN-13/ISBN-13 needs 12 or 13 digits, got {n}")),
    }
    let parity = EAN13_PARITY[d[0] as usize].as_bytes();
    let mut bars = Vec::with_capacity(32);
    let mut x = 0u32;
    push_bits(&mut bars, &mut x, "101", true);
    for i in 0..6 {
        let dig = d[i + 1] as usize;
        let bits = if parity[i] == b'L' { EAN_L[dig] } else { EAN_G[dig] };
        push_bits(&mut bars, &mut x, bits, false);
    }
    push_bits(&mut bars, &mut x, "01010", true);
    for i in 0..6 {
        let dig = d[i + 7] as usize;
        push_bits(&mut bars, &mut x, EAN_R[dig], false);
    }
    push_bits(&mut bars, &mut x, "101", true);
    let label: String = if format == BarcodeFormat::Isbn13 && data.contains('-') {
        data.trim().to_string()
    } else {
        let s: String = d.iter().map(|v| (b'0' + v) as char).collect();
        format!("{} {} {}", &s[0..1], &s[1..7], &s[7..13])
    };
    Ok(EncodedBarcode { format, symbol_modules: x, quiet, bars, label })
}

fn upca_check(d: &[u8]) -> u8 {
    let sum: u32 = d.iter().enumerate().map(|(i, &v)| if i % 2 == 0 { 3 * v as u32 } else { v as u32 }).sum();
    ((10 - (sum % 10)) % 10) as u8
}

fn encode_upca(data: &str, quiet: u32) -> Result<EncodedBarcode, String> {
    let mut d = digits_only(data)?;
    match d.len() {
        11 => d.push(upca_check(&d)),
        12 => {
            let expected = upca_check(&d[..11]);
            if d[11] != expected {
                return Err(format!("UPC-A check digit mismatch: expected {expected}, got {}", d[11]));
            }
        }
        n => return Err(format!("UPC-A needs 11 or 12 digits, got {n}")),
    }
    let mut bars = Vec::with_capacity(32);
    let mut x = 0u32;
    push_bits(&mut bars, &mut x, "101", true);
    for i in 0..6 {
        push_bits(&mut bars, &mut x, EAN_L[d[i] as usize], i == 0);
    }
    push_bits(&mut bars, &mut x, "01010", true);
    for i in 6..12 {
        push_bits(&mut bars, &mut x, EAN_R[d[i] as usize], i == 11);
    }
    push_bits(&mut bars, &mut x, "101", true);
    let s: String = d.iter().map(|v| (b'0' + v) as char).collect();
    let label = format!("{} {} {} {}", &s[0..1], &s[1..6], &s[6..11], &s[11..12]);
    Ok(EncodedBarcode { format: BarcodeFormat::UpcA, symbol_modules: x, quiet, bars, label })
}

// =========================================================================
// Code 39
// =========================================================================

fn code39_pattern(c: char) -> Option<&'static str> {
    Some(match c {
        '0' => "nnnwwnwnn",
        '1' => "wnnwnnnnw",
        '2' => "nnwwnnnnw",
        '3' => "wnwwnnnnn",
        '4' => "nnnwwnnnw",
        '5' => "wnnwwnnnn",
        '6' => "nnwwwnnnn",
        '7' => "nnnwnnwnw",
        '8' => "wnnwnnwnn",
        '9' => "nnwwnnwnn",
        'A' => "wnnnnwnnw",
        'B' => "nnwnnwnnw",
        'C' => "wnwnnwnnn",
        'D' => "nnnnwwnnw",
        'E' => "wnnnwwnnn",
        'F' => "nnwnwwnnn",
        'G' => "nnnnnwwnw",
        'H' => "wnnnnwwnn",
        'I' => "nnwnnwwnn",
        'J' => "nnnnwwwnn",
        'K' => "wnnnnnnww",
        'L' => "nnwnnnnww",
        'M' => "wnwnnnnwn",
        'N' => "nnnnwnnww",
        'O' => "wnnnwnnwn",
        'P' => "nnwnwnnwn",
        'Q' => "nnnnnnwww",
        'R' => "wnnnnnwwn",
        'S' => "nnwnnnwwn",
        'T' => "nnnnwnwwn",
        'U' => "wwnnnnnnw",
        'V' => "nwwnnnnnw",
        'W' => "wwwnnnnnn",
        'X' => "nwnnwnnnw",
        'Y' => "wwnnwnnnn",
        'Z' => "nwwnwnnnn",
        '-' => "nwnnnnwnw",
        '.' => "wwnnnnwnn",
        ' ' => "nwwnnnwnn",
        '$' => "nwnwnwnnn",
        '/' => "nwnwnnnwn",
        '+' => "nwnnnwnwn",
        '%' => "nnnwnwnwn",
        '*' => "nwnnwnwnn",
        _ => return None,
    })
}

fn encode_code39(data: &str, quiet: u32) -> Result<EncodedBarcode, String> {
    let trimmed = data.trim().trim_matches('*').to_ascii_uppercase();
    if trimmed.is_empty() {
        return Err("Code 39 barcode data is empty".into());
    }
    let mut seq = Vec::with_capacity(trimmed.len() + 2);
    seq.push('*');
    for ch in trimmed.chars() {
        if code39_pattern(ch).is_none() || ch == '*' {
            return Err(format!("Code 39 does not support character '{ch}'"));
        }
        seq.push(ch);
    }
    seq.push('*');

    let mut bars = Vec::with_capacity(seq.len() * 5);
    let mut x = 0u32;
    for (ci, ch) in seq.iter().enumerate() {
        let pat = code39_pattern(*ch).unwrap();
        for (idx, elem) in pat.bytes().enumerate() {
            let w = if elem == b'w' { 3 } else { 1 };
            if idx % 2 == 0 {
                bars.push(Bar { x, w, tall: false });
            }
            x += w;
        }
        if ci + 1 < seq.len() {
            x += 1; // inter-character narrow gap
        }
    }
    Ok(EncodedBarcode {
        format: BarcodeFormat::Code39,
        symbol_modules: x,
        quiet,
        bars,
        label: format!("*{trimmed}*"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code128_encodes_ascii_and_numeric_runs() {
        let bc = encode("INV-2026-0042", Some("code128"), None).unwrap();
        assert_eq!(bc.format, BarcodeFormat::Code128);
        assert!(bc.symbol_modules > 60);
        // Stop code ends with a 2-module bar.
        assert_eq!(bc.bars.last().unwrap().w, 2);
        let svg = to_svg(&bc, 180.0, 48.0, "#000000", None, false);
        assert!(svg.contains("<svg ") && svg.contains("<path d="));
    }

    #[test]
    fn gs1_128_strips_parens_and_starts_with_fnc1() {
        let bc = encode("(01)09506000134352(17)261231", Some("gs1-128"), None).unwrap();
        assert_eq!(bc.format, BarcodeFormat::Gs1128);
        assert_eq!(bc.label, "(01)09506000134352(17)261231");
        assert!(bc.symbol_modules > 100);
    }

    #[test]
    fn isbn13_computes_and_verifies_check_digit() {
        let a = encode("978-0-306-40615", Some("isbn13"), None).unwrap();
        let b = encode("978-0-306-40615-7", Some("isbn13"), None).unwrap();
        assert_eq!(a.bars, b.bars);
        assert_eq!(a.symbol_modules, 95);
        assert!(encode("978-0-306-40615-0", Some("isbn13"), None).is_err());
    }

    #[test]
    fn upca_encodes_95_modules() {
        let a = encode("03600029145", Some("upca"), None).unwrap();
        let b = encode("036000291452", Some("upca"), None).unwrap();
        assert_eq!(a.bars, b.bars);
        assert_eq!(a.symbol_modules, 95);
        assert_eq!(b.label, "0 36000 29145 2");
    }

    #[test]
    fn code39_encodes_uppercase() {
        let bc = encode("sku-42a", Some("code39"), None).unwrap();
        assert_eq!(bc.label, "*SKU-42A*");
        // 9 chars (7 + 2 sentinels): each char is 3*3 + 6*1 = 15 modules, plus 8 gaps = 143.
        assert_eq!(bc.symbol_modules, 9 * 15 + 8);
    }
}
