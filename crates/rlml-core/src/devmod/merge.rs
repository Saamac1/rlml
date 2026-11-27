//! `rlml mod merge` (D 3.6, RLML-kehitystyokalut.md step 8): a mod's
//! `patch` rows go into the definitions they change as point fixes, and
//! the rest of the mod is exported as an include file as in step 7.

use std::collections::BTreeMap;

use super::{Overlay, all_files, archive_folder, differences, header_only, resolved};
use crate::load::{Files, Layers, Root};
use crate::model::{Model, Source};
use crate::syntax::{Entry, Modifier, RawBlock, RawItem};

/// A planned merge: each file's text before and after, the rows that
/// stay in the include file and why, and where the mod's folder goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    /// Game root paths with their old text (empty for a new file) and new
    /// text, in path order.
    pub changed: Vec<(String, String, String)>,
    /// The mod's rows and blocks that are exported rather than merged.
    pub residual: Vec<String>,
    /// The mod's folder and its archive folder.
    pub archive: (String, String),
}

/// A file's lines, each with a lasting id: an original line's number, or
/// a number past them for an added line.
struct Buf {
    old: String,
    lines: Vec<(usize, String)>,
    next: usize,
}

impl Buf {
    fn new(text: &str) -> Buf {
        let lines: Vec<(usize, String)> = text
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.to_string()))
            .collect();
        Buf {
            old: text.to_string(),
            next: 1_000_000,
            lines,
        }
    }

    fn at(&self, id: usize) -> Option<usize> {
        self.lines.iter().position(|(i, _)| *i == id)
    }

    fn text(&self) -> String {
        let mut s: String = self.lines.iter().map(|(_, l)| format!("{l}\n")).collect();
        if !self.old.ends_with('\n') && !self.old.is_empty() {
            s.pop();
        }
        s
    }
}

/// Where a row of a target is written: its file and line id.
type Loc = (String, usize);

/// A target's rows as the merge goes: by key, with where each is.
struct Target {
    /// The block's own file and its header's line, for added rows.
    file: String,
    header: usize,
    rows: Vec<(String, Loc)>,
}

struct Merger<'a> {
    files: &'a dyn Files,
    bufs: BTreeMap<String, Buf>,
    targets: BTreeMap<String, Target>,
    residual: Vec<String>,
}

/// The game root path of a file `name` the loader shows, if it is one the
/// merge may change: not the engine's library, not the player's.
fn own_file(name: &str) -> Option<&str> {
    (!name.starts_with('<')).then_some(name)
}

