//! The loaded game: every definition by its full name (D 2.1, D 4).
//!
//! Definitions of the same kind and full name replace earlier ones (a
//! warning when the replacement comes from another file without
//! `override`); different kinds with the same full name are a load error.
//! `patch` blocks change single rows of an existing block (D 4.8). Unnamed
//! sections (`settings`, `tiles`, `colors`, …) merge row by row.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::OnceLock;

use crate::error::{Diagnostic, Report};
use crate::load::{LanguageFile, Loaded};
use crate::names::{self, qualify};
use crate::rogue::{self, Decl, DeclKind};
use crate::syntax::{Entry, Modifier, RawBlock, RawItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Component,
    Entity,
    Deck,
    Table,
    Tileset,
    Layer,
    Tile,
    Map,
    Prefab,
    Menu,
    Panel,
    Layout,
    Log,
    Message,
    Sound,
    Color,
    Constant,
    Event,
    Handler,
    Function,
    Query,
    Command,
    Ai,
    Generator,
    Scene,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Kind::Component => "component",
            Kind::Entity => "entity",
            Kind::Deck => "deck",
            Kind::Table => "table",
            Kind::Tileset => "tileset",
            Kind::Layer => "terrain layer",
            Kind::Tile => "tile",
            Kind::Map => "map",
            Kind::Prefab => "prefab",
            Kind::Menu => "menu",
            Kind::Panel => "panel",
            Kind::Layout => "layout",
            Kind::Log => "log",
            Kind::Message => "message",
            Kind::Sound => "sound kind",
            Kind::Color => "color",
            Kind::Constant => "constant",
            Kind::Event => "event",
            Kind::Handler => "handler",
            Kind::Function => "function",
            Kind::Query => "query",
            Kind::Command => "command",
            Kind::Ai => "ai",
            Kind::Generator => "generator",
            Kind::Scene => "scene",
        };
        f.write_str(s)
    }
}

/// Where a definition came from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Block(Box<RawBlock>),
    /// A row of an unnamed section (`constants`, `colors`, `messages`,
    /// `sounds`, `terrain`).
    Row(RawItem),
    Decl(Decl),
    /// A tile, first seen at this row.
    Tile,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Def {
    pub kind: Kind,
    pub full: String,
    pub file: String,
    pub line: usize,
    pub overrides: bool,
    pub source: Source,
    /// The rows `patch` changed, by row key, with the file and line that
    /// changed them last: the value's origin (D 3.6).
    pub patched: Vec<PatchedRow>,
}

/// A row a patch changed: its key, and the file and line of the change.
pub type PatchedRow = (String, String, usize);

/// A mod's `mod` header (D 3.6).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModInfo {
    pub name: String,
    /// The folder under `dev/` it was read from.
    pub folder: String,
    pub about: String,
    /// The game's version it was made on.
    pub base: Option<u64>,
    /// `update existing`: already created instances take its changes.
    pub update_existing: bool,
}

impl Def {
    pub fn block(&self) -> Option<&RawBlock> {
        match &self.source {
            Source::Block(b) => Some(b),
            _ => None,
        }
    }

    pub fn place(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }
}

#[derive(Debug, Default)]
pub struct Model {
    pub defs: BTreeMap<String, Def>,
    /// The named `game` block, then unnamed library `game` blocks.
    pub games: Vec<RawBlock>,
    pub profiles: Vec<RawBlock>,
    /// Unnamed sections in load order, by kind (`settings`, `tiles`, …).
    pub sections: Vec<RawBlock>,
    /// Key contexts (`keys` and `pad`) by name.
    pub contexts: BTreeMap<String, Vec<RawBlock>>,
    pub code: Vec<Decl>,
    pub languages: Vec<LanguageFile>,
    /// Every loaded file in load order.
    pub files: Vec<crate::load::Place>,
    /// The loaded files' hash: the game's version (M 4.4).
    pub source_hash: u64,
    /// The game's own files' hash, before mods (D 3.6).
    pub base_hash: u64,
    /// The active mods in load order with their files' hashes, and their
    /// `mod` headers (D 3.6).
    pub mods: Vec<(String, u64)>,
    pub mod_infos: Vec<ModInfo>,
    /// Full names written with `override`, for the check that something
    /// was replaced.
    overrides: Vec<(String, Kind, String, usize)>,
    replaced: Vec<String>,
    /// Answers a running game asks for every frame, found once: the
    /// model does not change after it is built and checked.
    settings: OnceLock<BTreeMap<String, RawItem>>,
    text_index: OnceLock<TextIndex>,
    tile_rows: OnceLock<BTreeMap<String, Vec<Vec<String>>>>,
    /// Every event's handlers in the order they run (M 6.2).
    dispatch: OnceLock<crate::rt::dispatch::Dispatch>,
}

/// One part of a `layout` row's `when` (D 11.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutCond {
    /// `width < N` and the like: the axis (true for width), the
    /// comparison and N.
    Size {
        width: bool,
        op: CmpOp,
        n: usize,
    },
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
}

