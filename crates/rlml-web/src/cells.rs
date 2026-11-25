//! Frames as the page draws them: the cells that changed since the last
//! frame shown, each with its character, attributes and colors resolved
//! as the terminal resolves them (the role's look, the area's style, the
//! cell's own style over them, reverse video swapped).
//!
//! The encoding is a flat `u32` array, so it goes to the page without
//! copying: `[width, height, full, then per cell: index, char | flags,
//! fg, bg]`, the colors as `0xRRGGBB`. `full` is 1 when every cell is
//! there (a first frame or a new size). A character 0 draws only its
//! background.

use termplate_core::frame::{Cell, Frame, WIDE_CONT};
use termplate_core::style::{Color, StyleSheet};

/// The window's own colors where a style sets none.
pub const FG: u32 = 0xc8c8c8;
pub const BG: u32 = 0x000000;

pub const BOLD: u32 = 1 << 21;
pub const DIM: u32 = 1 << 22;
pub const ITALIC: u32 = 1 << 23;
pub const UNDERLINE: u32 = 1 << 24;
/// A two-column character: it covers the next cell too.
pub const WIDE: u32 = 1 << 25;
/// The character's bits under the flags.
pub const CHAR: u32 = (1 << 21) - 1;

/// The xterm colors of the 16 named ones, in palette order.
const BASE16: [u32; 16] = [
    0x000000, 0xcd0000, 0x00cd00, 0xcdcd00, 0x0000ee, 0xcd00cd, 0x00cdcd, 0xe5e5e5, 0x7f7f7f,
    0xff0000, 0x00ff00, 0xffff00, 0x5c5cff, 0xff00ff, 0x00ffff, 0xffffff,
];

/// A style color as `0xRRGGBB`; `None` for the window's own.
pub fn rgb(c: Color) -> Option<u32> {
    let n = match c {
        Color::Reset => return None,
        Color::Rgb(r, g, b) => return Some(((r as u32) << 16) | ((g as u32) << 8) | b as u32),
        Color::Ansi(n) => n,
        Color::Black => 0,
        Color::DarkRed => 1,
        Color::DarkGreen => 2,
        Color::DarkYellow => 3,
        Color::DarkBlue => 4,
        Color::DarkMagenta => 5,
        Color::DarkCyan => 6,
        Color::Grey => 7,
        Color::DarkGrey => 8,
        Color::Red => 9,
        Color::Green => 10,
        Color::Yellow => 11,
        Color::Blue => 12,
        Color::Magenta => 13,
        Color::Cyan => 14,
        Color::White => 15,
    };
    Some(ansi(n))
}

/// The 256-color palette: the 16 named colors, the 6×6×6 cube and the
/// grey ramp.
pub fn ansi(n: u8) -> u32 {
    const STEPS: [u32; 6] = [0, 95, 135, 175, 215, 255];
    match n {
        0..=15 => BASE16[n as usize],
        16..=231 => {
            let i = n as u32 - 16;
            (STEPS[(i / 36) as usize] << 16)
                | (STEPS[(i / 6 % 6) as usize] << 8)
                | STEPS[(i % 6) as usize]
        }
        _ => {
            let v = 8 + 10 * (n as u32 - 232);
            (v << 16) | (v << 8) | v
        }
    }
}

/// A cell's character with its flags, and its colors.
pub fn look(sheet: &StyleSheet, frame: &Frame, i: usize) -> [u32; 3] {
    let cell = &frame.cells[i];
    let mut style = sheet.resolve(frame.area_of(cell), cell.role);
    if let Some(own) = cell.style {
        style = style.over(own);
    }
    let mut fg = style.fg.and_then(rgb).unwrap_or(FG);
    let mut bg = style.bg.and_then(rgb).unwrap_or(BG);
    if style.attrs.reverse {
        std::mem::swap(&mut fg, &mut bg);
    }
    let a = style.attrs;
    let mut ch = cell.ch as u32;
    for (on, flag) in [
        (a.bold, BOLD),
        (a.dim, DIM),
        (a.italic, ITALIC),
        (a.underline, UNDERLINE),
    ] {
        if on {
            ch |= flag;
        }
    }
    if frame.cells.get(i + 1).is_some_and(|c| c.ch == WIDE_CONT)
        && !(i + 1).is_multiple_of(frame.width)
    {
        ch |= WIDE;
    }
    [ch, fg, bg]
}

fn same(a: &Frame, b: &Frame, i: usize) -> bool {
    let (x, y): (&Cell, &Cell) = (&a.cells[i], &b.cells[i]);
    x.ch == y.ch && x.role == y.role && x.style == y.style && a.area_of(x) == b.area_of(y)
}

/// The cells of `frame` that differ from `prev`, encoded. A changed
/// second half of a wide character sends the character instead.
pub fn encode(sheet: &StyleSheet, prev: Option<&Frame>, frame: &Frame) -> Vec<u32> {
    let n = frame.width * frame.height;
    let prev = prev.filter(|p| p.width == frame.width && p.height == frame.height);
    let mut changed: Vec<bool> = (0..n)
        .map(|i| prev.is_none_or(|p| !same(p, frame, i)))
        .collect();
    for i in 1..n {
        if changed[i] && frame.cells[i].ch == WIDE_CONT && !i.is_multiple_of(frame.width) {
            changed[i - 1] = true;
        }
    }
    let mut out = vec![
        frame.width as u32,
        frame.height as u32,
        prev.is_none() as u32,
    ];
    // a second half is drawn with its character, unless it starts a row
    let second = |i: usize| frame.cells[i].ch == WIDE_CONT && !i.is_multiple_of(frame.width);
    for i in (0..n).filter(|&i| changed[i] && !second(i)) {
        out.push(i as u32);
        out.extend(look(sheet, frame, i));
    }
    out
}
