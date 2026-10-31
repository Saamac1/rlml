//! Menus (D 11.2, M 7.5): a menu's items and actions, the stack of open
//! layers and the menu window.
//!
//! * An item without an action runs the command of its name or the engine
//!   action of its name in lower case (`- RESUME` → `resume`).
//! * `hide when` and `disabled when` are evaluated once, when the menu
//!   opens; a disabled item shows dim and cannot be chosen.
//! * `open NAME` returns to a layer already open, closing those above;
//!   `open NAME stack` always adds a layer.
//! * `back` and `close` do not close a `final` menu.
//! * The title, body and item texts are `<MENU>.TITLE`, `<MENU>.TEXT` and
//!   `<MENU>.<ITEM>`. List menus are not here yet.

use rlml_core::model::{Kind, Model};
use rlml_core::names;
use rlml_core::rs::ast::Expr;
use rlml_core::rt::eval::{BodyKind, Env};
use rlml_core::rt::value::Value;
use rlml_core::rt::world::World;
use rlml_core::syntax::RawItem;
use termplate_core::frame::{Frame, Role, str_width, wrap};
use termplate_core::layout::Rect;
use termplate_core::style::Border;

/// What choosing an item does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemAction {
    /// A command by full name.
    Command(String),
    /// An engine action: `resume`, `quit`, `new_game`, `exit`, `back`,
    /// `close`, … (M 9.1).
    Engine(String),
    /// `open MENU [stack]`.
    Open { menu: String, stack: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The item's text key under the menu (`RESUME`).
    pub key: String,
    pub action: ItemAction,
    pub hide: Option<Expr>,
    pub disabled: Option<Expr>,
}

/// A `menu` block read for play.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    pub full: String,
    pub ns: Option<String>,
    /// The key context, `MENU` by default (D 11.2).
    pub context: String,
    pub is_final: bool,
    /// `size LxK`; `*` or none fits the content.
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub items: Vec<Item>,
}

/// The words of `when` up to the next `hide` or `disabled` as an
/// expression.
fn condition(row: &RawItem, word: &str) -> Option<Expr> {
    let words: Vec<&str> = row.tokens.iter().map(|t| t.text.as_str()).collect();
    let start = words.windows(2).position(|w| w[0] == word && w[1] == "when")? + 2;
    let end = (start..words.len()).find(|&i| (words[i] == "hide" || words[i] == "disabled") && words.get(i + 1) == Some(&"when")).unwrap_or(words.len());
    rlml_core::rs::parse::parse_expr(row.span(start, end)).ok()
}

/// The action written after an item's key, or its default (D 11.2).
fn action(m: &Model, ns: Option<&str>, key: &str, rest: &RawItem) -> Option<ItemAction> {
    let ns = rest.namespace.as_deref().or(ns);
    let written = !rest.tokens.is_empty() && !["hide", "disabled"].contains(&rest.key());
    if !written {
        if let Ok(d) = m.resolve_kind(ns, key, &[Kind::Command]) {
            return Some(ItemAction::Command(d.full.clone()));
        }
        return Some(ItemAction::Engine(key.to_lowercase()));
    }
    let w = rest.key();
    if w == "open" {
        let target = rest.word(1)?;
        let menu = m.resolve(ns, target.split('(').next().unwrap_or(target)).ok()?.full.clone();
        return Some(ItemAction::Open { menu, stack: rest.word(2) == Some("stack") });
    }
    if names::is_vocabulary(w) {
        return Some(ItemAction::Engine(w.to_string()));
    }
    m.resolve_kind(ns, w, &[Kind::Command]).ok().map(|d| ItemAction::Command(d.full.clone()))
}

/// `LxK` with `*` for "fit".
fn size(word: &str) -> (Option<usize>, Option<usize>) {
    let (w, h) = word.split_once('x').unwrap_or((word, "*"));
    (w.parse().ok(), h.parse().ok())
}

