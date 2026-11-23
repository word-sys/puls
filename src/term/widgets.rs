use crate::term::buffer::Buffer;
use crate::term::layout::{solve_constraints, Constraint, Rect};
use crate::term::style::{Modifier, Style};
use crate::term::text::{char_width, str_width, Alignment, Line, Span, Text};

pub trait Widget {
    fn render(self, area: Rect, buf: &mut Buffer);
}

pub trait StatefulWidget {
    type State;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State);
}

pub struct Clear;

impl Widget for Clear {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.clear_area(area);
        buf.set_style(area, Style::default().bg(crate::term::style::Color::Black));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Borders(u8);

impl Borders {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(0b1111);

    fn has_all(self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BorderType {
    #[default]
    Rounded,
}

pub mod block {
    use crate::term::text::{Alignment, Line, Span};

    #[derive(Clone, Debug)]
    pub struct Title<'a> {
        pub content: Line<'a>,
        pub alignment: Alignment,
    }

    impl<'a> Title<'a> {
        pub fn from<T: Into<Line<'a>>>(content: T) -> Self {
            Self {
                content: content.into(),
                alignment: Alignment::Left,
            }
        }

        pub fn alignment(mut self, alignment: Alignment) -> Self {
            self.alignment = alignment;
            self
        }
    }

    impl<'a> From<&'a str> for Title<'a> {
        fn from(s: &'a str) -> Self {
            Title::from(Line::from(s))
        }
    }

    impl<'a> From<String> for Title<'a> {
        fn from(s: String) -> Self {
            Title::from(Line::from(s))
        }
    }

    impl<'a> From<Span<'a>> for Title<'a> {
        fn from(s: Span<'a>) -> Self {
            Title::from(Line::from(s))
        }
    }

    impl<'a> From<Line<'a>> for Title<'a> {
        fn from(l: Line<'a>) -> Self {
            Title::from(l)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Block<'a> {
    titles: Vec<block::Title<'a>>,
    title_style: Style,
    borders: Borders,
    border_type: BorderType,
    border_style: Style,
    style: Style,
}

impl<'a> Block<'a> {
    pub fn title<T: Into<block::Title<'a>>>(mut self, title: T) -> Self {
        self.titles.push(title.into());
        self
    }

    pub fn title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    pub fn border_type(mut self, border_type: BorderType) -> Self {
        self.border_type = border_type;
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn inner(&self, area: Rect) -> Rect {
        if !self.borders.has_all() || area.width < 2 || area.height < 2 {
            return area;
        }
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    }
}

