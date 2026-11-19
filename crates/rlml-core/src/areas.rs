//! Named map areas (D 8.11): unions of rectangles, nested by their cells.
//!
//! An area is meaning only: it does not change cells. Rows with the same
//! name form one area; area A is a parent of area B when every cell of B
//! is in A and A has more cells. The nearest parent is the one with the
//! fewest cells, and an area's level is the length of its chain of nearest
//! parents. A cell's
//! most specific area is the smallest one it is in; a tie goes to the area
//! whose first row is later.

use std::collections::BTreeSet;

use crate::fields::is_pos;
use crate::names;
use crate::syntax::RawItem;

/// A rectangle of cells: top-left corner and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn inside(&self, width: i32, height: i32) -> bool {
        self.x >= 0 && self.y >= 0 && self.x + self.w <= width && self.y + self.h <= height
    }

    fn cells(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        (self.y..self.y + self.h).flat_map(move |y| (self.x..self.x + self.w).map(move |x| (x, y)))
    }
}

/// An `areas:` row `- X,Y size WxH as NAME` (D 8.11): the name and the
/// rectangle. The rectangle is at least 1×1; whether it is inside the map
/// is the caller's check.
pub fn parse_row(item: &RawItem) -> Result<(String, Rect), String> {
    let words: Vec<&str> = item.tokens.iter().map(|t| t.text.as_str()).collect();
    let form = "an `areas:` row is `- X,Y size WxH as NAME`";
    let [pos, "size", size, "as", name] = words.as_slice() else { return Err(form.into()) };
    if !is_pos(pos) {
        return Err(format!("{form}; `{pos}` is not a position"));
    }
    let (x, y) = pos.split_once(',').unwrap();
    let Some((w, h)) = size.split_once('x').and_then(|(w, h)| Some((w.parse::<i32>().ok()?, h.parse::<i32>().ok()?))) else {
        return Err(format!("{form}; the size is written `WxH`, such as `size 26x17`"));
    };
    if w < 1 || h < 1 {
        return Err(format!("area `{name}`: the size {size} must be at least 1x1"));
    }
    if !names::is_name(name) {
        return Err(format!("`{name}` is not an area NAME"));
    }
    Ok((name.to_string(), Rect { x: x.parse().unwrap(), y: y.parse().unwrap(), w, h }))
}

/// One named area of a map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedArea {
    pub name: String,
    /// Cells in reading order.
    pub cells: Vec<(i32, i32)>,
    /// Index of the area's first row among the map's rows.
    pub first_row: usize,
    /// The nearest parent (D 8.11).
    pub parent: Option<usize>,
    /// The length of the chain of nearest parents; outermost areas are on
    /// level 0.
    pub level: usize,
}

impl NamedArea {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.cells.binary_search_by_key(&(y, x), |(cx, cy)| (*cy, *cx)).is_ok()
    }

    fn set(&self) -> BTreeSet<(i32, i32)> {
        self.cells.iter().copied().collect()
    }
}

/// Two areas that share cells without one being the other's parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlap {
    /// The same cells: one level, two names.
    Same(usize, usize),
    /// Shared cells, but neither is inside the other.
    Partial(usize, usize),
}

/// A map's named areas in the order of their first rows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Areas {
    pub list: Vec<NamedArea>,
}