impl LayoutCond {
    pub fn holds(&self, cols: usize, rows: usize, portrait: bool) -> bool {
        match *self {
            LayoutCond::Portrait => portrait,
            LayoutCond::Landscape => !portrait,
            LayoutCond::Size { width, op, n } => {
                let v = if width { cols } else { rows };
                match op {
                    CmpOp::Lt => v < n,
                    CmpOp::Le => v <= n,
                    CmpOp::Gt => v > n,
                    CmpOp::Ge => v >= n,
                }
            }
        }
    }
}

/// A `when` condition's words: parts joined by `and` (D 11.1).
pub fn layout_cond(words: &[&str]) -> Result<Vec<LayoutCond>, String> {
    let mut out = Vec::new();
    for part in words.split(|w| *w == "and") {
        out.push(match part {
            ["portrait"] => LayoutCond::Portrait,
            ["landscape"] => LayoutCond::Landscape,
            [axis @ ("width" | "height"), op, n] => {
                let op = match *op {
                    "<" => CmpOp::Lt,
                    "<=" => CmpOp::Le,
                    ">" => CmpOp::Gt,
                    ">=" => CmpOp::Ge,
                    other => {
                        return Err(format!("`{other}` is no comparison (`<`, `<=`, `>`, `>=`)"));
                    }
                };
                let n = n
                    .parse()
                    .map_err(|_| format!("`{n}` is no number of cells"))?;
                LayoutCond::Size {
                    width: *axis == "width",
                    op,
                    n,
                }
            }
            [] => return Err("an empty part of `when`".into()),
            other => {
                return Err(format!(
                    "`{}` is not `width`/`height` COMPARISON N, `portrait` or `landscape`",
                    other.join(" ")
                ));
            }
        });
    }
    Ok(out)
}

/// Text keys by language: the file and row of each key's text.
type TextIndex = BTreeMap<String, BTreeMap<String, (usize, usize)>>;

impl Model {
    pub fn kind(&self, full: &str) -> Option<Kind> {
        self.defs.get(full).map(|d| d.kind)
    }

    /// Resolves a reference from namespace `ns` (D 4.4). The error lists
    /// every name tried.
    pub fn resolve(&self, ns: Option<&str>, name: &str) -> Result<&Def, String> {
        let full = names::resolve(ns, name, |c| self.defs.contains_key(c))
            .map_err(|tried| format!("`{name}` is not defined (tried {tried})"))?;
        Ok(&self.defs[&full])
    }

    /// Resolves a reference and checks its kind after the lookup.
    pub fn resolve_kind(
        &self,
        ns: Option<&str>,
        name: &str,
        kinds: &[Kind],
    ) -> Result<&Def, String> {
        let def = self.resolve(ns, name)?;
        if kinds.contains(&def.kind) {
            Ok(def)
        } else {
            let want: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
            Err(format!(
                "`{}` is a {} ({}), not a {}",
                def.full,
                def.kind,
                def.place(),
                want.join(" or ")
            ))
        }
    }

    /// Resolves a tile reference: the index is not part of the name (D 4.4).
    pub fn resolve_tile(&self, ns: Option<&str>, written: &str) -> Result<String, String> {
        let t = names::tile_name(written)?;
        let def = self.resolve_kind(ns, &t.base, &[Kind::Tile])?;
        Ok(def.full.clone())
    }

