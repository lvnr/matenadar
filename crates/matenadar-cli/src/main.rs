//! `matenadar`: build, check, inspect and change Matenadar documents.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

mod bench;
mod serve;

use anyhow::{Context, Result, anyhow, bail};
use matenadar_model::{Document, Op, validate};
use matenadar_typst::{CheckOptions, Paths, Session, Severity, check, inspect};

const HELP: &str = "matenadar — lays out Matenadar documents

Usage:
  matenadar build <doc.json> [-o out.pdf] [--png DIR] [--svg DIR] [--dpi N] [--pages 1-3,7] [--bleed] [--check]
  matenadar epub <doc.json> [-o out.epub] [--layout reflowable|fixed] [--dpi 200] [--embed-fonts]
                 [--cover image.jpg] [--identifier urn:isbn:…]
  matenadar check <doc.json> [--safe 0.25in] [--min-ppi 300]
  matenadar validate <doc.json>
  matenadar layout <doc.json> [--pages 1-3]
  matenadar locate <doc.json> --page N --x PT --y PT
  matenadar apply <doc.json> <ops.json> [-o new.json]
  matenadar fonts
  matenadar bench <doc.json> [--edits 50]
  matenadar serve                       (JSON lines on stdin; see serve.rs)

Common options:
  --root DIR          asset root (default: the document's folder)
  --fonts DIR         another font folder (repeatable)
  --no-system-fonts   don't use the system's fonts
  --json              machine-readable output
";

struct Args {
    cmd: String,
    pos: Vec<String>,
    opts: Vec<(String, Option<String>)>,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut it = std::env::args().skip(1);
        let cmd = it.next().unwrap_or_else(|| "help".into());
        let mut pos = Vec::new();
        let mut opts = Vec::new();
        let flags = ["--bleed", "--check", "--json", "--no-system-fonts", "--embed-fonts"];
        let mut rest: Vec<String> = it.collect();
        rest.reverse();
        while let Some(a) = rest.pop() {
            if let Some(name) = a.strip_prefix("--").map(|_| a.clone()).or_else(|| (a == "-o").then(|| "-o".to_string())) {
                if flags.contains(&name.as_str()) {
                    opts.push((name, None));
                } else {
                    let v = rest.pop().ok_or_else(|| anyhow!("{name} needs a value"))?;
                    opts.push((name, Some(v)));
                }
            } else {
                pos.push(a);
            }
        }
        Ok(Self { cmd, pos, opts })
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.opts.iter().rev().find(|(n, _)| n == name).and_then(|(_, v)| v.as_deref())
    }

    fn all(&self, name: &str) -> Vec<&str> {
        self.opts.iter().filter(|(n, _)| n == name).filter_map(|(_, v)| v.as_deref()).collect()
    }

    fn flag(&self, name: &str) -> bool {
        self.opts.iter().any(|(n, _)| n == name)
    }

    fn doc_path(&self) -> Result<PathBuf> {
        self.pos.first().map(PathBuf::from).ok_or_else(|| anyhow!("give the document's path"))
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn load(args: &Args) -> Result<(Document, Paths)> {
    let path = args.doc_path()?;
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let doc = Document::from_json(&text).map_err(|e| anyhow!("{}: {e}", path.display()))?;
    let root = match args.get("--root") {
        Some(r) => PathBuf::from(r),
        None => path.parent().map(Path::to_path_buf).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from(".")),
    };
    let paths = Paths {
        root: root.canonicalize().unwrap_or(root),
        font_dirs: args.all("--fonts").into_iter().map(PathBuf::from).collect(),
        system_fonts: !args.flag("--no-system-fonts"),
    };
    Ok((doc, paths))
}

fn session(args: &Args) -> Result<Session> {
    let (doc, paths) = load(args)?;
    let problems = validate(&doc);
    if !problems.is_empty() {
        let list: Vec<String> = problems.iter().map(|p| format!("  {}: {}", p.path, p.message)).collect();
        bail!("the document has problems:\n{}", list.join("\n"));
    }
    Session::new(doc, paths).map_err(|e| anyhow!("layout failed: {e}"))
}

/// Parses "1-3,7" into 1-based page numbers.
fn pages(spec: Option<&str>, count: usize) -> Result<Vec<usize>> {
    let Some(spec) = spec else { return Ok((1..=count).collect()) };
    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        match part.split_once('-') {
            Some((a, b)) => {
                let a: usize = a.parse()?;
                let b: usize = if b.is_empty() { count } else { b.parse()? };
                out.extend(a..=b.min(count));
            }
            None => out.push(part.parse()?),
        }
    }
    out.retain(|p| *p >= 1 && *p <= count);
    Ok(out)
}