impl Widget for Block<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if self.style != Style::default() {
            buf.set_style(area, self.style);
        }
        if !self.borders.has_all() {
            return;
        }

        let bstyle = self.style.patch(self.border_style);
        let (tl, tr, bl, br, hz, vt) = match self.border_type {
            BorderType::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
        };

        let left = area.x;
        let right = area.right().saturating_sub(1);
        let top = area.y;
        let bottom = area.bottom().saturating_sub(1);

        if area.width >= 2 && area.height >= 2 {
            buf.set_char(left, top, tl, bstyle);
            buf.set_char(right, top, tr, bstyle);
            buf.set_char(left, bottom, bl, bstyle);
            buf.set_char(right, bottom, br, bstyle);
            for x in (left + 1)..right {
                buf.set_char(x, top, hz, bstyle);
                buf.set_char(x, bottom, hz, bstyle);
            }
            for y in (top + 1)..bottom {
                buf.set_char(left, y, vt, bstyle);
                buf.set_char(right, y, vt, bstyle);
            }
        }

        if area.width > 2 {
            let max_title_w = (area.width - 2) as usize;
            let base_tstyle = if self.title_style == Style::default() {
                bstyle.add_modifier(Modifier::BOLD)
            } else {
                self.style.patch(self.title_style)
            };
            for title in &self.titles {
                let tw = title.content.width().min(max_title_w);
                let tx = match title.alignment {
                    Alignment::Left => left + 1,
                    Alignment::Center => left + 1 + ((max_title_w.saturating_sub(tw)) / 2) as u16,
                    Alignment::Right => right.saturating_sub(tw as u16),
                };
                buf.set_line(tx, top, &title.content, max_title_w, base_tstyle);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Wrap {
    pub trim: bool,
}

#[derive(Clone, Debug)]
pub struct Paragraph<'a> {
    text: Text<'a>,
    block: Option<Block<'a>>,
    style: Style,
    alignment: Alignment,
    wrap: Option<Wrap>,
}

impl<'a> Paragraph<'a> {
    pub fn new<T: Into<Text<'a>>>(text: T) -> Self {
        Self {
            text: text.into(),
            block: None,
            style: Style::default(),
            alignment: Alignment::Left,
            wrap: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn wrap(mut self, wrap: Wrap) -> Self {
        self.wrap = Some(wrap);
        self
    }
}

impl Widget for Paragraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        if inner.width == 0 || inner.height == 0 {
            return;
        }
        if self.style != Style::default() {
            buf.set_style(inner, self.style);
        }

        let max_w = inner.width as usize;
        let rendered_lines: Vec<Vec<(String, Style)>> = if let Some(wrap_cfg) = self.wrap {
            wrap_text_lines(&self.text.lines, max_w, wrap_cfg.trim)
        } else {
            self.text
                .lines
                .iter()
                .map(|line| {
                    line.spans
                        .iter()
                        .map(|s| (s.content.to_string(), s.style))
                        .collect()
                })
                .collect()
        };

        for (row_idx, segs) in rendered_lines.iter().enumerate() {
            if row_idx >= inner.height as usize {
                break;
            }
            let y = inner.y + row_idx as u16;
            let line_w: usize = segs.iter().map(|(t, _)| str_width(t)).sum();
            let mut x = match self.alignment {
                Alignment::Left => inner.x,
                Alignment::Center => inner.x + (max_w.saturating_sub(line_w) / 2) as u16,
                Alignment::Right => inner.x + max_w.saturating_sub(line_w) as u16,
            };
            let right_limit = inner.right();
            for (text, style) in segs {
                if x >= right_limit {
                    break;
                }
                let avail = (right_limit - x) as usize;
                x = buf.set_stringn(x, y, text, avail, self.style.patch(*style));
            }
        }
    }
}

fn wrap_text_lines(lines: &[Line<'_>], max_width: usize, trim: bool) -> Vec<Vec<(String, Style)>> {
    let mut out = Vec::new();
    if max_width == 0 {
        return out;
    }

    for line in lines {
        if line.spans.is_empty() {
            out.push(Vec::new());
            continue;
        }
        let mut cur_line: Vec<(String, Style)> = Vec::new();
        let mut cur_w = 0usize;

        for span in &line.spans {
            let mut chunk = String::new();
            let mut chunk_w = 0usize;
            for ch in span.content.chars() {
                if ch == '\n' {
                    if !chunk.is_empty() {
                        cur_line.push((std::mem::take(&mut chunk), span.style));
                    }
                    out.push(std::mem::take(&mut cur_line));
                    cur_w = 0;
                    chunk_w = 0;
                    continue;
                }
                let cw = char_width(ch);
                if cur_w + chunk_w + cw > max_width && (cur_w + chunk_w) > 0 {
                    if !chunk.is_empty() {
                        cur_line.push((std::mem::take(&mut chunk), span.style));
                    }
                    out.push(std::mem::take(&mut cur_line));
                    cur_w = 0;
                    chunk_w = 0;
                    if trim && ch.is_whitespace() {
                        continue;
                    }
                }
                chunk.push(ch);
                chunk_w += cw;
            }
            if !chunk.is_empty() {
                cur_w += chunk_w;
                cur_line.push((chunk, span.style));
            }
        }
        out.push(cur_line);
    }

    out
}

#[derive(Clone, Debug, Default)]
pub struct Gauge<'a> {
    block: Option<Block<'a>>,
    gauge_style: Style,
    ratio: f64,
    label: Option<Span<'a>>,
}

