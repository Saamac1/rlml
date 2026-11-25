//! Finding and reading files (D 3, M 2.1).
//!
//! Loading starts from the game's main file and follows `include` lines
//! depth first: an included file's content loads at the include line, and
//! the same file loads only once. A path is looked up in three places: the
//! including file's folder, the game root and last the engine's library.
//! The result is one stream of definitions in load order.

use std::collections::{BTreeMap, HashSet};

use unicode_normalization::UnicodeNormalization;

use crate::error::Report;
use crate::syntax::{Entry, ParsedFile, RawItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Root {
    /// The game's project folder.
    Game,
    /// The engine's search path, which holds `lib/std.rlml`.
    Engine,
}

/// Read-only access to the game and the engine library. Paths use `/` and
/// are relative to the root.
pub trait Files {
    fn read(&self, root: Root, path: &str) -> Option<String>;
    /// Entries of folder `dir` (`""` is the root): files by name, folders
    /// with a trailing `/`. `None` when the folder does not exist.
    fn list(&self, root: Root, dir: &str) -> Option<Vec<String>>;
}

/// Files in memory, for tests and tools.
#[derive(Debug, Clone, Default)]
pub struct MemFiles {
    pub game: BTreeMap<String, String>,
    pub engine: BTreeMap<String, String>,
}

impl MemFiles {
    fn tree(&self, root: Root) -> &BTreeMap<String, String> {
        match root {
            Root::Game => &self.game,
            Root::Engine => &self.engine,
        }
    }
}

impl Files for MemFiles {
    fn read(&self, root: Root, path: &str) -> Option<String> {
        self.tree(root).get(path).cloned()
    }

    fn list(&self, root: Root, dir: &str) -> Option<Vec<String>> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        let mut out: Vec<String> = Vec::new();
        for path in self.tree(root).keys() {
            let Some(rest) = path.strip_prefix(&prefix) else {
                continue;
            };
            let entry = match rest.split_once('/') {
                Some((folder, _)) => format!("{folder}/"),
                None => rest.to_string(),
            };
            if !out.contains(&entry) {
                out.push(entry);
            }
        }
        (!out.is_empty()).then_some(out)
    }
}

/// A loaded file's place: its root and path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Place {
    pub root: Root,
    pub path: String,
}

impl Place {
    /// The name shown in diagnostics.
    pub fn display(&self) -> String {
        match self.root {
            Root::Game => self.path.clone(),
            Root::Engine => format!("<engine>/{}", self.path),
        }
    }

    fn dir(&self) -> &str {
        self.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
    }
}

/// Everything loaded, in load order.
#[derive(Debug, Clone, Default)]
pub struct Loaded {
    /// Blocks and code blocks; `include` lines are resolved away.
    pub entries: Vec<Entry>,
    /// Language files: language, namespace and `KEY text` lines.
    pub languages: Vec<LanguageFile>,
    /// Every loaded file in load order.
    pub files: Vec<Place>,
}

#[derive(Debug, Clone)]
pub struct LanguageFile {
    pub file: String,
    pub language: String,
    pub namespace: Option<String>,
    pub texts: Vec<RawItem>,
}

const EXTENSIONS: &[&str] = &[".rlml.md", ".rlml", ".md", ".rogue"];

fn has_data_extension(name: &str) -> bool {
    EXTENSIONS.iter().any(|e| name.ends_with(e))
}

/// Files a wildcard skips (D 3).
fn skipped_by_wildcard(name: &str) -> bool {
    name.starts_with('_')
        || name.starts_with('.')
        || ["README.md", "README.rlml", "README.rlml.md"].contains(&name)
}

fn join(dir: &str, path: &str) -> String {
    let mut parts: Vec<&str> = if dir.is_empty() {
        vec![]
    } else {
        dir.split('/').collect()
    };
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            p => parts.push(p),
        }
    }
    parts.join("/")
}

