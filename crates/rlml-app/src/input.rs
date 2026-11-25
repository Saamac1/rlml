//! Keys and contexts (D 11.3, M 8).
//!
//! Key names are read with Termplate's `parse_binding`, so `k`, `up`,
//! `esc`, `shift+q`, `plus` and `num8` mean the same in both. Contexts
//! stack at run time: `ALWAYS` is heard above all of them, then the top of
//! the stack; a `modal` context (the default) stops there, a
//! `passthrough` one lets keys it does not bind go down. The engine's
//! contexts have default bindings, and the game's row for the same action
//! in the same context replaces the default.

use std::collections::BTreeMap;

use rlml_core::model::{Kind, Model};
use rlml_core::names;
use rlml_core::rs::ast::{ItemKind, Source};
use rlml_core::rt::value::{CommandValue, Value};
use termplate_core::keys::{Binding, KeyCode, parse_binding};

/// What a key does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// A command by full name.
    Command(String),
    /// An engine action (`pass`, `confirm`, `cursor.north`, …, M 9.1).
    Engine(String),
    /// `open NAME [stack]` (D 11.2).
    Open { menu: String, stack: bool },
    /// `toggle PANEL` (D 11.3).
    Toggle(String),
    /// `focus MENU`: the keys go to a docked menu (D 11.3).
    Focus(String),
    /// `read LOG`: the log opens to be read back (D 11.4).
    Read(String),
    /// `tab NAME`: a tabs area shows NAME (D 11.1).
    Tab(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Modal,
    Passthrough,
}

/// A device that presses keys (M 8.2): a keyboard part (D 11.3), 1 for
/// the section without `keyboard N`, or a pad by its connection id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Device {
    Keyboard(u8),
    Pad(usize),
}

/// A pad press (D 11.3): a button or stick direction with the buttons held
/// with it, as written `lb+start`; `held` is sorted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PadPress {
    pub button: String,
    pub held: Vec<String>,
}

/// The pad's place-based names (D 11.3).
pub const PAD_NAMES: &[&str] = &[
    "south",
    "east",
    "west",
    "north",
    "lb",
    "rb",
    "lt",
    "rt",
    "start",
    "select",
    "lstick",
    "rstick",
    "dpad.up",
    "dpad.down",
    "dpad.left",
    "dpad.right",
    "lstick.up",
    "lstick.down",
    "lstick.left",
    "lstick.right",
    "rstick.up",
    "rstick.down",
    "rstick.left",
    "rstick.right",
];

impl PadPress {
    /// `south`, `dpad.up`, `lb+start`.
    pub fn parse(s: &str) -> Result<PadPress, String> {
        let mut parts: Vec<&str> = s.split('+').collect();
        let button = parts.pop().unwrap_or("");
        for p in parts.iter().chain([&button]) {
            if !PAD_NAMES.contains(p) {
                return Err(format!("unknown pad button `{p}` in `{s}` (D 11.3)"));
            }
        }
        let mut held: Vec<String> = parts.into_iter().map(String::from).collect();
        held.sort();
        Ok(PadPress {
            button: button.to_string(),
            held,
        })
    }
}

/// A press from a device.
#[derive(Debug, Clone, PartialEq)]
pub enum Press {
    Key(Binding),
    Pad(PadPress),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub mode: Mode,
    /// Keyboard part 1's rows, which every keyboard part uses when the
    /// context has no section of its own for it (D 11.3).
    pub rows: Vec<(Binding, Action)>,
    /// `keys CONTEXT keyboard N` sections by part.
    pub parts: BTreeMap<u8, Vec<(Binding, Action)>>,
    /// `pad CONTEXT` rows.
    pub pad: Vec<(PadPress, Action)>,
    /// `pad CONTEXT deadzone P%`, as a share of the stick's reach.
    pub deadzone: Option<f32>,
}

impl Context {
    fn new() -> Context {
        Context {
            mode: Mode::Modal,
            rows: vec![],
            parts: BTreeMap::new(),
            pad: vec![],
            deadzone: None,
        }
    }

    fn find(&self, device: Device, press: &Press) -> Option<&Action> {
        match (device, press) {
            (Device::Keyboard(p), Press::Key(b)) => {
                let rows = self.parts.get(&p).unwrap_or(&self.rows);
                rows.iter().find(|(x, _)| x == b).map(|(_, a)| a)
            }
            (Device::Pad(_), Press::Pad(p)) => {
                self.pad.iter().find(|(x, _)| x == p).map(|(_, a)| a)
            }
            _ => None,
        }
    }
}

