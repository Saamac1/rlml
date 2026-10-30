//! RogueScript declarations (RS 12.1).
//!
//! The loader reads only the top-level declarations of a code block: their
//! kind, name and, for components, events and commands, their parameters.
//! Bodies are kept as text; statements and expressions are not parsed yet.
//!
//! ```text
//! [override] component NAME[(fields)][:]
//! [override] event NAME(params)
//! [override] (before|on|after) EVENT(names) [as NAME] [priority N] [when …]:
//! [override] [present] fn NAME(params) [-> type]:
//! [override] query NAME(params) -> type:
//! [override] command NAME(params) [cost N]:   |   command NAME = CALL
//! [override] ai NAME:
//! [override] generator NAME(params) [size WxH]:
//! [override] map NAME [transient] [wrap] [nosave] generate CALL
//! [override] scene NAME(params) …:
//! ```

use crate::error::Diagnostic;
use crate::syntax::{RawCode, matching_close, split_top_level};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Component,
    Event,
    Handler,
    Function,
    Query,
    Command,
    Ai,
    Generator,
    Map,
    Scene,
}

/// `name: type [= default]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsParam {
    pub name: String,
    pub ty: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decl {
    pub kind: DeclKind,
    /// Written name; empty for an unnamed handler.
    pub name: String,
    pub overrides: bool,
    pub params: Vec<RsParam>,
    /// The event a handler listens to.
    pub event: Option<String>,
    pub file: String,
    pub line: usize,
    pub namespace: Option<String>,
}

const KEYWORDS: &[(&str, DeclKind)] = &[
    ("component", DeclKind::Component),
    ("event", DeclKind::Event),
    ("before", DeclKind::Handler),
    ("on", DeclKind::Handler),
    ("after", DeclKind::Handler),
    ("fn", DeclKind::Function),
    ("query", DeclKind::Query),
    ("command", DeclKind::Command),
    ("ai", DeclKind::Ai),
    ("generator", DeclKind::Generator),
    ("map", DeclKind::Map),
    ("scene", DeclKind::Scene),
];

/// Reads the top-level declarations of a code block.
pub fn declarations(code: &RawCode) -> (Vec<Decl>, Vec<Diagnostic>) {
    let mut decls = Vec::new();
    let mut errors = Vec::new();
    let mut open_block = false;
    for (i, line) in code.lines.iter().enumerate() {
        let lineno = code.line + i;
        if line.contains('\t') {
            errors.push(Diagnostic::error(&code.file, lineno, "tab in RogueScript: indent with four spaces (RS 2.2)"));
            continue;
        }
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent > 0 {
            if !open_block {
                errors.push(Diagnostic::error(&code.file, lineno, "indented line outside a block"));
            } else if indent % 4 != 0 {
                errors.push(Diagnostic::error(&code.file, lineno, "indentation must be a multiple of four spaces (RS 2.2)"));
            }
            continue;
        }
        match declaration(code, lineno, t) {
            Ok(d) => {
                open_block = t.split("//").next().unwrap_or(t).trim_end().ends_with(':');
                decls.push(d);
            }
            Err(e) => {
                open_block = true;
                errors.push(e);
            }
        }
    }
    (decls, errors)
}

fn declaration(code: &RawCode, lineno: usize, line: &str) -> Result<Decl, Diagnostic> {
    let err = |m: String| Diagnostic::error(&code.file, lineno, m);
    let mut rest = line;
    let overrides = word(rest) == "override";
    if overrides {
        rest = after_word(rest);
    }
    if word(rest) == "present" {
        rest = after_word(rest);
    }
    let kw = word(rest);
    let kind = KEYWORDS.iter().find(|(k, _)| *k == kw).map(|(_, d)| *d).ok_or_else(|| err(format!("expected a declaration, found `{}`", word(line))))?;
    rest = after_word(rest);
    let name_end = rest.find(|c: char| c.is_whitespace() || c == '(' || c == ':').unwrap_or(rest.len());
    let written = &rest[..name_end];
    let tail = &rest[name_end..];
    let params = match tail.strip_prefix('(') {
        Some(_) => {
            let close = matching_close(tail, 0).ok_or_else(|| err("missing `)`".into()))?;
            let inner = &tail[1..close];
            if kind == DeclKind::Handler {
                // a handler names the event's parameters without types
                inner.split(',').map(str::trim).filter(|n| !n.is_empty()).map(|n| RsParam { name: n.to_string(), ty: String::new(), default: None }).collect()
            } else {
                parse_params(inner).map_err(err)?
            }
        }
        None => vec![],
    };
    let mut decl = Decl {
        kind,
        name: written.to_string(),
        overrides,
        params,
        event: None,
        file: code.file.clone(),
        line: lineno,
        namespace: code.namespace.clone(),
    };
    if kind == DeclKind::Handler {
        decl.event = Some(written.to_string());
        decl.name = tail.split_whitespace().skip_while(|w| *w != "as").nth(1).map(|n| n.trim_end_matches(':').to_string()).unwrap_or_default();
        if !decl.name.is_empty() && !crate::names::is_name(&decl.name) {
            return Err(err(format!("bad handler name `{}`", decl.name)));
        }
    } else if !crate::names::is_name(written) {
        return Err(err(format!("`{kw}` needs an upper-case NAME, not `{written}`")));
    }
    Ok(decl)
}