    /// Rows of every unnamed section of `kind` in load order.
    pub fn section_rows<'a>(
        &'a self,
        kind: &'a str,
    ) -> impl Iterator<Item = (&'a RawBlock, &'a RawItem)> {
        self.sections
            .iter()
            .filter(move |b| b.kind == kind)
            .flat_map(|b| b.items.iter().map(move |i| (b, i)))
    }

    /// Merged `settings` rows: the last row of each key wins (D 4.8).
    /// Every event's handlers by phase, in the order they run, laid out
    /// once (M 6.2).
    pub fn dispatch(&self) -> &crate::rt::dispatch::Dispatch {
        self.dispatch
            .get_or_init(|| crate::rt::dispatch::Dispatch::build(self))
    }

    pub fn settings(&self) -> &BTreeMap<String, RawItem> {
        self.settings.get_or_init(|| {
            let mut out = BTreeMap::new();
            for (_, item) in self.section_rows("settings") {
                let (_, item) = item.marker();
                out.insert(item.key().to_string(), item);
            }
            out
        })
    }

    pub fn named_game(&self) -> Option<&RawBlock> {
        self.games.iter().find(|g| !g.name.is_empty())
    }

    /// The layout in use: `game.layout` or the only one (D 11.1).
    /// The default layout: the `layout` row without a `when`, or the only
    /// layout (D 11.1). `view_width()` reads it, whatever the window.
    pub fn layout(&self) -> Option<&RawBlock> {
        self.layout_full(None)
            .and_then(|n| self.defs.get(&n))
            .and_then(|d| d.block())
    }

    /// The layout for a window of `cols`×`rows` cells, `portrait` when it
    /// is taller than wide in pixels: the first `layout` row whose `when`
    /// holds, else the default (D 11.1).
    pub fn layout_for(&self, cols: usize, rows: usize, portrait: bool) -> Option<&RawBlock> {
        self.layout_full(Some((cols, rows, portrait)))
            .and_then(|n| self.defs.get(&n))
            .and_then(|d| d.block())
    }

    /// The `game` block's `layout` rows: each layout's full name and its
    /// condition, `None` without `when`; a row that does not read is left
    /// out (the check reports it).
    pub fn layout_rows(&self) -> Vec<(String, Option<Vec<LayoutCond>>)> {
        let Some(g) = self.named_game() else {
            return vec![];
        };
        let ns = g.namespace.as_deref();
        g.items
            .iter()
            .filter(|i| i.key() == "layout")
            .filter_map(|i| {
                let full = self
                    .resolve_kind(ns, i.word(1)?, &[Kind::Layout])
                    .ok()?
                    .full
                    .clone();
                let cond = match i.tokens.iter().position(|t| t.is("when")) {
                    Some(at) => Some(
                        layout_cond(
                            &i.tokens[at + 1..]
                                .iter()
                                .map(|t| t.text.as_str())
                                .collect::<Vec<_>>(),
                        )
                        .ok()?,
                    ),
                    None => None,
                };
                Some((full, cond))
            })
            .collect()
    }

    fn layout_full(&self, window: Option<(usize, usize, bool)>) -> Option<String> {
        let rows = self.layout_rows();
        if rows.is_empty() {
            return self
                .defs
                .values()
                .find(|d| d.kind == Kind::Layout)
                .map(|d| d.full.clone());
        }
        if let Some((c, r, p)) = window
            && let Some((full, _)) = rows.iter().find(|(_, cond)| {
                cond.as_ref()
                    .is_some_and(|cs| cs.iter().all(|x| x.holds(c, r, p)))
            })
        {
            return Some(full.clone());
        }
        rows.iter()
            .find(|(_, cond)| cond.is_none())
            .or(rows.first())
            .map(|(f, _)| f.clone())
    }

    /// The border style `panels` rows give area `name` (D 9.3): the last
    /// known style written for it, or `None`. The terminal draws it and the
    /// core takes its cells off the map area, so both read it here.
    pub fn panel_border(&self, name: &str) -> Option<&'static str> {
        let mut border = None;
        for (_, item) in self.section_rows("panels") {
            let (_, row) = item.marker();
            if names::qualify(row.namespace.as_deref(), row.key()) != name && row.key() != name {
                continue;
            }
            let words: Vec<&str> = row.tokens.iter().map(|t| t.text.as_str()).collect();
            if let Some(b) = words
                .iter()
                .position(|w| *w == "border")
                .and_then(|i| words.get(i + 1))
                .and_then(|b| BORDER_STYLES.iter().find(|s| *s == b))
            {
                border = Some(*b);
            }
        }
        border
    }

    /// The map area as the layout's legend writes it (D 11.1, M 7.1): its
    /// written size per axis and whether its border is drawn. The rule for
    /// the border lives here only; the terminal and the core both read it.
    pub fn map_area(&self) -> MapArea {
        match self.layout() {
            Some(b) => self.map_area_in(b),
            None => MapArea::default(),
        }
    }

    /// A layout's map area as its legend writes it: the default layout's
    /// is `view_width()`'s, the chosen one's is drawn (D 11.1).
    pub fn map_area_in(&self, b: &RawBlock) -> MapArea {
        let cells = |s: Option<&str>| s.and_then(|s| s.parse::<i32>().ok()).filter(|&n| n > 0);
        let styled = self.panel_border("map").is_some_and(|b| b != "none");
        for item in b.sub("legend") {
            let Ok((_, rest)) = glyph_char(item) else {
                continue;
            };
            let words: Vec<&str> = rest.tokens.iter().map(|t| t.text.as_str()).collect();
            if words.first() == Some(&"map") {
                let (w, h) = legend_size(&words);
                let (width, height) = (cells(w), cells(h));
                // a border needs 2x2 cells; an axis without a written size
                // takes the window's room, which is at least that
                let border =
                    styled && width.is_none_or(|n| n >= 2) && height.is_none_or(|n| n >= 2);
                return MapArea {
                    width,
                    height,
                    border,
                };
            }
        }
        MapArea {
            border: styled,
            ..MapArea::default()
        }
    }

    /// A text key's row in a language, as `texts` merges them (D 10).
    pub fn text_item(&self, language: &str, key: &str) -> Option<&RawItem> {
        let index = self.text_index.get_or_init(|| {
            let mut out = TextIndex::new();
            for (fi, f) in self.languages.iter().enumerate() {
                let keys = out.entry(f.language.clone()).or_default();
                for (ti, t) in f.texts.iter().enumerate() {
                    keys.insert(qualify(f.namespace.as_deref(), t.key()), (fi, ti));
                }
            }
            out
        });
        let &(fi, ti) = index.get(language)?.get(key)?;
        Some(&self.languages[fi].texts[ti])
    }

    pub fn has_text(&self, language: &str, key: &str) -> bool {
        self.text_item(language, key).is_some()
    }

    /// A written name's full name: a definition, a tile with its variant
    /// (`WALL.2`), else as written.
    pub fn full_name(&self, written: &str, ns: Option<&str>) -> String {
        if let Ok(d) = self.resolve(ns, written) {
            return d.full.clone();
        }
        if let Ok(t) = names::tile_name(written)
            && let Ok(full) = self.resolve_tile(ns, written)
        {
            return written.replacen(&t.base, &full, 1);
        }
        written.to_string()
    }

    /// The `tiles` rows by the full name they are for (`default` as is),
    /// each row's words after its key, in load order (D 7.1).
    pub fn tile_rows(&self) -> &BTreeMap<String, Vec<Vec<String>>> {
        self.tile_rows.get_or_init(|| {
            let mut out: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
            for (_, item) in self.section_rows("tiles") {
                let (_, row) = item.marker();
                let key = if row.key() == "default" {
                    "default".to_string()
                } else {
                    self.full_name(row.key(), row.namespace.as_deref())
                };
                out.entry(key)
                    .or_default()
                    .push(row.tokens.iter().skip(1).map(|t| t.text.clone()).collect());
            }
            out
        })
    }

    /// Text keys of one language, merged row by row (D 10).
    pub fn texts(&self, language: &str) -> BTreeMap<String, &RawItem> {
        let mut out = BTreeMap::new();
        for f in self.languages.iter().filter(|f| f.language == language) {
            for t in &f.texts {
                out.insert(qualify(f.namespace.as_deref(), t.key()), t);
            }
        }
        out
    }
}

