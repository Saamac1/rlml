//! Block-level syntax of RLML data files (D 2.2, D 2.5, D 4.2, D 10).
//!
//! ```text
//! namespace STD                (file start only)
//! include lib/std.rlml
//!
//! [patch|override|new] KIND NAME[(params)] [header words…]
//! <drawing>                    (map, prefab, layout and panel; may follow a
//!                               blank line and is usually in a ``` fence)
//! - [override|new|drop] item words…
//! legend:                      (sub-blocks `legend:`, `vars:` and `at:`
//! - legend item words…          continue over blank lines until a header)
//! ```
//!
//! A code block fenced as `roguescript` holds RogueScript (RS 2.1).
//!
//! * A block ends at a blank line. Markdown headings (`#`) are free text.
//! * Comments: a line whose first character is `>` followed by a space or
//!   the line end, and a trailing `**…**`, `***…***` or `~~…~~` (D 2.5).
//!   Drawings have no comments.
//! * A language file starts with `language KIELI` and then holds bare
//!   `KEY text` lines (D 10).

use crate::error::Diagnostic;
use crate::token::{Token, tokenize};

pub const BLOCK_KINDS: &[&str] = &[
    "game", "profile", "settings", "autosave", "constants", "component", "entity", "deck", "table", "terrain", "tileset", "tiles",
    "tiles.glyphs", "map", "prefab", "colors", "panels", "logs", "presentation", "layers", "layout", "menu", "keys", "pad", "log",
    "messages", "panel", "sounds",
];

pub const DRAWING_KINDS: &[&str] = &["map", "prefab", "layout", "panel"];

/// Sections without a name; their header words are all arguments, and rows
/// from every file merge by row key (D 4.8).
pub const SECTION_KINDS: &[&str] = &[
    "settings", "autosave", "constants", "terrain", "tiles", "tiles.glyphs", "colors", "panels", "logs", "presentation", "layers",
    "messages", "sounds",
];

/// Blocks whose name is optional: a library's unnamed `game` or `profile`
/// adds components (D 3.3, D 3.5).
pub const OPTIONAL_NAME_KINDS: &[&str] = &["game", "profile"];

pub const SUB_BLOCKS: &[&str] = &["legend", "vars", "at", "areas"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    None,
    Patch,
    Override,
    New,
    Drop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawItem {
    pub file: String,
    pub line: usize,
    /// Namespace of the file the item was written in.
    pub namespace: Option<String>,
    /// Text after the leading `- `.
    pub text: String,
    pub tokens: Vec<Token>,
}

impl RawItem {
    pub fn new(file: &str, line: usize, namespace: Option<String>, text: &str) -> Result<RawItem, Diagnostic> {
        let tokens = tokenize(text).map_err(|e| Diagnostic::error(file, line, e.0))?;
        Ok(RawItem { file: file.to_string(), line, namespace, text: text.to_string(), tokens })
    }

    /// The first word.
    pub fn key(&self) -> &str {
        self.tokens.first().map(|t| t.text.as_str()).unwrap_or("")
    }

    pub fn word(&self, i: usize) -> Option<&str> {
        self.tokens.get(i).map(|t| t.text.as_str())
    }

    pub fn err(&self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::error(&self.file, self.line, msg)
    }

    pub fn warn(&self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::warning(&self.file, self.line, msg)
    }

    /// Original text of tokens `from..to`.
    pub fn span(&self, from: usize, to: usize) -> &str {
        crate::token::span_text(&self.text, &self.tokens, from, to)
    }

    /// Original text from token `from` to the end of the line.
    pub fn rest(&self, from: usize) -> &str {
        self.span(from, self.tokens.len())
    }

    /// The item without its first `n` words.
    pub fn shift(&self, n: usize) -> RawItem {
        let Some(first) = self.tokens.get(n) else {
            return RawItem { text: String::new(), tokens: vec![], ..self.clone() };
        };
        let off = first.start;
        RawItem {
            text: self.text[off..].to_string(),
            tokens: self.tokens[n..].iter().map(|t| Token { start: t.start - off, end: t.end - off, ..t.clone() }).collect(),
            ..self.clone()
        }
    }

    /// Splits a leading row marker (`override`, `new`, `drop`) off the item.
    pub fn marker(&self) -> (Modifier, RawItem) {
        let m = match self.key() {
            "override" => Modifier::Override,
            "new" => Modifier::New,
            "drop" => Modifier::Drop,
            _ => return (Modifier::None, self.clone()),
        };
        (m, self.shift(1))
    }
}

/// A sub-block such as `legend:`.
#[derive(Debug, Clone, PartialEq)]
pub struct Sub {
    pub name: String,
    pub line: usize,
    pub items: Vec<RawItem>,
}

/// A header parameter: `name` or `name: type`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawBlock {
    pub modifier: Modifier,
    pub kind: String,
    pub name: String,
    /// `Some` when the header had `(…)` after the name.
    pub params: Option<Vec<Param>>,
    pub args: Vec<Token>,
    pub args_text: String,
    pub file: String,
    pub line: usize,
    pub namespace: Option<String>,
    pub drawing: Vec<String>,
    /// Line number of the first drawing line.
    pub drawing_line: usize,
    pub items: Vec<RawItem>,
    pub subs: Vec<Sub>,
}

