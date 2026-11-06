//! A frame of the game (M 7.1–7.4): the layout's areas solved for the
//! window, the map around the camera in color, the log and the panels'
//! borders and colors (D 9.3, D 11.1, D 11.4).

use std::collections::BTreeMap;

use rlml_core::model::{Kind, glyph_char};
use rlml_core::names;
use rlml_core::rt::value::{MapId, Pos};
use rlml_core::rt::view::{Look, Rgb};
use rlml_core::rt::world::World;
use rlml_core::syntax::RawItem;
use termplate_core::frame::{Frame, Role, wrap};
use termplate_core::layout::{self, AreaSize, Dim, Dims, Rect};
use termplate_core::style::{Border, Color, Style};

/// How the camera follows its target (D 11.1, M 7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    Center,
    Margin(i32),
}

/// One area of the layout: its character, source and own words.
#[derive(Debug, Clone, PartialEq)]
pub struct Area {
    pub ch: char,
    /// `map`, `hover`, or a log or panel name as written.
    pub source: String,
    pub follow: Follow,
}

/// The presentation's own state between frames.
#[derive(Debug, Clone, Default)]
pub struct View {
    /// The camera's top-left cell on each map (M 7.1: it stays when its
    /// target is gone).
    pub camera: BTreeMap<MapId, (i32, i32)>,
    /// The member whose turn marker shows now (M 7.1); the app sets it
    /// and blinks it.
    pub marked: Option<rlml_core::rt::value::EntityId>,
}

/// A style color for the terminal: 24-bit when the terminal says it takes
/// it (`COLORTERM` is `truecolor` or `24bit`), else the nearest color of
/// the 256-color palette, which every common terminal shows. macOS
/// Terminal before version 26, for one, shows no 24-bit colors.
pub fn rgb(c: Rgb) -> Color {
    static TRUECOLOR: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let truecolor = *TRUECOLOR.get_or_init(|| std::env::var("COLORTERM").is_ok_and(|v| v == "truecolor" || v == "24bit"));
    if truecolor { Color::Rgb(c[0], c[1], c[2]) } else { Color::Ansi(ansi256(c)) }
}

/// The nearest color of the 256-color palette: the 6×6×6 cube or, for a
/// grey, the 24-step grey ramp, whichever is closer.
pub fn ansi256(c: Rgb) -> u8 {
    const STEPS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let near = |v: u8| (0..6).min_by_key(|&i| (STEPS[i] - v as i32).abs()).unwrap();
    let (r, g, b) = (near(c[0]), near(c[1]), near(c[2]));
    let cube = [STEPS[r], STEPS[g], STEPS[b]];
    let avg = (c[0] as i32 + c[1] as i32 + c[2] as i32) / 3;
    let grey = ((avg - 8).max(0) / 10).min(23);
    let grey_v = 8 + 10 * grey;
    let dist = |t: [i32; 3]| (0..3).map(|i| (t[i] - c[i] as i32).pow(2)).sum::<i32>();
    if dist([grey_v; 3]) < dist(cube) { 232 + grey as u8 } else { (16 + 36 * r + 6 * g + b) as u8 }
}

fn style_of(look: &Look) -> Style {
    Style { fg: look.fg.map(rgb), bg: look.bg.map(rgb), ..Style::default() }
}

/// The layout in use: `game.layout` or the only one (D 11.1).
fn layout_block(w: &World) -> Option<rlml_core::syntax::RawBlock> {
    w.m.layout().cloned()
}

/// Areas and their written sizes from the layout's legend.
fn areas(rows: &[RawItem]) -> (Vec<Area>, Dims) {
    let mut out = Vec::new();
    let mut dims = Dims::new();
    for item in rows {
        let Ok((ch, rest)) = glyph_char(item) else { continue };
        let words: Vec<&str> = rest.tokens.iter().map(|t| t.text.as_str()).collect();
        let (sw, sh) = rlml_core::model::legend_size(&words);
        let dim = |v: Option<&str>| v.and_then(|s| Dim::parse(s).ok()).flatten();
        let size: (Option<Dim>, Option<Dim>) = (dim(sw), dim(sh));
        let mut follow = Follow::Center;
        let mut i = 1;
        while i < words.len() {
            match words[i] {
                "follow" => {
                    follow = match (words.get(i + 1), words.get(i + 2).and_then(|n| n.parse().ok())) {
                        (Some(&"margin"), Some(n)) => Follow::Margin(n),
                        _ => Follow::Center,
                    };
                    i += if words.get(i + 1) == Some(&"margin") { 3 } else { 2 };
                }
                _ => i += 1,
            }
        }
        dims.insert(ch, size);
        out.push(Area { ch, source: words.first().copied().unwrap_or("").to_string(), follow });
    }
    (out, dims)
}