impl Merger<'_> {
    fn buf(&mut self, path: &str) -> Option<&mut Buf> {
        if !self.bufs.contains_key(path) {
            let text = self.files.read(Root::Game, path)?;
            self.bufs.insert(path.to_string(), Buf::new(&text));
        }
        self.bufs.get_mut(path)
    }

    /// The target of `patch` block `p` in the game without the mod: its
    /// rows and where they are written.
    fn target(&mut self, base: &Model, p: &RawBlock) -> Result<String, String> {
        let full = crate::names::qualify(p.namespace.as_deref(), &p.name);
        let key = format!("{} {full}", p.kind);
        if !self.targets.contains_key(&key) {
            let block: &RawBlock = match p.kind.as_str() {
                "game" if p.name.is_empty() => base
                    .games
                    .iter()
                    .find(|g| !g.name.is_empty())
                    .ok_or("no named game block")?,
                "keys" | "pad" => base
                    .contexts
                    .get(&full)
                    .and_then(|bs| bs.iter().rev().find(|b| b.kind == p.kind))
                    .ok_or("no such context")?,
                _ => match base.defs.get(&full).map(|d| &d.source) {
                    Some(Source::Block(b)) => b,
                    _ => return Err(format!("`{full}` is no block")),
                },
            };
            let Some(file) = own_file(&block.file) else {
                return Err(format!("`{full}` is in {}", block.file));
            };
            let rows = block
                .items
                .iter()
                .map(|i| (i.marker().1.key().to_string(), (i.file.clone(), i.line)))
                .collect();
            self.targets.insert(
                key.clone(),
                Target {
                    file: file.to_string(),
                    header: block.line,
                    rows,
                },
            );
        }
        Ok(key)
    }

    /// One row of a patch into its target's text: `Err` with why it
    /// stays in the include file.
    fn row(&mut self, base: &Model, p: &RawBlock, item: &RawItem) -> Result<(), String> {
        let (marker, row) = item.marker();
        let key = row.key().to_string();
        let tk = self.target(base, p)?;
        let t = &self.targets[&tk];
        let at = t.rows.iter().position(|(k, _)| *k == key);
        match (marker, at) {
            (Modifier::Drop, Some(i)) => {
                let (file, id) = t.rows[i].1.clone();
                let path = own_file(&file)
                    .ok_or(format!("the row is in {file}"))?
                    .to_string();
                let b = self.buf(&path).ok_or("cannot read the file")?;
                let j = b.at(id).ok_or("the row moved")?;
                b.lines.remove(j);
                self.targets
                    .values_mut()
                    .for_each(|t| t.rows.retain(|(_, l)| *l != (file.clone(), id)));
                Ok(())
            }
            (Modifier::Drop, None) | (Modifier::New, Some(_)) => Err("the load refuses it".into()),
            (_, Some(i)) => {
                let (file, id) = t.rows[i].1.clone();
                let path = own_file(&file)
                    .ok_or(format!("the row is in {file}"))?
                    .to_string();
                let old = base_row_text(base, p, &key, &file, id, &self.bufs, self.files)?;
                let b = self.buf(&path).ok_or("cannot read the file")?;
                let j = b.at(id).ok_or("the row moved")?;
                let line = &b.lines[j].1;
                let start = line.find(&old).ok_or("the row is not on one line")?;
                let new = &row.text;
                // only what changes: the common start and end stay
                let pre = old
                    .char_indices()
                    .zip(new.chars())
                    .take_while(|((_, a), b)| a == b)
                    .last()
                    .map_or(0, |((i, a), _)| i + a.len_utf8());
                let (o, n) = (&old[pre..], &new[pre..]);
                let post = o
                    .chars()
                    .rev()
                    .zip(n.chars().rev())
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a.len_utf8())
                    .sum::<usize>();
                let line = format!(
                    "{}{}{}",
                    &line[..start + pre],
                    &n[..n.len() - post],
                    &line[start + pre + o.len() - post..]
                );
                b.lines[j].1 = line;
                Ok(())
            }
            (_, None) => {
                // a new row after the block's last own row
                let file = t.file.clone();
                let header = t.header;
                let anchor = t
                    .rows
                    .iter()
                    .rev()
                    .find(|(_, (f, _))| *f == file)
                    .map_or(header, |(_, (_, id))| *id);
                let b = self.buf(&file).ok_or("cannot read the file")?;
                let j = b.at(anchor).ok_or("the block moved")?;
                let indent: String = if anchor == header {
                    String::new()
                } else {
                    b.lines[j]
                        .1
                        .chars()
                        .take_while(|c| c.is_whitespace())
                        .collect()
                };
                let id = b.next;
                b.next += 1;
                b.lines
                    .insert(j + 1, (id, format!("{indent}- {}", row.text)));
                if let Some(t) = self.targets.get_mut(&tk) {
                    t.rows.push((key, (file, id)));
                }
                Ok(())
            }
        }
    }
}

/// The text of a target row as written now: a row changed by this merge
/// already is read from its line.
fn base_row_text(
    base: &Model,
    _p: &RawBlock,
    key: &str,
    file: &str,
    id: usize,
    bufs: &BTreeMap<String, Buf>,
    files: &dyn Files,
) -> Result<String, String> {
    // the row's text as loaded, when the merge has not changed its line
    let loaded = rows_of(base)
        .find(|i| i.file == file && i.line == id && i.marker().1.key() == key)
        .map(|i| i.text.clone());
    let line = match bufs.get(file) {
        Some(b) => b.at(id).map(|j| b.lines[j].1.clone()),
        None => files
            .read(Root::Game, file)
            .and_then(|t| t.lines().nth(id - 1).map(str::to_string)),
    };
    let line = line.ok_or("the row moved")?;
    match loaded {
        Some(t) if line.contains(&t) => Ok(t),
        _ => {
            // a row this merge wrote: its text after `- `
            let t = crate::syntax::strip_emphasis(line.trim());
            Ok(t.strip_prefix("- ").unwrap_or(t).to_string())
        }
    }
}

/// Every block row the model holds.
fn rows_of(m: &Model) -> impl Iterator<Item = &RawItem> {
    let defs = m.defs.values().filter_map(|d| match &d.source {
        Source::Block(b) => Some(b.items.iter()),
        _ => None,
    });
    let games = m.games.iter().map(|b| b.items.iter());
    let keys = m.contexts.values().flatten().map(|b| b.items.iter());
    defs.flatten().chain(games.flatten()).chain(keys.flatten())
}

