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
    /// `map`, `hover`, `sentence`, `menu`, or a log or panel name as
    /// written.
    pub source: String,
    pub follow: Follow,
    /// A docked menu's name as written: `- v menu VERBS` (D 11.1).
    pub menu: Option<String>,
    /// The map area's `cursor TYYLI` as written: `underline`, `reverse`,
    /// `bold` or `bg COLOR` (D 11.1).
    pub cursor: Vec<String>,
    /// A `tabs` area's contents as written (`PACK(player)`, D 11.1).
    pub tabs: Vec<String>,
    /// `read`: a click on the log opens it to be read (D 11.4).
    pub read: bool,
    /// `center`: the panel is centred in a larger area (D 11.1).
    pub center: bool,
}

/// The size a tab's content needs: a panel's drawing, a menu's rows (as
/// wide as nothing asks); a log takes what it gets (`None`).
fn content_size(world: &World, tab: &str) -> Option<(usize, usize)> {
    let name = tab.split('(').next().unwrap_or("");
    let d = world.m.resolve(None, name).ok()?;
    match d.kind {
        Kind::Panel => crate::panel::block(world, &d.full).map(crate::panel::size),
        Kind::Menu => d.block().map(|b| (0, b.items.len())),
        _ => None,
    }
}

/// The buttons' look and the pressed one's: the `panels` rows `button`
/// and `button.pressed` (D 9.3).
pub fn button_look(world: &World, tc: bool) -> ((Border, Style), Style) {
    (panel_style(world, "button", tc), panel_style(world, "button.pressed", tc).1)
}

/// The tab row's height: three rows for buttons with a border, else one.
fn tab_row_height(world: &World, tc: bool) -> usize {
    if panel_style(world, "button", tc).0 != Border::None { 3 } else { 1 }
}

/// A tabs area's key: its contents as written, so the same tabs keep
/// their choice in another layout (D 11.1).
pub fn tabs_key(tabs: &[String]) -> String {
    tabs.join(" ")
}

/// A docked menu's row as the app gives it to draw (M 7.1, M 8.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockRow {
    pub label: String,
    /// The shortcut, right-aligned: the verb's binding for the seat whose
    /// turn it is, or empty.
    pub key: String,
    pub enabled: bool,
}

/// A docked menu to draw: its rows, the row under the pointer, the
/// focused row and the chosen verb's row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Docked {
    pub rows: Vec<DockRow>,
    pub pointed: Option<usize>,
    pub focused: Option<usize>,
    pub chosen: Option<usize>,
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
    /// Where the map was last drawn, inside its border: the cells
    /// `cell_at` maps to the map (M 8.1).
    pub map_rect: Option<Rect>,
    /// The `hover` area's text: what is under the pointer (M 8.1); the app
    /// sets it.
    pub hover: Option<String>,
    /// Cells to mark on the map: open cursors, orders' targets and the
    /// mouse pointer's cell (M 7.1, M 8.1).
    pub cursors: Vec<rlml_core::rt::value::Pos>,
    /// Docked menus by full name, as the app sets them, and where each was
    /// last drawn, inside its border, for the mouse.
    pub docked: BTreeMap<String, Docked>,
    pub docked_rects: BTreeMap<String, Rect>,
    /// The `sentence` area's text (M 8.1); the app sets it.
    pub sentence: Option<String>,
    /// The window is taller than wide in pixels: the app sets it, and the
    /// layout is chosen by it and the window's cells (D 11.1).
    pub portrait: bool,
    /// The chosen tab of each tabs area by its key (`tabs_key`), and the
    /// key of the area whose tab changed last (D 11.1).
    pub tabs: BTreeMap<String, usize>,
    pub last_tabs: Option<String>,
    /// Where the last frame drew each tab title: the area's key, the tab
    /// and its cells; and each tabs area's key with its tabs' names.
    pub tab_rects: Vec<(String, usize, Rect)>,
    pub tab_areas: Vec<(String, Vec<String>)>,
    /// The pointer's cell and whether the turn's seat can act, for the
    /// buttons; the app sets them (D 11.5).
    pub pointer: Option<(usize, usize)>,
    pub enabled: bool,
    /// The last frame's buttons: their cells and actions.
    pub buttons: Vec<(Rect, crate::menu::ItemAction)>,
    /// The button held down now, which shows `button.pressed` (D 9.3).
    pub pressed: Option<Rect>,
    /// The last frame's `read` log areas: their cells and logs.
    pub read_rects: Vec<(Rect, String)>,
    /// Colors as 24 bits, else the nearest of the 256-color palette; the
    /// front end sets it (`rgb`).
    pub truecolor: bool,
}