/// The engine's contexts and their default bindings (D 11.3).
fn defaults() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    let cursor = [
        ("cursor.north", "up k num8"),
        ("cursor.south", "down j num2"),
        ("cursor.west", "left h num4"),
        ("cursor.east", "right l num6"),
        ("cursor.northwest", "y num7"),
        ("cursor.northeast", "u num9"),
        ("cursor.southwest", "b num1"),
        ("cursor.southeast", "n num3"),
        ("next", "tab"),
        ("prev", "shift+tab"),
        ("confirm", "enter space"),
        ("back", "esc"),
    ];
    let menu = vec![
        ("cursor.north", "up k"),
        ("cursor.south", "down j"),
        ("confirm", "enter space"),
        ("back", "esc backspace"),
        ("page.up", "pageup"),
        ("page.down", "pagedown"),
        ("first", "home"),
        ("last", "end"),
    ];
    // a dialogue's `adjust` items move with left and right (RS 9.7)
    let dialogue: Vec<_> = menu
        .iter()
        .copied()
        .chain([("cursor.west", "left h"), ("cursor.east", "right l")])
        .collect();
    vec![
        ("MENU", menu.clone()),
        ("DIALOGUE", dialogue),
        (
            "SCENE",
            vec![
                ("confirm", "enter space"),
                ("skip", "tab"),
                ("restart", "r"),
                ("rewind", "backspace"),
            ],
        ),
        // a free device's `join` is a joined one's `confirm` (M 8.2)
        (
            "JOIN",
            vec![("join", "enter"), ("back", "esc"), ("skip", "tab")],
        ),
        ("AIM", cursor.to_vec()),
        ("LOOK", cursor.to_vec()),
        ("DIRECTION", cursor.to_vec()),
        ("ALWAYS", vec![]),
    ]
}

/// The engine's pad defaults (M 8): the d-pad moves, `south` confirms,
/// `east` goes back.
fn pad_defaults() -> Vec<(&'static str, Vec<(&'static str, &'static str)>)> {
    let cursor = vec![
        ("cursor.north", "dpad.up lstick.up"),
        ("cursor.south", "dpad.down lstick.down"),
        ("cursor.west", "dpad.left lstick.left"),
        ("cursor.east", "dpad.right lstick.right"),
        ("next", "rb"),
        ("prev", "lb"),
        ("confirm", "south"),
        ("back", "east"),
    ];
    let menu = vec![
        ("cursor.north", "dpad.up"),
        ("cursor.south", "dpad.down"),
        ("confirm", "south"),
        ("back", "east"),
    ];
    vec![
        ("MENU", menu.clone()),
        ("DIALOGUE", menu),
        (
            "SCENE",
            vec![("confirm", "south"), ("skip", "start"), ("rewind", "east")],
        ),
        (
            "JOIN",
            vec![
                ("join", "start south"),
                ("back", "east"),
                ("skip", "select"),
            ],
        ),
        ("AIM", cursor.clone()),
        ("LOOK", cursor.clone()),
        ("DIRECTION", cursor),
    ]
}

#[derive(Debug, Clone, Default)]
pub struct Keys {
    pub contexts: BTreeMap<String, Context>,
    /// The game's base context (`game.keys`, D 3.3).
    pub base: Option<String>,
    /// Key names that could not be read.
    pub warnings: Vec<String>,
}