fn block_kind(kind: &str) -> Option<Kind> {
    Some(match kind {
        "component" => Kind::Component,
        "entity" => Kind::Entity,
        "deck" => Kind::Deck,
        "table" => Kind::Table,
        "tileset" => Kind::Tileset,
        "map" => Kind::Map,
        "prefab" => Kind::Prefab,
        "menu" => Kind::Menu,
        "panel" => Kind::Panel,
        "layout" => Kind::Layout,
        "log" => Kind::Log,
        _ => return None,
    })
}

fn decl_kind(kind: DeclKind) -> Kind {
    match kind {
        DeclKind::Component => Kind::Component,
        DeclKind::Event => Kind::Event,
        DeclKind::Handler => Kind::Handler,
        DeclKind::Function => Kind::Function,
        DeclKind::Query => Kind::Query,
        DeclKind::Command => Kind::Command,
        DeclKind::Ai => Kind::Ai,
        DeclKind::Generator => Kind::Generator,
        DeclKind::Map => Kind::Map,
        DeclKind::Scene => Kind::Scene,
    }
}

/// Builds the model from loaded files. Diagnostics go to `report`.
pub fn build(loaded: Loaded, report: &mut Report) -> Model {
    let mut m = Model {
        languages: loaded.languages,
        files: loaded.files,
        source_hash: loaded.source_hash,
        base_hash: loaded.base_hash,
        mods: loaded.mods,
        ..Default::default()
    };
    // (the block, strict: an `overrides/` file's place, where a row it
    // does not know needs `new`)
    let mut patches: Vec<(RawBlock, bool)> = Vec::new();
    for entry in loaded.entries {
        match entry {
            Entry::Include { .. } => {}
            Entry::Code(code) => {
                let (decls, errors) = rogue::declarations(&code);
                for e in errors {
                    report.push(e);
                }
                for d in decls {
                    if !d.name.is_empty() {
                        let full = qualify(d.namespace.as_deref(), &d.name);
                        let (file, line, ov) = (d.file.clone(), d.line, d.overrides);
                        m.define(
                            report,
                            Def {
                                kind: decl_kind(d.kind),
                                full,
                                file,
                                line,
                                overrides: ov,
                                source: Source::Decl(d.clone()),
                                patched: vec![],
                            },
                        );
                    }
                    m.code.push(d);
                }
            }
            Entry::Block(b) => {
                let b = *b;
                if b.modifier == Modifier::Patch {
                    patches.push((b, false));
                    continue;
                }
                // the player's `overrides/`: its blocks are places, as if
                // written `patch`, but sections merge as everywhere (D 3.2)
                if crate::load::is_override(&b.file) {
                    if b.kind == "profile" {
                        report.push(b.err("a `profile` block cannot be in `overrides/`: the profile is the engine's (D 3.5)"));
                        continue;
                    }
                    if b.modifier == Modifier::None
                        && !crate::syntax::SECTION_KINDS.contains(&b.kind.as_str())
                    {
                        patches.push((b, true));
                        continue;
                    }
                }
                if b.kind == "mod" {
                    m.mod_header(report, b);
                    continue;
                }
                match b.kind.as_str() {
                    "game" => m.games.push(b),
                    "profile" => m.profiles.push(b),
                    "keys" | "pad" => {
                        let full = qualify(b.namespace.as_deref(), &b.name);
                        m.contexts.entry(full).or_default().push(b);
                    }
                    kind if crate::syntax::SECTION_KINDS.contains(&kind) => {
                        m.section_defs(report, &b);
                        m.sections.push(b);
                    }
                    kind => {
                        let Some(k) = block_kind(kind) else { continue };
                        let full = qualify(b.namespace.as_deref(), &b.name);
                        let def = Def {
                            kind: k,
                            full,
                            file: b.file.clone(),
                            line: b.line,
                            overrides: b.modifier == Modifier::Override,
                            source: Source::Block(Box::new(b)),
                            patched: vec![],
                        };
                        m.define(report, def);
                    }
                }
            }
        }
    }
    m.tiles(report);
    let mut changed_by: HashMap<(String, String), String> = HashMap::new();
    for (p, strict) in patches {
        m.patch(report, p, strict, &mut changed_by);
    }
    m.check_mods(report);
    m.check_overrides(report);
    m
}

