//! Taking a development mod into the game (D 3.6, RLML-kehitystyokalut.md
//! steps 7 and 8).
//!
//! `rlml mod export` copies the mod's files under `mods/NAME/` and adds an
//! include line at the end of the game's main file. The export is planned
//! in memory first: the game loads with the mod and, through an
//! [`Overlay`], as exported without it, and the two models' resolved
//! definitions must be the same. Only then does the front end write.

use std::collections::BTreeMap;

mod merge;
pub use merge::{Merge, merge, unified};

use crate::load::{Files, Layers, Root};
use crate::model::Model;
use crate::syntax::Entry;

/// The game's files with some of the game root's files written or removed:
/// what the files would be after an export, for loading before writing.
pub struct Overlay<'a> {
    pub base: &'a dyn Files,
    /// Game root paths and their new text; `None` removes the file.
    pub put: BTreeMap<String, Option<String>>,
}

impl Files for Overlay<'_> {
    fn read(&self, root: Root, path: &str) -> Option<String> {
        match (root, self.put.get(path)) {
            (Root::Game, Some(text)) => text.clone(),
            _ => self.base.read(root, path),
        }
    }

    fn list(&self, root: Root, dir: &str) -> Option<Vec<String>> {
        let mut out = self.base.list(root, dir).unwrap_or_default();
        if root == Root::Game {
            let prefix = if dir.is_empty() {
                String::new()
            } else {
                format!("{dir}/")
            };
            for (path, text) in &self.put {
                let Some(rest) = path.strip_prefix(&prefix) else {
                    continue;
                };
                let entry = match rest.split_once('/') {
                    Some((folder, _)) => format!("{folder}/"),
                    None => rest.to_string(),
                };
                if text.is_some() && !out.contains(&entry) {
                    out.push(entry);
                } else if text.is_none() && !entry.ends_with('/') {
                    out.retain(|e| *e != entry);
                }
            }
            // a folder whose files are all removed is gone
            out.retain(|e| {
                !e.ends_with('/') || {
                    let sub = format!("{prefix}{e}");
                    !self.put.keys().any(|p| p.starts_with(&sub))
                        || all_files(self, &sub[..sub.len() - 1]).next().is_some()
                }
            });
        }
        (!out.is_empty()).then_some(out)
    }
}

/// Every file under `dir` in the game root, by path, in order.
fn all_files<'a>(files: &'a dyn Files, dir: &str) -> impl Iterator<Item = String> + 'a {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_string()];
    while let Some(d) = stack.pop() {
        for e in files.list(Root::Game, &d).unwrap_or_default() {
            let path = if d.is_empty() {
                e.clone()
            } else {
                format!("{d}/{e}")
            };
            match path.strip_suffix('/') {
                Some(folder) => stack.push(folder.to_string()),
                None if files.read(Root::Game, &path).is_some() => out.push(path),
                None => {}
            }
        }
    }
    out.sort();
    out.into_iter()
}

/// `text` without its places: `pos: Pos { … }`, `file: "…"` and the
/// `line:` and `col:` numbers of a parse tree's debug form.
fn unplaced(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let next = ["pos: Pos {", "file: \"", "line: ", "col: "]
            .iter()
            .filter_map(|p| rest.find(p).map(|i| (i, *p)))
            .min();
        let Some((i, p)) = next else {
            break;
        };
        out.push_str(&rest[..i]);
        rest = &rest[i + p.len()..];
        rest = match p {
            "pos: Pos {" => rest.find('}').map_or("", |j| &rest[j + 1..]),
            "file: \"" => rest.find('"').map_or("", |j| &rest[j + 1..]),
            _ => rest.trim_start_matches(|c: char| c.is_ascii_digit()),
        };
    }
    out.push_str(rest);
    out
}

/// The model's resolved definitions without where they were written: what
/// an export must keep (RLML-kehitystyokalut.md step 7). Keys name a
/// definition, a block or a declaration in load order.
pub fn resolved(m: &Model) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (full, d) in &m.defs {
        out.insert(
            format!("{:?} {full}", d.kind),
            unplaced(&format!("{} {:?}", d.overrides, d.source)),
        );
    }
    let blocks = [("game", &m.games), ("profile", &m.profiles)];
    for (what, list) in blocks {
        for (i, b) in list.iter().enumerate() {
            out.insert(
                format!("{what} {} #{i}", b.name),
                unplaced(&format!("{b:?}")),
            );
        }
    }
    for (i, b) in m.sections.iter().enumerate() {
        out.insert(format!("{} #{i}", b.kind), unplaced(&format!("{b:?}")));
    }
    for (name, list) in &m.contexts {
        out.insert(format!("keys {name}"), unplaced(&format!("{list:?}")));
    }
    for (i, d) in m.code.iter().enumerate() {
        out.insert(
            format!("code #{i} {:?} {}", d.kind, d.name),
            unplaced(&format!("{d:?}")),
        );
    }
    for (i, l) in m.languages.iter().enumerate() {
        out.insert(
            format!("language {} #{i}", l.language),
            unplaced(&format!("{:?} {:?}", l.namespace, l.texts)),
        );
    }
    out
}

