use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use crate::term::layout::Rect;
use crate::term::style::{Color, Modifier, Style};
use crate::term::sys;
use crate::term::text::{char_width, Line, Span};
use crate::term::widgets::{StatefulWidget, Widget};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferCell {
    pub symbol: String,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    pub skip: bool,
}

impl Default for BufferCell {
    fn default() -> Self {
        Self {
            symbol: " ".to_string(),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::NONE,
            skip: false,
        }
    }
}

impl BufferCell {
    pub fn reset(&mut self) {
        self.symbol.clear();
        self.symbol.push(' ');
        self.fg = Color::Reset;
        self.bg = Color::Reset;
        self.modifier = Modifier::NONE;
        self.skip = false;
    }

    pub fn set_style(&mut self, style: Style) {
        if let Some(fg) = style.fg {
            self.fg = fg;
        }
        if let Some(bg) = style.bg {
            self.bg = bg;
        }
        self.modifier = self.modifier | style.modifier;
    }
}

#[derive(Clone, Debug)]
pub struct Buffer {
    pub area: Rect,
    pub cells: Vec<BufferCell>,
}

impl Buffer {
    pub fn empty(area: Rect) -> Self {
        let len = area.width as usize * area.height as usize;
        Self {
            area,
            cells: vec![BufferCell::default(); len],
        }
    }

    pub fn reset(&mut self) {
        for cell in &mut self.cells {
            cell.reset();
        }
    }

    pub fn resize(&mut self, area: Rect) {
        self.area = area;
        let len = area.width as usize * area.height as usize;
        self.cells.clear();
        self.cells.resize_with(len, BufferCell::default);
    }

    fn index_of(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.area.x
            || x >= self.area.right()
            || y < self.area.y
            || y >= self.area.bottom()
        {
            return None;
        }
        let lx = (x - self.area.x) as usize;
        let ly = (y - self.area.y) as usize;
        Some(ly * self.area.width as usize + lx)
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut BufferCell> {
        let idx = self.index_of(x, y)?;
        self.cells.get_mut(idx)
    }

    pub fn set_char(&mut self, x: u16, y: u16, ch: char, style: Style) {
        if let Some(cell) = self.get_mut(x, y) {
            cell.symbol.clear();
            cell.symbol.push(ch);
            cell.skip = false;
            cell.set_style(style);
        }
    }

    pub fn set_stringn(&mut self, mut x: u16, y: u16, s: &str, max_width: usize, style: Style) -> u16 {
        if y < self.area.y || y >= self.area.bottom() || max_width == 0 {
            return x;
        }
        let mut remaining = max_width;
        for ch in s.chars() {
            let w = char_width(ch);
            if w == 0 {
                continue;
            }
            if w > remaining || x >= self.area.right() {
                break;
            }
            if let Some(cell) = self.get_mut(x, y) {
                cell.symbol.clear();
                cell.symbol.push(ch);
                cell.skip = false;
                cell.set_style(style);
            }
            if w == 2 && x + 1 < self.area.right() {
                if let Some(next_cell) = self.get_mut(x + 1, y) {
                    next_cell.symbol.clear();
                    next_cell.skip = true;
                    next_cell.set_style(style);
                }
            }
            x = x.saturating_add(w as u16);
            remaining -= w;
        }
        x
    }

    pub fn set_span(&mut self, x: u16, y: u16, span: &Span<'_>, max_width: usize, base_style: Style) -> u16 {
        self.set_stringn(x, y, &span.content, max_width, base_style.patch(span.style))
    }

    pub fn set_line(&mut self, mut x: u16, y: u16, line: &Line<'_>, max_width: usize, base_style: Style) -> u16 {
        let start_x = x;
        for span in &line.spans {
            let used = (x.saturating_sub(start_x)) as usize;
            if used >= max_width {
                break;
            }
            x = self.set_span(x, y, span, max_width - used, base_style);
        }
        x
    }

    pub fn set_style(&mut self, area: Rect, style: Style) {
        let x0 = area.x.max(self.area.x);
        let y0 = area.y.max(self.area.y);
        let x1 = area.right().min(self.area.right());
        let y1 = area.bottom().min(self.area.bottom());
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some(cell) = self.get_mut(x, y) {
                    cell.set_style(style);
                }
            }
        }
    }

    pub fn clear_area(&mut self, area: Rect) {
        let x0 = area.x.max(self.area.x);
        let y0 = area.y.max(self.area.y);
        let x1 = area.right().min(self.area.right());
        let y1 = area.bottom().min(self.area.bottom());
        for y in y0..y1 {
            for x in x0..x1 {
                if let Some(cell) = self.get_mut(x, y) {
                    cell.reset();
                }
            }
        }
    }
}

