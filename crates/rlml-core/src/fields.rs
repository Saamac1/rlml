//! Component fields and their values (D 5.1–5.3, D 6.4).
//!
//! A component row names its fields (`- HP now 8 max 10`), or leaves the
//! name out when the component has one field (`- MELEE 1d6`). Engine
//! components are vocabulary and written in lower case.

use crate::model::{Def, Kind, Model, Source};
use crate::names;
use crate::syntax::RawItem;
use crate::token::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    /// The type as written: `number`, `collection 26 merge STACKABLE`.
    pub ty: String,
    pub default: Option<String>,
}

impl Field {
    fn new(name: &str, ty: &str, default: Option<&str>) -> Field {
        Field { name: name.to_string(), ty: ty.to_string(), default: default.map(String::from) }
    }

    pub fn base_type(&self) -> &str {
        self.ty.split_whitespace().next().unwrap_or("")
    }
}

pub const DIRS: &[&str] = &["north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest", "none"];
pub const SIDES: &[&str] = &["front", "frontright", "right", "backright", "back", "backleft", "left", "frontleft", "none"];

const TYPES: &[&str] = &[
    "number", "percent", "dice", "bool", "text", "deck", "table", "name", "dir", "side", "pos", "area", "card", "entity", "collection",
    "list", "label", "dict", "handle",
];

/// Engine components and their fields (D 5.3, D 12.3, M 9.1). `blocks` and
/// `timer` have their own row forms.
pub fn engine_component(name: &str) -> Option<Vec<Field>> {
    let f = Field::new;
    Some(match name {
        "render" => vec![f("tile", "name", None)],
        "movement" => vec![
            f("mode", "mode", None),
            f("speed", "number", Some("10")),
            f("through", "name", None),
            f("dirs", "number", Some("8")),
            f("corners", "bool", Some("false")),
        ],
        "ai" => vec![f("behavior", "name", None)],
        "sight" => vec![
            f("radius", "number", None),
            f("arc", "number", Some("360")),
            f("dir", "dir", None),
            f("dark", "number", Some("1")),
            f("through", "name", None),
        ],
        "light" => vec![f("radius", "number", None), f("arc", "number", Some("360"))],
        "facing" => vec![f("dir", "dir", None)],
        "interior" => vec![f("map", "name", None)],
        "shape" => vec![f("prefab", "name", None)],
        "label" => vec![f("name", "label", None)],
        "wire" => vec![f("name", "label", None)],
        "hearing" => vec![f("acuity", "number", Some("0"))],
        "name" => vec![f("text", "text", None)],
        "revealed" => vec![],
        "blocks" | "timer" | "sound" => vec![],
        _ => return None,
    })
}

/// Fields of a component definition, from its data rows or RogueScript
/// parameters.
pub fn component_fields(def: &Def) -> Vec<Field> {
    match &def.source {
        Source::Block(b) => b
            .items
            .iter()
            .map(|i| {
                let words: Vec<&str> = i.tokens.iter().map(|t| t.text.as_str()).collect();
                let d = words.iter().position(|w| *w == "default");
                let ty_end = d.unwrap_or(words.len());
                Field {
                    name: i.key().to_string(),
                    ty: i.span(1, ty_end).to_string(),
                    default: d.map(|d| i.rest(d + 1).to_string()),
                }
            })
            .collect(),
        Source::Decl(d) => d.params.iter().map(|p| Field { name: p.name.clone(), ty: p.ty.clone(), default: p.default.clone() }).collect(),
        _ => vec![],
    }
}

/// Checks a field type as written in a component definition.
pub fn check_type(ty: &str) -> Result<(), String> {
    let words: Vec<&str> = ty.split_whitespace().collect();
    let Some(base) = words.first() else { return Err("missing field type".into()) };
    if !TYPES.contains(base) {
        return Err(format!("unknown field type `{base}`"));
    }
    match *base {
        "deck" if words.len() == 2 && words[1] == "copy" => Ok(()),
        "collection" => {
            let mut i = 1;
            if words.get(i).is_some_and(|w| w.parse::<u32>().is_ok()) {
                i += 1;
            }
            if words.get(i) == Some(&"merge") {
                if !words.get(i + 1).is_some_and(|w| names::is_name(w)) {
                    return Err("`merge` takes a tag NAME".into());
                }
                i += 2;
            }
            if words.get(i) == Some(&"modifies") {
                i += 1;
            }
            if i == words.len() { Ok(()) } else { Err(format!("bad collection type `{ty}`")) }
        }
        "list" => match words.get(1) {
            Some(t) if TYPES.contains(t) || names::is_name(t) => Ok(()),
            _ => Err(format!("`list` needs a member type, as in `list name`: `{ty}`")),
        },
        "dict" => match (words.get(1), words.get(2)) {
            (Some(k), Some(_)) if ["name", "number", "entity"].contains(k) && words.len() == 3 => Ok(()),
            _ => Err(format!("`dict` takes a key type (name, number or entity) and a value type: `{ty}`")),
        },
        _ if words.len() == 1 => Ok(()),
        _ => Err(format!("bad field type `{ty}`")),
    }
}