/// Plans merging the mod `name` into the game at `main` (D 3.6): each
/// `patch` row into the row it changes, written in place with only its
/// changed part, `drop` as a removed line and a new row after the block's
/// last. The rest of the mod is exported under `mods/NAME/`. The game is
/// loaded with the mod and as merged, and a merge that changes the
/// resolved definitions is refused.
pub fn merge(files: &dyn Files, main: &str, name: &str) -> Result<Merge, Vec<String>> {
    let fail = |e: String| Err(vec![e]);
    let from = format!("dev/{name}");
    let mod_files: Vec<String> = all_files(files, &from).collect();
    if name.is_empty() || name.contains('/') || name.starts_with('.') || mod_files.is_empty() {
        return fail(format!(
            "no mod `{name}`: there is no folder dev/{name}/ (D 3.6)"
        ));
    }
    let layers = Layers {
        mods: vec![name.to_string()],
        overrides: false,
    };
    let (with, report) = crate::check_game_with(files, main, &layers);
    if report.has_errors() {
        return Err(report.errors().map(|d| d.to_string()).collect());
    }
    let (base, report) = crate::check_game_with(files, main, &Layers::default());
    if report.has_errors() {
        return Err(report.errors().map(|d| d.to_string()).collect());
    }
    let mut m = Merger {
        files,
        bufs: BTreeMap::new(),
        targets: BTreeMap::new(),
        residual: Vec::new(),
    };
    // the mod's files that stay, with the lines merged out of them
    let mut rest: Vec<(String, String)> = Vec::new();
    let loaded: Vec<&str> = with
        .files
        .iter()
        .filter(|p| p.root == Root::Game && p.path.starts_with(&format!("{from}/")))
        .map(|p| p.path.as_str())
        .collect();
    for path in &mod_files {
        let text = files.read(Root::Game, path).unwrap_or_default();
        if header_only(path, &text)? {
            continue;
        }
        let keep = |m: &mut Merger, why: String| m.residual.push(why);
        if !loaded.contains(&path.as_str()) || path.ends_with(".rogue") {
            if loaded.contains(&path.as_str()) {
                keep(&mut m, format!("{path}: RogueScript is exported as it is"));
            }
            rest.push((path.clone(), text));
            continue;
        }
        let parsed = crate::syntax::parse(path, &text)
            .map_err(|ds| ds.iter().map(|d| d.to_string()).collect::<Vec<_>>())?;
        if parsed.language.is_some() {
            keep(
                &mut m,
                format!("{path}: a language file is exported as it is"),
            );
            rest.push((path.clone(), text));
            continue;
        }
        // the lines that went into the game
        let mut gone: Vec<usize> = Vec::new();
        let mut stays = false;
        for e in &parsed.entries {
            let b = match e {
                Entry::Block(b) if b.modifier == Modifier::Patch => b,
                Entry::Block(b) => {
                    keep(
                        &mut m,
                        format!(
                            "{}:{}: `{} {}` is not a patch",
                            path, b.line, b.kind, b.name
                        ),
                    );
                    stays = true;
                    continue;
                }
                Entry::Code(c) => {
                    keep(&mut m, format!("{}:{}: RogueScript", path, c.line));
                    stays = true;
                    continue;
                }
                Entry::Include { line, .. } => {
                    keep(&mut m, format!("{path}:{line}: an include"));
                    stays = true;
                    continue;
                }
            };
            if !b.subs.is_empty() {
                keep(
                    &mut m,
                    format!(
                        "{}:{} `patch {} {}` adds sub-blocks",
                        path, b.line, b.kind, b.name
                    ),
                );
                stays = true;
                continue;
            }
            let mut all = true;
            for item in &b.items {
                match m.row(&base, b, item) {
                    Ok(()) => gone.push(item.line),
                    Err(why) => {
                        keep(
                            &mut m,
                            format!("{}:{}: `{}`: {why}", path, item.line, item.text),
                        );
                        all = false;
                    }
                }
            }
            if all {
                gone.push(b.line);
            } else {
                stays = true;
            }
        }
        if stays {
            let kept: Vec<&str> = text
                .lines()
                .enumerate()
                .filter(|(i, _)| !gone.contains(&(i + 1)))
                .map(|(_, l)| l)
                .collect();
            rest.push((path.clone(), squeeze(&kept)));
        }
    }
    // the residual goes as an export does
    let to = format!("mods/{name}");
    let mut put: BTreeMap<String, Option<String>> =
        mod_files.iter().map(|p| (p.clone(), None)).collect();
    let mut changed: Vec<(String, String, String)> = m
        .bufs
        .iter()
        .map(|(p, b)| (p.clone(), b.old.clone(), b.text()))
        .filter(|(_, o, n)| o != n)
        .collect();
    let rest: Vec<(String, String)> = rest
        .into_iter()
        .filter(|(_, t)| {
            t.lines()
                .any(|l| !l.trim().is_empty() && !crate::syntax::is_comment(l.trim()))
        })
        .collect();
    if !rest.is_empty() {
        if files.list(Root::Game, &to).is_some() {
            return fail(format!("{to}/ is there already for the rows that stay"));
        }
        let include = format!("include {to}/**");
        let main_text = changed
            .iter()
            .find(|(p, _, _)| p == main)
            .map(|(_, _, n)| n.clone())
            .or_else(|| files.read(Root::Game, main))
            .unwrap_or_default();
        let old_main = files.read(Root::Game, main).unwrap_or_default();
        let about = with
            .mod_infos
            .iter()
            .find(|i| i.folder == name)
            .map(|i| i.about.clone())
            .unwrap_or_default();
        let mut new_main = main_text;
        if !new_main.ends_with('\n') {
            new_main.push('\n');
        }
        new_main.push('\n');
        if !about.is_empty() {
            new_main.push_str(&format!("> Modi {name}: {about}\n"));
        }
        new_main.push_str(&include);
        new_main.push('\n');
        changed.retain(|(p, _, _)| p != main);
        changed.push((main.to_string(), old_main, new_main));
        for (p, t) in rest {
            changed.push((format!("{to}{}", &p[from.len()..]), String::new(), t));
        }
        changed.sort();
    }
    put.extend(changed.iter().map(|(p, _, n)| (p.clone(), Some(n.clone()))));
    let overlay = Overlay { base: files, put };
    let (after, report) = crate::check_game_with(&overlay, main, &Layers::default());
    if report.has_errors() {
        let mut e = vec!["the merged game does not load:".to_string()];
        e.extend(report.errors().map(|d| d.to_string()));
        return Err(e);
    }
    let diff = differences(&resolved(&with), &resolved(&after));
    if !diff.is_empty() {
        let mut e = vec!["the merge would change the game's definitions:".to_string()];
        e.extend(diff);
        return Err(e);
    }
    Ok(Merge {
        changed,
        residual: m.residual,
        archive: (from, archive_folder(files, name)),
    })
}