impl Model {
    fn define(&mut self, report: &mut Report, def: Def) {
        if def.overrides {
            self.overrides
                .push((def.full.clone(), def.kind, def.file.clone(), def.line));
        }
        if let Some(old) = self.defs.get(&def.full) {
            if old.kind != def.kind {
                report.error(
                    &def.file,
                    def.line,
                    format!(
                        "`{}` is defined as a {} here and as a {} at {}",
                        def.full,
                        def.kind,
                        old.kind,
                        old.place()
                    ),
                );
                return;
            }
            if old.overrides && !def.overrides {
                // an `override` wins over an unmarked definition in any order (D 4.5)
                self.replaced.push(def.full.clone());
                return;
            }
            if old.overrides && def.overrides {
                report.error(
                    &def.file,
                    def.line,
                    format!(
                        "second `override` of `{}` (first at {})",
                        def.full,
                        old.place()
                    ),
                );
                return;
            }
            if old.file != def.file && !def.overrides && old.kind != Kind::Tile {
                report.warning(
                    &def.file,
                    def.line,
                    format!(
                        "`{}` replaces the {} at {}; write `override` if this is intended",
                        def.full,
                        old.kind,
                        old.place()
                    ),
                );
            }
            self.replaced.push(def.full.clone());
        }
        self.defs.insert(def.full.clone(), def);
    }

    /// Names defined by section rows.
    fn section_defs(&mut self, report: &mut Report, b: &RawBlock) {
        let kind = match b.kind.as_str() {
            "constants" => Kind::Constant,
            "colors" => Kind::Color,
            "messages" => Kind::Message,
            "sounds" => Kind::Sound,
            "terrain" => Kind::Layer,
            _ => return,
        };
        for item in &b.items {
            let (marker, item) = item.marker();
            let key = item.key();
            if !names::is_ref(key) {
                report.push(item.err(format!("`{key}` is not a NAME")));
                continue;
            }
            let full = qualify(item.namespace.as_deref(), key);
            self.define(
                report,
                Def {
                    kind,
                    full,
                    file: item.file.clone(),
                    line: item.line,
                    overrides: marker == Modifier::Override,
                    source: Source::Row(item),
                    patched: vec![],
                },
            );
        }
    }

    /// Tiles are named where they are described or produced: `tiles` and
    /// `tiles.glyphs` rows and map and prefab legends (D 7).
    fn tiles(&mut self, report: &mut Report) {
        let mut seen: Vec<(String, String, usize)> = Vec::new();
        for (_, item) in self
            .section_rows("tiles")
            .chain(self.section_rows("tiles.glyphs"))
        {
            let (_, item) = item.marker();
            if item.key() == "default" {
                continue;
            }
            match names::tile_name(item.key()) {
                Ok(t) => seen.push((
                    qualify(item.namespace.as_deref(), &t.base),
                    item.file.clone(),
                    item.line,
                )),
                Err(e) => report.push(item.err(e)),
            }
        }
        for def in self
            .defs
            .values()
            .filter(|d| matches!(d.kind, Kind::Map | Kind::Prefab))
        {
            let Some(b) = def.block() else { continue };
            // `at:` rows start with a position, legend rows with a character
            let rows = b
                .sub("legend")
                .map(|i| glyph_char(i).map(|(_, r)| r).unwrap_or_else(|_| i.shift(1)))
                .chain(b.sub("at").map(|i| i.shift(1)));
            for item in rows {
                for t in row_tiles(&item) {
                    if let Ok(t) = names::tile_name(&t) {
                        seen.push((
                            qualify(item.namespace.as_deref(), &t.base),
                            item.file.clone(),
                            item.line,
                        ));
                    }
                }
            }
        }
        for (full, file, line) in seen {
            match self.defs.get(&full) {
                Some(d) if d.kind == Kind::Tile => {}
                Some(d) => report.error(
                    &file,
                    line,
                    format!("tile `{full}` is also a {} ({})", d.kind, d.place()),
                ),
                None => {
                    self.defs.insert(
                        full.clone(),
                        Def {
                            kind: Kind::Tile,
                            full,
                            file,
                            line,
                            overrides: false,
                            source: Source::Tile,
                            patched: vec![],
                        },
                    );
                }
            }
        }
    }