/// What differs between two resolved sets, a line each.
pub fn differences(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in before {
        match after.get(k) {
            None => out.push(format!("only with the mod: {k}")),
            Some(w) if w != v => out.push(format!("changed: {k}")),
            Some(_) => {}
        }
    }
    for k in after.keys().filter(|k| !before.contains_key(*k)) {
        out.push(format!("only in the export: {k}"));
    }
    out
}

/// A planned export: the files to write, the main file's new text and
/// where the mod's folder goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Export {
    /// New game root paths and their text.
    pub files: Vec<(String, String)>,
    /// The main file's path and new text.
    pub main: (String, String),
    /// The mod's folder and its archive folder.
    pub archive: (String, String),
    /// The include line added.
    pub include: String,
}

/// Plans exporting the mod `name` of the game at `main` (D 3.6): the mod's
/// files under `mods/NAME/` without its header, and an include line at
/// the end of the main file. The game loads with the mod and as exported,
/// and an export that changes the resolved definitions is refused.
pub fn export(files: &dyn Files, main: &str, name: &str) -> Result<Export, Vec<String>> {
    let fail = |e: String| Err(vec![e]);
    let from = format!("dev/{name}");
    let mod_files: Vec<String> = all_files(files, &from).collect();
    if name.is_empty() || name.contains('/') || name.starts_with('.') || mod_files.is_empty() {
        return fail(format!(
            "no mod `{name}`: there is no folder dev/{name}/ (D 3.6)"
        ));
    }
    let to = format!("mods/{name}");
    if files.list(Root::Game, &to).is_some() {
        return fail(format!(
            "{to}/ is there already: the mod is exported, or the name is taken"
        ));
    }
    let include = format!("include {to}/**");
    let Some(main_text) = files.read(Root::Game, main) else {
        return fail(format!("{main}: main file not found"));
    };
    if main_text.lines().any(|l| l.trim() == include) {
        return fail(format!("{main} has `{include}` already"));
    }
    // the game with the mod, as it is played
    let layers = Layers {
        mods: vec![name.to_string()],
        overrides: false,
    };
    let (with, report) = crate::check_game_with(files, main, &layers);
    if report.has_errors() {
        return Err(report.errors().map(|d| d.to_string()).collect());
    }
    let about = with
        .mod_infos
        .iter()
        .find(|i| i.folder == name)
        .map(|i| i.about.clone())
        .unwrap_or_default();
    // the files go as they are, but the header stays in dev/
    let mut out = Vec::new();
    for path in &mod_files {
        let text = files.read(Root::Game, path).unwrap_or_default();
        if header_only(path, &text)? {
            continue;
        }
        let rest = &path[from.len()..];
        out.push((format!("{to}{rest}"), text));
    }
    let mut new_main = main_text.clone();
    if !new_main.ends_with('\n') {
        new_main.push('\n');
    }
    new_main.push('\n');
    if !about.is_empty() {
        new_main.push_str(&format!("> Modi {name}: {about}\n"));
    }
    new_main.push_str(&include);
    new_main.push('\n');
    // the game as exported, without the mod
    let mut put: BTreeMap<String, Option<String>> =
        mod_files.iter().map(|p| (p.clone(), None)).collect();
    put.extend(out.iter().map(|(p, t)| (p.clone(), Some(t.clone()))));
    put.insert(main.to_string(), Some(new_main.clone()));
    let overlay = Overlay { base: files, put };
    let (after, report) = crate::check_game_with(&overlay, main, &Layers::default());
    if report.has_errors() {
        let mut e = vec!["the exported game does not load:".to_string()];
        e.extend(report.errors().map(|d| d.to_string()));
        return Err(e);
    }
    let diff = differences(&resolved(&with), &resolved(&after));
    if !diff.is_empty() {
        let mut e = vec!["the export would change the game's definitions:".to_string()];
        e.extend(diff);
        return Err(e);
    }
    Ok(Export {
        files: out,
        main: (main.to_string(), new_main),
        archive: (from, archive_folder(files, name)),
        include,
    })
}

/// Is the file the mod's header and nothing else? A header with other
/// blocks in its file cannot be left behind.
fn header_only(path: &str, text: &str) -> Result<bool, Vec<String>> {
    if path.ends_with(".rogue") {
        return Ok(false);
    }
    let parsed = crate::syntax::parse(path, text)
        .map_err(|ds| ds.iter().map(|d| d.to_string()).collect::<Vec<_>>())?;
    let is_header = |e: &Entry| matches!(e, Entry::Block(b) if b.kind == "mod");
    if !parsed.entries.iter().any(is_header) {
        return Ok(false);
    }
    if parsed.entries.iter().all(is_header) && parsed.language.is_none() {
        return Ok(true);
    }
    Err(vec![format!(
        "{path}: the `mod` header shares its file with other blocks; move it to its own file, mod.rlml (D 3.6)"
    )])
}

/// `dev/.arkisto/NAME`, or with a number when that is taken.
fn archive_folder(files: &dyn Files, name: &str) -> String {
    let mut n = 1;
    loop {
        let folder = if n == 1 {
            format!("dev/.arkisto/{name}")
        } else {
            format!("dev/.arkisto/{name}-{n}")
        };
        if files.list(Root::Game, &folder).is_none() {
            return folder;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_go() {
        assert_eq!(
            unplaced(
                r#"A { file: "dev/x/a.rlml", line: 12, pos: Pos { line: 3, col: 5 }, text: "b" }"#
            ),
            r#"A { , , , text: "b" }"#
        );
    }
}