pub fn is_int(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// `3d6+2`, `1d20`, `d6`.
pub fn is_dice(s: &str) -> bool {
    let (body, bonus) = match s.find(['+', '-']) {
        Some(i) if i > 0 => (&s[..i], Some(&s[i + 1..])),
        _ => (s, None),
    };
    let Some((n, sides)) = body.split_once('d') else { return false };
    (n.is_empty() || n.chars().all(|c| c.is_ascii_digit()))
        && !sides.is_empty()
        && sides.chars().all(|c| c.is_ascii_digit())
        && bonus.is_none_or(|b| !b.is_empty() && b.chars().all(|c| c.is_ascii_digit()))
}

/// `2..5`, `-1..3`, `15..`.
pub fn is_range(s: &str) -> bool {
    match s.split_once("..") {
        Some((a, b)) => is_int(a) && (b.is_empty() || is_int(b)),
        None => false,
    }
}

/// A number value: an integer, or dice or a range rolled when the
/// instance is created (D 5.2).
pub fn is_number_value(s: &str) -> bool {
    is_int(s) || is_dice(s) || is_range(s)
}

pub fn is_pos(s: &str) -> bool {
    matches!(s.split_once(','), Some((x, y)) if !x.is_empty() && !y.is_empty() && x.chars().all(|c| c.is_ascii_digit()) && y.chars().all(|c| c.is_ascii_digit()))
}

/// A parsed component row.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Full name, or the engine component's vocabulary name.
    pub component: String,
    /// Fields given on the row, then the remaining fields' defaults.
    pub values: Vec<(String, String)>,
}

impl Row {
    pub fn get(&self, field: &str) -> Option<&str> {
        self.values.iter().find(|(f, _)| f == field).map(|(_, v)| v.as_str())
    }
}

/// Values checked against the model.
pub struct Values<'a> {
    pub m: &'a Model,
    pub ns: Option<&'a str>,
}