    /// A `patch` block, or a place in `overrides/` (`strict`): its rows
    /// change the target's by row key (D 4.8). A named definition, the
    /// named `game` block or a key context can be patched. Two mods that
    /// change the same row are warned of; the later wins (D 3.6).
    fn patch(
        &mut self,
        report: &mut Report,
        p: RawBlock,
        strict: bool,
        changed_by: &mut HashMap<(String, String), String>,
    ) {
        let full = qualify(p.namespace.as_deref(), &p.name);
        let by = crate::load::mod_of(&p.file).map(str::to_string);
        // what the patch changes: its rows, and where a definition keeps
        // its rows' origin
        let (target, origin): (&mut Vec<RawItem>, Option<&mut Vec<PatchedRow>>) =
            match p.kind.as_str() {
                // an unnamed `game` in `overrides/` places the game's own
                "game" if p.name.is_empty() => {
                    let Some(g) = self.games.iter_mut().find(|g| !g.name.is_empty()) else {
                        report.push(p.err("there is no named `game` block to patch"));
                        return;
                    };
                    (&mut g.items, None)
                }
                "keys" | "pad" => {
                    let Some(blocks) = self.contexts.get_mut(&full) else {
                        report.push(p.err(format!("`{} {full}`: nothing to patch", p.kind)));
                        return;
                    };
                    let Some(last) = blocks.iter_mut().rev().find(|b| b.kind == p.kind) else {
                        report.push(p.err(format!("`{} {full}`: nothing to patch", p.kind)));
                        return;
                    };
                    // a context's rows are its blocks' rows; the place's
                    // rows go into the last of them
                    (&mut last.items, None)
                }
                _ => {
                    let Some(def) = self.defs.get_mut(&full) else {
                        report.push(p.err(format!("`patch {} {full}`: nothing to patch", p.kind)));
                        return;
                    };
                    let Def {
                        source,
                        patched,
                        kind,
                        ..
                    } = def;
                    let Source::Block(target) = source else {
                        report.push(p.err(format!(
                            "`{full}` is a {kind} written in RogueScript and cannot be patched"
                        )));
                        return;
                    };
                    if target.kind != p.kind {
                        report.push(
                            p.err(format!("`patch {}`: `{full}` is a {}", p.kind, target.kind)),
                        );
                        return;
                    }
                    if p.kind == "table" || !p.drawing.is_empty() {
                        report.push(p.err("a table or a drawing cannot be patched (D 4.8)"));
                        return;
                    }
                    for sub in p.subs.iter().cloned() {
                        target.subs.push(sub);
                    }
                    (&mut target.items, Some(patched))
                }
            };
        let mut origin = origin;
        for item in p.items {
            let (marker, row) = item.marker();
            let key = row.key().to_string();
            let at = target.iter().position(|i| i.marker().1.key() == key);
            // two mods on the same row: the later wins, with a warning
            if let Some(by) = &by {
                let k = (format!("{} {full}", p.kind), key.clone());
                if let Some(before) = changed_by.get(&k)
                    && before != by
                {
                    report.warning(
                        &item.file,
                        item.line,
                        format!("mods `{before}` and `{by}` both change `{key}` of `{full}`: `{by}` wins, as it loads later (D 3.6)"),
                    );
                }
                changed_by.insert(k, by.clone());
            }
            if let Some(o) = origin.as_deref_mut() {
                o.retain(|(k, _, _)| *k != key);
                o.push((key.clone(), item.file.clone(), item.line));
            }
            match (marker, at) {
                (Modifier::Drop, Some(i)) => {
                    target.remove(i);
                }
                (Modifier::Drop, None) => {
                    report.push(item.err(format!("`drop {key}`: `{full}` has no such row")))
                }
                (Modifier::New, Some(_)) => {
                    report.push(item.err(format!("`new {key}`: `{full}` already has this row")))
                }
                (Modifier::New, None) => target.push(row),
                (_, Some(i)) => target[i] = row,
                // in `overrides/` an unknown row is a typo until it says
                // `new` (D 3.2)
                (_, None) if strict => report.push(item.err(format!(
                    "`{key}`: `{full}` has no such row; a new one is written `new {key}` (D 3.2)"
                ))),
                (_, None) => target.push(row),
            }
        }
    }

    /// A mod's `mod NAME` header (D 3.6).
    fn mod_header(&mut self, report: &mut Report, b: RawBlock) {
        let Some(folder) = crate::load::mod_of(&b.file) else {
            report.push(b.err("a `mod` header belongs in a mod's folder, dev/NAME/ (D 3.6)"));
            return;
        };
        let mut info = ModInfo {
            name: b.name.clone(),
            folder: folder.to_string(),
            ..ModInfo::default()
        };
        for item in &b.items {
            match (item.key(), item.word(1)) {
                ("about", _) => info.about = item.rest(1).trim_matches('"').to_string(),
                ("base", Some(h)) => match u64::from_str_radix(h, 16) {
                    Ok(h) => info.base = Some(h),
                    Err(_) => report.push(item.err("`base` takes the game's hash in hexadecimal")),
                },
                ("update", Some("existing")) => info.update_existing = true,
                (k, _) => report.push(item.err(format!(
                    "unknown `mod` row `{k}`: the rows are about, base and update existing (D 3.6)"
                ))),
            }
        }
        self.mod_infos.push(info);
    }

