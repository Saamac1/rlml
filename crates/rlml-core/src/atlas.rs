//! Atlases (D 8.12): a large world as map sheets.
//!
//! `atlas NAME sheet WxH grid CxR [fill TILE]` splits a world of C·W × R·H
//! cells into sheets named `NAME.<column><row>`: the column in letters
//! (`A`…`Z`, then `AA`, `AB` …) and the row as a number from 01, with at
//! least two digits and as many as the row count needs. A sheet is a map;
//! its edges join its neighbours (M 6.10).

use crate::areas::Rect;
use crate::model::{Def, Kind, Model};
use crate::syntax::RawBlock;

/// An atlas header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atlas {
    /// The atlas's full name.
    pub name: String,
    /// Sheet width and height in cells.
    pub sheet: (i32, i32),
    /// Columns and rows of sheets.
    pub grid: (i32, i32),
    /// The tile of a missing sheet's default layer, as written.
    pub fill: Option<String>,
}

impl Atlas {
    /// Reads the header words after `atlas NAME`.
    pub fn parse(full: &str, b: &RawBlock) -> Result<Atlas, String> {
        let words: Vec<&str> = b.args.iter().map(|t| t.text.as_str()).collect();
        let form = "an atlas is `atlas NAME sheet WxH grid CxR [fill TILE]`";
        let (sheet, grid, fill) = match words.as_slice() {
            ["sheet", s, "grid", g] => (*s, *g, None),
            ["sheet", s, "grid", g, "fill", f] => (*s, *g, Some(f.to_string())),
            _ => return Err(form.into()),
        };
        let pair = |s: &str| s.split_once('x').and_then(|(a, b)| Some((a.parse::<i32>().ok()?, b.parse::<i32>().ok()?))).filter(|(a, b)| *a > 0 && *b > 0);
        let sheet = pair(sheet).ok_or_else(|| format!("{form}; `sheet {sheet}` is not a size such as `64x48`"))?;
        let grid = pair(grid).ok_or_else(|| format!("{form}; `grid {grid}` is not a size such as `10x14`"))?;
        Ok(Atlas { name: full.to_string(), sheet, grid, fill })
    }

    /// The atlas's size in cells.
    pub fn size(&self) -> (i32, i32) {
        (self.grid.0 * self.sheet.0, self.grid.1 * self.sheet.1)
    }

    /// The sheet id of column `c` and row `r`, both from 0 (`B10` is 1, 9).
    pub fn sheet_id(&self, c: i32, r: i32) -> String {
        let digits = self.grid.1.to_string().len().max(2);
        format!("{}{:0digits$}", column_name(c), r + 1)
    }

    /// The full name of a sheet.
    pub fn sheet_name(&self, c: i32, r: i32) -> String {
        format!("{}.{}", self.name, self.sheet_id(c, r))
    }

    /// Column and row of a sheet id written exactly as [`Atlas::sheet_id`]
    /// writes it, or why it is not one.
    pub fn parse_sheet(&self, id: &str) -> Result<(i32, i32), String> {
        let split = id.find(|c: char| !c.is_ascii_uppercase()).unwrap_or(id.len());
        let (letters, digits) = id.split_at(split);
        let form = || format!("`{id}` is not a sheet of atlas `{}`: a sheet is `{}.<column><row>`, such as `{}`", self.name, self.name, self.sheet_name(1, 9.min(self.grid.1 - 1)));
        if letters.is_empty() || digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(form());
        }
        let c = column_index(letters);
        let r = digits.parse::<i32>().map_err(|_| form())? - 1;
        if c >= self.grid.0 || r < 0 || r >= self.grid.1 {
            return Err(format!("sheet `{}.{id}` is outside the {}x{} grid of atlas `{}`", self.name, self.grid.0, self.grid.1, self.name));
        }
        if self.sheet_id(c, r) != id {
            return Err(format!("sheet `{}.{id}` is written `{}`", self.name, self.sheet_name(c, r)));
        }
        Ok((c, r))
    }

    /// The part of an atlas rectangle on sheet (`c`, `r`), in the sheet's
    /// coordinates (D 8.12).
    pub fn clip(&self, rect: Rect, c: i32, r: i32) -> Option<Rect> {
        let (ox, oy) = (c * self.sheet.0, r * self.sheet.1);
        let (x0, y0) = (rect.x.max(ox), rect.y.max(oy));
        let (x1, y1) = ((rect.x + rect.w).min(ox + self.sheet.0), (rect.y + rect.h).min(oy + self.sheet.1));
        (x0 < x1 && y0 < y1).then(|| Rect { x: x0 - ox, y: y0 - oy, w: x1 - x0, h: y1 - y0 })
    }
}

