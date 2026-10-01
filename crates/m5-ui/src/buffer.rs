//! Cell buffer: the in-memory screen that widgets draw into (DESIGN 5.2.1).
//!
//! A [`Buffer`] is a `w x h` grid of [`Cell`]s. A cell holds one grapheme
//! cluster, its display width and a [`Style`]. A wide (two column) grapheme
//! occupies two cells: the first holds the text, the second is a
//! "continuation" with `width == 0` and empty text.
//!
//! Rendering the buffer to a terminal (`diff_render`) is not part of this module.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Terminal colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Color {
    /// The terminal's default colour.
    #[default]
    Default,
    /// Palette index: 0..=15 basic colours, 16..=255 the 256-colour palette.
    Indexed(u8),
    /// 24-bit colour.
    Rgb(u8, u8, u8),
}

/// Text attributes (bit set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Attrs(u8);

impl Attrs {
    pub const NONE: Attrs = Attrs(0);
    pub const BOLD: Attrs = Attrs(1);
    pub const UNDERLINE: Attrs = Attrs(2);
    pub const ITALIC: Attrs = Attrs(4);
    pub const REVERSE: Attrs = Attrs(8);
    pub const BLINK: Attrs = Attrs(16);

    /// True if every attribute in `other` is set in `self`.
    pub fn contains(self, other: Attrs) -> bool {
        self.0 & other.0 == other.0
    }

    /// True if no attribute is set.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for Attrs {
    type Output = Attrs;
    fn bitor(self, rhs: Attrs) -> Attrs {
        Attrs(self.0 | rhs.0)
    }
}

/// Foreground, background and attributes of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Attrs,
}

impl Style {
    pub fn new(fg: Color, bg: Color, attrs: Attrs) -> Style {
        Style { fg, bg, attrs }
    }
}

/// Rectangle in buffer coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, w: u16, h: u16) -> Rect {
        Rect { x, y, w, h }
    }
}

/// One screen cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// One grapheme cluster; empty for the continuation of a wide grapheme.
    pub text: String,
    /// Display width: 1, 2, or 0 for a continuation cell.
    pub width: u8,
    pub style: Style,
}

impl Cell {
    /// A space with the given style.
    pub fn blank(style: Style) -> Cell {
        Cell {
            text: " ".to_string(),
            width: 1,
            style,
        }
    }

    fn continuation(style: Style) -> Cell {
        Cell {
            text: String::new(),
            width: 0,
            style,
        }
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell::blank(Style::default())
    }
}

/// Grid of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    w: u16,
    h: u16,
    cells: Vec<Cell>,
}

impl Buffer {
    /// New buffer filled with default blank cells.
    pub fn new(w: u16, h: u16) -> Buffer {
        Buffer {
            w,
            h,
            cells: vec![Cell::default(); usize::from(w) * usize::from(h)],
        }
    }

    pub fn width(&self) -> u16 {
        self.w
    }

    pub fn height(&self) -> u16 {
        self.h
    }

    fn index(&self, x: u16, y: u16) -> usize {
        usize::from(y) * usize::from(self.w) + usize::from(x)
    }