impl Values<'_> {
    /// Checks a component row `COMPONENT values…` (D 5.2).
    pub fn row(&self, item: &RawItem) -> Result<Row, String> {
        let comp = item.key();
        let args = &item.tokens[1..];
        if names::is_vocabulary(comp) {
            let fields = engine_component(comp).ok_or_else(|| format!("unknown engine component `{comp}`"))?;
            return match comp {
                "blocks" => self.blocks(args).map(|_| Row { component: comp.into(), values: vec![] }),
                "timer" | "sound" => Ok(Row { component: comp.into(), values: vec![] }),
                _ => self.fields(comp, &fields, args).map(|values| Row { component: comp.into(), values }),
            };
        }
        let def = self.m.resolve_kind(self.ns, comp, &[Kind::Component])?;
        let fields = component_fields(def);
        let values = self.fields(&def.full, &fields, args)?;
        Ok(Row { component: def.full.clone(), values })
    }

    /// `blocks move sight sound`, `blocks WALK`, `blocks none` (D 7.5, D 12.3).
    pub fn blocks(&self, args: &[Token]) -> Result<(), String> {
        if args.is_empty() {
            return Err("`blocks` needs values: move, sight, sound, a movement mode or none".into());
        }
        for a in args {
            let ok = ["move", "sight", "sound"].contains(&a.text.as_str()) || names::is_name(&a.text) || (a.text == "none" && args.len() == 1);
            if !ok {
                return Err(if a.text == "none" { "`none` is not written with other values".into() } else { format!("`blocks {}`: expected move, sight, sound, a movement mode or none", a.text) });
            }
        }
        Ok(())
    }

    fn fields(&self, comp: &str, fields: &[Field], args: &[Token]) -> Result<Vec<(String, String)>, String> {
        let mut given: Vec<(String, String)> = Vec::new();
        if !args.is_empty() {
            if fields.is_empty() {
                return Err(format!("`{comp}` is a tag and takes no values"));
            }
            let is_field = |t: &Token| !t.quoted && fields.iter().any(|f| f.name == t.text);
            if is_field(&args[0]) {
                let mut i = 0;
                while i < args.len() {
                    let name = &args[i].text;
                    let field = fields.iter().find(|f| &f.name == name).ok_or_else(|| {
                        format!("`{comp}` has no field `{name}` (fields: {})", fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "))
                    })?;
                    let end = (i + 1..args.len()).find(|&j| is_field(&args[j])).unwrap_or(args.len());
                    if given.iter().any(|(f, _)| f == name) {
                        return Err(format!("field `{name}` given twice"));
                    }
                    self.value(&field.ty, &args[i + 1..end]).map_err(|e| format!("`{comp}.{name}`: {e}"))?;
                    given.push((name.clone(), join(&args[i + 1..end])));
                    i = end;
                }
            } else if fields.len() == 1 {
                self.value(&fields[0].ty, args).map_err(|e| format!("`{comp}.{}`: {e}", fields[0].name))?;
                given.push((fields[0].name.clone(), join(args)));
            } else if names::is_vocabulary(&args[0].text) && !args[0].quoted {
                return Err(format!(
                    "`{comp}` has no field `{}` (fields: {})",
                    args[0].text,
                    fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", ")
                ));
            } else {
                return Err(format!(
                    "`{comp}` has {} fields, so the row must name them: {}",
                    fields.len(),
                    fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        for f in fields {
            if !given.iter().any(|(n, _)| n == &f.name)
                && let Some(d) = &f.default
            {
                given.push((f.name.clone(), d.clone()));
            }
        }
        Ok(given)
    }

    /// Checks a value of field type `ty` (D 5.1).
    pub fn value(&self, ty: &str, words: &[Token]) -> Result<(), String> {
        let tw: Vec<&str> = ty.split_whitespace().collect();
        let base = tw.first().copied().unwrap_or("");
        let one = || -> Result<&str, String> {
            match words {
                [w] => Ok(w.text.as_str()),
                [] => Err("missing value".into()),
                _ => Err(format!("expected one value, found `{}`", join(words))),
            }
        };
        match base {
            "number" => one().and_then(|w| if is_number_value(w) { Ok(()) } else { Err(format!("`{w}` is not a number")) }),
            "dice" => one().and_then(|w| if is_number_value(w) { Ok(()) } else { Err(format!("`{w}` is not dice")) }),
            "percent" => one().and_then(|w| match w.strip_suffix('%') {
                Some(n) if is_int(n) => Ok(()),
                _ => Err(format!("`{w}` is not a percentage such as `50%`")),
            }),
            "bool" => one().and_then(|w| if w == "true" || w == "false" { Ok(()) } else { Err(format!("`{w}` is not true or false")) }),
            "text" => match words {
                [w] if w.quoted => Ok(()),
                _ => Err(format!("text is written in quotes: `\"{}\"`", join(words))),
            },
            "dir" => one().and_then(|w| {
                if DIRS.contains(&w) {
                    Ok(())
                } else if SIDES.contains(&w) {
                    Err(format!("`{w}` is a relative direction (`side`), not a direction (`dir`)"))
                } else {
                    Err(format!("`{w}` is not a direction"))
                }
            }),
            "side" => one().and_then(|w| {
                if SIDES.contains(&w) {
                    Ok(())
                } else if DIRS.contains(&w) {
                    Err(format!("`{w}` is a direction (`dir`), not a relative direction (`side`)"))
                } else {
                    Err(format!("`{w}` is not a relative direction"))
                }
            }),
            "pos" => one().and_then(|w| if is_pos(w) { Ok(()) } else { Err(format!("`{w}` is not a position `X,Y`")) }),
            "deck" => one().and_then(|w| self.m.resolve_kind(self.ns, w, &[Kind::Deck]).map(|_| ())),
            "table" => one().and_then(|w| self.m.resolve_kind(self.ns, w, &[Kind::Table]).map(|_| ())),
            "name" => one().and_then(|w| self.name(w)),
            "label" => one().and_then(|w| if names::is_name(w) { Ok(()) } else { Err(format!("`{w}` is not a label NAME")) }),
            // a movement mode is any upper-case name (D 7.5)
            "mode" => one().and_then(|w| if names::is_name(w) { Ok(()) } else { Err(format!("`{w}` is not a movement mode NAME")) }),
            "entity" | "handle" => one().and_then(|w| if w == "none" { Ok(()) } else { Err(format!("an {base} field starts as `none` in data")) }),
            "area" => Err("an area starts empty in data; logic sets it".into()),
            "card" => self.card(words),
            "collection" => self.draw(words, &[Kind::Deck, Kind::Table, Kind::Entity]),
            "list" => {
                let member = tw.get(1).copied().unwrap_or("");
                if ["area"].contains(&member) || names::is_name(member) {
                    return Err(format!("a list of {member} starts empty in data"));
                }
                words.iter().try_for_each(|w| self.value(member, std::slice::from_ref(w)))
            }
            "dict" => {
                if tw.get(1) == Some(&"entity") {
                    return Err("a dictionary with entity keys starts empty in data".into());
                }
                if !words.len().is_multiple_of(2) {
                    return Err("a dictionary is written as `KEY value` pairs".into());
                }
                for pair in words.chunks(2) {
                    self.value(tw.get(1).copied().unwrap_or(""), &pair[..1])?;
                    self.value(tw.get(2).copied().unwrap_or(""), &pair[1..])?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// A `name` value: `none`, any defined name or a tile (D 5.1).
    fn name(&self, w: &str) -> Result<(), String> {
        if w == "none" {
            return Ok(());
        }
        match names::tile_name(w) {
            Ok(t) if !t.variants.is_empty() || t.index.is_some() => self.m.resolve_tile(self.ns, w).map(|_| ()),
            _ => self.m.resolve(self.ns, w).map(|_| ()),
        }
    }

    /// A draw: `SOURCE [N | all] [copy]` from a deck or table, or an entity
    /// with field overrides (D 6.2, D 6.4).
    pub fn draw(&self, words: &[Token], kinds: &[Kind]) -> Result<(), String> {
        let Some(first) = words.first() else { return Err("missing value".into()) };
        if first.text == "none" && words.len() == 1 {
            return Ok(());
        }
        let def = self.m.resolve_kind(self.ns, &first.text, kinds)?;
        if def.kind == Kind::Entity {
            return self.card(words);
        }
        let mut rest = &words[1..];
        if let Some(n) = rest.first()
            && (n.text == "all" || is_number_value(&n.text))
        {
            if n.text == "all" && def.kind == Kind::Table {
                return Err("`all` draws a whole deck; a table is rolled (D 6.5)".into());
            }
            rest = &rest[1..];
        }
        if rest.first().is_some_and(|t| t.text == "copy") {
            if def.kind == Kind::Table {
                return Err("`copy` belongs to decks; a table has no state (D 6.5)".into());
            }
            rest = &rest[1..];
        }
        if rest.is_empty() { Ok(()) } else { Err(format!("unexpected `{}` after the draw", join(rest))) }
    }

    /// A card: an entity, an optional count and field overrides written as
    /// component rows (`RING_MAIL ENCHANT 2`, `LIFT label GATE_A`).
    pub fn card(&self, words: &[Token]) -> Result<(), String> {
        let Some(first) = words.first() else { return Ok(()) };
        self.m.resolve_kind(self.ns, &first.text, &[Kind::Entity])?;
        let mut rest = &words[1..];
        if rest.first().is_some_and(|t| is_number_value(&t.text)) {
            rest = &rest[1..];
        }
        // each override starts at a component name
        let starts: Vec<usize> = (0..rest.len()).filter(|&i| self.is_component(&rest[i].text)).collect();
        if !rest.is_empty() && starts.first() != Some(&0) {
            return Err(format!("`{}` is not a component of the card's overrides", rest[0].text));
        }
        for (k, &s) in starts.iter().enumerate() {
            let e = starts.get(k + 1).copied().unwrap_or(rest.len());
            let text = join(&rest[s..e]);
            let item = RawItem::new("", 0, self.ns.map(String::from), &text).map_err(|d| d.message)?;
            self.row(&item)?;
        }
        Ok(())
    }

    fn is_component(&self, w: &str) -> bool {
        if names::is_vocabulary(w) {
            return engine_component(w).is_some();
        }
        self.m.resolve(self.ns, w).is_ok_and(|d| d.kind == Kind::Component)
    }
}

pub fn join(words: &[Token]) -> String {
    words.iter().map(|t| if t.quoted { format!("\"{}\"", t.text) } else { t.text.clone() }).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use crate::load::{MemFiles, load};
    use crate::model::build;

    const DATA: &str = "component HP\n- now number\n- max number default now\n\ncomponent MELEE\n- damage dice\n\ncomponent EXIT\n- to name\n- at label default START\n\ncomponent BOSS\n\ncomponent RESIST\n- FIRE percent\n\ncomponent PACK\n- items collection 26 merge STACKABLE\n\ncomponent BONDS\n- toward dict entity number\n\nentity RAT\n- HP now 3\n\ndeck KIT\n- RAT\n\nmap CAVE\n\n```\n.\n```\n\nlegend:\n- . FLOOR\n";

    fn model() -> Model {
        let f = MemFiles { game: [("g.rlml".to_string(), DATA.to_string())].into(), ..Default::default() };
        let (l, mut r) = load(&f, "g.rlml");
        let m = build(l, &mut r);
        assert!(r.0.is_empty(), "{r:?}");
        m
    }

    fn row(m: &Model, text: &str) -> Result<Row, String> {
        Values { m, ns: None }.row(&RawItem::new("t", 1, None, text).unwrap())
    }

    #[test]
    fn named_fields_shorthand_and_defaults() {
        let m = model();
        let r = row(&m, "HP now 8").unwrap();
        assert_eq!(r.component, "HP");
        assert_eq!(r.get("max"), Some("now"));
        assert_eq!(row(&m, "MELEE 1d6").unwrap().get("damage"), Some("1d6"));
        assert_eq!(row(&m, "EXIT to CAVE").unwrap().get("at"), Some("START"));
        assert!(row(&m, "HP 8").unwrap_err().contains("must name them: now, max"));
        assert!(row(&m, "HP now 8 now 9").unwrap_err().contains("twice"));
        assert!(row(&m, "HP hp 8").unwrap_err().contains("no field `hp`"));
        assert!(row(&m, "BOSS 1").unwrap_err().contains("tag"));
        assert!(row(&m, "RESIST FIRE 20%").is_ok());
        assert!(row(&m, "RESIST FIRE 20").unwrap_err().contains("percentage"));
    }

    #[test]
    fn value_types() {
        let m = model();
        assert!(row(&m, "MELEE x").unwrap_err().contains("not dice"));
        assert!(row(&m, "HP now 1d4+1").is_ok());
        assert!(row(&m, "EXIT to NOWHERE").unwrap_err().contains("not defined"));
        assert!(row(&m, "PACK KIT all copy").is_ok());
        assert!(row(&m, "PACK HP").unwrap_err().contains("is a component"));
        assert!(row(&m, "BONDS RAT 5").unwrap_err().contains("starts empty"));
    }

    #[test]
    fn engine_components() {
        let m = model();
        assert!(row(&m, "render FLOOR").is_ok());
        assert!(row(&m, "render FLOOR.dark").is_ok(), "a variant of a known tile");
        assert!(row(&m, "sight radius 6 dir front").unwrap_err().contains("relative"));
        assert!(row(&m, "hearing").is_ok());
        assert!(row(&m, "blocks move sight sound").is_ok());
        assert!(row(&m, "blocks none move").unwrap_err().contains("not written with other values"));
        assert!(row(&m, "glow 3").unwrap_err().contains("unknown engine component"));
        assert!(row(&m, "name \"Iolo\"").is_ok());
        assert!(row(&m, "name Iolo").unwrap_err().contains("quotes"));
    }

    #[test]
    fn cards_and_draws() {
        let m = model();
        let v = Values { m: &m, ns: None };
        let toks = |s: &str| crate::token::tokenize(s).unwrap();
        assert!(v.card(&toks("RAT HP now 2")).is_ok());
        assert!(v.card(&toks("RAT 3 label GATE")).is_ok());
        assert!(v.card(&toks("RAT HP 2")).unwrap_err().contains("must name them"));
        assert!(v.card(&toks("RAT oops")).unwrap_err().contains("not a component"));
        assert!(v.draw(&toks("KIT 2 copy"), &[Kind::Deck, Kind::Table]).is_ok());
        assert!(v.draw(&toks("KIT 2 junk"), &[Kind::Deck]).unwrap_err().contains("unexpected"));
    }

    #[test]
    fn types() {
        assert!(check_type("collection 26 merge STACKABLE modifies").is_ok());
        assert!(check_type("list SCORE 10 sort score desc").is_ok());
        assert!(check_type("dict name number").is_ok());
        assert!(check_type("dict text number").is_err());
        assert!(check_type("float").unwrap_err().contains("unknown field type"));
        assert!(is_dice("3d6+2") && is_dice("d6") && !is_dice("3d") && !is_dice("x"));
        assert!(is_range("-1..3") && is_range("15..") && !is_range("..3"));
    }
}