/// A panel style row for an area name: border and colors (D 9.3).
fn panel_style(w: &World, name: &str) -> (Border, Style) {
    let mut border = Border::None;
    let mut style = Style::default();
    for (_, item) in w.m.section_rows("panels") {
        let (_, row) = item.marker();
        let ns = row.namespace.as_deref();
        if names::qualify(ns, row.key()) != name && row.key() != name {
            continue;
        }
        let words: Vec<&str> = row.tokens.iter().map(|t| t.text.as_str()).collect();
        let after = |k: &str| words.iter().position(|x| *x == k).and_then(|i| words.get(i + 1));
        style.fg = after("fg").and_then(|c| w.color(c, ns)).map(rgb).or(style.fg);
        style.bg = after("bg").and_then(|c| w.color(c, ns)).map(rgb).or(style.bg);
    }
    if let Some(b) = w.m.panel_border(name).and_then(Border::parse) {
        border = b;
    }
    (border, style)
}

/// A message key's color by the most specific `logs` row (D 11.4).
fn log_color(w: &World, key: &str) -> Option<Color> {
    let mut best: Option<(usize, Rgb)> = None;
    for (_, item) in w.m.section_rows("logs") {
        let (_, row) = item.marker();
        let ns = row.namespace.as_deref();
        let name = names::qualify(ns, row.key());
        let matches = key == name || key.starts_with(&format!("{name}.")) || key == row.key() || key.starts_with(&format!("{}.", row.key()));
        if !matches {
            continue;
        }
        let fg = row.tokens.iter().position(|t| t.is("fg")).and_then(|i| row.word(i + 1)).and_then(|c| w.color(c, ns));
        if let Some(c) = fg
            && best.is_none_or(|(len, _)| row.key().len() > len)
        {
            best = Some((row.key().len(), c));
        }
    }
    best.map(|(_, c)| rgb(c))
}

/// Paints a rectangle's cells with a style.
fn paint(frame: &mut Frame, r: Rect, style: Style) {
    if style == Style::default() {
        return;
    }
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            if x < frame.width && y < frame.height {
                let i = y * frame.width + x;
                frame.cells[i].style = Some(frame.cells[i].style.map_or(style, |s| style.over(s)));
            }
        }
    }
}

fn put_look(frame: &mut Frame, x: usize, y: usize, look: &Look) {
    frame.put(x, y, look.ch, Role::Normal);
    if x < frame.width && y < frame.height {
        let i = y * frame.width + x;
        frame.cells[i].style = Some(style_of(look));
    }
}

impl View {
    /// The camera's top-left cell for an area of `w`×`h` on `map`
    /// (M 7.1): the player kept in the middle or within a margin, a map
    /// smaller than the area centred, the last place kept without a player.
    fn camera(&mut self, world: &World, map: MapId, w: i32, h: i32, follow: Follow) -> (i32, i32) {
        let (mw, mh) = (world.s.maps[map].width, world.s.maps[map].height);
        let target = world.s.player.filter(|p| world.s.get(*p).alive).and_then(|p| world.s.at(p)).filter(|p| p.map == map);
        let old = self.camera.get(&map).copied();
        let axis = |len: i32, size: i32, at: Option<i32>, old: Option<i32>| -> i32 {
            if len <= size {
                // a smaller map is centred
                return -((size - len) / 2);
            }
            let Some(t) = at else { return old.unwrap_or(0) };
            let wanted = match (follow, old) {
                (Follow::Margin(n), Some(o)) => {
                    let n = n.min((size - 1) / 2);
                    if t < o + n {
                        t - n
                    } else if t > o + size - 1 - n {
                        t - size + 1 + n
                    } else {
                        o
                    }
                }
                _ => t - size / 2,
            };
            wanted.clamp(0, len - size)
        };
        let cam = (axis(mw, w, target.map(|p| p.x), old.map(|o| o.0)), axis(mh, h, target.map(|p| p.y), old.map(|o| o.1)));
        self.camera.insert(map, cam);
        cam
    }