    /// Cell at `(x, y)`, or `None` outside the buffer.
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        if x < self.w && y < self.h {
            Some(&self.cells[self.index(x, y)])
        } else {
            None
        }
    }

    /// Before overwriting `(x, y)`: if it is half of a wide grapheme,
    /// blank the other half so no orphan continuation or head remains.
    fn clear_overlap(&mut self, x: u16, y: u16) {
        let i = self.index(x, y);
        match self.cells[i].width {
            0 if x > 0 => {
                let style = self.cells[i - 1].style;
                self.cells[i - 1] = Cell::blank(style);
            }
            2 if x + 1 < self.w => {
                let style = self.cells[i + 1].style;
                self.cells[i + 1] = Cell::blank(style);
            }
            _ => {}
        }
    }

    /// Write one grapheme of display width `width` (1 or 2) at `(x, y)`.
    /// The caller guarantees that it fits inside the buffer.
    fn set_cell(&mut self, x: u16, y: u16, text: &str, width: u8, style: Style) {
        self.clear_overlap(x, y);
        if width == 2 {
            self.clear_overlap(x + 1, y);
        }
        let i = self.index(x, y);
        self.cells[i] = Cell {
            text: text.to_string(),
            width,
            style,
        };
        if width == 2 {
            self.cells[i + 1] = Cell::continuation(style);
        }
    }

    /// Draw `s` starting at `(x, y)` using at most `max_w` columns (and never
    /// beyond the right edge). Returns the number of columns used.
    ///
    /// Text is split into extended grapheme clusters. Control characters and
    /// zero-width clusters are skipped. A wide cluster that does not fit in the
    /// remaining columns is not drawn and drawing stops.
    pub fn put_str(&mut self, x: u16, y: u16, s: &str, style: Style, max_w: u16) -> u16 {
        if y >= self.h || x >= self.w {
            return 0;
        }
        let limit = max_w.min(self.w - x);
        let mut used: u16 = 0;
        for g in s.graphemes(true) {
            if g.chars().next().is_none_or(|c| c.is_control()) {
                continue;
            }
            let gw = UnicodeWidthStr::width(g).min(2) as u16;
            if gw == 0 {
                continue;
            }
            if used + gw > limit {
                break;
            }
            self.set_cell(x + used, y, g, gw as u8, style);
            used += gw;
        }
        used
    }

    /// Fill `rect` (clipped to the buffer) with `ch`. Control and zero-width
    /// characters are replaced by a space. A wide `ch` is repeated every two
    /// columns; a column left over at the right edge becomes a blank.
    pub fn fill(&mut self, rect: Rect, ch: char, style: Style) {
        let right = rect.x.saturating_add(rect.w).min(self.w);
        let bottom = rect.y.saturating_add(rect.h).min(self.h);
        let (ch, cw) = match UnicodeWidthChar::width(ch) {
            Some(1) => (ch, 1u16),
            Some(2) => (ch, 2u16),
            _ => (' ', 1u16),
        };
        let text = ch.to_string();
        for y in rect.y..bottom {
            let mut x = rect.x;
            while x < right {
                if x + cw <= right {
                    self.set_cell(x, y, &text, cw as u8, style);
                    x += cw;
                } else {
                    self.set_cell(x, y, " ", 1, style);
                    x += 1;
                }
            }
        }
    }

    /// Text of the buffer, one line per row, joined with `\n`. Continuation
    /// cells contribute nothing, so a wide character takes one `char` here.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for y in 0..self.h {
            if y > 0 {
                out.push('\n');
            }
            for x in 0..self.w {
                out.push_str(&self.cells[self.index(x, y)].text);
            }
        }
        out
    }

    /// Style map for snapshot tests, laid out like [`Buffer::to_text`]: `.` is
    /// the default style, other styles get letters `a`, `b`, ... in order of
    /// first appearance (row by row); more than 26 distinct styles map to `?`.
    pub fn to_style_map(&self) -> String {
        let mut seen: Vec<Style> = Vec::new();
        let mut out = String::new();
        for y in 0..self.h {
            if y > 0 {
                out.push('\n');
            }
            for x in 0..self.w {
                let cell = &self.cells[self.index(x, y)];
                if cell.width == 0 {
                    continue;
                }
                if cell.style == Style::default() {
                    out.push('.');
                    continue;
                }
                let n = match seen.iter().position(|s| *s == cell.style) {
                    Some(n) => n,
                    None => {
                        seen.push(cell.style);
                        seen.len() - 1
                    }
                };
                out.push(if n < 26 {
                    char::from(b'a' + n as u8)
                } else {
                    '?'
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> Style {
        Style::new(Color::Indexed(1), Color::Default, Attrs::BOLD)
    }

    fn blue() -> Style {
        Style::new(Color::Rgb(0, 0, 255), Color::Indexed(7), Attrs::NONE)
    }

    #[test]
    fn new_buffer_is_blank() {
        let b = Buffer::new(3, 2);
        assert_eq!(b.to_text(), "   \n   ");
        assert_eq!(b.to_style_map(), "...\n...");
        assert_eq!((b.width(), b.height()), (3, 2));
        assert!(b.cell(3, 0).is_none());
        assert!(b.cell(0, 2).is_none());
    }

    #[test]
    fn put_str_ascii_returns_columns() {
        let mut b = Buffer::new(8, 1);
        assert_eq!(b.put_str(2, 0, "abc", red(), 100), 3);
        assert_eq!(b.to_text(), "  abc   ");
        assert_eq!(b.to_style_map(), "..aaa...");
    }

    #[test]
    fn put_str_respects_max_w_and_edge() {
        let mut b = Buffer::new(5, 1);
        assert_eq!(b.put_str(0, 0, "abcdef", Style::default(), 3), 3);
        assert_eq!(b.to_text(), "abc  ");
        assert_eq!(b.put_str(3, 0, "xyz", Style::default(), 100), 2);
        assert_eq!(b.to_text(), "abcxy");
    }

    #[test]
    fn put_str_outside_buffer_does_nothing() {
        let mut b = Buffer::new(2, 2);
        assert_eq!(b.put_str(2, 0, "a", red(), 5), 0);
        assert_eq!(b.put_str(0, 2, "a", red(), 5), 0);
        assert_eq!(b.to_text(), "  \n  ");
    }

    #[test]
    fn wide_char_takes_two_cells() {
        let mut b = Buffer::new(4, 1);
        assert_eq!(b.put_str(0, 0, "a漢b", red(), 10), 4);
        assert_eq!(b.cell(1, 0).unwrap().width, 2);
        assert_eq!(b.cell(2, 0).unwrap().width, 0);
        assert!(b.cell(2, 0).unwrap().text.is_empty());
        assert_eq!(b.to_text(), "a漢b");
        assert_eq!(b.to_style_map(), "aaa");
    }

    #[test]
    fn wide_char_that_does_not_fit_stops_drawing() {
        let mut b = Buffer::new(3, 1);
        assert_eq!(b.put_str(0, 0, "ab漢c", Style::default(), 10), 2);
        assert_eq!(b.to_text(), "ab ");
    }

    #[test]
    fn combining_mark_stays_in_one_cell() {
        let mut b = Buffer::new(3, 1);
        assert_eq!(b.put_str(0, 0, "e\u{301}x", Style::default(), 10), 2);
        assert_eq!(b.cell(0, 0).unwrap().text, "e\u{301}");
        assert_eq!(b.cell(1, 0).unwrap().text, "x");
    }

    #[test]
    fn control_chars_are_skipped() {
        let mut b = Buffer::new(4, 1);
        assert_eq!(b.put_str(0, 0, "a\tb\nc", Style::default(), 10), 3);
        assert_eq!(b.to_text(), "abc ");
    }

    #[test]
    fn overwriting_half_of_wide_char_blanks_the_other_half() {
        let mut b = Buffer::new(4, 1);
        b.put_str(0, 0, "漢字", red(), 10);
        // overwrite the continuation of the first character
        b.put_str(1, 0, "x", blue(), 1);
        assert_eq!(b.to_text(), " x字");
        // overwrite the head of the second character
        b.put_str(2, 0, "y", blue(), 1);
        assert_eq!(b.to_text(), " xy ");
        assert!(b.cells.iter().all(|c| c.width == 1));
    }

    #[test]
    fn wide_char_over_two_wide_chars_leaves_no_orphans() {
        let mut b = Buffer::new(4, 1);
        b.put_str(0, 0, "漢字", red(), 10);
        b.put_str(1, 0, "月", blue(), 10);
        assert_eq!(b.to_text(), " 月 ");
        assert_eq!(
            b.cells.iter().map(|c| c.width).collect::<Vec<_>>(),
            vec![1, 2, 0, 1]
        );
    }

    #[test]
    fn fill_clips_to_buffer() {
        let mut b = Buffer::new(4, 3);
        b.fill(Rect::new(2, 1, 10, 10), '#', red());
        assert_eq!(b.to_text(), "    \n  ##\n  ##");
        assert_eq!(b.to_style_map(), "....\n..aa\n..aa");
    }

    #[test]
    fn fill_with_wide_char_blanks_leftover_column() {
        let mut b = Buffer::new(5, 1);
        b.fill(Rect::new(0, 0, 5, 1), '漢', red());
        assert_eq!(b.to_text(), "漢漢 ");
    }

    #[test]
    fn fill_with_control_char_uses_space() {
        let mut b = Buffer::new(2, 1);
        b.put_str(0, 0, "ab", Style::default(), 2);
        b.fill(Rect::new(0, 0, 2, 1), '\u{7}', blue());
        assert_eq!(b.to_text(), "  ");
        assert_eq!(b.to_style_map(), "aa");
    }

    #[test]
    fn style_map_letters_follow_first_appearance() {
        let mut b = Buffer::new(6, 2);
        b.put_str(0, 0, "ab", blue(), 2);
        b.put_str(2, 0, "cd", red(), 2);
        b.put_str(0, 1, "ef", red(), 2);
        b.put_str(2, 1, "gh", blue(), 2);
        assert_eq!(b.to_style_map(), "aabb..\nbbaa..");
    }

    #[test]
    fn attrs_bits() {
        let a = Attrs::BOLD | Attrs::UNDERLINE;
        assert!(a.contains(Attrs::BOLD));
        assert!(a.contains(Attrs::UNDERLINE));
        assert!(!a.contains(Attrs::ITALIC));
        assert!(Attrs::NONE.is_empty());
        assert!(!a.is_empty());
    }
}
