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
    pub fn new(font_dirs: &[PathBuf], system: bool) -> Self {
        let mut store = FontStore::new();
        for dir in font_dirs {
            store.extend(typst_kit::fonts::scan(dir));
        }
        if system {
            store.extend(typst_kit::fonts::system());
        }
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

/// The Typst world for one generated document: its source, fonts, the
/// files under the asset root and the compiler's own files. No packages,
/// no network, nothing outside the root.
pub struct MtWorld {
    library: LazyHash<Library>,
    fonts: Arc<Fonts>,
    main: Source,
    root: PathBuf,
    virt: Virtual,
}

impl MtWorld {
    pub fn new(source: String, root: PathBuf, fonts: Arc<Fonts>, virt: Virtual) -> Self {
        let id = RootedPath::new(VirtualRoot::Project, VirtualPath::new("main.typ").expect("valid path")).intern();
        Self { library: LazyHash::new(Library::default()), fonts, main: Source::new(id, source), root, virt }
    }

    /// Replaces the source. Typst reparses only the changed range.
    pub fn set_source(&mut self, text: &str) {
        self.main.replace(text);
    }

    pub fn main_source(&self) -> &Source {
        &self.main
    }

    pub fn root(&self) -> &Path {
        &self.root
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
        let path = id.vpath().realize(&self.root).map_err(|_| FileError::AccessDenied)?;
        fs::read(&path).map(Bytes::new).map_err(|e| FileError::from_io(e, &path))
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