impl Menu {
    /// The menu `full`, or `None` when it is not a menu or is a list menu.
    pub fn read(m: &Model, full: &str) -> Option<Menu> {
        let def = m.defs.get(full).filter(|d| d.kind == Kind::Menu)?;
        let b = def.block()?;
        if b.has_arg("list") {
            return None;
        }
        let ns = b.namespace.clone();
        let context = b
            .arg_after("context")
            .map(|c| names::candidates(ns.as_deref(), c).into_iter().find(|n| m.contexts.contains_key(n)).unwrap_or_else(|| c.to_string()))
            .unwrap_or_else(|| "MENU".into());
        let (width, height) = b.arg_after("size").map(size).unwrap_or((None, None));
        let mut items = Vec::new();
        for item in &b.items {
            let (_, row) = item.marker();
            let key = row.key().to_string();
            let Some(action) = action(m, ns.as_deref(), &key, &row.shift(1)) else { continue };
            items.push(Item { key, action, hide: condition(&row, "hide"), disabled: condition(&row, "disabled") });
        }
        Some(Menu { full: full.to_string(), ns, context, is_final: b.has_arg("final"), width, height, items })
    }
}

/// One shown row: the item's index and whether it can be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub item: usize,
    pub enabled: bool,
}

/// An open menu: its rows as evaluated when it opened and the cursor.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub menu: Menu,
    pub rows: Vec<Row>,
    pub cursor: usize,
}

/// A condition's value; one that cannot be evaluated is false, so a
/// broken `hide when` shows the item and a broken `disabled when` lets it
/// be chosen.
fn holds(w: &mut World, menu: &Menu, e: &Expr) -> bool {
    let mut env = Env::new(BodyKind::Query, &menu.full, menu.ns.clone(), "<menu>");
    env.actor = w.s.player;
    matches!(w.eval(e, &mut env), Ok(Value::Bool(true)))
}

impl Layer {
    /// Opens `menu`: every item's conditions are evaluated now (D 11.2).
    pub fn open(w: &mut World, menu: Menu) -> Layer {
        let mut rows = Vec::new();
        for (i, item) in menu.items.iter().enumerate() {
            if item.hide.as_ref().is_some_and(|e| holds(w, &menu, e)) {
                continue;
            }
            let enabled = !item.disabled.as_ref().is_some_and(|e| holds(w, &menu, e));
            rows.push(Row { item: i, enabled });
        }
        let cursor = rows.iter().position(|r| r.enabled).unwrap_or(0);
        Layer { menu, rows, cursor }
    }

    /// Moves the cursor to the next enabled row up or down, wrapping.
    pub fn move_by(&mut self, step: isize) {
        let n = self.rows.len() as isize;
        if n == 0 {
            return;
        }
        let mut i = self.cursor as isize;
        for _ in 0..n {
            i = (i + step).rem_euclid(n);
            if self.rows[i as usize].enabled {
                self.cursor = i as usize;
                return;
            }
        }
    }

    /// The item under the cursor when it can be chosen.
    pub fn chosen(&self) -> Option<&Item> {
        let row = self.rows.get(self.cursor).filter(|r| r.enabled)?;
        Some(&self.menu.items[row.item])
    }
}

/// The open menus, bottom first (M 7.5).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Menus {
    pub layers: Vec<Layer>,
}

impl Menus {
    pub fn is_open(&self) -> bool {
        !self.layers.is_empty()
    }

    pub fn top(&mut self) -> Option<&mut Layer> {
        self.layers.last_mut()
    }

    pub fn position(&self, full: &str) -> Option<usize> {
        self.layers.iter().rposition(|l| l.menu.full == full)
    }

