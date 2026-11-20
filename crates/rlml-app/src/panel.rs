//! Panels in layout areas (D 11.5, M 7.7): the drawing is the picture,
//! and the legend's `text`, `value` and `param` fields are filled each
//! frame. `value` and the area's arguments are queries, so a panel never
//! changes the state.

use rlml_core::model::{Kind, glyph_char};
use rlml_core::rs::ast::Expr;
use rlml_core::rt::eval::{BodyKind, Env};
use rlml_core::rt::value::Value;
use rlml_core::rt::view::Look;
use rlml_core::rt::world::World;
use rlml_core::syntax::{RawBlock, split_top_level};
use termplate_core::frame::{Frame, Role, wrap};
use termplate_core::layout::Rect;
use termplate_core::model::Align;

/// What a field shows (D 11.5).
enum Show {
    Text(String),
    Value(Result<Expr, String>),
    Param(String),
}

struct Field {
    ch: char,
    show: Show,
    align: Align,
}

/// The panel a layout area names, by its full name; `None` when the name
/// is no panel.
pub fn block<'m>(w: &World<'m>, name: &str) -> Option<&'m RawBlock> {
    let m = w.m;
    let d = m.resolve(None, name).ok()?;
    if d.kind != Kind::Panel {
        return None;
    }
    d.block()
}

/// The size of a panel's drawing: an area in a layout is at least this
/// big (D 11.1, D 11.5).
pub fn size(b: &RawBlock) -> (usize, usize) {
    (b.drawing.iter().map(|l| l.chars().count()).max().unwrap_or(0), b.drawing.len())
}

fn fields(b: &RawBlock) -> Vec<Field> {
    let mut out = Vec::new();
    for item in b.sub("legend") {
        let Ok((ch, rest)) = glyph_char(item) else { continue };
        let words: Vec<&str> = rest.tokens.iter().map(|t| t.text.as_str()).collect();
        let (align, end) = match words.last() {
            Some(&"left") => (Align::Left, words.len() - 1),
            Some(&"right") => (Align::Right, words.len() - 1),
            Some(&"center") => (Align::Center, words.len() - 1),
            _ => (Align::Left, words.len()),
        };
        let show = match words.first() {
            Some(&"text") => Show::Text(words.get(1).unwrap_or(&"").to_string()),
            Some(&"param") => Show::Param(words.get(1).unwrap_or(&"").to_string()),
            Some(&"value") if end > 1 => Show::Value(rlml_core::rs::parse::parse_expr(rest.span(1, end)).map_err(|d| d.message)),
            _ => continue,
        };
        out.push(Field { ch, show, align });
    }
    out
}

/// A shown cell: a character, with a look where a `{#TILE}` glyph put
/// it (D 10).
type Cell = (char, Option<Look>);

fn plain(t: &str) -> Vec<Cell> {
    t.chars().map(|c| (c, None)).collect()
}

/// A text key's text as cells, the glyphs in their looks.
fn styled(w: &World, lang: &str, key: &str, args: &[(String, Value)]) -> Vec<Cell> {
    w.text_runs(lang, key, args).into_iter().flat_map(|(t, look)| t.chars().map(move |c| (c, look)).collect::<Vec<_>>()).collect()
}

/// A value as a panel shows it, as lines: by the placeholder's rules,
/// `none` empty, and a list or a collection one member a line (M 7.7). A
/// name that is a text key shows its text with its glyphs.
fn shown(w: &World, lang: &str, v: &Value) -> Vec<Vec<Cell>> {
    match v {
        Value::List(vs) | Value::Collection(vs) => vs.iter().flat_map(|v| shown(w, lang, v)).collect(),
        Value::None => vec![vec![]],
        Value::Text(t) => vec![plain(t)],
        Value::Number(n) => vec![plain(&n.to_string())],
        Value::Name(n) if w.m.kind(n) != Some(Kind::Entity) && w.m.texts(lang).contains_key(n.as_str()) => vec![styled(w, lang, n, &[])],
        Value::Entity(_) | Value::Name(_) => vec![plain(&w.name_text(lang, v))],
        other => vec![plain(&other.to_string())],
    }
}

