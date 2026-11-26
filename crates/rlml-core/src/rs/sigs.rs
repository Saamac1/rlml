//! Engine functions, events and fields (M 6.8, M 9.1–9.6).
//!
//! The functions are written as a table in nearly the notation of M 9.3,
//! one version per line, so the table can be compared with the
//! specification row by row:
//!
//! ```text
//! LAJI nimi(param: tyyppi, [valinnainen: tyyppi], …) -> paluu [konteksti]
//! ```
//!
//! * `LAJI` is `changes`, `receiver` (changes its receiver), `query` or
//!   `present` (M 9.2).
//! * A parameter in brackets has a default. A name starting with `_` is
//!   positional only. `…` takes further named arguments (text parameters
//!   of `say`, `log` and `text_of`).
//! * Type aliases: `area` is M 9.3's *alue* (an area, a position, an entity
//!   on a map or a list of positions, see [`super::types::fits`]), `set` is
//!   *joukko*, `anylist` and `anydict` are any list or dictionary, and `?` is
//!   any value.
//! * Return `member` is a member of the first argument and `same` the first
//!   argument's type (`choose`, `shuffled`).
//! * The context is `generator`, `coroutine` or `query_or_present`; without
//!   one the function is allowed everywhere.

use std::sync::OnceLock;

use super::types::{self, Member, Ty};

/// How an engine function relates to the game's state (M 9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnKind {
    /// Changes the game's state.
    Changes,
    /// Changes its receiver: a local copy or list is fine in a query.
    ChangesReceiver,
    Query,
    Present,
}