fn run() -> Result<ExitCode> {
    let args = Args::parse()?;
    let json = args.flag("--json");
    match args.cmd.as_str() {
        "build" => {
            let start = Instant::now();
            let s = session(&args)?;
            let doc_path = args.doc_path()?;
            let out = args.get("-o").map(PathBuf::from).unwrap_or_else(|| doc_path.with_extension("pdf"));
            let pdf = s.pdf().map_err(|e| anyhow!("PDF export failed: {e}"))?;
            fs::write(&out, pdf)?;
            let wanted = pages(args.get("--pages"), s.page_count())?;
            let dpi: f32 = args.get("--dpi").map(str::parse).transpose()?.unwrap_or(100.0);
            let bleed = args.flag("--bleed");
            if let Some(dir) = args.get("--png") {
                fs::create_dir_all(dir)?;
                for p in &wanted {
                    let png = s.png(*p, dpi, bleed).ok_or_else(|| anyhow!("no page {p}"))?;
                    fs::write(Path::new(dir).join(format!("p-{p:03}.png")), png)?;
                }
            }
            if let Some(dir) = args.get("--svg") {
                fs::create_dir_all(dir)?;
                for p in &wanted {
                    let svg = s.svg(*p, bleed).ok_or_else(|| anyhow!("no page {p}"))?;
                    fs::write(Path::new(dir).join(format!("p-{p:03}.svg")), svg)?;
                }
            }
            let issues = if args.flag("--check") { check(&s, &check_options(&args)?) } else { Vec::new() };
            let total = start.elapsed();
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "pdf": out,
                        "pages": s.page_count(),
                        "layout_ms": s.compile_time().as_secs_f64() * 1000.0,
                        "total_ms": total.as_secs_f64() * 1000.0,
                        "problems": s.problems(),
                        "warnings": s.warnings(),
                        "issues": issues,
                    })
                );
            } else {
                println!(
                    "{} pages → {} (layout {:.0} ms, total {:.0} ms)",
                    s.page_count(),
                    out.display(),
                    s.compile_time().as_secs_f64() * 1000.0,
                    total.as_secs_f64() * 1000.0
                );
                for p in s.problems() {
                    println!("  problem: {p}");
                }
                for w in s.warnings() {
                    println!("  warning: {w}");
                }
                print_issues(&issues);
            }
            Ok(if issues.iter().any(|i| i.severity == Severity::Error) { ExitCode::from(1) } else { ExitCode::SUCCESS })
        }
        "check" => {
            let s = session(&args)?;
            let issues = check(&s, &check_options(&args)?);
            if json {
                println!("{}", serde_json::to_string_pretty(&issues)?);
            } else if issues.is_empty() {
                println!("no issues on {} pages", s.page_count());
            } else {
                print_issues(&issues);
            }
            Ok(if issues.iter().any(|i| i.severity == Severity::Error) { ExitCode::from(1) } else { ExitCode::SUCCESS })
        }
        "validate" => {
            let (doc, _) = load(&args)?;
            let problems = validate(&doc);
            if json {
                println!("{}", serde_json::to_string_pretty(&problems)?);
            } else if problems.is_empty() {
                println!("valid");
            } else {
                for p in &problems {
                    println!("{}: {}", p.path, p.message);
                }
            }
            Ok(if problems.is_empty() { ExitCode::SUCCESS } else { ExitCode::from(1) })
        }
        "layout" => {
            let s = session(&args)?;
            let wanted = pages(args.get("--pages"), s.page_count())?;
            let report: Vec<_> = inspect::report(&s).into_iter().filter(|p| wanted.contains(&p.page)).collect();
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(ExitCode::SUCCESS)
        }
        "locate" => {
            let s = session(&args)?;
            let page: usize = args.get("--page").ok_or_else(|| anyhow!("--page"))?.parse()?;
            let x: f64 = args.get("--x").ok_or_else(|| anyhow!("--x"))?.parse()?;
            let y: f64 = args.get("--y").ok_or_else(|| anyhow!("--y"))?.parse()?;
            match s.locate(page, x, y) {
                Some((node, offset)) => println!("{}", serde_json::json!({ "node": node, "offset": offset })),
                None => println!("null"),
            }
            Ok(ExitCode::SUCCESS)
        }
        "apply" => {
            let mut s = session(&args)?;
            let ops_path = args.pos.get(1).ok_or_else(|| anyhow!("give the operations file"))?;
            let text = fs::read_to_string(ops_path)?;
            let ops: Vec<Op> = match serde_json::from_str::<Vec<Op>>(&text) {
                Ok(v) => v,
                Err(_) => vec![serde_json::from_str::<Op>(&text).context("reading operations")?],
            };
            let mut changes = Vec::new();
            for op in &ops {
                changes.push(s.apply(op).map_err(|e| anyhow!("{e}"))?);
            }
            let out = args.get("-o").map(PathBuf::from).unwrap_or(args.doc_path()?);
            fs::write(&out, s.document().to_json())?;
            println!("{}", serde_json::to_string_pretty(&changes)?);
            Ok(ExitCode::SUCCESS)
        }
        "fonts" => {
            let paths = Paths {
                root: PathBuf::from(args.get("--root").unwrap_or(".")),
                font_dirs: args.all("--fonts").into_iter().map(PathBuf::from).collect(),
                system_fonts: !args.flag("--no-system-fonts"),
            };
            let fonts = matenadar_typst::Fonts::new(&paths.font_dirs, paths.system_fonts);
            for f in fonts.families() {
                println!("{f}");
            }
            Ok(ExitCode::SUCCESS)
        }
        "bench" => {
            let s = session(&args)?;
            let edits: usize = args.get("--edits").map(str::parse).transpose()?.unwrap_or(50);
            println!("{}", serde_json::to_string_pretty(&bench::bench(s, edits)?)?);
            Ok(ExitCode::SUCCESS)
        }
        "serve" => {
            serve::serve()?;
            Ok(ExitCode::SUCCESS)
        }
        "epub" => {
            let start = Instant::now();
            let (doc, paths) = load(&args)?;
            let problems = validate(&doc);
            if !problems.is_empty() {
                let list: Vec<String> = problems.iter().map(|p| format!("  {}: {}", p.path, p.message)).collect();
                bail!("the document has problems:\n{}", list.join("\n"));
            }
            let out = args.get("-o").map(PathBuf::from).unwrap_or_else(|| args.doc_path().unwrap_or_default().with_extension("epub"));
            let opts = matenadar_epub::Options {
                layout: match args.get("--layout") {
                    Some("fixed") => matenadar_epub::Layout::Fixed,
                    Some("reflowable") | None => matenadar_epub::Layout::Reflowable,
                    Some(other) => bail!("--layout is reflowable or fixed, not {other}"),
                },
                dpi: args.get("--dpi").map(str::parse).transpose()?.unwrap_or(200.0),
                embed_fonts: args.flag("--embed-fonts"),
                cover: args.get("--cover").map(PathBuf::from),
                identifier: args.get("--identifier").map(str::to_string),
            };
            let epub = matenadar_epub::build(&doc, &paths, &opts).map_err(|e| anyhow!("EPUB failed: {e}"))?;
            fs::write(&out, &epub)?;
            let total = start.elapsed();
            if json {
                println!("{}", serde_json::json!({"epub": out, "bytes": epub.len(), "total_ms": total.as_secs_f64() * 1000.0}));
            } else {
                println!("{} ({:.1} MB, {} ms)", out.display(), epub.len() as f64 / 1e6, total.as_millis());
            }
            Ok(ExitCode::SUCCESS)
        }
        "debug-typst" => {
            let (doc, paths) = load(&args)?;
            print!("{}", matenadar_typst::source_of(&doc, &paths));
            Ok(ExitCode::SUCCESS)
        }
        "help" | "--help" | "-h" => {
            print!("{HELP}");
            Ok(ExitCode::SUCCESS)
        }
        other => bail!("unknown command \"{other}\"\n\n{HELP}"),
    }
}

fn check_options(args: &Args) -> Result<CheckOptions> {
    let mut o = CheckOptions::default();
    if let Some(s) = args.get("--safe") {
        o.safe_pt = matenadar_model::Length::parse(s).map_err(|e| anyhow!(e))?.resolve(11.0, 0.0);
    }
    if let Some(g) = args.get("--gutter") {
        o.gutter_pt = Some(matenadar_model::Length::parse(g).map_err(|e| anyhow!(e))?.resolve(11.0, 0.0));
    }
    if let Some(p) = args.get("--min-ppi") {
        o.min_ppi = p.parse()?;
    }
    Ok(o)
}

fn print_issues(issues: &[matenadar_typst::Issue]) {
    for i in issues {
        let sev = match i.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        let at = match (&i.page, &i.node) {
            (Some(p), Some(n)) => format!(" [p{p} {n}]"),
            (Some(p), None) => format!(" [p{p}]"),
            (None, Some(n)) => format!(" [{n}]"),
            _ => String::new(),
        };
        println!("  {sev} {}{at}: {}", i.code, i.message);
    }
}
