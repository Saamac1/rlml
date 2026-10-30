//! RogueScript types (RS 2.6) and when a value fits a place (M 9.3, RS 6.2).
//!
//! Checking is gradual: `Unknown` fits everything and everything fits it,
//! so a type that cannot be inferred never causes an error. Data field
//! types (D 5.1) and RogueScript types are the same thing and are read with
//! the same function.

use std::fmt;

/// What a collection holds (RS 2.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Member {
    Entity,
    Card,
    Name,
    /// Heard sounds, from `heard()` (M 6.8).
    Sound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Number,
    Percent,
    Dice,
    Range,
    Bool,
    Text,
    Name,
    Dir,
    Side,
    Pos,
    Area,
    Entity,
    Card,
    Collection(Member),
    List(Box<Ty>),
    Dict(Box<Ty>, Box<Ty>),
    Deck,
    Table,
    Command,
    Scene,
    /// A component value; `Some` names the component.
    Component(Option<String>),
    Handle,
    Camera,
    Light,
    Sound,
    Label,
    /// `today()` (M 9.3).
    Date,
    /// A presentation time such as `30ms`.
    Time,
    /// `40x20`.
    Size,
    /// `all` in a draw (`draw(d, n: number|dice|all)`).
    All,
    /// One of several types, as in engine signatures (`number|dice|all`).
    Union(Vec<Ty>),
    None,
    Unknown,
}

impl Ty {
    pub fn list(t: Ty) -> Ty {
        Ty::List(Box::new(t))
    }

    pub fn is_known(&self) -> bool {
        !matches!(self, Ty::Unknown)
    }

    /// The type of one member: what `choose`, `.first`, `.last`, `for` and
    /// `it` give (RS 2.6, RS 5.3).
    pub fn member(&self) -> Ty {
        match self {
            Ty::Collection(Member::Entity) => Ty::Entity,
            Ty::Collection(Member::Card) => Ty::Card,
            Ty::Collection(Member::Name) => Ty::Name,
            Ty::Collection(Member::Sound) => Ty::Sound,
            Ty::List(t) => (**t).clone(),
            Ty::Area => Ty::Pos,
            Ty::Range => Ty::Number,
            Ty::Dict(k, _) => (**k).clone(),
            _ => Ty::Unknown,
        }
    }

    /// Can the value be looped over and filtered (`for`, `where`, `in`)?
    pub fn is_set(&self) -> bool {
        matches!(self, Ty::Collection(_) | Ty::List(_) | Ty::Area | Ty::Range | Ty::Dict(..) | Ty::Unknown)
    }
}