impl Keys {
    pub fn from_model(m: &Model) -> Keys {
        let mut keys = Keys::default();
        for (name, rows) in defaults() {
            let mut ctx = Context::new();
            for (action, names) in rows {
                for n in names.split_whitespace() {
                    if let Ok(b) = parse_binding(n) {
                        ctx.rows.push((b, Action::Engine(action.to_string())));
                    }
                }
            }
            keys.contexts.insert(name.to_string(), ctx);
        }
        for (name, rows) in pad_defaults() {
            let ctx = keys
                .contexts
                .entry(name.to_string())
                .or_insert_with(Context::new);
            for (action, names) in rows {
                for n in names.split_whitespace() {
                    if let Ok(p) = PadPress::parse(n) {
                        ctx.pad.push((p, Action::Engine(action.to_string())));
                    }
                }
            }
        }
        for (full, blocks) in &m.contexts {
            let name = full.clone();
            for b in blocks
                .iter()
                .filter(|b| b.kind == "keys" || b.kind == "pad")
            {
                let part = rlml_core::check::keyboard_part(b).unwrap_or(1);
                let ctx = keys
                    .contexts
                    .entry(name.clone())
                    .or_insert_with(Context::new);
                if b.kind == "keys" && part == 1 && b.has_arg("passthrough") {
                    ctx.mode = Mode::Passthrough;
                }
                if b.kind == "pad"
                    && let Some(p) = b
                        .arg_after("deadzone")
                        .and_then(|d| d.strip_suffix('%'))
                        .and_then(|d| d.parse::<f32>().ok())
                {
                    ctx.deadzone = Some(p / 100.0);
                }
                for item in &b.items {
                    let (_, row) = item.marker();
                    let (action, n) = match row.key() {
                        "open" => (
                            Action::Open {
                                menu: row
                                    .word(1)
                                    .and_then(|w| m.resolve(row.namespace.as_deref(), w).ok())
                                    .map(|d| d.full.clone())
                                    .unwrap_or_default(),
                                stack: row.word(2) == Some("stack"),
                            },
                            if row.word(2) == Some("stack") { 3 } else { 2 },
                        ),
                        "toggle" => (Action::Toggle(row.word(1).unwrap_or("").to_string()), 2),
                        "focus" => (
                            Action::Focus(
                                row.word(1)
                                    .and_then(|w| m.resolve(row.namespace.as_deref(), w).ok())
                                    .map(|d| d.full.clone())
                                    .unwrap_or_default(),
                            ),
                            2,
                        ),
                        "tab" => (
                            Action::Tab(
                                row.word(1)
                                    .map(|w| {
                                        m.resolve(row.namespace.as_deref(), w)
                                            .map_or(w.to_string(), |d| d.full.clone())
                                    })
                                    .unwrap_or_default(),
                            ),
                            2,
                        ),
                        "read" => (
                            Action::Read(
                                row.word(1)
                                    .map(|w| {
                                        m.resolve(row.namespace.as_deref(), w)
                                            .map_or(w.to_string(), |d| d.full.clone())
                                    })
                                    .unwrap_or_default(),
                            ),
                            2,
                        ),
                        w if names::is_vocabulary(w) => (Action::Engine(w.to_string()), 1),
                        w => match m.resolve_kind(row.namespace.as_deref(), w, &[Kind::Command]) {
                            Ok(d) => (Action::Command(d.full.clone()), 1),
                            Err(_) => continue,
                        },
                    };
                    let ctx = keys.contexts.get_mut(&name).unwrap();
                    if b.kind == "pad" {
                        // the game's row for an action replaces the default's buttons
                        ctx.pad.retain(|(_, a)| *a != action);
                        for k in row.tokens.iter().skip(n) {
                            match PadPress::parse(&k.text) {
                                Ok(p) => {
                                    ctx.pad.retain(|(x, _)| *x != p);
                                    ctx.pad.push((p, action.clone()));
                                }
                                Err(e) => keys
                                    .warnings
                                    .push(format!("{}:{}: {e}", item.file, item.line)),
                            }
                        }
                        continue;
                    }
                    // the game's row for an action replaces the default's keys
                    let rows = if part == 1 {
                        &mut ctx.rows
                    } else {
                        ctx.parts.entry(part).or_default()
                    };
                    rows.retain(|(_, a)| *a != action);
                    for k in row.tokens.iter().skip(n) {
                        match parse_binding(&k.text) {
                            Ok(b) => {
                                rows.retain(|(x, _)| *x != b);
                                rows.push((b, action.clone()));
                            }
                            Err(e) => keys
                                .warnings
                                .push(format!("{}:{}: {e}", item.file, item.line)),
                        }
                    }
                }
            }
        }
        let game = m.named_game();
        let ns = game.and_then(|g| g.namespace.as_deref());
        keys.base = game
            .and_then(|g| g.items.iter().find(|i| i.key() == "keys"))
            .and_then(|i| i.word(1))
            .and_then(|n| {
                names::candidates(ns, n)
                    .into_iter()
                    .find(|c| keys.contexts.contains_key(c))
            });
        // the base context's mouse default (M 8): `travel_to click`, unless
        // the game binds `travel_to` or `click` there itself
        if let Some(ctx) = keys.base.clone().and_then(|b| keys.contexts.get_mut(&b)) {
            let click = Binding::mouse(termplate_core::keys::Mouse::Click);
            let travel = Action::Engine("travel_to".into());
            if !ctx.rows.iter().any(|(b, a)| *a == travel || *b == click) {
                ctx.rows.push((click, travel));
            }
            // and `verbs rclick`, the verb coin
            let rclick = Binding::mouse(termplate_core::keys::Mouse::RightClick);
            let verbs = Action::Engine("verbs".into());
            if !ctx.rows.iter().any(|(b, a)| *a == verbs || *b == rclick) {
                ctx.rows.push((rclick, verbs));
            }
        }
        keys
    }

