//! Saves and the profile in the browser (M 5.2, M 5.6). The core's store
//! is synchronous, and the worker's storage, the origin private file
//! system (OPFS), is not: a [`Journal`] keeps the files in memory, read
//! from OPFS before the game starts, and lists what changed, which the
//! worker writes to OPFS in the background after each step.
//!
//! Paths are the data folder's (D 3.2): `saves/auto-1`, `profile`. A file
//! the worker writes with `createWritable` takes its new text when the
//! stream closes, so a page closed mid-write leaves the old one.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use rlml_core::rt::store::Store;

#[derive(Default)]
struct Inner {
    files: BTreeMap<String, String>,
    /// Writes (`Some`) and removals (`None`) in order, for the worker.
    changes: Vec<(String, Option<String>)>,
}

/// The game's data folder in memory, shared by its stores.
#[derive(Clone, Default)]
pub struct Journal(Rc<RefCell<Inner>>);

impl Journal {
    /// The folder as OPFS has it, `(path, text)` each.
    pub fn new(files: Vec<(String, String)>) -> Journal {
        Journal(Rc::new(RefCell::new(Inner { files: files.into_iter().collect(), changes: vec![] })))
    }

    /// A store of the files under `dir` (`saves/`), or of the folder's
    /// own files with `""`.
    pub fn store(&self, dir: &str) -> JournalStore {
        JournalStore { j: self.clone(), dir: dir.to_string() }
    }

    /// What changed since the last call, in order.
    pub fn take_changes(&self) -> Vec<(String, Option<String>)> {
        std::mem::take(&mut self.0.borrow_mut().changes)
    }

    pub fn files(&self) -> BTreeMap<String, String> {
        self.0.borrow().files.clone()
    }
}

pub struct JournalStore {
    j: Journal,
    dir: String,
}

impl JournalStore {
    fn path(&self, name: &str) -> String {
        format!("{}{name}", self.dir)
    }
}

impl Store for JournalStore {
    fn read(&self, name: &str) -> Option<String> {
        self.j.0.borrow().files.get(&self.path(name)).cloned()
    }

    fn write(&mut self, name: &str, text: &str) -> Result<(), String> {
        let path = self.path(name);
        let mut inner = self.j.0.borrow_mut();
        inner.files.insert(path.clone(), text.to_string());
        inner.changes.push((path, Some(text.to_string())));
        Ok(())
    }

    fn remove(&mut self, name: &str) {
        let path = self.path(name);
        let mut inner = self.j.0.borrow_mut();
        if inner.files.remove(&path).is_some() {
            inner.changes.push((path, None));
        }
    }

    fn list(&self) -> Vec<String> {
        let inner = self.j.0.borrow();
        inner.files.keys().filter_map(|p| p.strip_prefix(self.dir.as_str())).filter(|n| !n.contains('/')).map(String::from).collect()
    }
}