/// Reads a type as written in RogueScript (`list area`, `collection card`,
/// `dict entity number`) or in a data field (`collection 26 merge
/// STACKABLE`, `deck copy`, `list SCORE 10 sort score desc`). `component`
/// tells whether an upper-case name is a component, which is allowed only
/// as a list member (RS 12.1).
pub fn parse(text: &str, component: &dyn Fn(&str) -> bool) -> Result<Ty, String> {
    if text.contains('|') {
        return text.split('|').map(|t| parse(t.trim(), component)).collect::<Result<Vec<_>, _>>().map(Ty::Union);
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    // words after the type are a field type's size, merge and sort (D 5.1)
    words_to_ty(&words, component, false).map(|(ty, _)| ty)
}

fn words_to_ty(words: &[&str], component: &dyn Fn(&str) -> bool, member: bool) -> Result<(Ty, usize), String> {
    let Some(base) = words.first() else { return Err("missing type".into()) };
    let simple = match *base {
        "number" => Some(Ty::Number),
        "percent" => Some(Ty::Percent),
        "dice" => Some(Ty::Dice),
        "range" => Some(Ty::Range),
        "bool" => Some(Ty::Bool),
        "text" => Some(Ty::Text),
        "name" => Some(Ty::Name),
        "dir" => Some(Ty::Dir),
        "side" => Some(Ty::Side),
        "pos" => Some(Ty::Pos),
        "area" => Some(Ty::Area),
        "entity" => Some(Ty::Entity),
        "card" => Some(Ty::Card),
        "table" => Some(Ty::Table),
        "command" => Some(Ty::Command),
        "scene" => Some(Ty::Scene),
        "component" => Some(Ty::Component(None)),
        "handle" => Some(Ty::Handle),
        "camera" => Some(Ty::Camera),
        "light" => Some(Ty::Light),
        "label" => Some(Ty::Label),
        "sound" => Some(Ty::Sound),
        "date" => Some(Ty::Date),
        "time" => Some(Ty::Time),
        "size" => Some(Ty::Size),
        "all" => Some(Ty::All),
        "none" => Some(Ty::None),
        _ => None,
    };
    if let Some(t) = simple {
        return Ok((t, 1));
    }
    match *base {
        // `deck copy` is a field's own copy of a deck; its value is a deck (RS 2.6)
        "deck" => Ok((Ty::Deck, if words.get(1) == Some(&"copy") { 2 } else { 1 })),
        "collection" => match words.get(1) {
            Some(&"card") => Ok((Ty::Collection(Member::Card), 2)),
            Some(&"name") => Ok((Ty::Collection(Member::Name), 2)),
            Some(&"sound") => Ok((Ty::Collection(Member::Sound), 2)),
            _ => Ok((Ty::Collection(Member::Entity), 1)),
        },
        "list" => {
            let (m, n) = words_to_ty(&words[1..], component, true)?;
            if matches!(m, Ty::List(_) | Ty::Dict(..) | Ty::Deck | Ty::Collection(_)) {
                return Err(format!("a list member is not a list, dictionary, deck or collection: `{}`", words.join(" ")));
            }
            Ok((Ty::list(m), 1 + n))
        }
        "dict" => {
            let (k, nk) = words_to_ty(&words[1..], component, false)?;
            if !matches!(k, Ty::Entity | Ty::Name | Ty::Number) {
                return Err(format!("a dictionary key is entity, name or number, not `{k}`"));
            }
            let (v, nv) = words_to_ty(&words[1 + nk..], component, false)?;
            if matches!(v, Ty::Collection(_) | Ty::Deck | Ty::Dict(..)) {
                return Err(format!("a dictionary value is not a collection, deck or dictionary: `{v}`"));
            }
            Ok((Ty::Dict(Box::new(k), Box::new(v)), 1 + nk + nv))
        }
        w if crate::names::is_name(w) => {
            if !member {
                return Err(format!("`{w}`: an upper-case type is a component and is allowed only as a list member (`list {w}`)"));
            }
            if !component(w) {
                return Err(format!("`{w}` is not a component"));
            }
            Ok((Ty::Component(Some(w.to_string())), 1))
        }
        w => Err(format!("unknown type `{w}`")),
    }
}

/// Types that are areas where an area is asked for: a position, an entity
/// on a map and a list of positions (RS 6.2, M 9.3 *alue*).
fn is_area_like(t: &Ty) -> bool {
    match t {
        Ty::Area | Ty::Pos | Ty::Entity => true,
        Ty::List(m) => matches!(**m, Ty::Pos | Ty::Unknown),
        _ => false,
    }
}

/// Does a value of type `arg` fit a place of type `param`?
pub fn fits(arg: &Ty, param: &Ty) -> bool {
    use Ty::*;
    match (arg, param) {
        (Unknown, _) | (_, Unknown) => true,
        (_, Union(alts)) => alts.iter().any(|p| fits(arg, p)),
        (Union(alts), _) => alts.iter().any(|a| fits(a, param)),
        // `none` is "no value" for every type but a truth value (STD 7:
        // `GROUP_STANCE -> number` returns `none`)
        (None, Bool) => false,
        (None, _) => true,
        (a, p) if a == p => true,
        // dice and ranges are rolled where a number is read, and a number is
        // constant dice (D 5.2)
        (Dice | Range, Number) | (Number, Dice) => true,
        (_, Area) => is_area_like(arg),
        // a list of entity handles is read like a collection (RS 2.6)
        (List(m), Collection(Member::Entity)) => matches!(**m, Entity | Unknown),
        (List(a), List(p)) => fits(a, p),
        (Dict(ak, av), Dict(pk, pv)) => fits(ak, pk) && fits(av, pv),
        (Component(Some(_)), Component(Option::None)) => true,
        // a label is a name of the map
        (Label, Name) | (Name, Label) => true,
        _ => false,
    }
}

impl fmt::Display for Member {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Member::Entity => "",
            Member::Card => " card",
            Member::Name => " name",
            Member::Sound => " sound",
        })
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Ty::Number => "number",
            Ty::Percent => "percent",
            Ty::Dice => "dice",
            Ty::Range => "range",
            Ty::Bool => "bool",
            Ty::Text => "text",
            Ty::Name => "name",
            Ty::Dir => "dir",
            Ty::Side => "side",
            Ty::Pos => "pos",
            Ty::Area => "area",
            Ty::Entity => "entity",
            Ty::Card => "card",
            Ty::Collection(m) => return write!(f, "collection{m}"),
            Ty::List(t) => return write!(f, "list {t}"),
            Ty::Dict(k, v) => return write!(f, "dict {k} {v}"),
            Ty::Deck => "deck",
            Ty::Table => "table",
            Ty::Command => "command",
            Ty::Scene => "scene",
            Ty::Component(Some(n)) => return f.write_str(n),
            Ty::Component(None) => "component",
            Ty::Handle => "handle",
            Ty::Camera => "camera",
            Ty::Light => "light",
            Ty::Sound => "sound",
            Ty::Label => "label",
            Ty::Date => "date",
            Ty::Time => "time",
            Ty::Size => "size",
            Ty::All => "all",
            Ty::Union(alts) => {
                for (i, a) in alts.iter().enumerate() {
                    if i > 0 {
                        f.write_str("|")?;
                    }
                    write!(f, "{a}")?;
                }
                return Ok(());
            }
            Ty::None => "none",
            Ty::Unknown => "?",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ty::*;

    fn ty(s: &str) -> Ty {
        parse(s, &|n| n == "SCORE").unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    fn err(s: &str) -> String {
        parse(s, &|n| n == "SCORE").unwrap_err()
    }

    #[test]
    fn reads_types() {
        assert_eq!(ty("number"), Number);
        assert_eq!(ty("list area"), Ty::list(Area));
        assert_eq!(ty("collection"), Collection(Member::Entity));
        assert_eq!(ty("collection card"), Collection(Member::Card));
        assert_eq!(ty("dict entity number"), Dict(Box::new(Entity), Box::new(Number)));
        assert_eq!(ty("number|dice|all"), Union(vec![Number, Dice, All]));
        assert_eq!(ty("name|text"), Union(vec![Name, Text]));
    }

    #[test]
    fn reads_data_field_types() {
        assert_eq!(ty("collection 26 merge STACKABLE modifies"), Collection(Member::Entity));
        assert_eq!(ty("deck copy"), Deck);
        assert_eq!(ty("list SCORE 10 sort score desc"), Ty::list(Component(Some("SCORE".into()))));
        assert_eq!(ty("dict name number"), Dict(Box::new(Name), Box::new(Number)));
    }

    #[test]
    fn type_errors() {
        assert!(err("float").contains("unknown type"));
        assert!(err("SCORE").contains("only as a list member"));
        assert!(err("list GOBLIN").contains("not a component"));
        assert!(err("list list number").contains("not a list"));
        assert!(err("dict text number").contains("key is entity, name or number"));
        assert!(err("dict name deck").contains("not a collection, deck"));
    }

    #[test]
    fn display_round_trips() {
        for s in ["number", "list area", "collection card", "dict entity number", "list SCORE", "number|dice|all"] {
            assert_eq!(ty(s).to_string(), s);
        }
    }

    #[test]
    fn fitting() {
        assert!(fits(&Unknown, &Number) && fits(&Number, &Unknown));
        assert!(fits(&Number, &Number));
        assert!(!fits(&Text, &Number), "HURT(target, \"5\")");
        assert!(!fits(&Dir, &Pos), "STEP(north)");
        assert!(!fits(&Side, &Dir) && !fits(&Dir, &Side), "D 5.1");
        assert!(fits(&Dice, &Number), "emit HIT(target, attacker, attacker.MELEE.damage)");
        assert!(!fits(&Number, &Range), "RS 2.6: a number is not a range");
        for a in [Pos, Entity, Area, Ty::list(Pos), Ty::list(Unknown)] {
            assert!(fits(&a, &Area), "{a} as an area");
        }
        assert!(!fits(&Collection(Member::Entity), &Area));
        assert!(fits(&Ty::list(Entity), &Collection(Member::Entity)));
        assert!(!fits(&Collection(Member::Card), &Collection(Member::Entity)));
        assert!(fits(&Ty::list(Unknown), &Ty::list(Area)), "an empty list takes its type from its use");
        assert!(fits(&None, &Entity) && fits(&None, &Number) && !fits(&None, &Bool));
        assert!(fits(&All, &Union(vec![Number, Dice, All])));
        assert!(!fits(&Text, &Union(vec![Number, Dice, All])));
        assert!(fits(&Component(Some("HP".into())), &Component(Option::None)));
        assert!(fits(&Label, &Name) && fits(&Name, &Label));
    }

    #[test]
    fn members() {
        assert_eq!(Collection(Member::Card).member(), Card);
        assert_eq!(Ty::list(Area).member(), Area);
        assert_eq!(Area.member(), Pos);
        assert_eq!(Range.member(), Number);
        assert!(Area.is_set() && !Pos.is_set());
    }
}