    /// The keyboard parts any context has a section for, part 1 first.
    pub fn keyboard_parts(&self) -> Vec<u8> {
        let mut parts: Vec<u8> = self
            .contexts
            .values()
            .flat_map(|c| c.parts.keys().copied())
            .collect();
        parts.push(1);
        parts.sort();
        parts.dedup();
        parts
    }

    /// The keyboard part a key belongs to (D 11.3): the part whose section
    /// binds it, or part 1. The checker keeps a key in one part per
    /// context, so the base context and `ALWAYS` decide first.
    pub fn part_of(&self, b: &Binding) -> u8 {
        let order = self
            .base
            .iter()
            .map(String::as_str)
            .chain(["ALWAYS"])
            .chain(self.contexts.keys().map(String::as_str));
        for name in order {
            if let Some(c) = self.contexts.get(name)
                && let Some((p, _)) = c
                    .parts
                    .iter()
                    .find(|(_, rows)| rows.iter().any(|(x, _)| x == b))
            {
                return *p;
            }
        }
        1
    }

    /// A pad's deadzone in a context stack: the top context's, or 50%.
    pub fn deadzone(&self, stack: &[String]) -> f32 {
        stack
            .iter()
            .rev()
            .find_map(|n| self.contexts.get(n).and_then(|c| c.deadzone))
            .unwrap_or(0.5)
    }

    /// The action of a press from a device in a context stack (bottom
    /// first): `ALWAYS` first, then the top down while contexts pass
    /// presses through (D 11.3).
    pub fn lookup_on(&self, stack: &[String], device: Device, press: &Press) -> Option<&Action> {
        if let Some(a) = self
            .contexts
            .get("ALWAYS")
            .and_then(|c| c.find(device, press))
        {
            return Some(a);
        }
        for name in stack.iter().rev() {
            if let Some(a) = self.contexts.get(name).and_then(|c| c.find(device, press)) {
                return Some(a);
            }
            if self
                .contexts
                .get(name)
                .is_none_or(|c| c.mode == Mode::Modal)
            {
                return None;
            }
        }
        None
    }

    /// Is an action `ALWAYS`'s for this press?
    /// A press in one context only, without `ALWAYS`: the join screen's
    /// `back` is `esc`, which pauses elsewhere.
    pub fn lookup_only(&self, context: &str, device: Device, press: &Press) -> Option<&Action> {
        self.contexts
            .get(context)
            .and_then(|c| c.find(device, press))
    }

    pub fn is_always(&self, device: Device, press: &Press) -> bool {
        self.contexts
            .get("ALWAYS")
            .is_some_and(|c| c.find(device, press).is_some())
    }

    /// The first key or button that runs `command` from `device` in a
    /// context stack, as written in a `keys` or `pad` row (M 7.3: a verb's
    /// shortcut). `None` when it is not bound.
    pub fn shortcut(&self, stack: &[String], device: Device, command: &str) -> Option<String> {
        self.shortcut_for(stack, device, &Action::Command(command.to_string()))
    }

