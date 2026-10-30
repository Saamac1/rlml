//! RogueScript declarations as the model sees them (RS 12.1): kind, name,
//! parameters and place, with the parsed declaration for the checks that
//! read bodies. The parser is in [`crate::rs`].

use std::sync::Arc;

use crate::error::Diagnostic;
use crate::rs::ast::{Item, ItemKind, Param};
use crate::syntax::RawCode;

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

/// `name: type [= default]`, written back as text.
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
    pub item: Arc<Item>,
}

fn params(ps: &[Param]) -> Vec<RsParam> {
    ps.iter().map(|p| RsParam { name: p.name.clone(), ty: p.ty.to_string(), default: p.default.as_ref().map(|d| d.to_string()) }).collect()
}

/// Parses a code block and lists its top-level declarations.
pub fn declarations(code: &RawCode) -> (Vec<Decl>, Vec<Diagnostic>) {
    let (items, errors) = crate::rs::parse::parse(code);
    let decls = items
        .into_iter()
        .map(|item| {
            let (kind, ps, event) = match &item.kind {
                ItemKind::Component { fields, .. } => (DeclKind::Component, params(fields), None),
                ItemKind::Event { params: p } => (DeclKind::Event, params(p), None),
                ItemKind::Handler(h) => (
                    DeclKind::Handler,
                    h.names.iter().map(|n| RsParam { name: n.clone(), ty: String::new(), default: None }).collect(),
                    Some(h.event.clone()),
                ),
                ItemKind::Function { query: true, params: p, .. } => (DeclKind::Query, params(p), None),
                ItemKind::Function { params: p, .. } => (DeclKind::Function, params(p), None),
                ItemKind::Command { params: p, .. } => (DeclKind::Command, params(p), None),
                ItemKind::CommandAlias(_) => (DeclKind::Command, vec![], None),
                ItemKind::Ai { .. } => (DeclKind::Ai, vec![], None),
                ItemKind::Generator { params: p, .. } => (DeclKind::Generator, params(p), None),
                ItemKind::Map { .. } => (DeclKind::Map, vec![], None),
                ItemKind::Scene { params: p, .. } | ItemKind::Timeline { params: p, .. } => (DeclKind::Scene, params(p), None),
            };
            Decl {
                kind,
                name: item.name.clone(),
                overrides: item.overrides,
                params: ps,
                event,
                file: code.file.clone(),
                line: item.pos.line,
                namespace: code.namespace.clone(),
                item: Arc::new(item),
            }
        })
        .collect();
    (decls, errors)
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
        assert_eq!(d[1].namespace.as_deref(), Some("STD"));
    }

    #[test]
    fn handlers_and_components() {
        let d = decls(
            "after HIT(target, attacker, amount) as HIT_MESSAGE priority -10 when WITNESSED(target):\n    log(X)\non begin() as SPAWN_PLAYER:\n    x = 1\non turn():\n    x = 2\ncomponent FOLLOW(target: entity = none, distance: number = 1)\n",
        );
        assert_eq!((d[0].event.as_deref(), d[0].name.as_str()), (Some("HIT"), "HIT_MESSAGE"));
        assert_eq!(d[1].name, "SPAWN_PLAYER");
        assert_eq!(d[2].name, "", "unnamed handler");
        assert_eq!(d[3].params[1].default.as_deref(), Some("1"));
    }

    #[test]
    fn sources_are_not_part_of_the_type() {
        let d = decls(
            "command FEED(item: entity from actor.PACK.items where it has FOOD, n: number count) cost 120:\n    return\ncomponent PACK(items: collection 26 merge STACKABLE)\n",
        );
        assert_eq!((d[0].params[0].ty.as_str(), d[0].params[1].ty.as_str()), ("entity", "number"));
        assert_eq!(d[1].params[0].ty, "collection 26 merge STACKABLE");
    }

    #[test]
    fn errors() {
        let (_, e) = declarations(&code("fn F():\n\treturn\n"));
        assert!(e[0].message.contains("tab"));
        let (_, e) = declarations(&code("x = 1\n"));
        assert!(e[0].message.contains("expected a declaration"));
        let (_, e) = declarations(&code("command move():\n    return\n"));
        assert!(e[0].message.contains("upper-case NAME"));
    }
}