    /// `open NAME [stack]`: returns to the layer already open unless
    /// `stack`, else adds one. False when `full` is not a menu.
    pub fn open(&mut self, w: &mut World, full: &str, stack: bool) -> bool {
        let Some(menu) = Menu::read(w.m, full) else { return false };
        if !stack && let Some(i) = self.position(full) {
            self.layers.truncate(i);
        }
        let layer = Layer::open(w, menu);
        self.layers.push(layer);
        true
    }

    /// `back`: closes the top layer unless it is `final`.
    pub fn back(&mut self) {
        if self.layers.last().is_some_and(|l| !l.menu.is_final) {
            self.layers.pop();
        }
    }

    /// `close`: closes the layers down to the topmost `final` one.
    pub fn close(&mut self) {
        let keep = self.layers.iter().rposition(|l| l.menu.is_final).map_or(0, |i| i + 1);
        self.layers.truncate(keep);
    }

    /// Leaving the menus for the game or out of it: every layer goes,
    /// `final` ones too.
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// The key contexts of the open layers, bottom first.
    pub fn contexts(&self) -> impl Iterator<Item = &str> {
        self.layers.iter().map(|l| l.menu.context.as_str())
    }
}

/// A text key's text, or `None` when the language has no such key.
fn text_of(w: &World, lang: &str, key: &str) -> Option<String> {
    let t = w.text(lang, key, &[]);
    (t != key).then_some(t)
}

/// A key without a text as words: `NEW_GAME` → `New game`.
fn plain(key: &str) -> String {
    let s = key.rsplit('.').next().unwrap_or(key).replace('_', " ").to_lowercase();
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// Draws the top layer as a window in the middle of the frame: the title
/// in its border, the body text and the items, the cursor's row selected
/// and disabled ones dim.
pub fn draw(frame: &mut Frame, w: &World, lang: &str, layer: &Layer) {
    let full = &layer.menu.full;
    let title = text_of(w, lang, &format!("{full}.TITLE"));
    let body = text_of(w, lang, &format!("{full}.TEXT"));
    let items: Vec<String> = layer.rows.iter().map(|r| {
        let key = &layer.menu.items[r.item].key;
        text_of(w, lang, &format!("{full}.{key}")).unwrap_or_else(|| plain(key))
    }).collect();
    let fit = items.iter().map(|s| str_width(s) + 2).chain(title.iter().map(|t| str_width(t) + 2)).max().unwrap_or(0).max(12);
    let width = layer.menu.width.unwrap_or(fit + 2).min(frame.width).max(4);
    let inner = width - 2;
    let body_lines = body.as_deref().map(|b| wrap(b, inner)).unwrap_or_default();
    let gap = usize::from(!body_lines.is_empty());
    let height = layer.menu.height.unwrap_or(body_lines.len() + gap + items.len() + 2).min(frame.height).max(3);
    let r = Rect::new((frame.width - width) / 2, (frame.height - height) / 2, width, height);
    frame.set_area(vec!["MENU".into(), full.clone()]);
    frame.fill(r, ' ', Role::Normal);
    frame.border(r, Border::Single, Role::Border);
    if let Some(t) = &title {
        let t = format!(" {t} ");
        let x = r.x + (width.saturating_sub(str_width(&t))) / 2;
        frame.text(x, r.y, inner, &t, Role::Title);
    }
    let mut y = r.y + 1;
    let bottom = r.y + height - 1;
    for line in &body_lines {
        if y < bottom {
            frame.text(r.x + 1, y, inner, line, Role::Normal);
            y += 1;
        }
    }
    y += gap;
    for (i, (row, text)) in layer.rows.iter().zip(&items).enumerate() {
        if y >= bottom {
            break;
        }
        let role = if !row.enabled {
            Role::Disabled
        } else if i == layer.cursor {
            Role::Selected
        } else {
            Role::Normal
        };
        let line = termplate_core::frame::fit(&format!(" {text}"), inner, termplate_core::model::Align::Left);
        frame.text(r.x + 1, y, inner, &line, role);
        y += 1;
    }
}