    /// Draws the game into a frame of the window's size.
    pub fn draw(&mut self, world: &mut World, lang: &str, width: usize, height: usize) -> Frame {
        let mut frame = Frame::new(width, height);
        let block = layout_block(world);
        let (areas, solved) = match &block {
            Some(b) => {
                let rows: Vec<RawItem> = b.sub("legend").cloned().collect();
                let (areas, dims) = areas(&rows);
                match layout::analyze(&b.drawing) {
                    Ok(mut grid) => {
                        let _ = grid.set_sizes(&dims);
                        let solved = grid.solve(&BTreeMap::<char, AreaSize>::new(), width, height);
                        (areas, solved)
                    }
                    Err(e) => {
                        frame.text(0, 0, width, &format!("layout: {e}"), Role::Normal);
                        return frame;
                    }
                }
            }
            // without a layout the map fills the window
            None => {
                let mut rects = BTreeMap::new();
                rects.insert('m', Rect::new(0, 0, width, height));
                (vec![Area { ch: 'm', source: "map".into(), follow: Follow::Center }], layout::Solved { rects, too_small: false, min_width: 0, min_height: 0 })
            }
        };
        if solved.too_small {
            frame.text(0, 0, width, &format!("The window is too small: {}x{} is needed.", solved.min_width, solved.min_height), Role::Normal);
            return frame;
        }
        for area in &areas {
            let Some(&rect) = solved.rects.get(&area.ch) else { continue };
            let name = if area.source == "map" || area.source == "hover" { area.source.clone() } else { area.source.split('(').next().unwrap_or("").to_string() };
            frame.set_area(vec![name.clone()]);
            let (border, style) = panel_style(world, &name);
            paint(&mut frame, rect, style);
            let inner = if border != Border::None && rect.w >= 2 && rect.h >= 2 {
                frame.border(rect, border, Role::Normal);
                Rect::new(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2)
            } else {
                rect
            };
            match area.source.as_str() {
                "map" => self.draw_map(&mut frame, world, inner, area.follow),
                "hover" => {}
                _ => {
                    if world.m.resolve(None, &name).is_ok_and(|d| d.kind == Kind::Log) {
                        draw_log(&mut frame, world, lang, &name, inner);
                    }
                }
            }
        }
        frame
    }

    fn draw_map(&mut self, frame: &mut Frame, world: &mut World, r: Rect, follow: Follow) {
        let Some(map) = world.s.active_map() else { return };
        let cells = world.render_cells(map);
        let (x0, y0) = self.camera(world, map, r.w as i32, r.h as i32, follow);
        for sy in 0..r.h {
            for sx in 0..r.w {
                let (mx, my) = (x0 + sx as i32, y0 + sy as i32);
                if let Some(look) = (my >= 0 && mx >= 0).then(|| cells.get(my as usize).and_then(|row| row.get(mx as usize))).flatten() {
                    put_look(frame, r.x + sx, r.y + sy, look);
                }
            }
        }
        // the turn marker: the member's cell in reverse video, which every
        // terminal shows whatever its colors (M 7.1)
        if let Some(p) = self.marked.and_then(|e| world.s.at(e)).filter(|p| p.map == map) {
            let (sx, sy) = (p.x - x0, p.y - y0);
            if sx >= 0 && sy >= 0 && (sx as usize) < r.w && (sy as usize) < r.h {
                let i = (r.y + sy as usize) * frame.width + r.x + sx as usize;
                if let Some(cell) = frame.cells.get_mut(i) {
                    let mut style = cell.style.clone().unwrap_or_default();
                    style.attrs.reverse = true;
                    cell.style = Some(style);
                }
            }
        }
    }
}

/// The log's newest messages that fit, wrapped at word gaps, each in its
/// `logs` color (D 11.4, M 7.4).
fn draw_log(frame: &mut Frame, world: &World, lang: &str, name: &str, r: Rect) {
    let history = world.m.defs.get(name).and_then(|d| d.block()).and_then(|b| b.arg_after("history")).and_then(|n| n.parse::<usize>().ok()).unwrap_or(usize::MAX);
    let selects = log_rows(world, name);
    let mut lines: Vec<(String, Option<Color>)> = Vec::new();
    let entries: Vec<_> = world.log.iter().filter(|l| selects.is_empty() || selects.iter().any(|s| l.key == *s || l.key.starts_with(&format!("{s}.")))).collect();
    for l in entries.iter().rev().take(history).rev() {
        let text = world.text(lang, &l.key, &l.args);
        let color = log_color(world, &l.key);
        for part in wrap(&text, r.w.max(1)) {
            lines.push((part, color));
        }
    }
    let shown = lines.len().saturating_sub(r.h);
    for (i, (line, color)) in lines[shown..].iter().enumerate() {
        frame.text(r.x, r.y + i, r.w, line, Role::Normal);
        if let Some(c) = color {
            paint(frame, Rect::new(r.x, r.y + i, r.w, 1), Style { fg: Some(*c), ..Style::default() });
        }
    }
}

/// The message keys or groups a `log` block selects (D 11.4).
fn log_rows(world: &World, name: &str) -> Vec<String> {
    world.m.defs.get(name).and_then(|d| d.block()).map(|b| b.items.iter().map(|i| names::qualify(i.namespace.as_deref(), i.key())).collect()).unwrap_or_default()
}

/// The map cell at a screen position of a map area, for tests and the
/// mouse later.
pub fn cell_at(view: &View, map: MapId, r: Rect, x: usize, y: usize) -> Option<Pos> {
    let (x0, y0) = *view.camera.get(&map)?;
    r.contains(x, y).then(|| Pos { map, x: x0 + (x - r.x) as i32, y: y0 + (y - r.y) as i32 })
}
