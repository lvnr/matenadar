use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;

/// Fonts: the document's own folders first, then the system's, then Typst's
/// bundled faces. Scanning is the slow part of starting, so a set of fonts
/// is built once and shared.
pub struct Fonts {
    store: FontStore,
}

impl Fonts {
    /// Fonts found in folders and, if `system`, the system's. In WebAssembly
    /// there are no folders to scan: use [`Fonts::from_data`].
    pub fn new(font_dirs: &[PathBuf], system: bool) -> Self {
        #[allow(unused_mut)]
        let mut store = FontStore::new();
        #[cfg(not(target_arch = "wasm32"))]
        {
            for dir in font_dirs {
                store.extend(typst_kit::fonts::scan(dir));
            }
            if system {
                store.extend(typst_kit::fonts::system());
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (font_dirs, system);
        Self::finish(store)
    }

    /// Fonts given as the contents of font files (each may hold several faces).
    pub fn from_data(files: impl IntoIterator<Item = Bytes>) -> Self {
        let mut store = FontStore::new();
        for data in files {
            for font in Font::iter(data) {
                let info = font.info().clone();
                store.push((font, info));
            }
        }
        Self::finish(store)
    }

    fn finish(#[allow(unused_mut)] mut store: FontStore) -> Self {
        #[cfg(feature = "embedded-fonts")]
        store.extend(typst_kit::fonts::embedded());
        Self { store }
    }

    pub fn book(&self) -> &LazyHash<FontBook> {
        self.store.book()
    }

    pub fn font(&self, index: usize) -> Option<Font> {
        self.store.font(index)
    }

    /// The font a family name and variant select, as Typst would.
    pub fn select(&self, family: &str, weight: u16, italic: bool) -> Option<Font> {
        use typst::text::{FontStretch, FontStyle, FontVariant, FontWeight};
        let variant = FontVariant::new(
            if italic { FontStyle::Italic } else { FontStyle::Normal },
            FontWeight::from_number(weight),
            FontStretch::NORMAL,
        );
        let id = self.book().select(&family.to_lowercase(), variant)?;
        self.font(id)
    }

    /// Every family name, sorted.
    pub fn families(&self) -> Vec<String> {
        let mut v: Vec<String> = self.book().families().map(|(name, _)| name.to_string()).collect();
        v.sort();
        v.dedup();
        v
    }
}

/// Files made by the compiler (drawings, codes, lettering), served under `/@mt/`.
#[derive(Clone, Default)]
pub struct Virtual(Arc<Mutex<HashMap<String, Bytes>>>);

impl Virtual {
    pub fn insert(&self, name: &str, data: Vec<u8>) -> String {
        let path = format!("/@mt/{name}");
        self.0.lock().expect("virtual files").insert(path.clone(), Bytes::new(data));
        path
    }

    pub fn get(&self, path: &str) -> Option<Bytes> {
        self.0.lock().expect("virtual files").get(path).cloned()
    }

    pub fn clear(&self) {
        self.0.lock().expect("virtual files").clear();
    }
}

/// The document's own files (pictures, drawings): given in memory, or read
/// from under the asset root. In the browser there is no file system, so
/// every file is given.
#[derive(Clone, Default)]
pub struct Files {
    root: Option<PathBuf>,
    /// Given files by path under the root, with a hash of their contents.
    given: Arc<Mutex<HashMap<String, (Bytes, u128)>>>,
}

impl Files {
    /// Files under `root`, plus any given later.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: Some(root.into()), given: Default::default() }
    }

    /// Only files that are given.
    pub fn memory() -> Self {
        Self::default()
    }

    /// Gives (or replaces) the file at `path`, relative to the root.
    pub fn give(&self, path: &str, data: Vec<u8>) {
        let data = Bytes::new(data);
        let hash = typst::utils::hash128(&data);
        self.given.lock().expect("files").insert(clean(path).to_string(), (data, hash));
    }

    pub fn read(&self, path: &str) -> Option<Bytes> {
        let rel = clean(path);
        if rel.split('/').any(|c| c == "..") {
            return None;
        }
        if let Some((data, _)) = self.given.lock().expect("files").get(rel) {
            return Some(data.clone());
        }
        fs::read(self.root.as_ref()?.join(rel)).ok().map(Bytes::new)
    }

    pub fn exists(&self, path: &str) -> bool {
        let rel = clean(path);
        self.given.lock().expect("files").contains_key(rel) || self.root.as_ref().is_some_and(|r| r.join(rel).is_file())
    }

    /// A number that changes when the file does: a given file's hash, or a
    /// file's modification time.
    pub fn version(&self, path: &str) -> Option<u128> {
        let rel = clean(path);
        if let Some((_, hash)) = self.given.lock().expect("files").get(rel) {
            return Some(*hash);
        }
        let modified = fs::metadata(self.root.as_ref()?.join(rel)).and_then(|m| m.modified()).ok()?;
        modified.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_nanos())
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }
}

fn clean(path: &str) -> &str {
    path.trim_start_matches('/')
}

/// The Typst world for one generated document: its source, fonts, the
/// document's files and the compiler's own files. No packages, no network,
/// nothing outside the root.
pub struct MtWorld {
    library: LazyHash<Library>,
    fonts: Arc<Fonts>,
    main: Source,
    files: Files,
    virt: Virtual,
}

impl MtWorld {
    pub fn new(source: String, files: Files, fonts: Arc<Fonts>, virt: Virtual) -> Self {
        let id = RootedPath::new(VirtualRoot::Project, VirtualPath::new("main.typ").expect("valid path")).intern();
        Self { library: LazyHash::new(Library::default()), fonts, main: Source::new(id, source), files, virt }
    }

    /// Replaces the source. Typst reparses only the changed range.
    pub fn set_source(&mut self, text: &str) {
        self.main.replace(text);
    }

    pub fn main_source(&self) -> &Source {
        &self.main
    }

    pub fn files(&self) -> &Files {
        &self.files
    }

    pub fn fonts(&self) -> &Arc<Fonts> {
        &self.fonts
    }
}

impl World for MtWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() { Ok(self.main.clone()) } else { Err(FileError::AccessDenied) }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if !matches!(id.root(), VirtualRoot::Project) {
            return Err(FileError::AccessDenied);
        }
        let vpath = id.vpath().get_with_slash();
        if vpath.starts_with("/@mt/") {
            return self.virt.get(vpath).ok_or_else(|| FileError::NotFound(PathBuf::from(vpath)));
        }
        self.files.read(vpath).ok_or_else(|| FileError::NotFound(PathBuf::from(vpath)))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

impl typst_ide::IdeWorld for MtWorld {
    fn upcast(&self) -> &dyn World {
        self
    }
}