fn word(s: &str) -> &str {
    s.split(|c: char| c.is_whitespace() || c == '(').next().unwrap_or("")
}

fn after_word(s: &str) -> &str {
    s[word(s).len()..].trim_start()
}

/// Parses `a: number = 1, b: list area` (RS 12.1 `params` and `fields`).
pub fn parse_params(s: &str) -> Result<Vec<RsParam>, String> {
    let mut out = Vec::new();
    for p in split_top_level(s, ',').into_iter().map(str::trim).filter(|p| !p.is_empty()) {
        let (name, rest) = p.split_once(':').ok_or_else(|| format!("parameter `{p}` needs a type (`name: type`)"))?;
        let name = name.trim();
        if !crate::names::is_vocabulary(name) && !crate::names::is_name(name) {
            return Err(format!("bad parameter name `{name}`"));
        }
        // a source (`from …`, `aim …`, `ask`, `count`) follows the type and default
        let rest = cut_source(rest.trim());
        let (ty, default) = match split_top_level(rest, '=').as_slice() {
            [ty] => (ty.trim().to_string(), None),
            [ty, d] => (ty.trim().to_string(), Some(d.trim().to_string())),
            _ => return Err(format!("bad parameter `{p}`")),
        };
        out.push(RsParam { name: name.to_string(), ty, default });
    }
    Ok(out)
}

fn cut_source(s: &str) -> &str {
    let mut pos = 0;
    for w in s.split_whitespace() {
        let at = pos + s[pos..].find(w).unwrap();
        if ["from", "aim", "ask", "count"].contains(&w) {
            return s[..at].trim_end();
        }
        pos = at + w.len();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(src: &str) -> RawCode {
        RawCode { file: "t".into(), line: 10, namespace: Some("STD".into()), lines: src.lines().map(String::from).collect() }
    }

    fn decls(src: &str) -> Vec<Decl> {
        let (d, e) = declarations(&code(src));
        assert!(e.is_empty(), "{e:?}");
        d
    }

    #[test]
    fn reads_declarations() {
        let d = decls(
            "event HIT(target: entity, attacker: entity, amount: number, cause: name = none)\n\n// c\nfn HURT(target: entity, amount: number):\n    emit HIT(target, none, amount)\n\ncommand MOVE.NORTH = MOVE(north)\nai HUNTER:\n    return WAIT()\n",
        );
        assert_eq!(d.iter().map(|d| (d.kind, d.name.as_str())).collect::<Vec<_>>(), [
            (DeclKind::Event, "HIT"),
            (DeclKind::Function, "HURT"),
            (DeclKind::Command, "MOVE.NORTH"),
            (DeclKind::Ai, "HUNTER")
        ]);
        assert_eq!(d[0].params[3], RsParam { name: "cause".into(), ty: "name".into(), default: Some("none".into()) });
        assert_eq!(d[1].line, 13);
    }

    #[test]
    fn handlers_and_components() {
        let d = decls(
            "after HIT(target, attacker, amount) as HIT_MESSAGE priority -10 when WITNESSED(target):\n    log(X)\non begin() as SPAWN_PLAYER:\n    x = 1\non turn():\n    x = 2\ncomponent FOLLOW(target: entity = none, distance: number = 1)\n",
        );
        assert_eq!((d[0].event.as_deref(), d[0].name.as_str()), (Some("HIT"), "HIT_MESSAGE"));
        assert_eq!(d[1].name, "SPAWN_PLAYER");
        assert_eq!(d[2].name, "", "unnamed handler");
        assert_eq!(d[3].params.len(), 2);
    }

    #[test]
    fn sources_are_not_part_of_the_type() {
        let p = parse_params("item: entity from actor.PACK.items where it has FOOD, target: pos aim 1 at entities(in: adjacent(actor))").unwrap();
        assert_eq!((p[0].ty.as_str(), p[1].ty.as_str()), ("entity", "pos"));
        let p = parse_params("items: collection 26 merge STACKABLE, n: number count").unwrap();
        assert_eq!((p[0].ty.as_str(), p[1].ty.as_str()), ("collection 26 merge STACKABLE", "number"));
    }

    #[test]
    fn layout_errors() {
        let (_, e) = declarations(&code("fn F():\n\treturn\n"));
        assert!(e[0].message.contains("tab"));
        let (_, e) = declarations(&code("fn F():\n  return\n"));
        assert!(e[0].message.contains("multiple of four"));
        let (_, e) = declarations(&code("x = 1\n"));
        assert!(e[0].message.contains("expected a declaration"));
        let (_, e) = declarations(&code("command move():\n    return\n"));
        assert!(e[0].message.contains("upper-case NAME"));
    }
}