impl<'a> Gauge<'a> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn gauge_style(mut self, style: Style) -> Self {
        self.gauge_style = style;
        self
    }

    pub fn percent(mut self, percent: u16) -> Self {
        self.ratio = (percent.min(100) as f64) / 100.0;
        self
    }

    pub fn ratio(mut self, ratio: f64) -> Self {
        self.ratio = ratio.clamp(0.0, 1.0);
        self
    }

    pub fn label<T: Into<Span<'a>>>(mut self, label: T) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Widget for Gauge<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let bar_color = self.gauge_style.fg.unwrap_or(crate::term::style::Color::Cyan);
        let unfilled_bg = self.gauge_style.bg.unwrap_or(crate::term::style::Color::Black);
        let track_style = Style::default()
            .fg(crate::term::style::Color::DarkGray)
            .bg(unfilled_bg);
        let bar_draw_style = Style::default()
            .fg(bar_color)
            .bg(unfilled_bg)
            .add_modifier(self.gauge_style.modifier);

        let fill_f = inner.width as f64 * self.ratio.clamp(0.0, 1.0);
        let filled_cols = fill_f.floor() as u16;
        let frac = fill_f - filled_cols as f64;
        let partial_ch = match (frac * 8.0) as u8 {
            1 => Some('▏'),
            2 => Some('▎'),
            3 => Some('▍'),
            4 => Some('▌'),
            5 => Some('▋'),
            6 => Some('▊'),
            7 => Some('▉'),
            _ => None,
        };

        let label_text = self
            .label
            .map(|s| s.content.into_owned())
            .unwrap_or_else(|| format!("{}%", (self.ratio * 100.0).round() as u16));
        let label_w = str_width(&label_text);
        let label_x = inner.x + (inner.width as usize).saturating_sub(label_w) as u16 / 2;

        let draw_bar_row = |buf: &mut Buffer, y: u16| {
            for dx in 0..inner.width {
                let x = inner.x + dx;
                if dx < filled_cols {
                    buf.set_char(x, y, '█', bar_draw_style);
                } else if dx == filled_cols {
                    if let Some(ch) = partial_ch {
                        buf.set_char(x, y, ch, bar_draw_style);
                    } else {
                        buf.set_char(x, y, '░', track_style);
                    }
                } else {
                    buf.set_char(x, y, '░', track_style);
                }
            }
        };

        if inner.height >= 2 {
            // 2+ rows: dedicated high-contrast label row above a sleek 1-row progress bar
            let top_y = inner.y + (inner.height - 2) / 2;
            let bar_y = top_y + 1;
            let label_style = Style::default()
                .fg(crate::term::style::Color::White)
                .bg(unfilled_bg)
                .add_modifier(Modifier::BOLD);
            buf.set_stringn(label_x, top_y, &label_text, inner.width as usize, label_style);
            draw_bar_row(buf, bar_y);
        } else {
            // 1-row compact gauge: draw bar + overlay high-contrast centered label
            let y = inner.y;
            draw_bar_row(buf, y);

            let filled_fg = match bar_color {
                crate::term::style::Color::White
                | crate::term::style::Color::Gray
                | crate::term::style::Color::LightYellow
                | crate::term::style::Color::LightGreen
                | crate::term::style::Color::LightCyan => crate::term::style::Color::Black,
                _ => crate::term::style::Color::White,
            };

            let mut cx = label_x;
            for ch in label_text.chars() {
                let w = char_width(ch);
                if w == 0 {
                    continue;
                }
                if cx >= inner.right() {
                    break;
                }
                let rel_x = cx.saturating_sub(inner.x);
                let is_filled = rel_x < filled_cols || (rel_x == filled_cols && frac >= 0.5);
                if let Some(cell) = buf.get_mut(cx, y) {
                    cell.reset();
                    cell.symbol.clear();
                    cell.symbol.push(ch);
                    cell.modifier = Modifier::BOLD;
                    if is_filled {
                        cell.fg = filled_fg;
                        cell.bg = bar_color;
                    } else {
                        cell.fg = crate::term::style::Color::White;
                        cell.bg = unfilled_bg;
                    }
                }
                cx = cx.saturating_add(w as u16);
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Sparkline<'a> {
    data: &'a [u64],
    style: Style,
}

impl<'a> Sparkline<'a> {
    pub fn data(mut self, data: &'a [u64]) -> Self {
        self.data = data;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Widget for Sparkline<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        const BARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let baseline_style = Style::default().fg(crate::term::style::Color::DarkGray);
        let bottom_y = area.bottom() - 1;
        for dx in 0..area.width {
            buf.set_char(area.x + dx, bottom_y, '▁', baseline_style);
        }
        if self.data.is_empty() {
            return;
        }
        let max_val = self.data.iter().copied().max().unwrap_or(1).max(1);
        let visible_len = self.data.len().min(area.width as usize);
        let start = self.data.len() - visible_len;
        let max_levels = area.height as usize * 8;

        for (i, &val) in self.data[start..].iter().enumerate() {
            let x = area.x + i as u16;
            let mut units = ((val as u128 * max_levels as u128) / max_val as u128) as usize;
            if val > 0 && units == 0 {
                units = 1;
            }
            for row_from_bottom in 0..area.height as usize {
                let y = bottom_y - row_from_bottom as u16;
                let level = units.min(8);
                units = units.saturating_sub(8);
                if level > 0 {
                    buf.set_char(x, y, BARS[level], self.style);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tabs<'a> {
    titles: Vec<Line<'a>>,
    selected: usize,
    block: Option<Block<'a>>,
    style: Style,
    highlight_style: Style,
    divider: Span<'a>,
}

impl<'a> Tabs<'a> {
    pub fn new<T: Into<Line<'a>>>(titles: Vec<T>) -> Self {
        Self {
            titles: titles.into_iter().map(Into::into).collect(),
            selected: 0,
            block: None,
            style: Style::default(),
            highlight_style: Style::default().add_modifier(Modifier::REVERSED),
            divider: Span::raw(" │ "),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn select(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn divider<T: Into<Span<'a>>>(mut self, divider: T) -> Self {
        self.divider = divider.into();
        self
    }
}

impl Widget for Tabs<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        buf.set_style(inner, self.style);

        let mut x = inner.x;
        let y = inner.y;
        let right = inner.right();
        let last_idx = self.titles.len().saturating_sub(1);

        for (i, title) in self.titles.iter().enumerate() {
            if x >= right {
                break;
            }
            let tab_start_x = x;
            x = buf.set_line(x, y, title, (right - x) as usize, self.style);
            if i == self.selected && x > tab_start_x {
                let mut hl = self.highlight_style;
                if hl.bg == Some(crate::term::style::Color::White) {
                    hl.bg = Some(crate::term::style::Color::DarkGray);
                }
                buf.set_style(Rect::new(tab_start_x, y, x - tab_start_x, 1), hl);
            }
            if i < last_idx && x < right {
                x = buf.set_span(x, y, &self.divider, (right - x) as usize, self.style);
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Cell<'a> {
    content: Text<'a>,
    style: Style,
}

impl<'a> Cell<'a> {
    pub fn from<T: Into<Text<'a>>>(content: T) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a, T: Into<Text<'a>>> From<T> for Cell<'a> {
    fn from(val: T) -> Self {
        Cell::from(val)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Row<'a> {
    cells: Vec<Cell<'a>>,
    style: Style,
    bottom_margin: u16,
}

impl<'a> Row<'a> {
    pub fn new<I>(cells: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cell<'a>>,
    {
        Self {
            cells: cells.into_iter().map(Into::into).collect(),
            style: Style::default(),
            bottom_margin: 0,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn bottom_margin(mut self, margin: u16) -> Self {
        self.bottom_margin = margin;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableState {
    selected: Option<usize>,
    offset: usize,
}

impl TableState {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if index.is_none() {
            self.offset = 0;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Table<'a> {
    rows: Vec<Row<'a>>,
    widths: Vec<Constraint>,
    header: Option<Row<'a>>,
    block: Option<Block<'a>>,
    highlight_style: Style,
    highlight_symbol: Option<&'a str>,
    column_spacing: u16,
}

impl<'a> Table<'a> {
    pub fn new<R, W>(rows: R, widths: W) -> Self
    where
        R: IntoIterator<Item = Row<'a>>,
        W: IntoIterator,
        W::Item: Into<Constraint>,
    {
        Self {
            rows: rows.into_iter().collect(),
            widths: widths.into_iter().map(Into::into).collect(),
            header: None,
            block: None,
            highlight_style: Style::default(),
            highlight_symbol: None,
            column_spacing: 1,
        }
    }

    pub fn header(mut self, header: Row<'a>) -> Self {
        self.header = Some(header);
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = Some(symbol);
        self
    }

    pub fn column_spacing(mut self, spacing: u16) -> Self {
        self.column_spacing = spacing;
        self
    }

    fn render_internal(self, area: Rect, buf: &mut Buffer, state: &mut TableState) {
        let inner = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let sym_str = if state.selected.is_some() {
            self.highlight_symbol.unwrap_or("")
        } else {
            ""
        };
        let sym_w = str_width(sym_str) as u16;
        let num_cols = self.widths.len();
        let total_spacing = self.column_spacing.saturating_mul(num_cols.saturating_sub(1) as u16);
        let col_avail = inner.width.saturating_sub(sym_w).saturating_sub(total_spacing);
        let col_widths = solve_constraints(&self.widths, col_avail);

        let mut y = inner.y;
        if let Some(ref header) = self.header {
            if y < inner.bottom() {
                render_table_row(buf, inner.x + sym_w, y, inner.right(), header, &col_widths, self.column_spacing, header.style);
                y = y.saturating_add(1);
                // Draw subtle horizontal column underline when bottom_margin >= 1 or space permits
                let draw_divider = header.bottom_margin >= 1 || inner.height >= 6;
                if draw_divider && y < inner.bottom() {
                    let div_style = Style::default().fg(crate::term::style::Color::DarkGray);
                    let mut cx = inner.x + sym_w;
                    for &w in &col_widths {
                        if cx >= inner.right() {
                            break;
                        }
                        let eff_w = w.min(inner.right() - cx);
                        for dx in 0..eff_w {
                            buf.set_char(cx + dx, y, '─', div_style);
                        }
                        cx = cx.saturating_add(w).saturating_add(self.column_spacing);
                    }
                    y = y.saturating_add(header.bottom_margin.max(1));
                }
            }
        }

        let avail_rows = inner.bottom().saturating_sub(y) as usize;
        if avail_rows == 0 || self.rows.is_empty() {
            return;
        }

        if let Some(sel) = state.selected {
            let sel = sel.min(self.rows.len() - 1);
            if sel < state.offset {
                state.offset = sel;
            } else if sel >= state.offset + avail_rows {
                state.offset = sel + 1 - avail_rows;
            }
        } else {
            state.offset = 0;
        }

        for (i, row) in self.rows.iter().enumerate().skip(state.offset) {
            if y >= inner.bottom() {
                break;
            }
            let is_selected = state.selected == Some(i);
            render_table_row(
                buf,
                inner.x + sym_w,
                y,
                inner.right(),
                row,
                &col_widths,
                self.column_spacing,
                row.style,
            );
            if is_selected {
                let mut hl = row.style.patch(self.highlight_style);
                if hl.bg == Some(crate::term::style::Color::White) {
                    hl.bg = Some(crate::term::style::Color::DarkGray);
                }
                if sym_w > 0 {
                    buf.set_stringn(inner.x, y, sym_str, sym_w as usize, hl);
                }
                buf.set_style(Rect::new(inner.x, y, inner.width, 1), hl);
            }
            y = y.saturating_add(1).saturating_add(row.bottom_margin);
        }
    }
}

fn render_table_row(
    buf: &mut Buffer,
    start_x: u16,
    y: u16,
    max_right: u16,
    row: &Row<'_>,
    col_widths: &[u16],
    spacing: u16,
    row_style: Style,
) {
    let mut x = start_x;
    for (col_idx, &w) in col_widths.iter().enumerate() {
        if x >= max_right {
            break;
        }
        let eff_w = w.min(max_right - x);
        if let Some(cell) = row.cells.get(col_idx) {
            let cell_style = row_style.patch(cell.style);
            if cell_style != Style::default() {
                buf.set_style(Rect::new(x, y, eff_w, 1), cell_style);
            }
            if let Some(first_line) = cell.content.lines.first() {
                buf.set_line(x, y, first_line, eff_w as usize, cell_style);
            }
        }
        x = x.saturating_add(w).saturating_add(spacing);
    }
}

impl Widget for Table<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut dummy = TableState::default();
        self.render_internal(area, buf, &mut dummy);
    }
}

impl StatefulWidget for Table<'_> {
    type State = TableState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.render_internal(area, buf, state);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Marker {
    #[default]
    Braille,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GraphType {
    #[default]
    Line,
}

#[derive(Clone, Debug, Default)]
pub struct Dataset<'a> {
    name: String,
    data: &'a [(f64, f64)],
    marker: Marker,
    graph_type: GraphType,
    style: Style,
}

impl<'a> Dataset<'a> {
    pub fn name<T: Into<String>>(mut self, name: T) -> Self {
        self.name = name.into();
        self
    }

    pub fn data(mut self, data: &'a [(f64, f64)]) -> Self {
        self.data = data;
        self
    }

    pub fn marker(mut self, marker: Marker) -> Self {
        self.marker = marker;
        self
    }

    pub fn graph_type(mut self, graph_type: GraphType) -> Self {
        self.graph_type = graph_type;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct Axis<'a> {
    bounds: [f64; 2],
    labels: Vec<Span<'a>>,
    style: Style,
}

impl<'a> Axis<'a> {
    pub fn bounds(mut self, bounds: [f64; 2]) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn labels(mut self, labels: Vec<Span<'a>>) -> Self {
        self.labels = labels;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

#[derive(Clone, Debug)]
pub struct Chart<'a> {
    datasets: Vec<Dataset<'a>>,
    block: Option<Block<'a>>,
    x_axis: Axis<'a>,
    y_axis: Axis<'a>,
}

impl<'a> Chart<'a> {
    pub fn new(datasets: Vec<Dataset<'a>>) -> Self {
        Self {
            datasets,
            block: None,
            x_axis: Axis::default().bounds([0.0, 1.0]),
            y_axis: Axis::default().bounds([0.0, 1.0]),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn x_axis(mut self, axis: Axis<'a>) -> Self {
        self.x_axis = axis;
        self
    }

    pub fn y_axis(mut self, axis: Axis<'a>) -> Self {
        self.y_axis = axis;
        self
    }
}

// Braille dot bitmask indexed by [row 0..4][col 0..2]
const BRAILLE_DOTS: [[u8; 2]; 4] = [
    [0x01, 0x08],
    [0x02, 0x10],
    [0x04, 0x20],
    [0x40, 0x80],
];

impl Widget for Chart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = if let Some(block) = self.block {
            let inner = block.inner(area);
            block.render(area, buf);
            inner
        } else {
            area
        };
        if inner.width < 3 || inner.height < 2 {
            return;
        }

        let y_label_w = self
            .y_axis
            .labels
            .iter()
            .map(Span::width)
            .max()
            .unwrap_or(0)
            .min(inner.width.saturating_sub(2) as usize) as u16;
        let has_y_axis = y_label_w > 0;
        let has_x_axis = !self.x_axis.labels.is_empty();
        let axis_x = inner.x + y_label_w;
        let bottom_y = inner.bottom() - 1;

        let plot_x = if has_y_axis { axis_x + 1 } else { inner.x };
        let plot_y = inner.y;
        let plot_w = inner.right().saturating_sub(plot_x) as usize;
        let plot_h = if has_x_axis {
            bottom_y.saturating_sub(plot_y) as usize
        } else {
            inner.height as usize
        };
        let plot_bottom = plot_y + plot_h.saturating_sub(1) as u16;

        let axis_style = if self.y_axis.style != Style::default() {
            self.y_axis.style
        } else {
            self.x_axis.style
        };

        // Draw Y-axis labels (right-aligned) & vertical rule with tick marks
        if has_y_axis {
            for y in inner.y..=plot_bottom {
                buf.set_char(axis_x, y, '│', axis_style);
            }
            let n_labels = self.y_axis.labels.len();
            for (idx, lbl) in self.y_axis.labels.iter().enumerate() {
                let ly = if n_labels <= 1 {
                    plot_bottom
                } else {
                    let frac = idx as f64 / (n_labels - 1) as f64;
                    let off = ((plot_bottom - inner.y) as f64 * (1.0 - frac)).round() as u16;
                    inner.y + off
                };
                let lw = lbl.width().min(y_label_w as usize);
                let lx = inner.x + y_label_w.saturating_sub(lw as u16);
                buf.set_span(lx, ly, lbl, lw, axis_style);
                buf.set_char(axis_x, ly, '┤', axis_style);
            }
            // Add midpoint 50% tick when 0%..100% bounds have vertical room
            if n_labels == 2 && plot_h >= 5 && (self.y_axis.bounds[1] - 100.0).abs() < 0.1 {
                let mid_y = inner.y + (plot_bottom - inner.y) / 2;
                if mid_y > inner.y && mid_y < plot_bottom {
                    let mid_lbl = "50%";
                    let lw = mid_lbl.len().min(y_label_w as usize);
                    let lx = inner.x + y_label_w.saturating_sub(lw as u16);
                    buf.set_stringn(lx, mid_y, mid_lbl, lw, axis_style);
                    buf.set_char(axis_x, mid_y, '┤', axis_style);
                }
            }
        }

        if has_x_axis {
            if has_y_axis {
                buf.set_char(axis_x, bottom_y, '└', axis_style);
            }
            for x in plot_x..inner.right() {
                buf.set_char(x, bottom_y, '─', axis_style);
            }
        }

        if plot_w == 0 || plot_h == 0 {
            return;
        }

        let [x_min, mut x_max] = self.x_axis.bounds;
        if (x_max - 60.0).abs() < 0.01 {
            x_max = 59.0;
        }
        let [y_min, y_max] = self.y_axis.bounds;
        let x_span = (x_max - x_min).abs().max(f64::EPSILON);
        let y_span = (y_max - y_min).abs().max(f64::EPSILON);

        let sub_w = plot_w * 2;
        let sub_h = plot_h * 4;

        let map_pt = |px: f64, py: f64| -> (i32, i32) {
            let nx = ((px - x_min) / x_span).clamp(0.0, 1.0);
            let ny = ((py - y_min) / y_span).clamp(0.0, 1.0);
            let sx = (nx * (sub_w.saturating_sub(1)) as f64).round() as i32;
            let sy = ((1.0 - ny) * (sub_h.saturating_sub(1)) as f64).round() as i32;
            (sx, sy)
        };

        for ds in &self.datasets {
            let _ = (ds.marker, ds.graph_type);
            if ds.data.is_empty() {
                continue;
            }
            let mut grid = vec![0u8; plot_w * plot_h];
            let mut plot_dot = |sx: i32, sy: i32| {
                if sx >= 0 && sy >= 0 && (sx as usize) < sub_w && (sy as usize) < sub_h {
                    let cx = (sx as usize) / 2;
                    let cy = (sy as usize) / 4;
                    let dx = (sx as usize) % 2;
                    let dy = (sy as usize) % 4;
                    grid[cy * plot_w + cx] |= BRAILLE_DOTS[dy][dx];
                }
            };

            if ds.data.len() == 1 {
                let (sx, sy) = map_pt(ds.data[0].0, ds.data[0].1);
                plot_dot(sx, sy);
            } else {
                for pair in ds.data.windows(2) {
                    let (mut x0, mut y0) = map_pt(pair[0].0, pair[0].1);
                    let (x1, y1) = map_pt(pair[1].0, pair[1].1);
                    let dx = (x1 - x0).abs();
                    let sx_step = if x0 < x1 { 1 } else { -1 };
                    let dy = -(y1 - y0).abs();
                    let sy_step = if y0 < y1 { 1 } else { -1 };
                    let mut err = dx + dy;
                    loop {
                        plot_dot(x0, y0);
                        if x0 == x1 && y0 == y1 {
                            break;
                        }
                        let e2 = 2 * err;
                        if e2 >= dy {
                            err += dy;
                            x0 += sx_step;
                        }
                        if e2 <= dx {
                            err += dx;
                            y0 += sy_step;
                        }
                    }
                }
            }

            let dot_style = ds.style.add_modifier(Modifier::BOLD);
            for cy in 0..plot_h {
                for cx in 0..plot_w {
                    let mask = grid[cy * plot_w + cx];
                    if mask != 0 {
                        let ch = char::from_u32(0x2800 + mask as u32).unwrap_or('⣿');
                        buf.set_char(plot_x + cx as u16, plot_y + cy as u16, ch, dot_style);
                    }
                }
            }
        }

        // Compact legend badge in top-right corner when space allows
        if plot_w >= 14 && plot_h >= 2 {
            let mut ly = plot_y;
            for ds in &self.datasets {
                if ds.name.is_empty() || ly > plot_bottom {
                    continue;
                }
                let badge = format!(" ━ {} ", ds.name);
                let nw = str_width(&badge).min(plot_w);
                let lx = inner.right().saturating_sub(nw as u16);
                buf.set_stringn(lx, ly, &badge, nw, ds.style.add_modifier(Modifier::BOLD));
                ly += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::style::Color;

    #[test]
    fn test_block_and_paragraph_rendering() {
        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        let p = Paragraph::new("Hello PULS")
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Title ")
                    .title(block::Title::from(" Right ").alignment(Alignment::Right))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
        p.render(area, &mut buf);

        assert_eq!(buf.cells[0].symbol, "╭");
        assert_eq!(buf.cells[19].symbol, "╮");
        let top_row: String = buf.cells[0..20].iter().map(|c| c.symbol.as_str()).collect();
        assert!(top_row.contains("Title"));
        assert!(top_row.contains("Right"));
        let mid_row: String = buf.cells[20..40].iter().map(|c| c.symbol.as_str()).collect();
        assert!(mid_row.contains("Hello PULS"));
    }

    #[test]
    fn test_gauge_sparkline_and_braille_chart() {
        let area = Rect::new(0, 0, 30, 8);
        let mut buf = Buffer::empty(area);

        let gauge = Gauge::default()
            .percent(50)
            .gauge_style(Style::default().fg(Color::Green))
            .label("50%");
        gauge.render(Rect::new(0, 0, 20, 1), &mut buf);
        let row0: String = buf.cells[0..20].iter().map(|c| c.symbol.as_str()).collect();
        assert!(row0.contains("50%"));
        assert!(row0.contains('█'));

        let spark = Sparkline::default()
            .data(&[0, 25, 50, 100])
            .style(Style::default().fg(Color::Cyan));
        spark.render(Rect::new(0, 1, 4, 1), &mut buf);
        assert_eq!(buf.cells[30 + 3].symbol, "█");

        let pts = [(0.0, 0.0), (30.0, 50.0), (60.0, 100.0)];
        let chart = Chart::new(vec![Dataset::default()
            .name("CPU")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&pts)])
        .x_axis(Axis::default().bounds([0.0, 60.0]))
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(vec![Span::raw("0%"), Span::raw("100%")]),
        );
        chart.render(Rect::new(0, 2, 30, 6), &mut buf);
        let has_braille = buf
            .cells
            .iter()
            .any(|c| c.symbol.chars().next().is_some_and(|ch| ('\u{2800}'..='\u{28FF}').contains(&ch)));
        assert!(has_braille, "Braille chart should emit Unicode Braille cells");
    }

    #[test]
    fn test_table_stateful_scroll_and_highlight() {
        let area = Rect::new(0, 0, 24, 4);
        let mut buf = Buffer::empty(area);
        let rows: Vec<Row> = (0..10).map(|i| Row::new(vec![format!("row-{}", i)])).collect();
        let table = Table::new(rows, [Constraint::Min(10)])
            .header(Row::new(vec!["Name"]))
            .highlight_symbol(">> ")
            .highlight_style(Style::default().fg(Color::Yellow));
        let mut state = TableState::default();
        state.select(Some(7));
        StatefulWidget::render(table, area, &mut buf, &mut state);

        let all_text: String = buf.cells.iter().map(|c| c.symbol.as_str()).collect();
        assert!(all_text.contains("Name"));
        assert!(all_text.contains(">> row-7"));
    }
}