/// `A`…`Z`, `AA`, `AB` … for columns 0, 1 …
pub fn column_name(c: i32) -> String {
    let mut n = c + 1;
    let mut out = Vec::new();
    while n > 0 {
        out.push(b'A' + ((n - 1) % 26) as u8);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

fn column_index(letters: &str) -> i32 {
    letters.bytes().fold(0i32, |a, b| a.saturating_mul(26).saturating_add((b - b'A') as i32 + 1)) - 1
}

/// The atlases of a model by full name.
pub fn atlases(m: &Model) -> Vec<(&Def, Result<Atlas, String>)> {
    m.defs.values().filter(|d| d.kind == Kind::Atlas).filter_map(|d| d.block().map(|b| (d, Atlas::parse(&d.full, b)))).collect()
}

/// The atlas a map name belongs to, and the sheet's column and row: the
/// name is `<ATLAS>.<SHEET>` for an atlas of the model. The error says why
/// a name in an atlas's group is not one of its sheets.
pub fn sheet_of(m: &Model, map: &str) -> Option<Result<(Atlas, i32, i32), String>> {
    let (atlas, id) = map.rsplit_once('.')?;
    let def = m.defs.get(atlas).filter(|d| d.kind == Kind::Atlas)?;
    let a = match Atlas::parse(&def.full, def.block()?) {
        Ok(a) => a,
        Err(e) => return Some(Err(e)),
    };
    Some(a.parse_sheet(id).map(|(c, r)| (a, c, r)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas(cols: i32, rows: i32) -> Atlas {
        Atlas { name: "MAAILMA".into(), sheet: (64, 48), grid: (cols, rows), fill: None }
    }

    #[test]
    fn sheet_names() {
        let a = atlas(30, 14);
        assert_eq!([a.sheet_id(0, 0), a.sheet_id(1, 9), a.sheet_id(25, 0), a.sheet_id(26, 0), a.sheet_id(27, 13)], ["A01", "B10", "Z01", "AA01", "AB14"]);
        assert_eq!(atlas(3, 120).sheet_id(2, 4), "C005");
        assert_eq!(column_name(701), "ZZ");
        assert_eq!(column_name(702), "AAA");
        assert_eq!(a.parse_sheet("AB12"), Ok((27, 11)));
        assert_eq!(a.parse_sheet("B10"), Ok((1, 9)));
    }

    #[test]
    fn sheet_errors() {
        let a = atlas(10, 14);
        assert!(a.parse_sheet("K01").unwrap_err().contains("outside the 10x14 grid"));
        assert!(a.parse_sheet("A15").unwrap_err().contains("outside"));
        assert!(a.parse_sheet("A00").unwrap_err().contains("outside"));
        assert!(a.parse_sheet("B1").unwrap_err().contains("is written `MAAILMA.B01`"));
        assert!(a.parse_sheet("B001").unwrap_err().contains("is written `MAAILMA.B01`"));
        assert!(a.parse_sheet("SATAMA").unwrap_err().contains("is not a sheet of atlas `MAAILMA`"));
        assert!(a.parse_sheet("b10").unwrap_err().contains("is not a sheet"));
    }

    #[test]
    fn clipping_to_a_sheet() {
        let a = Atlas { name: "M".into(), sheet: (6, 3), grid: (3, 2), fill: None };
        let island = Rect { x: 4, y: 1, w: 4, h: 2 };
        assert_eq!(a.clip(island, 0, 0), Some(Rect { x: 4, y: 1, w: 2, h: 2 }));
        assert_eq!(a.clip(island, 1, 0), Some(Rect { x: 0, y: 1, w: 2, h: 2 }));
        assert_eq!(a.clip(island, 2, 0), None);
        assert_eq!(a.clip(island, 0, 1), None);
        assert_eq!(a.size(), (18, 6));
    }
}