    /// The active mods against their headers: a changed game is warned of
    /// (D 3.6).
    fn check_mods(&self, report: &mut Report) {
        for (name, _) in &self.mods {
            let Some(info) = self.mod_infos.iter().find(|i| &i.folder == name) else {
                report.warning(
                    &format!("dev/{name}/"),
                    0,
                    format!("mod `{name}` has no `mod` header in dev/{name}/mod.rlml (D 3.6)"),
                );
                continue;
            };
            if let Some(base) = info.base
                && base != self.base_hash
            {
                report.warning(
                    &format!("dev/{name}/mod.rlml"),
                    0,
                    format!(
                        "mod `{}` was made on another version of the game ({base:016x}, now {:016x}); it loads anyway (D 3.6)",
                        info.name, self.base_hash
                    ),
                );
            }
        }
    }

    fn check_overrides(&self, report: &mut Report) {
        for (full, kind, file, line) in &self.overrides {
            if !self.replaced.contains(full) {
                report.error(
                    file,
                    *line,
                    format!("`override` of {kind} `{full}`, but there is nothing to replace"),
                );
            }
        }
    }
}

/// Tile names a legend or `at:` row produces, given the row without its
/// character or position: the default layer's tile and every `LAYER TILE`
/// pair, up to `with`, `as` or `wire` (D 8.2, D 8.3).
pub fn row_tiles(rest: &RawItem) -> Vec<String> {
    let words: Vec<String> = rest
        .tokens
        .iter()
        .map(|t| t.text.clone())
        .take_while(|w| !["with", "as", "wire"].contains(&w.as_str()))
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    if words.len() % 2 == 1 {
        out.push(words[0].clone());
        i = 1;
    }
    while i + 1 < words.len() {
        out.push(words[i + 1].clone());
        i += 2;
    }
    out.retain(|w| w != "none");
    out
}

/// The map area of a layout (D 11.1): written cell counts, border
/// included, and whether the border is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MapArea {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub border: bool,
}

impl MapArea {
    /// The map's cells inside the border; a border may leave none.
    pub fn cells(&self) -> (Option<i32>, Option<i32>) {
        let inset = if self.border { 2 } else { 0 };
        (
            self.width.map(|n| n - inset),
            self.height.map(|n| n - inset),
        )
    }
}

/// The written width and height of a layout legend row's words (D 11.1):
/// `size WxH`, `size.width W` and `size.height H`, later words winning. A
/// size word is a cell count, `N%` or `*`; a word that is not one leaves
/// the earlier size, and a `size` pair counts only when both halves are.
pub fn legend_size<'a>(words: &[&'a str]) -> (Option<&'a str>, Option<&'a str>) {
    let mut size = (None, None);
    for (i, w) in words.iter().enumerate() {
        let next = words
            .get(i + 1)
            .copied()
            .filter(|n| size_word(n) || n.contains('x'));
        match *w {
            "size" => {
                if let Some((a, b)) = next
                    .and_then(|n| n.split_once('x'))
                    .filter(|(a, b)| size_word(a) && size_word(b))
                {
                    size = (Some(a), Some(b));
                }
            }
            "size.width" if next.is_some_and(size_word) => size.0 = next,
            "size.height" if next.is_some_and(size_word) => size.1 = next,
            _ => {}
        }
    }
    size
}

/// A layout size word (D 11.1): cells `N` (1 or more), `N%` (1–100) or `*`.
pub fn size_word(s: &str) -> bool {
    match s.strip_suffix('%') {
        Some(p) => p.parse::<u8>().is_ok_and(|n| (1..=100).contains(&n)),
        None => s == "*" || s.parse::<usize>().is_ok_and(|n| n > 0),
    }
}

/// Border styles of `panels` rows (D 9.3).
pub const BORDER_STYLES: &[&str] = &["none", "single", "double", "rounded", "heavy", "ascii"];

/// The drawing character that starts a legend or `tiles.glyphs` row and the
/// rest of the row. A space is written `' '` (D 8.1, D 9.2).
pub fn glyph_char(item: &RawItem) -> Result<(char, RawItem), String> {
    if item.text.starts_with("' '") {
        let rest = item.text[3..].trim_start();
        let shifted = RawItem::new(&item.file, item.line, item.namespace.clone(), rest)
            .map_err(|e| e.message)?;
        return Ok((' ', shifted));
    }
    let first = item.tokens.first().ok_or("missing character")?;
    let mut chars = item.text[first.start..first.end].chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok((c, item.shift(1))),
        _ => Err(format!(
            "`{}` is not one character",
            &item.text[first.start..first.end]
        )),
    }
}