/// Expands a pattern whose last part is `*`, `**` or `*.ext` in one root.
fn glob(files: &dyn Files, root: Root, pattern: &str) -> Option<Vec<String>> {
    let (dir, last) = pattern.rsplit_once('/').unwrap_or(("", pattern));
    let recursive = last.starts_with("**");
    let suffix = last.trim_start_matches('*');
    let mut out = Vec::new();
    let mut stack = vec![dir.to_string()];
    let mut found_dir = false;
    while let Some(d) = stack.pop() {
        let Some(entries) = files.list(root, &d) else {
            continue;
        };
        found_dir = true;
        for e in entries {
            if skipped_by_wildcard(e.trim_end_matches('/')) {
                continue;
            }
            let path = join(&d, &e);
            if e.ends_with('/') {
                if recursive {
                    stack.push(path);
                }
            } else if has_data_extension(&e) && e.ends_with(suffix) {
                out.push(path);
            }
        }
    }
    out.sort();
    found_dir.then_some(out)
}

struct Loader<'a> {
    files: &'a dyn Files,
    seen: HashSet<Place>,
    out: Loaded,
    report: Report,
}

/// Loads the game from `main` (a path in the game root) and every file it
/// includes.
pub fn load(files: &dyn Files, main: &str) -> (Loaded, Report) {
    let mut l = Loader {
        files,
        seen: HashSet::new(),
        out: Loaded::default(),
        report: Report::default(),
    };
    let place = Place {
        root: Root::Game,
        path: main.to_string(),
    };
    if files.read(Root::Game, main).is_none() {
        l.report.error(main, 0, "main file not found");
    } else {
        l.file(place);
    }
    (l.out, l.report)
}