/// Lines with runs of blank lines made one, and none at the ends.
fn squeeze(lines: &[&str]) -> String {
    let mut out = String::new();
    let mut blank = true;
    for l in lines {
        let b = l.trim().is_empty();
        if b && blank {
            continue;
        }
        out.push_str(l);
        out.push('\n');
        blank = b;
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// A unified diff of `old` and `new` with `context` lines around each
/// change, headed by `path`; empty when they are the same.
pub fn unified(path: &str, old: &str, new: &str, context: usize) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    if a == b {
        return String::new();
    }
    // the longest common subsequence, from the end
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    // the edit script: ' ', '-' or '+' with the line and both line numbers
    let mut ops: Vec<(char, &str, usize, usize)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            ops.push((' ', a[i], i, j));
            i += 1;
            j += 1;
        } else if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            ops.push(('-', a[i], i, j));
            i += 1;
        } else {
            ops.push(('+', b[j], i, j));
            j += 1;
        }
    }
    let old_name = if old.is_empty() {
        "/dev/null".to_string()
    } else {
        format!("a/{path}")
    };
    let mut out = format!("--- {old_name}\n+++ b/{path}\n");
    let changes: Vec<usize> = (0..ops.len()).filter(|&k| ops[k].0 != ' ').collect();
    let mut k = 0;
    while k < changes.len() {
        let start = changes[k].saturating_sub(context);
        let mut end = changes[k];
        while k < changes.len() && changes[k] <= end + 2 * context + 1 {
            end = changes[k];
            k += 1;
        }
        let end = (end + context + 1).min(ops.len());
        let hunk = &ops[start..end];
        let (oa, ob) = (hunk[0].2, hunk[0].3);
        let na = hunk.iter().filter(|o| o.0 != '+').count();
        let nb = hunk.iter().filter(|o| o.0 != '-').count();
        out.push_str(&format!(
            "@@ -{},{na} +{},{nb} @@\n",
            if na == 0 { oa } else { oa + 1 },
            if nb == 0 { ob } else { ob + 1 }
        ));
        for (c, l, _, _) in hunk {
            out.push_str(&format!("{c}{l}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diff_shows_the_change_with_its_context() {
        let d = unified("a.rlml", "a\nb\nc\nd\ne\n", "a\nb\nC\nd\ne\nf\n", 1);
        assert_eq!(
            d,
            "--- a/a.rlml\n+++ b/a.rlml\n@@ -2,4 +2,5 @@\n b\n-c\n+C\n d\n e\n+f\n"
        );
        assert_eq!(unified("a", "x\n", "x\n", 3), "");
    }
}