/// A diagnostic for `item`, used by the checks.
pub fn at(item: &RawItem, msg: impl Into<String>) -> Diagnostic {
    item.err(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{MemFiles, load};

    fn model(files: &[(&str, &str)]) -> (Model, Report) {
        let f = MemFiles {
            game: files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect(),
            ..Default::default()
        };
        let (loaded, mut report) = load(&f, files[0].0);
        let m = build(loaded, &mut report);
        (m, report)
    }

    fn messages(r: &Report) -> Vec<String> {
        r.0.iter().map(|d| d.to_string()).collect()
    }

    #[test]
    fn full_names_and_lookup() {
        let (m, r) = model(&[
            ("game.rlml", "include lib.rlml\n\nentity RAT\n- HP now 3\n"),
            (
                "lib.rlml",
                "namespace STD\n\ncomponent HP\n- now number\n\n```roguescript\nai HUNTER:\n    return WAIT()\n```\n",
            ),
        ]);
        assert!(r.0.is_empty(), "{r:?}");
        assert_eq!(m.resolve(None, "HP").unwrap().full, "STD.HP");
        assert_eq!(m.resolve(Some("STD"), "HUNTER").unwrap().kind, Kind::Ai);
        assert!(
            m.resolve(None, "NOPE")
                .unwrap_err()
                .contains("tried NOPE, STD.NOPE")
        );
        assert!(
            m.resolve_kind(None, "RAT", &[Kind::Map])
                .unwrap_err()
                .contains("is a entity")
        );
    }

    #[test]
    fn replacement_rules() {
        let (_, r) = model(&[
            ("a.rlml", "include b.rlml\n\nmenu M\n- A\n\nmenu M\n- B\n"),
            ("b.rlml", "menu M\n- C\n"),
        ]);
        let msgs = messages(&r);
        assert_eq!(msgs.len(), 1, "same-file replacement is silent: {msgs:?}");
        assert!(msgs[0].contains("warning: `M` replaces the menu at b.rlml:1"));

        let (_, r) = model(&[
            ("a.rlml", "include b.rlml\n\noverride menu M\n- A\n"),
            ("b.rlml", "menu M\n- C\n"),
        ]);
        assert!(r.0.is_empty(), "{r:?}");

        let (_, r) = model(&[("a.rlml", "override menu M\n- A\n")]);
        assert!(messages(&r)[0].contains("nothing to replace"));

        let (_, r) = model(&[("a.rlml", "menu M\n- A\n\nentity M\n- X\n")]);
        assert!(
            messages(&r)[0].contains("`M` is defined as a entity here and as a menu at a.rlml:1")
        );
    }

    #[test]
    fn patches_change_rows() {
        let (m, r) = model(&[(
            "a.rlml",
            "entity P\n- HP now 10\n- MELEE 1d4\n- PACK\n\npatch entity P\n- HP now 15\n- drop PACK\n- new SNEAKING\n\npatch entity Q\n- X\n",
        )]);
        let rows: Vec<&str> = m.defs["P"]
            .block()
            .unwrap()
            .items
            .iter()
            .map(|i| i.text.as_str())
            .collect();
        assert_eq!(rows, ["HP now 15", "MELEE 1d4", "SNEAKING"]);
        assert!(messages(&r)[0].contains("nothing to patch"));
    }

    #[test]
    fn tiles_come_from_rows_and_legends() {
        let (m, r) = model(&[(
            "a.rlml",
            "tiles\n- WALL blocks move\n- DOOR.open blocks none\n\nmap M\n\n```\n#+.\n```\n\nlegend:\n- # WALL\n- . FLOOR COVER WATER.2 with RAT as START\n- + STAIR.down\n",
        )]);
        assert!(r.0.is_empty(), "{r:?}");
        for t in ["WALL", "DOOR", "FLOOR", "WATER", "STAIR"] {
            assert_eq!(m.kind(t), Some(Kind::Tile), "{t}");
        }
        assert_eq!(m.resolve_tile(None, "WATER.3").unwrap(), "WATER");
    }

    #[test]
    fn sections_define_names_and_merge() {
        let (m, r) = model(&[(
            "a.rlml",
            "colors\n- STONE 5a5a6e\n\nsettings\n- style ascii\n- round 500ms\n\nsettings\n- round 250ms\n",
        )]);
        assert!(r.0.is_empty(), "{r:?}");
        assert_eq!(m.kind("STONE"), Some(Kind::Color));
        assert_eq!(m.settings()["round"].rest(1), "250ms");
    }
}

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn legend_sizes() {
        assert_eq!(
            legend_size(&["map", "size", "20x10"]),
            (Some("20"), Some("10"))
        );
        assert_eq!(
            legend_size(&["map", "size.width", "20", "size", "10xfoo"]),
            (Some("20"), None),
            "a broken pair is dropped whole"
        );
        assert_eq!(
            legend_size(&["map", "size", "24x*", "size.height", "50%"]),
            (Some("24"), Some("50%"))
        );
        assert_eq!(legend_size(&["map", "size.width", "0"]), (None, None));
        assert!(size_word("100%") && !size_word("101%") && size_word("*") && !size_word("-3"));
    }
}