/// Where a function may be called (M 9.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Anywhere,
    Generator,
    /// A coroutine scene or a timeline (`say`).
    Coroutine,
    /// A query, presentation, panel, list menu or naming query (`text_of`).
    QueryOrPresent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ret {
    Ty(Ty),
    /// A member of the first argument.
    Member,
    /// The first argument's type.
    Same,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SigParam {
    pub name: String,
    pub ty: Ty,
    pub optional: bool,
    /// Positional only: the specification's name is not a valid argument name.
    pub positional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sig {
    pub name: String,
    pub kind: FnKind,
    pub params: Vec<SigParam>,
    /// Takes further named arguments (`…`).
    pub rest: bool,
    pub ret: Ret,
    pub context: Context,
    /// The table line, for messages that list versions.
    pub text: String,
}

const TABLE: &str = "
# Tila ja ilmentymät
changes spawn(x: name, at: area) -> entity
changes spawn(x: card, at: area) -> collection
changes spawn(x: deck, at: area) -> collection
changes spawn(x: table, at: area) -> collection
changes spawn(x: name|card|deck|table, into: collection) -> ?
changes spawn(x: name|card|deck|table, map: name, at: label) -> ?
changes destroy(e: entity) -> none
changes add(e: entity, k: component) -> entity
changes remove(e: entity, k: name) -> entity
changes move(e: entity, to: area|collection, [via: ?]) -> bool
changes tile(p: area, t: name|none, [layer: name], [label: label]) -> bool
changes tag(a: area, n: label) -> area
query entities([in: area]) -> collection
changes control(e: entity, [seat: number]) -> entity
changes travel(e: entity, map: name, [at: label], [align: bool]) -> entity
changes follow(c: camera, [e: entity]) -> none

# Ruutujoukot
query tiles() -> area
query changed([layer: name]) -> area
query at(h: name) -> area
query near(a: area, d: number|range) -> area
query from(a: area, x: number, y: number) -> area
query from(a: area, d: number, s: dir|side) -> area
query rect(k: pos, size: size) -> area
query rect(k: pos, width: number, height: number) -> area
query line(a: area, b: area) -> area
query path(a: area, b: area, [for: entity]) -> area
query label(n: label) -> area
query edge(s: dir) -> area
query pos(x: number, y: number) -> pos
query region(p: pos, of: area) -> area

# Satunnaisuus
query choose(j: set) -> member
query shuffled(j: set) -> same
query pick(j: set, n: number) -> same
query chance(p: percent) -> bool

# Listat
receiver insert(l: anylist, i: number, v: ?) -> same
receiver remove_at(l: anylist, i: number) -> same
query index_of(l: anylist, v: ?) -> number
receiver remove(d: anydict, k: ?) -> same

# Pakat ja taulut
receiver shuffle(d: deck) -> deck
receiver draw(d: deck, [n: number|dice|all]) -> collection card
query draw(t: table, [n: number|dice]) -> collection card
query peek(d: deck, [n: number]) -> collection card
receiver put(d: deck, card: card|name, [bottom: bool], [shuffle: bool]) -> deck
receiver add(d: deck, other: deck, [shuffle: bool]) -> deck
receiver reset(d: deck) -> deck
query copy(d: deck) -> deck

# Esto, maasto, linja ja lento
query blocks(p: area, what: ?, [for: entity], [tiles: bool], [except: entity]) -> bool
query tile_of(p: pos, [layer: name]) -> name
query tile_original(p: pos, [layer: name]) -> name
query adjacent(e: entity) -> area
query ray(a: pos, b: pos, [for: entity], [range: number], [mode: name]) -> area
present fly(tile: name, route: area, speed: time) -> none

# Kyselyt
query min(a: number, b: number) -> number
query max(a: number, b: number) -> number
query distance(a: area, b: area) -> number
query nearest(j: set, a: area) -> member
query sees(a: entity, b: area) -> bool
query sees(a: collection, b: area) -> bool
query in_view(x: area) -> bool
query seats() -> list number
query seat_of(e: entity) -> number
query members(n: number) -> collection
query active(n: number) -> entity
query controlled(e: entity) -> bool
query leashed(who: entity, to: pos) -> bool
query view_width() -> number
query view_height() -> number
query text_of(key: name, …) -> text query_or_present

# Generointi
changes carve(a: area, t: name, [layer: name], [from: area]) -> none
query footprint(p: name, at: pos) -> area
changes stamp(p: name, at: pos) -> none
changes external(program: text, …) -> bool generator
changes scatter(x: name|card|deck|table, count: number|range|dice, in: area) -> collection

# Kohtaukset ja tallennus
changes start(k: scene, [scope: name], [bind: entity|collection]) -> handle
changes stop(h: handle) -> none
changes stop(track: name) -> none coroutine
present say(key: name, …, [speaker: entity]) -> none coroutine
present log(key: name, …, [speaker: entity]) -> none
changes checkpoint() -> none
changes rollback() -> none
changes autosave() -> none
changes delete_save(target: ?) -> none

# Esitys
present play(sound: name, [at: area]) -> none
present pan(c: camera, to: area) -> none
present shake(c: camera, strength: number) -> none
present fade(v: light, to: percent) -> none
present light(a: area) -> light
present reveal(a: area) -> none
present today() -> date

# Äänet (M 6.8)
changes noise(p: pos, kind: name, [source: entity], [rounds: number]) -> none
query heard(a: entity) -> collection sound
query hears(a: entity, b: entity) -> bool
";

/// `joukko` (M 9.3): a collection, list, area, range or wildcard.
fn set() -> Ty {
    Ty::Union(vec![
        Ty::Collection(Member::Entity),
        Ty::Collection(Member::Card),
        Ty::Collection(Member::Name),
        Ty::Collection(Member::Sound),
        Ty::list(Ty::Unknown),
        Ty::Area,
        Ty::Range,
    ])
}

fn sig_type(text: &str) -> Result<Ty, String> {
    let alt = |t: &str| -> Result<Ty, String> {
        Ok(match t {
            "?" => Ty::Unknown,
            "set" => set(),
            "anylist" => Ty::list(Ty::Unknown),
            "anydict" => Ty::Dict(Box::new(Ty::Unknown), Box::new(Ty::Unknown)),
            t => types::parse(t, &|_| false)?,
        })
    };
    let alts: Vec<Ty> = text
        .split('|')
        .map(|t| alt(t.trim()))
        .collect::<Result<_, _>>()?;
    Ok(if alts.len() == 1 {
        alts.into_iter().next().unwrap()
    } else {
        Ty::Union(alts)
    })
}

fn parse_line(line: &str) -> Result<Sig, String> {
    let (kind, rest) = line.split_once(' ').ok_or("missing kind")?;
    let kind = match kind {
        "changes" => FnKind::Changes,
        "receiver" => FnKind::ChangesReceiver,
        "query" => FnKind::Query,
        "present" => FnKind::Present,
        k => return Err(format!("unknown kind `{k}`")),
    };
    let open = rest.find('(').ok_or("missing `(`")?;
    let close = rest.rfind(')').ok_or("missing `)`")?;
    let name = rest[..open].trim().to_string();
    let mut params = Vec::new();
    let mut more = false;
    for p in rest[open + 1..close]
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        if p == "…" {
            more = true;
            continue;
        }
        let optional = p.starts_with('[');
        let p = p.trim_start_matches('[').trim_end_matches(']');
        let (pname, ty) = p
            .split_once(':')
            .ok_or_else(|| format!("parameter `{p}` needs a type"))?;
        let pname = pname.trim();
        params.push(SigParam {
            name: pname.trim_start_matches('_').to_string(),
            ty: sig_type(ty.trim())?,
            optional,
            positional: pname.starts_with('_'),
        });
    }
    let after = rest[close + 1..]
        .trim()
        .strip_prefix("->")
        .ok_or("missing `->`")?
        .trim();
    let (ret, context) = match after.rsplit_once(' ') {
        Some((r, c)) if ["generator", "coroutine", "query_or_present"].contains(&c) => {
            (r.trim(), c)
        }
        _ => (after, ""),
    };
    let ret = match ret {
        "member" => Ret::Member,
        "same" => Ret::Same,
        r => Ret::Ty(sig_type(r)?),
    };
    let context = match context {
        "generator" => Context::Generator,
        "coroutine" => Context::Coroutine,
        "query_or_present" => Context::QueryOrPresent,
        _ => Context::Anywhere,
    };
    Ok(Sig {
        name,
        kind,
        params,
        rest: more,
        ret,
        context,
        text: line.to_string(),
    })
}

/// Every engine function version, in table order.
pub fn all() -> &'static [Sig] {
    static SIGS: OnceLock<Vec<Sig>> = OnceLock::new();
    SIGS.get_or_init(|| {
        TABLE
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| parse_line(l).unwrap_or_else(|e| panic!("signature table: `{l}`: {e}")))
            .collect()
    })
}