/// A style color: 24-bit when the front end takes it (a terminal whose
/// `COLORTERM` is `truecolor` or `24bit`, a browser), else the nearest
/// color of the 256-color palette, which every common terminal shows.
/// macOS Terminal before version 26, for one, shows no 24-bit colors.
pub fn rgb(c: Rgb, truecolor: bool) -> Color {
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

fn style_of(look: &Look, tc: bool) -> Style {
    Style { fg: look.fg.map(|c| rgb(c, tc)), bg: look.bg.map(|c| rgb(c, tc)), ..Style::default() }
}

/// The layout for the window: the first `layout` row whose `when` holds,
/// else the default (D 11.1).
fn layout_block(w: &World, cols: usize, rows: usize, portrait: bool) -> Option<rlml_core::syntax::RawBlock> {
    w.m.layout_for(cols, rows, portrait).cloned()
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
        let mut cursor = Vec::new();
        let mut i = 1;
        while i < words.len() {
            match words[i] {
                "cursor" => {
                    let n = if words.get(i + 1) == Some(&"bg") { 2 } else { 1 };
                    cursor = words.iter().skip(i + 1).take(n).map(|w| w.to_string()).collect();
                    i += 1 + n;
                }
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
        let source = words.first().copied().unwrap_or("").to_string();
        let menu = (source == "menu").then(|| words.get(1).map(|w| w.to_string())).flatten();
        let tabs = if source == "tabs" { words.iter().skip(1).take_while(|w| !w.starts_with("size")).map(|w| w.to_string()).collect() } else { vec![] };
        let (read, center) = (words.contains(&"read"), words.contains(&"center"));
        out.push(Area { ch, source, follow, menu, cursor, tabs, read, center });
    }
    (out, dims)
}

/// A panel style row for an area name: border and colors (D 9.3).
fn panel_style(w: &World, name: &str, tc: bool) -> (Border, Style) {
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
        style.fg = after("fg").and_then(|c| w.color(c, ns)).map(|c| rgb(c, tc)).or(style.fg);
        style.bg = after("bg").and_then(|c| w.color(c, ns)).map(|c| rgb(c, tc)).or(style.bg);
    }
    if let Some(b) = w.m.panel_border(name).and_then(Border::parse) {
        border = b;
    }
    (border, style)
}

/// A log area's `fade P%` and the color it fades toward: the area's
/// `bg`, else black (D 9.3).
fn panel_fade(w: &World, name: &str) -> Option<(f32, Rgb)> {
    let mut fade = None;
    let mut bg = None;
    for (_, item) in w.m.section_rows("panels") {
        let (_, row) = item.marker();
        let ns = row.namespace.as_deref();
        if names::qualify(ns, row.key()) != name && row.key() != name {
            continue;
        }
        let words: Vec<&str> = row.tokens.iter().map(|t| t.text.as_str()).collect();
        let after = |k: &str| words.iter().position(|x| *x == k).and_then(|i| words.get(i + 1).copied());
        fade = after("fade").and_then(rlml_core::rt::view::percent).or(fade);
        bg = after("bg").and_then(|c| w.color(c, ns)).or(bg);
    }
    fade.map(|f| (f, bg.unwrap_or([0, 0, 0])))
}

/// A message key's color by the most specific `logs` row (D 11.4).
fn log_color(w: &World, key: &str) -> Option<Rgb> {
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
    best.map(|(_, c)| c)
}

/// Paints a rectangle's cells with a style.
pub(crate) fn paint(frame: &mut Frame, r: Rect, style: Style) {
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

pub(crate) fn put_look(frame: &mut Frame, x: usize, y: usize, look: &Look, tc: bool) {
    frame.put(x, y, look.ch, Role::Normal);
    if x < frame.width && y < frame.height {
        let i = y * frame.width + x;
        frame.cells[i].style = Some(style_of(look, tc));
    }
}

impl View {
    /// The camera's top-left cell for an area of `w`×`h` on `map`
    /// (M 7.1): the player kept in the middle or within a margin, a map
    /// smaller than the area centred, the last place kept without a player.
    fn camera(&mut self, world: &World, map: MapId, w: i32, h: i32, follow: Follow) -> (i32, i32) {
        let (mw, mh) = (world.s.maps[map].width, world.s.maps[map].height);
        let player = world.s.player.filter(|p| world.s.get(*p).alive).and_then(|p| world.s.at(p)).filter(|p| p.map == map);
        let margin = |size: i32| match follow {
            Follow::Margin(n) => n.min((size - 1) / 2),
            Follow::Center => 0,
        };
        // the group's box on this map (M 7.1); with one member it is the
        // player's cell, as before
        let cells: Vec<(i32, i32)> = world.s.players().into_iter().filter_map(|e| world.s.at(e)).filter(|p| p.map == map).map(|p| (p.x, p.y)).collect();
        let span = |pick: fn(&(i32, i32)) -> i32| cells.iter().map(pick).min().zip(cells.iter().map(pick).max());
        let group = span(|c| c.0).zip(span(|c| c.1));
        // a group that does not fit the area falls back to the player
        let fits = |(lo, hi): (i32, i32), size: i32| hi - lo + 1 <= size - 2 * margin(size);
        let target = match group {
            Some((x, y)) if fits(x, w) && fits(y, h) => Some((x, y)),
            _ => player.map(|p| ((p.x, p.x), (p.y, p.y))),
        };
        let old = self.camera.get(&map).copied();
        let axis = |len: i32, size: i32, at: Option<(i32, i32)>, old: Option<i32>| -> i32 {
            if len <= size {
                // a smaller map is centred
                return -((size - len) / 2);
            }
            let Some((lo, hi)) = at else { return old.unwrap_or(0) };
            let wanted = match (follow, old) {
                (Follow::Margin(_), Some(o)) => {
                    let n = margin(size);
                    if lo < o + n {
                        lo - n
                    } else if hi > o + size - 1 - n {
                        hi - size + 1 + n
                    } else {
                        o
                    }
                }
                _ => (lo + hi) / 2 - size / 2,
            };
            wanted.clamp(0, len - size)
        };
        let cam = (axis(mw, w, target.map(|t| t.0), old.map(|o| o.0)), axis(mh, h, target.map(|t| t.1), old.map(|o| o.1)));
        self.camera.insert(map, cam);
        cam
    }

    /// Draws the game into a frame of the window's size.
    pub fn draw(&mut self, world: &mut World, lang: &str, width: usize, height: usize) -> Frame {
        let mut frame = Frame::new(width, height);
        // set again when the map is drawn: a too small window has none
        self.map_rect = None;
        self.docked_rects.clear();
        self.tab_rects.clear();
        self.tab_areas.clear();
        self.buttons.clear();
        self.read_rects.clear();
        let block = layout_block(world, width, height, self.portrait);
        let (areas, solved) = match &block {
            Some(b) => {
                let rows: Vec<RawItem> = b.sub("legend").cloned().collect();
                let (areas, dims) = areas(&rows);
                match layout::analyze(&b.drawing) {
                    Ok(mut grid) => {
                        let _ = grid.set_sizes(&dims);
                        // a panel's drawing and border are its area's least
                        // size (D 11.1, D 11.5)
                        let mut content = BTreeMap::<char, AreaSize>::new();
                        for a in &areas {
                            let name = a.source.split('(').next().unwrap_or("");
                            if let Some(pb) = crate::panel::block(world, name) {
                                let (pw, ph) = crate::panel::size(pb);
                                let edge = if panel_style(world, name, self.truecolor).0 != Border::None { 2 } else { 0 };
                                content.insert(a.ch, AreaSize { min_width: pw + edge, min_height: ph + edge });
                            }
                            // a tabs area holds its largest content and the tab row
                            if !a.tabs.is_empty() {
                                let (pw, ph) = a.tabs.iter().filter_map(|t| content_size(world, t)).fold((0, 0), |(w, h), (pw, ph)| (w.max(pw), h.max(ph)));
                                content.insert(a.ch, AreaSize { min_width: pw, min_height: ph + tab_row_height(world, self.truecolor) });
                            }
                        }
                        let solved = grid.solve(&content, width, height);
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
                (vec![Area { ch: 'm', source: "map".into(), follow: Follow::Center, menu: None, cursor: vec![], tabs: vec![], read: false, center: false }], layout::Solved { rects, too_small: false, min_width: 0, min_height: 0 })
            }
        };
        if solved.too_small {
            frame.text(0, 0, width, &format!("The window is too small: {}x{} is needed.", solved.min_width, solved.min_height), Role::Normal);
            return frame;
        }
        for area in &areas {
            let Some(&rect) = solved.rects.get(&area.ch) else { continue };
            let name = match (area.source.as_str(), &area.menu) {
                ("menu", Some(m)) => world.m.resolve(None, m).map(|d| d.full.clone()).unwrap_or_else(|_| m.clone()),
                ("map" | "hover" | "sentence", _) => area.source.clone(),
                _ => area.source.split('(').next().unwrap_or("").to_string(),
            };
            frame.set_area(vec![name.clone()]);
            let (mut border, style) = panel_style(world, &name, self.truecolor);
            // a written cell size is the map's view (M 7.1): the area may be
            // larger, since the last track takes the spare room (D 11.1), so
            // the map area is cut to it once, before its paint and border,
            // and the leash and the screen measure the same cells; the
            // model decides whether its border is drawn
            let rect = if area.source == "map" {
                let a = block.as_ref().map(|b| world.m.map_area_in(b)).unwrap_or_default();
                if !a.border {
                    border = Border::None;
                }
                written_map_rect(a, rect)
            } else {
                rect
            };
            paint(&mut frame, rect, style);
            let inner = if border != Border::None && rect.w >= 2 && rect.h >= 2 {
                frame.border(rect, border, Role::Normal);
                Rect::new(rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2)
            } else {
                rect
            };
            match area.source.as_str() {
                "map" => self.draw_map(&mut frame, world, inner, area.follow, &area.cursor),
                "hover" => {
                    if let Some(t) = &self.hover {
                        frame.text(inner.x, inner.y, inner.w, t, Role::Normal);
                    }
                }
                "sentence" => {
                    if let Some(t) = &self.sentence {
                        frame.text(inner.x, inner.y, inner.w, t, Role::Normal);
                    }
                }
                "menu" => {
                    self.docked_rects.insert(name.clone(), inner);
                    if let Some(d) = self.docked.get(&name) {
                        draw_docked(&mut frame, d, inner);
                    }
                }
                "tabs" => self.draw_tabs(&mut frame, world, lang, &area.tabs, inner),
                _ => {
                    // `center`: a smaller panel in the middle of its area
                    let at = match content_size(world, &area.source) {
                        Some((w, h)) if area.center => Rect::new(inner.x + inner.w.saturating_sub(w) / 2, inner.y + inner.h.saturating_sub(h) / 2, w.min(inner.w), h.min(inner.h)),
                        _ => inner,
                    };
                    if area.read {
                        self.read_rects.push((inner, name.clone()));
                    }
                    self.draw_content(&mut frame, world, lang, &area.source, at)
                }
            }
            // written text resets its cells' style: the area's colors go
            // under it again, and the content's own colors win
            paint(&mut frame, inner, style);
        }
        frame
    }

    /// An area's log, panel or docked menu, by its source as written.
    fn draw_content(&mut self, frame: &mut Frame, world: &mut World, lang: &str, source: &str, r: Rect) {
        let tc = self.truecolor;
        let written = source.split('(').next().unwrap_or("");
        let full = world.m.resolve(None, written).map(|d| (d.full.clone(), d.kind)).ok();
        match full {
            Some((name, Kind::Log)) => draw_log(frame, world, lang, &name, r, tc),
            Some((name, Kind::Menu)) => {
                self.docked_rects.insert(name.clone(), r);
                if let Some(d) = self.docked.get(&name) {
                    draw_docked(frame, d, r);
                }
            }
            Some((name, Kind::Panel)) => {
                let Some(pb) = crate::panel::block(world, &name) else { return };
                let mut ui = crate::panel::PanelUi {
                    truecolor: tc,
                    button: panel_style(world, "button", tc),
                    pressed_look: panel_style(world, "button.pressed", tc).1,
                    pressed: self.pressed,
                    pointer: self.pointer,
                    enabled: self.enabled,
                    buttons: vec![],
                };
                world.holding_view(|w| crate::panel::draw(frame, w, lang, pb, source, r, &mut ui));
                self.buttons.extend(ui.buttons);
            }
            _ => {}
        }
    }

    /// A tabs area (D 11.1, M 7.1): the titles `<NAME>.TAB` on top, as
    /// buttons of the `panels` row `button` (three rows high with its
    /// border, evenly wide) or on one row, the chosen one filled and the
    /// pointed one bold; the chosen content under them. The titles and the
    /// content are centred in the area when it is larger.
    fn draw_tabs(&mut self, frame: &mut Frame, world: &mut World, lang: &str, tabs: &[String], r: Rect) {
        if tabs.is_empty() || r.h == 0 || r.w == 0 {
            return;
        }
        let key = tabs_key(tabs);
        let chosen = self.tabs.get(&key).copied().unwrap_or(0).min(tabs.len() - 1);
        let fulls: Vec<String> = tabs.iter().map(|t| world.m.resolve(None, t.split('(').next().unwrap_or("")).map_or(t.clone(), |d| d.full.clone())).collect();
        self.tab_areas.push((key.clone(), fulls.clone()));
        let titles: Vec<String> = fulls
            .iter()
            .map(|full| {
                let k = format!("{full}.TAB");
                if world.m.has_text(lang, &k) { world.text(lang, &k, &[]) } else { full.rsplit('.').next().unwrap_or(full).to_string() }
            })
            .collect();
        let look = panel_style(world, "button", self.truecolor);
        let row_h = tab_row_height(world, self.truecolor).min(r.h);
        // the block of titles and content, centred down the area when every
        // content has a size of its own
        let heights: Option<Vec<usize>> = tabs.iter().map(|t| content_size(world, t).map(|s| s.1)).collect();
        let top = match heights {
            Some(hs) => r.y + r.h.saturating_sub(row_h + hs.into_iter().max().unwrap_or(0)) / 2,
            None => r.y,
        };
        let even = (r.w / tabs.len()).max(1);
        for (i, t) in titles.iter().enumerate() {
            let x = r.x + i * even;
            let w = if i + 1 == tabs.len() { r.x + r.w - x } else { even };
            if x >= r.x + r.w {
                break;
            }
            let cells = Rect::new(x, top, w, row_h);
            let pointed = self.pointer.is_some_and(|(px, py)| cells.contains(px, py));
            crate::panel::draw_button(frame, cells, t, look, i == chosen, pointed, false);
            self.tab_rects.push((key.clone(), i, cells));
        }
        let y = top + row_h;
        let content = Rect::new(r.x, y, r.w, (r.y + r.h).saturating_sub(y));
        // a narrower panel is centred across
        let content = match content_size(world, &tabs[chosen]) {
            Some((w, _)) if w > 0 && w < content.w => Rect::new(content.x + (content.w - w) / 2, content.y, w, content.h),
            _ => content,
        };
        self.draw_content(frame, world, lang, &tabs[chosen], content);
    }

    fn draw_map(&mut self, frame: &mut Frame, world: &mut World, r: Rect, follow: Follow, cursor: &[String]) {
        self.map_rect = Some(r);
        let Some(map) = world.s.active_map() else { return };
        let cells = world.render_cells(map);
        let (x0, y0) = self.camera(world, map, r.w as i32, r.h as i32, follow);
        for sy in 0..r.h {
            for sx in 0..r.w {
                let (mx, my) = (x0 + sx as i32, y0 + sy as i32);
                if let Some(look) = (my >= 0 && mx >= 0).then(|| cells.get(my as usize).and_then(|row| row.get(mx as usize))).flatten() {
                    put_look(frame, r.x + sx, r.y + sy, look, self.truecolor);
                }
            }
        }
        // cursors, orders and the pointer: the area's `cursor` style,
        // underlined without one (M 7.1)
        for p in self.cursors.iter().filter(|p| p.map == map) {
            let (sx, sy) = (p.x - x0, p.y - y0);
            if sx >= 0 && sy >= 0 && (sx as usize) < r.w && (sy as usize) < r.h {
                let i = (r.y + sy as usize) * frame.width + r.x + sx as usize;
                if let Some(cell) = frame.cells.get_mut(i) {
                    let mut style = cell.style.unwrap_or_default();
                    match cursor.first().map(String::as_str) {
                        Some("reverse") => style.attrs.reverse = true,
                        Some("bold") => style.attrs.bold = true,
                        Some("bg") => style.bg = cursor.get(1).and_then(|c| world.color(c, None)).map(|c| rgb(c, self.truecolor)).or(style.bg),
                        _ => style.attrs.underline = true,
                    }
                    cell.style = Some(style);
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

/// A docked menu: a row each, the shortcut at the right (M 7.1, M 8.1).
/// The chosen verb is in reverse video, the focused row bold and the row
/// under the pointer underlined; a disabled row is dim.
fn draw_docked(frame: &mut Frame, d: &Docked, r: Rect) {
    for (i, row) in d.rows.iter().enumerate().take(r.h) {
        let y = r.y + i;
        frame.text(r.x, y, r.w, &row.label, Role::Normal);
        let kw = termplate_core::frame::str_width(&row.key);
        if kw > 0 && kw + 1 < r.w {
            frame.text(r.x + r.w - kw, y, kw, &row.key, Role::Normal);
        }
        let mut style = Style::default();
        style.attrs.reverse = d.chosen == Some(i);
        style.attrs.bold = d.focused == Some(i);
        style.attrs.underline = d.pointed == Some(i);
        style.attrs.dim = !row.enabled;
        if style != Style::default() {
            paint(frame, Rect::new(r.x, y, r.w, 1), style);
        }
    }
}

/// The docked menu row under screen cell (x, y), from the last frame.
pub fn docked_at(view: &View, x: usize, y: usize) -> Option<(String, usize)> {
    view.docked_rects.iter().find(|(_, r)| r.contains(x, y)).map(|(m, r)| (m.clone(), y - r.y)).filter(|(m, i)| view.docked.get(m).is_some_and(|d| *i < d.rows.len()))
}

/// The log's newest messages that fit, wrapped at word gaps, each in its
/// `logs` color (D 11.4, M 7.4).
fn draw_log(frame: &mut Frame, world: &World, lang: &str, name: &str, r: Rect, tc: bool) {
    draw_log_back(frame, world, lang, name, r, tc, 0);
}

/// The log drawn `back` lines up from its newest, as the reading view
/// scrolls it (D 11.4, M 7.4); `back` is cut to the oldest line. Gives
/// the lines it holds and the `back` it drew.
pub fn draw_log_back(frame: &mut Frame, world: &World, lang: &str, name: &str, r: Rect, tc: bool, back: usize) -> (usize, usize) {
    let history = world.m.defs.get(name).and_then(|d| d.block()).and_then(|b| b.arg_after("history")).and_then(|n| n.parse::<usize>().ok()).unwrap_or(usize::MAX);
    let selects = log_rows(world, name);
    // each line with its color and its message's place in the log
    let mut lines: Vec<(String, Option<Rgb>, usize)> = Vec::new();
    let entries: Vec<_> = world.log.iter().filter(|l| selects.is_empty() || selects.iter().any(|s| l.key == *s || l.key.starts_with(&format!("{s}.")))).collect();
    let from = entries.len().saturating_sub(history);
    for (n, l) in entries.iter().enumerate().skip(from) {
        let text = world.text(lang, &l.key, &l.args);
        let color = log_color(world, &l.key);
        for part in wrap(&text, r.w.max(1)) {
            lines.push((part, color, n));
        }
    }
    let back = back.min(lines.len().saturating_sub(r.h));
    let end = lines.len() - back;
    let shown = &lines[end.saturating_sub(r.h)..end];
    // `fade`: the newest message is the brightest, the oldest shown the
    // dimmest (D 9.3)
    let fade = panel_fade(world, name);
    let (oldest, newest) = (shown.first().map_or(0, |l| l.2), shown.last().map_or(0, |l| l.2));
    for (i, (line, color, n)) in shown.iter().enumerate() {
        frame.text(r.x, r.y + i, r.w, line, Role::Normal);
        let share = match fade {
            Some((f, _)) if newest > oldest => f * (newest - n) as f32 / (newest - oldest) as f32,
            _ => 0.0,
        };
        let mut style = Style::default();
        match (color, fade) {
            (Some(c), Some((_, to))) => style.fg = Some(rgb(rlml_core::rt::view::blend(*c, to, share), tc)),
            (Some(c), None) => style.fg = Some(rgb(*c, tc)),
            // without a color of its own, an old line is drawn dim
            (None, _) => style.attrs.dim = share >= 0.5,
        }
        if style != Style::default() {
            paint(frame, Rect::new(r.x, r.y + i, r.w, 1), style);
        }
    }
    (lines.len(), back)
}

/// The message keys or groups a `log` block selects (D 11.4).
fn log_rows(world: &World, name: &str) -> Vec<String> {
    world.m.defs.get(name).and_then(|d| d.block()).map(|b| b.items.iter().map(|i| names::qualify(i.namespace.as_deref(), i.key())).collect()).unwrap_or_default()
}

/// The map cell at a screen position of a map area, for tests and the
/// mouse later.
/// The map area's rect cut to its written cell size, border included
/// (D 11.1, M 7.1).
fn written_map_rect(a: rlml_core::model::MapArea, rect: Rect) -> Rect {
    let cut = |len: usize, cells: Option<i32>| cells.map_or(len, |c| len.min(c.max(0) as usize));
    Rect::new(rect.x, rect.y, cut(rect.w, a.width), cut(rect.h, a.height))
}

/// The map cell under screen cell (x, y), where the map was last drawn
/// (`View::map_rect`), or `None` outside it; for tests and the mouse later.
pub fn cell_at(view: &View, map: MapId, x: usize, y: usize) -> Option<Pos> {
    let r = view.map_rect?;
    let (x0, y0) = *view.camera.get(&map)?;
    r.contains(x, y).then(|| Pos { map, x: x0 + (x - r.x) as i32, y: y0 + (y - r.y) as i32 })
}