impl Areas {
    /// The areas of a map's rows in order; a base map's rows come first
    /// (D 8.10).
    pub fn build(rows: &[(String, Rect)]) -> Areas {
        let mut list: Vec<NamedArea> = Vec::new();
        for (i, (name, rect)) in rows.iter().enumerate() {
            let k = match list.iter().position(|a| a.name == *name) {
                Some(k) => k,
                None => {
                    list.push(NamedArea { name: name.clone(), cells: vec![], first_row: i, parent: None, level: 0 });
                    list.len() - 1
                }
            };
            list[k].cells.extend(rect.cells());
        }
        for a in &mut list {
            a.cells.sort_by_key(|(x, y)| (*y, *x));
            a.cells.dedup();
        }
        let sets: Vec<BTreeSet<(i32, i32)>> = list.iter().map(NamedArea::set).collect();
        for b in 0..list.len() {
            let parents = (0..list.len()).filter(|&a| sets[a].len() > sets[b].len() && sets[b].is_subset(&sets[a]));
            // the nearest parent has the fewest cells; a tie goes to the later first row
            list[b].parent = parents.min_by_key(|&a| (sets[a].len(), std::cmp::Reverse(list[a].first_row)));
        }
        // the level is the length of the chain of nearest parents; a parent
        // always has more cells, so the chain ends
        for b in 0..list.len() {
            let (mut level, mut p) = (0, list[b].parent);
            while let Some(a) = p {
                level += 1;
                p = list[a].parent;
            }
            list[b].level = level;
        }
        Areas { list }
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&NamedArea> {
        self.list.iter().find(|a| a.name == name)
    }

    /// Pairs of areas that share cells without nesting (D 8.11), in row
    /// order; each pair once.
    pub fn overlaps(&self) -> Vec<Overlap> {
        let sets: Vec<BTreeSet<(i32, i32)>> = self.list.iter().map(NamedArea::set).collect();
        let mut out = Vec::new();
        for a in 0..sets.len() {
            for b in a + 1..sets.len() {
                if sets[a] == sets[b] {
                    out.push(Overlap::Same(a, b));
                } else if !sets[a].is_disjoint(&sets[b]) && !sets[a].is_subset(&sets[b]) && !sets[b].is_subset(&sets[a]) {
                    out.push(Overlap::Partial(a, b));
                }
            }
        }
        out
    }

    /// The areas a cell is in, in row order.
    pub fn at(&self, x: i32, y: i32) -> Vec<&NamedArea> {
        self.list.iter().filter(|a| a.contains(x, y)).collect()
    }

    /// The cell's most specific area: the smallest one it is in, and in a
    /// tie the one whose first row is later (D 8.11).
    pub fn deepest(&self, x: i32, y: i32) -> Option<&NamedArea> {
        self.list.iter().filter(|a| a.contains(x, y)).min_by_key(|a| (a.cells.len(), std::cmp::Reverse(a.first_row)))
    }

    /// The path to the cell's most specific area through nearest parents,
    /// outermost first (*Norsusaari › Vaaravuori*).
    pub fn path(&self, x: i32, y: i32) -> Vec<&NamedArea> {
        let Some(mut a) = self.deepest(x, y) else { return vec![] };
        let mut out = vec![a];
        while let Some(p) = a.parent {
            a = &self.list[p];
            out.push(a);
        }
        out.reverse();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn row(text: &str) -> Result<(String, Rect), String> {
        parse_row(&RawItem::new("a.rlml", 1, None, text).unwrap())
    }

    #[test]
    fn rows() {
        assert_eq!(row("8,6 size 26x17 as NORSUSAARI").unwrap(), ("NORSUSAARI".into(), rect(8, 6, 26, 17)));
        assert!(row("8,6 size 0x17 as A").unwrap_err().contains("at least 1x1"));
        assert!(row("8,6 size 2 as A").unwrap_err().contains("WxH"));
        assert!(row("8,6 2x2 as A").unwrap_err().contains("`- X,Y size WxH as NAME`"));
        assert!(row("x,6 size 2x2 as A").unwrap_err().contains("not a position"));
        assert!(row("1,1 size 2x2 as island").unwrap_err().contains("not an area NAME"));
        assert!(rect(8, 6, 26, 17).inside(64, 44) && !rect(60, 0, 5, 1).inside(64, 44));
    }

    #[test]
    fn nesting_and_the_most_specific_area() {
        // the example of D 8.11: an island of two rectangles, a mountain on it
        let a = Areas::build(&[
            ("NORSUSAARI".into(), rect(8, 6, 26, 17)),
            ("NORSUSAARI".into(), rect(30, 3, 7, 6)),
            ("VAARAVUORI".into(), rect(14, 9, 7, 5)),
            ("SYNKKA_SYVA".into(), rect(2, 24, 12, 10)),
        ]);
        assert_eq!(a.list.len(), 3);
        let island = a.get("NORSUSAARI").unwrap();
        assert_eq!(island.cells.len(), 26 * 17 + 7 * 6 - 4 * 3, "overlapping rectangles count once");
        assert!(island.contains(35, 3) && !island.contains(7, 6));
        let mountain = a.get("VAARAVUORI").unwrap();
        assert_eq!((mountain.parent, mountain.level), (Some(0), 1));
        assert_eq!((island.parent, island.level), (None, 0));
        let names = |v: Vec<&NamedArea>| v.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(a.path(15, 10)), ["NORSUSAARI", "VAARAVUORI"]);
        assert_eq!(names(a.at(15, 10)), ["NORSUSAARI", "VAARAVUORI"]);
        assert_eq!(a.deepest(9, 7).unwrap().name, "NORSUSAARI");
        assert!(a.deepest(0, 0).is_none() && a.path(0, 0).is_empty());
        assert!(a.overlaps().is_empty());
    }

    #[test]
    fn same_cells_and_partial_overlaps() {
        let a = Areas::build(&[("A".into(), rect(0, 0, 4, 4)), ("B".into(), rect(0, 0, 4, 4)), ("C".into(), rect(3, 3, 3, 3)), ("D".into(), rect(1, 1, 1, 1))]);
        assert_eq!(a.overlaps(), [Overlap::Same(0, 1), Overlap::Partial(0, 2), Overlap::Partial(1, 2)]);
        // A and B are on one level; neither is the other's parent
        assert_eq!((a.list[0].level, a.list[1].level), (0, 0));
        // D is inside both; the nearest parent's tie goes to the later row,
        // and the level follows the chain of nearest parents
        assert_eq!((a.list[3].parent, a.list[3].level), (Some(1), 1));
        assert_eq!(a.deepest(0, 0).unwrap().name, "B", "a tie goes to the later first row");
        // inside two partly overlapping areas: one nearest parent, level 1
        let a = Areas::build(&[("A".into(), rect(0, 0, 4, 4)), ("C".into(), rect(2, 2, 4, 4)), ("E".into(), rect(2, 2, 1, 1))]);
        assert_eq!((a.list[2].parent, a.list[2].level), (Some(1), 1), "a tie in size goes to the later row");
        let a = Areas::build(&[("A".into(), rect(0, 0, 4, 4)), ("C".into(), rect(2, 2, 3, 3)), ("E".into(), rect(2, 2, 1, 1))]);
        assert_eq!((a.list[2].parent, a.list[2].level), (Some(1), 1), "the smaller one is nearer");
    }
}
