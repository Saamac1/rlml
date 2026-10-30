//! Names and namespaces (D 2.1, D 4.4) and tile names (D 7.1).
//!
//! Names are written in upper case (`HP`, `LOOT.RARE`); lower case is
//! vocabulary. A file's `namespace` prefixes the names it defines, and a
//! reference is looked up in the file's namespace, then in each outer one,
//! then in the root and last as `STD.N`. `.NAME` refers to the root directly.

/// The standard library's namespace (STD 1.1).
pub const STD: &str = "STD";

/// One name part: `A–Z` first, then `A–Z 0–9 _`, no leading, trailing or
/// doubled underscore.
pub fn is_part(p: &str) -> bool {
    let mut chars = p.chars();
    matches!(chars.next(), Some('A'..='Z'))
        && p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && !p.ends_with('_')
        && !p.contains("__")
}

/// `HP`, `LOOT.RARE`.
pub fn is_name(s: &str) -> bool {
    !s.is_empty() && s.split('.').all(is_part)
}

/// A reference: a name, optionally `.`-prefixed for the root.
pub fn is_ref(s: &str) -> bool {
    is_name(s.strip_prefix('.').unwrap_or(s))
}

/// Vocabulary: `now`, `new_game`, `cursor.north`.
pub fn is_vocabulary(s: &str) -> bool {
    !s.is_empty()
        && s.split('.').all(|p| {
            p.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

/// The full name of a definition `name` written in namespace `ns`.
pub fn qualify(ns: Option<&str>, name: &str) -> String {
    match (ns, name.strip_prefix('.')) {
        (_, Some(root)) => root.to_string(),
        (Some(ns), None) => format!("{ns}.{name}"),
        (None, None) => name.to_string(),
    }
}

/// Candidate full names for a reference, in lookup order (D 4.4).
pub fn candidates(ns: Option<&str>, name: &str) -> Vec<String> {
    if let Some(root) = name.strip_prefix('.') {
        return vec![root.to_string()];
    }
    let mut out = Vec::new();
    if let Some(ns) = ns {
        let parts: Vec<&str> = ns.split('.').collect();
        for n in (1..=parts.len()).rev() {
            out.push(format!("{}.{name}", parts[..n].join(".")));
        }
    }
    for c in [name.to_string(), format!("{STD}.{name}")] {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// Resolves a reference; the error lists every name tried.
pub fn resolve(ns: Option<&str>, name: &str, exists: impl Fn(&str) -> bool) -> Result<String, String> {
    let tried = candidates(ns, name);
    tried.iter().find(|c| exists(c)).cloned().ok_or_else(|| tried.join(", "))
}

/// A tile index or index range (D 7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Index {
    One(u32),
    /// `a..b`; `None` is an open upper bound.
    Range(u32, Option<u32>),
}

/// A tile name split into its parts: `LAVA.crust.2` is base `LAVA`,
/// variants `crust` and index 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileName {
    /// Upper-case name parts, possibly `.`-prefixed for the root.
    pub base: String,
    pub variants: Vec<String>,
    pub index: Option<Index>,
}

impl TileName {
    /// The base name with its variants, without the index: the key a
    /// `tiles` row describes.
    pub fn key(&self) -> String {
        let mut s = self.base.clone();
        for v in &self.variants {
            s.push('.');
            s.push_str(v);
        }
        s
    }
}

fn index_number(s: &str) -> Result<u32, String> {
    if s.len() > 1 && s.starts_with('0') {
        return Err(format!("leading zeros in tile index `{s}`"));
    }
    match s.parse::<u32>() {
        Ok(n) if n <= i32::MAX as u32 => Ok(n),
        _ => Err(format!("bad tile index `{s}`")),
    }
}

/// Parses `DOOR.open`, `WATER.3`, `WATER.1..2` or `WATER.4..`.
pub fn tile_name(s: &str) -> Result<TileName, String> {
    let bad = || format!("`{s}` is not a tile name");
    let (root, body) = match s.strip_prefix('.') {
        Some(b) => (".", b),
        None => ("", s),
    };
    // a range contains `..`, which would split into empty parts
    let (body, index) = match body.find("..") {
        Some(at) => {
            let (head, tail) = (&body[..at], &body[at + 2..]);
            let (head, from) = head.rsplit_once('.').ok_or_else(bad)?;
            let from = index_number(from)?;
            let to = if tail.is_empty() { None } else { Some(index_number(tail)?) };
            if to.is_some_and(|t| t < from) {
                return Err(format!("empty tile index range in `{s}`"));
            }
            (head, Some(Index::Range(from, to)))
        }
        None => match body.rsplit_once('.') {
            Some((head, last)) if last.starts_with(|c: char| c.is_ascii_digit()) => (head, Some(Index::One(index_number(last)?))),
            _ => (body, None),
        },
    };
    let parts: Vec<&str> = body.split('.').collect();
    let upper = parts.iter().take_while(|p| is_part(p)).count();
    if upper == 0 || !parts[upper..].iter().all(|p| is_vocabulary(p)) {
        return Err(bad());
    }
    Ok(TileName {
        base: format!("{root}{}", parts[..upper].join(".")),
        variants: parts[upper..].iter().map(|p| p.to_string()).collect(),
        index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_format() {
        for ok in ["A", "MAIN_MENU", "LOOT.RARE", "SFX.HIT2", "A1_B"] {
            assert!(is_name(ok), "{ok}");
        }
        for bad in ["", "lower", "Mixed", "_A", "A_", "A__B", "3D_FIRE", "A..B", "A.", ".A", "A-B", "ÄÄ"] {
            assert!(!is_name(bad), "{bad}");
        }
        assert!(is_ref(".LOOT"));
        assert!(is_vocabulary("cursor.north"));
        assert!(!is_vocabulary("Now"));
    }

    #[test]
    fn qualification() {
        assert_eq!(qualify(Some("STD"), "HP"), "STD.HP");
        assert_eq!(qualify(Some("STD"), ".HP"), "HP");
        assert_eq!(qualify(None, "HP"), "HP");
    }

    #[test]
    fn lookup_order() {
        assert_eq!(candidates(Some("A.B"), "N"), ["A.B.N", "A.N", "N", "STD.N"]);
        assert_eq!(candidates(None, "N"), ["N", "STD.N"]);
        assert_eq!(candidates(Some("STD"), "HP"), ["STD.HP", "HP"]);
        assert_eq!(candidates(Some("A.B"), ".N"), ["N"]);
        assert_eq!(candidates(Some("UNDEAD"), "SFX.HIT"), ["UNDEAD.SFX.HIT", "SFX.HIT", "STD.SFX.HIT"]);
    }

    #[test]
    fn resolution() {
        let defined = ["A.N", "N", "M", "STD.HP"];
        let exists = |n: &str| defined.contains(&n);
        assert_eq!(resolve(Some("A.B"), "N", exists), Ok("A.N".into()), "inner shadows outer");
        assert_eq!(resolve(Some("A.B"), ".N", exists), Ok("N".into()));
        assert_eq!(resolve(None, "HP", exists), Ok("STD.HP".into()));
        assert_eq!(resolve(Some("A.B"), "X", exists), Err("A.B.X, A.X, X, STD.X".into()));
    }

    #[test]
    fn tile_names() {
        let t = tile_name("LAVA.crust.2").unwrap();
        assert_eq!((t.base.as_str(), t.variants.as_slice(), t.index), ("LAVA", &["crust".to_string()][..], Some(Index::One(2))));
        assert_eq!(t.key(), "LAVA.crust");
        assert_eq!(tile_name("WATER.1..2").unwrap().index, Some(Index::Range(1, Some(2))));
        assert_eq!(tile_name("WATER.4..").unwrap().index, Some(Index::Range(4, None)));
        assert_eq!(tile_name("GLYPH.NKI.1..36").unwrap().base, "GLYPH.NKI");
        assert_eq!(tile_name("STAIR.down").unwrap().key(), "STAIR.down");
        assert!(tile_name("WATER.03").unwrap_err().contains("leading zeros"));
        assert!(tile_name("WATER.3..1").is_err());
        assert!(tile_name("door").is_err());
        assert!(tile_name("DOOR.open.WIDE").is_err());
    }
}