/// The versions of an engine function.
pub fn versions(name: &str) -> Vec<&'static Sig> {
    all().iter().filter(|s| s.name == name).collect()
}

/// The function's kind for checks that do not pick a version: a function
/// whose versions differ (`draw`, `add`, `remove`) counts as changing its
/// receiver, so a local copy is allowed (M 9.4).
pub fn kind_of(name: &str) -> Option<FnKind> {
    let v = versions(name);
    let first = v.first()?.kind;
    Some(if v.iter().all(|s| s.kind == first) {
        first
    } else {
        FnKind::ChangesReceiver
    })
}

/// Parameters of engine events for a general handler; a component's
/// handler leaves out the first, the subject (M 9.6).
pub fn engine_event(name: &str) -> Option<Vec<(&'static str, Ty)>> {
    use Ty::*;
    Some(match name {
        "begin" => vec![],
        "turn" => vec![],
        "wake" => vec![("who", Entity)],
        "created" | "removed" => vec![("e", Entity)],
        "enter" => vec![("who", Entity), ("p", Pos), ("prev", Pos)],
        "leave" => vec![("who", Entity), ("p", Pos)],
        "arrive" | "depart" => vec![("who", Entity), ("map", Name), ("p", Pos)],
        "hear" => vec![("who", Entity), ("s", Sound)],
        "join" | "part" => vec![("seat", Number)],
        _ => return Option::None,
    })
}

/// Read-only fields the engine keeps on values (M 3.4, M 6.8, RS 2.6,
/// RS 5.3, M 9.3).
pub fn field(of: &Ty, name: &str) -> Option<Ty> {
    use Ty::*;
    Some(match (of, name) {
        (Entity, "at") => Pos,
        (Entity, "map") | (Entity, "kind") => Name,
        (Entity, "slot") => Number,
        (Entity, "stack") => Collection(Member::Entity),
        (Pos, "x") | (Pos, "y") => Number,
        (Sound, "kind") => Name,
        (Sound, "at") => Pos,
        (Sound, "source") => Entity,
        (Sound, "level") => Number,
        (Date, "year" | "month" | "day") => Number,
        (Deck, "count") => Number,
        (Collection(_) | List(_) | Area | Dict(..), "count") => Number,
        (Collection(_) | List(_) | Area, "first" | "last") => of.member(),
        (Dict(k, _), "keys") => Ty::list((**k).clone()),
        _ => return Option::None,
    })
}

/// The engine's `game` fields, all read-only for logic (D 3.3, M 9.1).
pub fn game_field(name: &str) -> Option<Ty> {
    use Ty::*;
    Some(match name {
        "start" | "keys" | "title" | "layout" | "naming" | "walk" | "walk_stop" | "leader"
        | "rest" | "follow_ai" => Name,
        "at" => Area,
        "seed" | "turn" | "time" | "call_depth" | "loop_steps" | "masking" | "follow_distance" => {
            Number
        }
        "party" => Name,
        "leash" => Bool,
        "seats" => Range,
        _ => return Option::None,
    })
}