impl Loader<'_> {
    fn file(&mut self, place: Place) {
        if !self.seen.insert(place.clone()) {
            return;
        }
        let name = place.display();
        let Some(src) = self.files.read(place.root, &place.path) else {
            self.report.error(&name, 0, "cannot read file");
            return;
        };
        // drawings count code points of the NFC form (D 8.1)
        let src: String = src.nfc().collect();
        self.out.files.push(place.clone());
        let parsed = if place.path.ends_with(".rogue") {
            crate::syntax::parse_rogue(&name, &src)
        } else {
            crate::syntax::parse(&name, &src)
        };
        let parsed: ParsedFile = match parsed {
            Ok(p) => p,
            Err(errors) => {
                for e in errors {
                    self.report.push(e);
                }
                return;
            }
        };
        if let Some((language, _)) = parsed.language {
            self.out.languages.push(LanguageFile {
                file: name,
                language,
                namespace: parsed.namespace,
                texts: parsed.texts,
            });
            return;
        }
        for entry in parsed.entries {
            match entry {
                Entry::Include { pattern, line } => self.include(&place, &name, &pattern, line),
                e => self.out.entries.push(e),
            }
        }
    }

    fn include(&mut self, from: &Place, from_name: &str, pattern: &str, line: usize) {
        let phases = [
            (from.root, join(from.dir(), pattern)),
            (Root::Game, join("", pattern)),
            (Root::Engine, join("", pattern)),
        ];
        if pattern.contains('*') {
            let last = pattern.rsplit('/').next().unwrap_or(pattern);
            if pattern[..pattern.len() - last.len()].contains('*')
                || !(last == "*"
                    || last == "**"
                    || last.starts_with("*.")
                    || last.starts_with("**."))
            {
                self.report.error(
                    from_name,
                    line,
                    format!(
                        "unsupported wildcard `{pattern}`: use `dir/*`, `dir/**` or `dir/*.ext`"
                    ),
                );
                return;
            }
            for (root, p) in phases {
                if let Some(paths) = glob(self.files, root, &p) {
                    if paths.is_empty() {
                        self.report.warning(
                            from_name,
                            line,
                            format!("`include {pattern}` matches no files"),
                        );
                    }
                    for path in paths {
                        self.file(Place { root, path });
                    }
                    return;
                }
            }
            self.report.error(
                from_name,
                line,
                format!("`include {pattern}`: folder not found"),
            );
            return;
        }
        if !has_data_extension(pattern) {
            self.report.error(from_name, line, format!("`include {pattern}`: the path needs its extension (.rlml, .md, .rlml.md or .rogue)"));
            return;
        }
        let mut tried = Vec::new();
        for (root, path) in phases {
            let place = Place { root, path };
            if self.files.read(root, &place.path).is_some() {
                self.file(place);
                return;
            }
            let shown = place.display();
            if !tried.contains(&shown) {
                tried.push(shown);
            }
        }
        self.report.error(
            from_name,
            line,
            format!(
                "`include {pattern}`: file not found (tried {})",
                tried.join(", ")
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::RawBlock;

    fn mem(game: &[(&str, &str)], engine: &[(&str, &str)]) -> MemFiles {
        MemFiles {
            game: game
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
            engine: engine
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
        }
    }

    fn names(l: &Loaded) -> Vec<String> {
        l.entries
            .iter()
            .filter_map(|e| match e {
                Entry::Block(b) => Some(RawBlock::clone(b).name),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn content_loads_at_the_include_line_once() {
        let f = mem(
            &[
                (
                    "game.rlml",
                    "menu A\n- X\n\ninclude lib/b.rlml\ninclude lib/b.rlml\n\nmenu D\n- X\n",
                ),
                ("lib/b.rlml", "menu B\n- X\n\ninclude c.rlml\n"),
                ("lib/c.rlml", "menu C\n- X\n\ninclude b.rlml\n"),
            ],
            &[],
        );
        let (l, r) = load(&f, "game.rlml");
        assert!(r.0.is_empty(), "{r:?}");
        assert_eq!(names(&l), ["A", "B", "C", "D"]);
        assert_eq!(l.files.len(), 3);
    }

    #[test]
    fn search_order_and_engine_library() {
        let f = mem(
            &[
                ("game.rlml", "include lib/std.rlml\ninclude lib/hp.rlml\n"),
                ("lib/hp.rlml", "menu GAME_HP\n- X\n"),
            ],
            &[
                ("lib/std.rlml", "include std/core.rlml\n"),
                ("lib/std/core.rlml", "menu CORE\n- X\n"),
                ("lib/hp.rlml", "menu STD_HP\n- X\n"),
            ],
        );
        let (l, r) = load(&f, "game.rlml");
        assert!(r.0.is_empty(), "{r:?}");
        assert_eq!(
            names(&l),
            ["CORE", "GAME_HP"],
            "the game's own lib/hp.rlml wins"
        );
        assert_eq!(l.files[1].display(), "<engine>/lib/std.rlml");
    }

    #[test]
    fn wildcards_sort_and_skip() {
        let f = mem(
            &[
                (
                    "game.rlml",
                    "include maps/*\ninclude content/**\ninclude logic/*.rogue\n",
                ),
                ("maps/b.rlml", "menu MB\n- X\n"),
                ("maps/a.md", "menu MA\n- X\n"),
                ("maps/_draft.rlml", "menu DRAFT\n- X\n"),
                ("maps/README.md", "not data"),
                ("maps/notes.txt", "x"),
                ("content/x/deep.rlml", "menu DEEP\n- X\n"),
                ("logic/ai.rogue", "ai GHOST:\n    return\n"),
                ("logic/x.rlml", "menu NOT_ROGUE\n- X\n"),
            ],
            &[],
        );
        let (l, r) = load(&f, "game.rlml");
        assert!(r.0.is_empty(), "{r:?}");
        assert_eq!(names(&l), ["MA", "MB", "DEEP"]);
        assert!(matches!(l.entries[3], Entry::Code(_)));
    }

    #[test]
    fn language_files_are_kept_apart() {
        let f = mem(
            &[
                ("game.rlml", "include text/*\n"),
                ("text/fi.rlml", "language fi\nA.NAME a\n"),
            ],
            &[],
        );
        let (l, _) = load(&f, "game.rlml");
        assert_eq!(l.languages[0].language, "fi");
        assert_eq!(l.languages[0].texts[0].key(), "A.NAME");
    }

    #[test]
    fn errors() {
        let f = mem(
            &[(
                "game.rlml",
                "include lib/std.rlml\ninclude maps/*\ninclude a/*/b.rlml\ninclude x\n",
            )],
            &[],
        );
        let (_, r) = load(&f, "game.rlml");
        let msgs: Vec<&str> = r.0.iter().map(|d| d.message.as_str()).collect();
        assert!(
            msgs[0].contains("file not found (tried lib/std.rlml, <engine>/lib/std.rlml)"),
            "{msgs:?}"
        );
        assert!(msgs[1].contains("folder not found"));
        assert!(msgs[2].contains("unsupported wildcard"));
        assert!(msgs[3].contains("needs its extension"));
    }

    #[test]
    fn source_is_nfc_normalized() {
        let f = mem(&[("game.rlml", "language fi\nA.NAME a\u{0308}\n")], &[]);
        let (l, _) = load(&f, "game.rlml");
        assert_eq!(l.languages[0].texts[0].rest(1), "ä");
    }
}