    /// The first key or button for any action: a docked menu's row shows
    /// it for a command, an engine action (`look`) or `open MENU` alike.
    pub fn shortcut_for(&self, stack: &[String], device: Device, want: &Action) -> Option<String> {
        let want = want.clone();
        // `ALWAYS` is heard above every stack
        let always = ["ALWAYS".to_string()];
        for name in always.iter().chain(stack.iter().rev()) {
            let Some(c) = self.contexts.get(name) else {
                continue;
            };
            let found = match device {
                Device::Keyboard(p) => c
                    .parts
                    .get(&p)
                    .unwrap_or(&c.rows)
                    .iter()
                    .find(|(_, a)| *a == want)
                    .map(|(b, _)| b.name()),
                Device::Pad(_) => c.pad.iter().find(|(_, a)| *a == want).map(|(p, _)| {
                    p.held
                        .iter()
                        .cloned()
                        .chain([p.button.clone()])
                        .collect::<Vec<_>>()
                        .join("+")
                }),
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// The action of a key in a context stack, for the key's own keyboard
    /// part.
    pub fn lookup(&self, stack: &[String], b: &Binding) -> Option<&Action> {
        self.lookup_on(
            stack,
            Device::Keyboard(self.part_of(b)),
            &Press::Key(b.clone()),
        )
    }
}

/// What a key press asks the game to do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Command(CommandValue),
    Engine(String),
    /// `open`; `always` when the key is `ALWAYS`'s, where the same key
    /// closes the menu it opened (M 8).
    Open {
        menu: String,
        stack: bool,
        always: bool,
    },
    Toggle(String),
    Focus(String),
    Read(String),
    Tab(String),
    /// The repeat count is being typed (D 11.3).
    Counting(String),
    /// The press is not bound here.
    Unbound,
}

/// Keys and the repeat count being typed.
#[derive(Debug, Clone)]
pub struct Input {
    pub keys: Keys,
    count: Option<String>,
}

/// The parameter a command's repeat count goes to (`count`, RS 7.1).
fn count_param(m: &Model, command: &str) -> Option<String> {
    let rlml_core::model::Source::Decl(d) = &m.defs.get(command)?.source else {
        return None;
    };
    let ItemKind::Command { params, .. } = &d.item.kind else {
        return None;
    };
    params
        .iter()
        .find(|p| matches!(p.source, Some(Source::Count)))
        .map(|p| p.name.clone())
}

impl Input {
    pub fn new(m: &Model) -> Input {
        Input {
            keys: Keys::from_model(m),
            count: None,
        }
    }

    /// The context stack of the game's play: its base context.
    pub fn base_stack(&self) -> Vec<String> {
        self.keys.base.iter().cloned().collect()
    }

    /// A key from its own keyboard part.
    pub fn press(&mut self, m: &Model, stack: &[String], b: Binding) -> Effect {
        let device = Device::Keyboard(self.keys.part_of(&b));
        self.press_on(m, stack, device, Press::Key(b))
    }

    /// A press from a device (M 8.2).
    pub fn press_on(
        &mut self,
        m: &Model,
        stack: &[String],
        device: Device,
        press: Press,
    ) -> Effect {
        // `repeat`: digits write the count, the first other key ends it and is
        // the command's key (D 11.3)
        if let Some(count) = &mut self.count {
            if let Press::Key(Binding::Key(KeyCode::Char(c), _)) = press
                && c.is_ascii_digit()
            {
                count.push(c);
                return Effect::Counting(count.clone());
            }
            let n = self.count.take().unwrap();
            return match self.keys.lookup_on(stack, device, &press).cloned() {
                Some(Action::Engine(a)) if a == "back" => Effect::Counting(String::new()),
                Some(Action::Command(c)) => {
                    let mut cmd = CommandValue {
                        name: c.clone(),
                        args: vec![],
                    };
                    if let (Some(p), Ok(k)) = (count_param(m, &c), n.parse::<i64>()) {
                        cmd.args.push((Some(p), Value::Number(k)));
                    }
                    Effect::Command(cmd)
                }
                Some(a) => self.effect(a, device, &press),
                None => Effect::Unbound,
            };
        }
        match self.keys.lookup_on(stack, device, &press).cloned() {
            Some(Action::Engine(a)) if a == "repeat" => {
                self.count = Some(String::new());
                Effect::Counting(String::new())
            }
            Some(a) => self.effect(a, device, &press),
            None => Effect::Unbound,
        }
    }

    fn effect(&self, a: Action, device: Device, press: &Press) -> Effect {
        match a {
            Action::Command(c) => Effect::Command(CommandValue {
                name: c,
                args: vec![],
            }),
            Action::Engine(e) => Effect::Engine(e),
            Action::Open { menu, stack } => Effect::Open {
                menu,
                stack,
                always: self.keys.is_always(device, press),
            },
            Action::Toggle(p) => Effect::Toggle(p),
            Action::Focus(m) => Effect::Focus(m),
            Action::Read(l) => Effect::Read(l),
            Action::Tab(t) => Effect::Tab(t),
        }
    }
}