/// A line fitted to `width` by its alignment, cut with `…` (D 11.5).
fn fitted(line: &[Cell], width: usize, align: Align) -> Vec<Cell> {
    if line.len() > width {
        let mut out: Vec<Cell> = line[..width.saturating_sub(1)].to_vec();
        if width > 0 {
            out.push(('…', None));
        }
        return out;
    }
    let pad = width - line.len();
    let left = match align {
        Align::Left => 0,
        Align::Right => pad,
        Align::Center => pad / 2,
    };
    let mut out = vec![(' ', None); left];
    out.extend_from_slice(line);
    out.resize(width, (' ', None));
    out
}

/// Draws panel `b` into `r`; `call` is the area's source as written, with
/// its arguments (`STATS(player)`); `truecolor` as for `draw::rgb`.
pub fn draw(frame: &mut Frame, w: &mut World, lang: &str, b: &RawBlock, call: &str, r: Rect, truecolor: bool) {
    let fields = fields(b);
    let names: Vec<String> = fields.iter().map(|f| f.ch.to_string()).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let detected = match termplate_core::panel::detect(&b.drawing, &refs) {
        Ok(d) => d,
        Err(e) => {
            frame.text(r.x, r.y, r.w, &format!("panel: {e}"), Role::Normal);
            return;
        }
    };
    for (y, row) in detected.picture.iter().enumerate().take(r.h) {
        for (x, &ch) in row.iter().enumerate().take(r.w) {
            if ch != ' ' {
                frame.put(r.x + x, r.y + y, ch, Role::Normal);
            }
        }
    }
    // the area's arguments bound to the header's parameters, as queries
    let mut env = Env::new(BodyKind::Query, &b.name, b.namespace.clone(), "<panel>");
    env.map = w.s.active_map();
    let args = call.find('(').and_then(|i| call.rfind(')').filter(|j| *j > i).map(|j| &call[i + 1..j]));
    if let (Some(params), Some(args)) = (&b.params, args) {
        for (p, src) in params.iter().zip(split_top_level(args, ',')) {
            let v = rlml_core::rs::parse::parse_expr(src.trim()).map_err(|d| d.message).and_then(|e| w.eval(&e, &mut env.clone()));
            env.locals.insert(p.name.clone(), v.unwrap_or(Value::None));
        }
    }
    for pos in &detected.fields {
        let Some(f) = fields.iter().find(|f| f.ch.to_string() == pos.name) else { continue };
        let texts = match &f.show {
            Show::Text(key) => vec![styled(w, lang, key, &[])],
            Show::Param(name) => env.locals.get(name).map(|v| shown(w, lang, v)).unwrap_or_default(),
            Show::Value(Err(e)) => vec![plain(e)],
            Show::Value(Ok(e)) => match w.eval(e, &mut env.clone()) {
                Ok(v) => shown(w, lang, &v),
                Err(e) => vec![plain(&e)],
            },
        };
        let rect = pos.rect;
        if rect.x >= r.w || rect.y >= r.h {
            continue;
        }
        let width = rect.w.min(r.w - rect.x);
        // a multi-line field wraps each member, and a member that fits
        // keeps its glyphs' looks; a one-line field shows the members side
        // by side
        let lines: Vec<Vec<Cell>> = if rect.h > 1 {
            texts.iter().flat_map(|t| if t.len() <= width { vec![t.clone()] } else { wrap(&t.iter().map(|c| c.0).collect::<String>(), width).iter().map(|l| plain(l)).collect() }).collect()
        } else {
            vec![texts.join(&[(',', None), (' ', None)][..])]
        };
        for (i, line) in lines.iter().take(rect.h.min(r.h - rect.y)).enumerate() {
            for (x, (ch, look)) in fitted(line, width, f.align).into_iter().enumerate() {
                let (sx, sy) = (r.x + rect.x + x, r.y + rect.y + i);
                match look {
                    Some(look) => crate::draw::put_look(frame, sx, sy, &Look { ch, ..look }, truecolor),
                    None => frame.put(sx, sy, ch, Role::Normal),
                }
            }
        }
    }
}