/// An engine component's field type (D 5.3), from the data definitions.
pub fn engine_component_field(component: &str, field: &str) -> Option<Ty> {
    let fields = crate::fields::engine_component(component)?;
    let f = fields.into_iter().find(|f| f.name == field)?;
    Some(match f.ty.as_str() {
        // a movement mode is a name (D 7.5)
        "mode" => Ty::Name,
        ty => types::parse(ty, &|_| false).unwrap_or(Ty::Unknown),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_reads() {
        let all = all();
        assert!(all.len() > 70, "{} versions", all.len());
        let spawn = versions("spawn");
        assert_eq!(spawn.len(), 6);
        assert_eq!(spawn[0].params[1].name, "at");
        let travel = &versions("travel")[0];
        assert!(
            travel.params[1].name == "map"
                && !travel.params[1].positional
                && travel.params[2].optional
        );
        assert_eq!(
            versions("draw")[0].params[1].ty.to_string(),
            "number|dice|all"
        );
        assert_eq!(versions("choose")[0].ret, Ret::Member);
        assert!(versions("say")[0].rest && versions("say")[0].context == Context::Coroutine);
        assert_eq!(versions("external")[0].context, Context::Generator);
    }

    /// Versions are told apart by the first parameter's type, by named
    /// parameters or by the number of arguments (M 9.4).
    #[test]
    fn versions_are_distinct() {
        let mut names: Vec<&str> = all().iter().map(|s| s.name.as_str()).collect();
        names.dedup();
        for name in names {
            let v = versions(name);
            for (i, a) in v.iter().enumerate() {
                for b in &v[i + 1..] {
                    let first =
                        |s: &Sig| s.params.first().map(|p| p.ty.clone()).unwrap_or(Ty::None);
                    let (fa, fb) = (first(a), first(b));
                    // a one-way conversion (a name is a card) is settled by the exact match
                    let by_type = fa != fb && !(types::fits(&fa, &fb) && types::fits(&fb, &fa));
                    let named = |s: &Sig| {
                        s.params
                            .iter()
                            .filter(|p| !p.positional)
                            .map(|p| p.name.clone())
                            .collect::<Vec<_>>()
                    };
                    let by_names = named(a).iter().any(|n| !named(b).contains(n))
                        || named(b).iter().any(|n| !named(a).contains(n));
                    let arity = |s: &Sig| s.params.iter().filter(|p| !p.optional).count();
                    let by_arity = arity(a) != arity(b);
                    // `from` is told apart by its third argument (M 9.2)
                    let by_third = name == "from";
                    assert!(
                        by_type || by_names || by_arity || by_third,
                        "{name}: `{}` and `{}` cannot be told apart",
                        a.text,
                        b.text
                    );
                }
            }
        }
    }

    #[test]
    fn kinds() {
        assert_eq!(kind_of("spawn"), Some(FnKind::Changes));
        assert_eq!(
            kind_of("draw"),
            Some(FnKind::ChangesReceiver),
            "deck receiver, table query"
        );
        assert_eq!(kind_of("add"), Some(FnKind::ChangesReceiver));
        assert_eq!(kind_of("chance"), Some(FnKind::Query));
        assert_eq!(kind_of("say"), Some(FnKind::Present));
        assert_eq!(kind_of("nope"), None);
    }

    #[test]
    fn fields_and_events() {
        assert_eq!(field(&Ty::Entity, "at"), Some(Ty::Pos));
        assert_eq!(
            field(&Ty::Collection(Member::Card), "first"),
            Some(Ty::Card)
        );
        assert_eq!(field(&Ty::Area, "first"), Some(Ty::Pos));
        assert_eq!(field(&Ty::Deck, "first"), None, "a deck has only `.count`");
        assert_eq!(
            field(&Ty::Table, "count"),
            None,
            "a table has no fields (RS 5.5)"
        );
        assert_eq!(game_field("start"), Some(Ty::Name));
        assert_eq!(engine_component_field("sight", "radius"), Some(Ty::Number));
        assert_eq!(engine_component_field("movement", "mode"), Some(Ty::Name));
        assert_eq!(engine_event("arrive").unwrap()[1], ("map", Ty::Name));
    }
}