pub struct Frame<'a> {
    buffer: &'a mut Buffer,
}

impl<'a> Frame<'a> {
    pub fn size(&self) -> Rect {
        self.buffer.area
    }

    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, self.buffer);
    }

    pub fn render_stateful_widget<W: StatefulWidget>(
        &mut self,
        widget: W,
        area: Rect,
        state: &mut W::State,
    ) {
        widget.render(area, self.buffer, state);
    }
}

pub struct Terminal<W: Write> {
    writer: W,
    current: Buffer,
    previous: Buffer,
    force_full_redraw: bool,
    out_buf: String,
}

impl<W: Write> Terminal<W> {
    pub fn new(writer: W) -> io::Result<Self> {
        let (w, h) = sys::terminal_size().unwrap_or((80, 24));
        let area = Rect::new(0, 0, w, h);
        Ok(Self {
            writer,
            current: Buffer::empty(area),
            previous: Buffer::empty(area),
            force_full_redraw: true,
            out_buf: String::with_capacity(32 * 1024),
        })
    }

    pub fn draw<F>(&mut self, render_fn: F) -> io::Result<()>
    where
        F: FnOnce(&mut Frame<'_>),
    {
        if let Ok((w, h)) = sys::terminal_size() {
            if w != self.current.area.width || h != self.current.area.height {
                let area = Rect::new(0, 0, w, h);
                self.current.resize(area);
                self.previous.resize(area);
                self.force_full_redraw = true;
            }
        }

        self.current.reset();
        {
            let mut frame = Frame {
                buffer: &mut self.current,
            };
            render_fn(&mut frame);
        }

        self.out_buf.clear();
        // Synchronized output markers prevent mid-frame tearing in modern terminals
        self.out_buf.push_str("\x1b[?2026h");
        if self.force_full_redraw {
            self.out_buf.push_str("\x1b[0m\x1b[2J");
        }
        emit_diff(&self.previous, &self.current, self.force_full_redraw, &mut self.out_buf);
        self.out_buf.push_str("\x1b[0m\x1b[?2026l");

        self.writer.write_all(self.out_buf.as_bytes())?;
        self.writer.flush()?;

        std::mem::swap(&mut self.current, &mut self.previous);
        self.force_full_redraw = false;
        Ok(())
    }

    pub fn backend_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    pub fn show_cursor(&mut self) -> io::Result<()> {
        sys::show_cursor(&mut self.writer)
    }
}

pub fn emit_diff(prev: &Buffer, curr: &Buffer, full: bool, out: &mut String) {
    let w = curr.area.width as usize;
    let h = curr.area.height as usize;

    let mut last_x: Option<usize> = None;
    let mut last_y: Option<usize> = None;
    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;
    let mut cur_mod = Modifier::NONE;
    let mut style_initialized = false;

    for y in 0..h {
        let row_off = y * w;
        for x in 0..w {
            let idx = row_off + x;
            let cell = &curr.cells[idx];
            if cell.skip {
                continue;
            }
            if !full && prev.cells.get(idx) == Some(cell) {
                continue;
            }

            if last_y != Some(y) || last_x != Some(x) {
                let _ = write!(out, "\x1b[{};{}H", y + 1, x + 1);
            }

            if !style_initialized || cell.modifier != cur_mod {
                out.push_str("\x1b[0m");
                if cell.modifier.contains(Modifier::BOLD) {
                    out.push_str("\x1b[1m");
                }
                if cell.modifier.contains(Modifier::REVERSED) {
                    out.push_str("\x1b[7m");
                }
                cur_mod = cell.modifier;
                cur_fg = Color::Reset;
                cur_bg = Color::Reset;
                style_initialized = true;
            }

            if cell.fg != cur_fg {
                cell.fg.write_fg_ansi(out);
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                cell.bg.write_bg_ansi(out);
                cur_bg = cell.bg;
            }

            out.push_str(&cell.symbol);
            let adv = cell.symbol.chars().next().map(char_width).unwrap_or(1).max(1);
            last_x = Some(x + adv);
            last_y = Some(y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_diff_emits_only_changed_cells() {
        let area = Rect::new(0, 0, 10, 2);
        let prev = Buffer::empty(area);
        let mut curr = Buffer::empty(area);
        curr.set_stringn(2, 1, "HI", 5, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));

        let mut out = String::new();
        emit_diff(&prev, &curr, false, &mut out);
        assert!(out.contains("\x1b[2;3H"));
        assert!(out.contains("\x1b[1m"));
        assert!(out.contains("\x1b[32m"));
        assert!(out.contains("HI"));
        // Unchanged cells should not trigger extra cursor jumps
        assert_eq!(out.matches("\x1b[").count(), 4); // move + reset + bold + fg
    }
}