impl RawBlock {
    pub fn err(&self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::error(&self.file, self.line, msg)
    }

    pub fn warn(&self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::warning(&self.file, self.line, msg)
    }

    pub fn sub(&self, name: &str) -> impl Iterator<Item = &RawItem> {
        self.subs.iter().filter(move |s| s.name == name).flat_map(|s| s.items.iter())
    }

    pub fn has_arg(&self, word: &str) -> bool {
        self.args.iter().any(|t| t.is(word))
    }

    /// The word after `word` in the header (`size 24x4` → `24x4`).
    pub fn arg_after(&self, word: &str) -> Option<&str> {
        let i = self.args.iter().position(|t| t.is(word))?;
        self.args.get(i + 1).map(|t| t.text.as_str())
    }

    fn drawing_pending(&self) -> bool {
        DRAWING_KINDS.contains(&self.kind.as_str())
            && self.drawing.is_empty()
            && self.items.is_empty()
            && self.subs.is_empty()
            && !self.has_arg("extends")
    }
}

/// A RogueScript code block or `.rogue` file (RS 2.1).
#[derive(Debug, Clone, PartialEq)]
pub struct RawCode {
    pub file: String,
    /// Line number of the first code line.
    pub line: usize,
    pub namespace: Option<String>,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Include { pattern: String, line: usize },
    Block(Box<RawBlock>),
    Code(RawCode),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParsedFile {
    pub file: String,
    pub namespace: Option<String>,
    pub entries: Vec<Entry>,
    /// `language KIELI` and its line, for a language file.
    pub language: Option<(String, usize)>,
    /// The `KEY text` lines of a language file.
    pub texts: Vec<RawItem>,
}

impl ParsedFile {
    pub fn blocks(&self) -> impl Iterator<Item = &RawBlock> {
        self.entries.iter().filter_map(|e| match e {
            Entry::Block(b) => Some(b.as_ref()),
            _ => None,
        })
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum State {
    /// Between blocks.
    Outside,
    /// In a block body.
    Body,
    /// A drawing block whose header was followed by a blank line: the
    /// drawing may still come.
    AwaitDrawing,
    /// Unfenced drawing: ends at a blank line or a sub-block.
    Drawing,
    /// Fenced drawing: ends at the closing fence.
    FencedDrawing,
    /// The block ended at a blank line; only a sub-block may continue it.
    Closed,
    /// In a sub-block; continues over blank lines.
    Sub,
    /// In a RogueScript code block.
    Code,
    /// Skipping a code block that is in error.
    SkipFence,
    /// In a language file, after `language KIELI`.
    Language,
}

fn header_kind(line: &str) -> Option<(Modifier, &str)> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let mut words = line.split_whitespace();
    let first = words.next()?;
    let modifier = match first {
        "patch" => Modifier::Patch,
        "override" => Modifier::Override,
        "new" => Modifier::New,
        _ => Modifier::None,
    };
    let kind = if modifier == Modifier::None { first } else { words.next()? };
    BLOCK_KINDS.iter().find(|k| **k == kind).map(|k| (modifier, *k))
}

/// `>` followed by a space or the end of the line (D 2.5).
pub fn is_comment(trimmed: &str) -> bool {
    trimmed == ">" || trimmed.starts_with("> ")
}

/// Removes a trailing emphasis comment: `**…**`, `***…***` or `~~…~~` at
/// the end of the line, opened right before text and closed right after it.
pub fn strip_emphasis(line: &str) -> &str {
    let t = line.trim_end();
    for marker in ["***", "**", "~~"] {
        if !t.ends_with(marker) || t.len() < 2 * marker.len() + 1 {
            continue;
        }
        let body_end = t.len() - marker.len();
        if t[..body_end].ends_with(char::is_whitespace) {
            continue;
        }
        // earliest opening marker that starts a word
        let mut search = 0;
        while let Some(pos) = t[search..body_end].find(marker) {
            let start = search + pos;
            let at_word_start = start == 0 || t[..start].ends_with(char::is_whitespace);
            let after = &t[start + marker.len()..body_end];
            if at_word_start && !after.is_empty() && !after.starts_with(char::is_whitespace) && !after.starts_with('*') {
                return t[..start].trim_end();
            }
            search = start + marker.len();
        }
    }
    line
}

fn fence_lang(line: &str) -> Option<&str> {
    let t = line.trim_start();
    t.starts_with("```").then(|| t.trim_start_matches('`').trim())
}

pub fn parse(file: &str, src: &str) -> Result<ParsedFile, Vec<Diagnostic>> {
    let mut out = ParsedFile { file: file.to_string(), ..Default::default() };
    let mut errors = Vec::new();
    let mut current: Option<RawBlock> = None;
    let mut code: Option<RawCode> = None;
    let mut state = State::Outside;
    let mut seen_definition = false;

    let flush = |current: &mut Option<RawBlock>, out: &mut ParsedFile| {
        if let Some(b) = current.take() {
            out.entries.push(Entry::Block(Box::new(b)));
        }
    };

    for (idx, raw_line) in src.lines().enumerate() {
        let lineno = idx + 1;
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();
        let fence = fence_lang(line);

        match state {
            State::SkipFence => {
                if fence.is_some() {
                    state = State::Outside;
                }
                continue;
            }
            State::Code => {
                if fence.is_some() {
                    out.entries.push(Entry::Code(code.take().unwrap()));
                    state = State::Outside;
                } else {
                    code.as_mut().unwrap().lines.push(line.to_string());
                }
                continue;
            }
            State::FencedDrawing => {
                if fence.is_some() {
                    state = State::Body;
                } else {
                    current.as_mut().unwrap().drawing.push(line.trim_end().to_string());
                }
                continue;
            }
            State::Drawing => {
                if trimmed.is_empty() {
                    state = State::Closed;
                    continue;
                }
                if !sub_header(trimmed).is_some() && fence.is_none() {
                    current.as_mut().unwrap().drawing.push(line.trim_end().to_string());
                    continue;
                }
                state = State::Body;
            }
            State::Language => {
                if trimmed.is_empty() || is_comment(trimmed) {
                    continue;
                }
                if header_kind(line).is_some() || line.starts_with("include ") || line.starts_with("language ") {
                    errors.push(Diagnostic::error(file, lineno, "a language file holds only `KEY text` lines after its `language` header"));
                    continue;
                }
                // the text is taken verbatim: no emphasis comments (D 10)
                match RawItem::new(file, lineno, out.namespace.clone(), trimmed) {
                    Ok(item) => out.texts.push(item),
                    Err(e) => errors.push(e),
                }
                continue;
            }
            _ => {}
        }

        if let Some(lang) = fence {
            let pending = matches!(state, State::Body | State::AwaitDrawing) && current.as_ref().is_some_and(RawBlock::drawing_pending);
            if lang.is_empty() && pending {
                current.as_mut().unwrap().drawing_line = lineno + 1;
                state = State::FencedDrawing;
            } else if lang == "roguescript" {
                flush(&mut current, &mut out);
                seen_definition = true;
                code = Some(RawCode { file: file.to_string(), line: lineno + 1, namespace: out.namespace.clone(), lines: vec![] });
                state = State::Code;
            } else {
                errors.push(Diagnostic::error(
                    file,
                    lineno,
                    if lang.is_empty() {
                        "a code block must hold RogueScript (```roguescript) or the drawing of the map, prefab, layout or panel above it".to_string()
                    } else {
                        format!("unknown code block language `{lang}`")
                    },
                ));
                flush(&mut current, &mut out);
                state = State::SkipFence;
            }
            continue;
        }

        if trimmed.is_empty() {
            state = match state {
                State::Sub => State::Sub,
                State::Body if current.as_ref().is_some_and(RawBlock::drawing_pending) => State::AwaitDrawing,
                State::AwaitDrawing => State::AwaitDrawing,
                State::Outside => State::Outside,
                _ => State::Closed,
            };
            continue;
        }
        if is_comment(trimmed) {
            continue;
        }

        // An unfenced drawing may start right after the header or after a
        // blank line; its first line can be anything but an item or a
        // sub-block header.
        if matches!(state, State::Body | State::AwaitDrawing)
            && !trimmed.starts_with("- ")
            && trimmed != "-"
            && sub_header(trimmed).is_none()
            && header_kind(line).is_none()
            && current.as_ref().is_some_and(RawBlock::drawing_pending)
        {
            let b = current.as_mut().unwrap();
            b.drawing_line = lineno;
            b.drawing.push(line.trim_end().to_string());
            state = State::Drawing;
            continue;
        }

        if trimmed.starts_with('#') {
            // markdown heading
            continue;
        }

        let code_line = strip_emphasis(line);
        let code_trimmed = code_line.trim();

        if let Some(word) = code_trimmed.split_whitespace().next().filter(|w| w.starts_with("tiles.") && *w != "tiles.glyphs")
            && !line.starts_with(char::is_whitespace) {
                errors.push(Diagnostic::error(file, lineno, format!("`{word}`: only `tiles.glyphs` depends on the presentation (D 9.2)")));
                flush(&mut current, &mut out);
                state = State::Closed;
                continue;
            }

        if let Some((modifier, kind)) = header_kind(code_line) {
            flush(&mut current, &mut out);
            seen_definition = true;
            match parse_header(file, lineno, code_line, modifier, kind, &out.namespace) {
                Ok(b) => {
                    current = Some(b);
                    state = State::Body;
                }
                Err(e) => {
                    errors.push(e);
                    state = State::Closed;
                }
            }
            continue;
        }

        if !line.starts_with(char::is_whitespace) {
            let mut words = code_trimmed.split_whitespace();
            match words.next() {
                Some("include") => {
                    flush(&mut current, &mut out);
                    state = State::Outside;
                    let pattern: Vec<&str> = words.collect();
                    if pattern.len() != 1 {
                        errors.push(Diagnostic::error(file, lineno, "include takes exactly one path"));
                    } else {
                        out.entries.push(Entry::Include { pattern: pattern[0].to_string(), line: lineno });
                    }
                    continue;
                }
                Some("namespace") => {
                    let name: Vec<&str> = words.collect();
                    if seen_definition || out.namespace.is_some() {
                        errors.push(Diagnostic::error(file, lineno, "`namespace` must come before the first definition, and only once"));
                    } else if name.len() != 1 || !crate::names::is_name(name[0]) {
                        errors.push(Diagnostic::error(file, lineno, "namespace takes one NAME"));
                    } else {
                        out.namespace = Some(name[0].to_string());
                    }
                    continue;
                }
                Some("language") => {
                    let lang: Vec<&str> = words.collect();
                    if seen_definition || !out.entries.is_empty() {
                        errors.push(Diagnostic::error(file, lineno, "`language` must be the first definition of a language file (D 10)"));
                    } else if lang.len() != 1 || !crate::names::is_vocabulary(lang[0]) {
                        errors.push(Diagnostic::error(file, lineno, "language takes one lower-case language name, such as `language fi`"));
                    } else {
                        out.language = Some((lang[0].to_string(), lineno));
                        state = State::Language;
                    }
                    seen_definition = true;
                    continue;
                }
                _ => {}
            }
        }

        if let Some(name) = sub_header(code_trimmed) {
            match current.as_mut() {
                Some(b) if b.subs.iter().any(|s| s.name == name) && name != "legend" => {
                    errors.push(Diagnostic::error(file, lineno, format!("duplicate `{name}:`")))
                }
                Some(b) => {
                    b.subs.push(Sub { name: name.to_string(), line: lineno, items: vec![] });
                    state = State::Sub;
                }
                None => errors.push(Diagnostic::error(file, lineno, format!("`{name}:` outside a block"))),
            }
            continue;
        }

        let item_text = code_trimmed.strip_prefix("- ").or_else(|| (code_trimmed == "-").then_some(""));
        match (state, current.as_mut()) {
            (State::Body | State::Sub | State::AwaitDrawing, Some(block)) => {
                if let Some(rest) = item_text {
                    match RawItem::new(file, lineno, out.namespace.clone(), rest) {
                        Ok(item) if item.tokens.is_empty() => errors.push(item.err("empty item")),
                        Ok(item) if state == State::Sub => block.subs.last_mut().unwrap().items.push(item),
                        Ok(item) => block.items.push(item),
                        Err(e) => errors.push(e),
                    }
                } else {
                    errors.push(Diagnostic::error(file, lineno, format!("unexpected line in `{}` block: `{trimmed}`", block.kind)));
                }
            }
            (State::Closed, Some(block)) if item_text.is_some() => errors.push(Diagnostic::error(
                file,
                lineno,
                format!("item after a blank line: the `{}` block ended at the blank line above", block.kind),
            )),
            _ => {
                let word = code_trimmed.split_whitespace().next().unwrap_or("");
                let msg = if item_text.is_some() {
                    "item outside a block".to_string()
                } else if line.starts_with(char::is_whitespace) {
                    format!("unexpected text `{trimmed}`")
                } else if ["workdir", "audio", "audioscript", "preload", "render", "buses"].contains(&word) {
                    format!("`{word}`: audio declarations (D 12) are not supported yet")
                } else {
                    format!("unknown block kind `{word}`")
                };
                errors.push(Diagnostic::error(file, lineno, msg));
            }
        }
    }
    if matches!(state, State::FencedDrawing | State::SkipFence | State::Code) {
        errors.push(Diagnostic::error(file, src.lines().count(), "unterminated code block"));
    }
    flush(&mut current, &mut out);
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// `legend:`, `vars:` or `at:`.
fn sub_header(trimmed: &str) -> Option<&str> {
    let name = trimmed.strip_suffix(':')?;
    SUB_BLOCKS.iter().find(|s| **s == name).copied()
}

/// Parses a `.rogue` file: RogueScript without code fences (RS 2.1), with an
/// optional `namespace` line first.
pub fn parse_rogue(file: &str, src: &str) -> Result<ParsedFile, Vec<Diagnostic>> {
    let mut out = ParsedFile { file: file.to_string(), ..Default::default() };
    let mut code = RawCode { file: file.to_string(), line: 1, namespace: None, lines: vec![] };
    for (idx, line) in src.lines().enumerate() {
        if code.lines.iter().all(|l| l.trim().is_empty() || l.trim_start().starts_with("//"))
            && let Some(ns) = line.strip_prefix("namespace ")
        {
            if !crate::names::is_name(ns.trim()) {
                return Err(vec![Diagnostic::error(file, idx + 1, "namespace takes one NAME")]);
            }
            out.namespace = Some(ns.trim().to_string());
            code.namespace = out.namespace.clone();
            code.lines.push(String::new());
            continue;
        }
        code.lines.push(line.trim_end_matches('\r').to_string());
    }
    out.entries.push(Entry::Code(code));
    Ok(out)
}

/// Splits `s` at top-level separators (outside quotes and parentheses).
pub fn split_top_level(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"') => quote = Some(c),
            (None, '(' | '[' | '{') => depth += 1,
            (None, ')' | ']' | '}') => depth = depth.saturating_sub(1),
            (None, c) if c == sep && depth == 0 => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// Finds the `)` matching the `(` at byte `open`.
pub fn matching_close(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote = false;
    for (i, c) in s[open..].char_indices() {
        match c {
            '"' => quote = !quote,
            '(' if !quote => depth += 1,
            ')' if !quote => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_header(
    file: &str,
    lineno: usize,
    line: &str,
    modifier: Modifier,
    kind: &str,
    namespace: &Option<String>,
) -> Result<RawBlock, Diagnostic> {
    let err = |m: String| Diagnostic::error(file, lineno, m);
    // skip the modifier and kind words
    let mut rest = line.trim_start();
    let skip = if modifier == Modifier::None { 1 } else { 2 };
    for _ in 0..skip {
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        rest = rest[end..].trim_start();
    }
    let section = SECTION_KINDS.contains(&kind);
    let name_end = rest.find(|c: char| c.is_whitespace() || c == '(').unwrap_or(rest.len());
    let mut name = rest[..name_end].to_string();
    let mut after = &rest[name_end..];
    if section || !crate::names::is_ref(&name) {
        // unnamed: every header word is an argument
        if !section && !OPTIONAL_NAME_KINDS.contains(&kind) {
            return Err(err(if name.is_empty() {
                format!("`{kind}` block needs a name")
            } else {
                format!("`{kind}` block needs an upper-case NAME, not `{name}`")
            }));
        }
        name.clear();
        after = rest;
    }
    let mut params = None;
    if after.starts_with('(') {
        let close = matching_close(after, 0).ok_or_else(|| err("missing `)` in header parameters".into()))?;
        let mut list = Vec::new();
        for p in split_top_level(&after[1..close], ',').into_iter().map(str::trim).filter(|p| !p.is_empty()) {
            let (n, ty) = match p.split_once(':') {
                Some((n, t)) => (n.trim(), Some(t.trim().to_string())),
                None => (p, None),
            };
            if !crate::names::is_vocabulary(n) {
                return Err(err(format!("bad parameter `{p}`")));
            }
            list.push(Param { name: n.to_string(), ty });
        }
        params = Some(list);
        after = &after[close + 1..];
    }
    let args_text = after.trim().to_string();
    let args = tokenize(&args_text).map_err(|e| err(e.0))?;
    Ok(RawBlock {
        modifier,
        kind: kind.to_string(),
        name,
        params,
        args,
        args_text,
        file: file.to_string(),
        line: lineno,
        namespace: namespace.clone(),
        drawing: Vec::new(),
        drawing_line: 0,
        items: Vec::new(),
        subs: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(src: &str) -> Vec<RawBlock> {
        parse("t.rlml", src).unwrap_or_else(|e| panic!("{e:?}")).blocks().cloned().collect()
    }

    fn errs(src: &str) -> Vec<Diagnostic> {
        parse("t", src).unwrap_err()
    }

    #[test]
    fn game_block_and_includes() {
        let pf = parse("game.rlml", "# Peli\n\ngame MINI\n- start CAVE_A\n- at SPAWN\n\ninclude lib/std.rlml\ninclude maps/*\n").unwrap();
        assert_eq!(pf.entries.len(), 3);
        let Entry::Block(b) = &pf.entries[0] else { panic!() };
        assert_eq!((b.kind.as_str(), b.name.as_str(), b.items.len()), ("game", "MINI", 2));
        assert_eq!(b.items[1].line, 5);
        assert_eq!(pf.entries[1], Entry::Include { pattern: "lib/std.rlml".into(), line: 7 });
    }

    #[test]
    fn sections_take_header_words_as_arguments() {
        let b = blocks("tiles.glyphs ascii\n- WALL # fg STONE\n\nsettings\n- style ascii\n");
        assert_eq!((b[0].name.as_str(), b[0].args_text.as_str()), ("", "ascii"));
        assert_eq!(b[0].items[0].text, "WALL # fg STONE");
        assert_eq!(b[1].kind, "settings");
        assert!(errs("tiles.blocks\n- WALL move\n")[0].message.contains("only `tiles.glyphs`"));
    }

    #[test]
    fn library_game_block_may_be_unnamed() {
        let b = blocks("game\n- FACTIONS GROUPS all\n");
        assert_eq!(b[0].name, "");
        assert!(errs("entity\n- HP now 3\n")[0].message.contains("needs a name"));
        assert!(errs("menu pause\n- A\n")[0].message.contains("upper-case NAME"));
    }

    #[test]
    fn fenced_map_after_blank_line_with_sub_blocks() {
        let src = "map CAVE size 6x3\n\n```\n######\n#.@..#\n######\n```\n\nlegend:\n- # WALL\n\n- . FLOOR\nvars:\n- DANGER 2\nat:\n- 1,1 as START\n";
        let b = &blocks(src)[0];
        assert_eq!(b.drawing, ["######", "#.@..#", "######"]);
        assert_eq!(b.drawing_line, 4);
        assert_eq!(b.sub("legend").count(), 2, "legend continues over a blank line");
        assert_eq!(b.sub("vars").count(), 1);
        assert_eq!(b.sub("at").next().unwrap().text, "1,1 as START");
        assert_eq!(b.arg_after("size"), Some("6x3"));
    }

    #[test]
    fn unfenced_drawing_ends_at_legend() {
        let b = &blocks("layout MAIN\nmms\nlll\nlegend:\n- m map\n")[0];
        assert_eq!(b.drawing, ["mms", "lll"]);
        assert_eq!(b.sub("legend").count(), 1);
    }

    #[test]
    fn extending_map_has_no_drawing() {
        let b = &blocks("map TOWN.MOON extends TOWN.A transient\n\nlegend:\n- w FLOOR\n")[0];
        assert!(b.drawing.is_empty());
        assert_eq!(b.sub("legend").count(), 1);
    }

    #[test]
    fn roguescript_blocks() {
        let pf = parse("t", "namespace STD\n\ncomponent HP\n- now number\n\n```roguescript\nfn F(x: number):\n    return\n```\n").unwrap();
        let Entry::Code(c) = &pf.entries[1] else { panic!() };
        assert_eq!(c.lines, ["fn F(x: number):", "    return"]);
        assert_eq!(c.line, 7);
        assert_eq!(c.namespace.as_deref(), Some("STD"));
        assert!(errs("```python\nx\n```\n")[0].message.contains("unknown code block language"));
        assert!(errs("menu M\n- A\n```\nx\n```\n")[0].message.contains("must hold RogueScript"));
        assert!(errs("```roguescript\nfn x()\n")[0].message.contains("unterminated"));
    }

    #[test]
    fn language_files() {
        let pf = parse("fi.rlml", "> Suomi\nnamespace UNDEAD\nlanguage fi\n\nPAUSE.TITLE Tauko\n> c\nINTRO.START Aloita **heti**.\n").unwrap();
        assert_eq!(pf.language, Some(("fi".into(), 3)));
        assert_eq!(pf.namespace.as_deref(), Some("UNDEAD"));
        assert_eq!(pf.texts[0].key(), "PAUSE.TITLE");
        assert_eq!(pf.texts[1].rest(1), "Aloita **heti**.", "no emphasis comments in language files");
        assert!(errs("menu M\n- A\n\nlanguage fi\n")[0].message.contains("first definition"));
        assert!(errs("language fi\nA a\nmenu M\n")[0].message.contains("only `KEY text`"));
        assert!(errs("language Finnish\n")[0].message.contains("lower-case"));
    }

    #[test]
    fn blank_line_ends_a_block() {
        let e = errs("menu M\n- A\n\n- B\n");
        assert!(e[0].message.contains("ended at the blank line"), "{e:?}");
        assert_eq!(e[0].line, 4);
    }

    #[test]
    fn comments_and_markers() {
        let b = blocks("> c\nentity GOBLIN **basic enemy**\n- HP now 8 ~~was 6~~\n- drop MELEE\n");
        assert_eq!(b[0].args_text, "");
        assert_eq!(b[0].items[0].text, "HP now 8");
        let (m, rest) = b[0].items[1].marker();
        assert_eq!((m, rest.key()), (Modifier::Drop, "MELEE"));
    }

    #[test]
    fn modifiers_and_params() {
        let b = blocks("patch entity STD.DEFAULT.PLAYER\n- HP now 15\n\nmenu SHOP.BUY(wares: collection, merchant: entity) list size 36x*\n- source wares\n");
        assert_eq!((b[0].modifier, b[0].name.as_str()), (Modifier::Patch, "STD.DEFAULT.PLAYER"));
        assert_eq!(b[1].params.as_ref().unwrap()[1], Param { name: "merchant".into(), ty: Some("entity".into()) });
        assert_eq!(b[1].args_text, "list size 36x*");
    }

    #[test]
    fn namespace_rules() {
        let pf = parse("t", "# Otsikko\n> c\nnamespace STD\nmenu M\n- A\n").unwrap();
        assert_eq!(pf.blocks().next().unwrap().items[0].namespace.as_deref(), Some("STD"));
        assert!(errs("menu M\n- A\n\nnamespace X\n")[0].message.contains("before the first definition"));
    }

    #[test]
    fn rogue_files() {
        let pf = parse_rogue("ai.rogue", "// c\nnamespace UNDEAD\nai GHOST:\n    return WAIT()\n").unwrap();
        let Entry::Code(c) = &pf.entries[0] else { panic!() };
        assert_eq!(c.namespace.as_deref(), Some("UNDEAD"));
        assert_eq!(c.lines[2], "ai GHOST:", "line numbers are kept");
    }
}
