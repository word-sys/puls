use ratatui::{
    prelude::*,
    widgets::{Block, Gauge, Paragraph, Sparkline},
};

pub struct EnhancedSparkline<'a> {
    data: &'a [u64],
    style: Style,
    max_value: Option<u64>,
    show_baseline: bool,
}

impl<'a> EnhancedSparkline<'a> {
    pub fn new(data: &'a [u64]) -> Self {
        Self {
            data,
            style: Style::default(),
            max_value: None,
            show_baseline: false,
        }
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    pub fn max_value(mut self, max: u64) -> Self {
        self.max_value = Some(max);
        self
    }
    
    pub fn show_baseline(mut self, show: bool) -> Self {
        self.show_baseline = show;
        self
    }
    
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        if self.data.is_empty() || area.width == 0 || area.height == 0 {
            return;
        }
        
        let sparkline = Sparkline::default()
            .data(self.data)
            .style(self.style);
        
        sparkline.render(area, buf);
        
        if self.show_baseline && area.height > 0 {
            let baseline_y = area.y + area.height - 1;
            for x in area.x..area.x + area.width {
                buf.get_mut(x, baseline_y).set_char('─');
            }
        }
    }
}

pub struct MultiGauge<'a> {
    gauges: Vec<GaugeData<'a>>,
    block: Option<Block<'a>>,
    direction: Direction,
}

pub struct GaugeData<'a> {
    pub label: &'a str,
    pub value: f64,
    pub max_value: f64,
    pub style: Style,
    pub show_percentage: bool,
}

impl<'a> MultiGauge<'a> {
    pub fn new(gauges: Vec<GaugeData<'a>>) -> Self {
        Self {
            gauges,
            block: None,
            direction: Direction::Horizontal,
        }
    }
    
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
    
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }
}

impl<'a> Widget for MultiGauge<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = match self.block {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        
        if self.gauges.is_empty() || area.width == 0 || area.height == 0 {
            return;
        }
        
        let constraints: Vec<Constraint> = self.gauges.iter()
            .map(|_| Constraint::Ratio(1, self.gauges.len() as u32))
            .collect();
        
        let layout = Layout::default()
            .direction(self.direction)
            .constraints(constraints)
            .split(area);
        
        for (gauge_data, gauge_area) in self.gauges.iter().zip(layout.iter()) {
            let percentage = if gauge_data.max_value > 0.0 {
                ((gauge_data.value / gauge_data.max_value) * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };
            
            let label = if gauge_data.show_percentage {
                format!("{}: {:.1}%", gauge_data.label, percentage)
            } else {
                format!("{}: {:.2}", gauge_data.label, gauge_data.value)
            };
            
            let gauge = Gauge::default()
                .label(label)
                .gauge_style(gauge_data.style)
                .ratio(percentage / 100.0);
            
            gauge.render(*gauge_area, buf);
        }
    }
}

pub struct ProgressBar<'a> {
    progress: f64,
    label: Option<&'a str>,
    style: Style,
    background_style: Style,
    show_percentage: bool,
    custom_text: Option<&'a str>,
}

impl<'a> ProgressBar<'a> {
    pub fn new(progress: f64) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            label: None,
            style: Style::default().fg(Color::Green),
            background_style: Style::default().fg(Color::DarkGray),
            show_percentage: true,
            custom_text: None,
        }
    }
    
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    pub fn background_style(mut self, style: Style) -> Self {
        self.background_style = style;
        self
    }
    
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }
    
    pub fn custom_text(mut self, text: &'a str) -> Self {
        self.custom_text = Some(text);
        self
    }
}

impl<'a> Widget for ProgressBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        
        let fill_width = ((area.width as f64) * self.progress) as u16;
        
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.get_mut(x, y);
                if x < area.x + fill_width {
                    cell.set_style(self.style);
                    cell.set_char('█');
                } else {
                    cell.set_style(self.background_style);
                    cell.set_char('░');
                }
            }
        }
        
        if let Some(text) = self.custom_text {
            self.render_text_overlay(area, buf, text);
            return;
        }
        if self.show_percentage {
            let percentage_text = format!("{}%", (self.progress * 100.0) as u8);
            self.render_text_overlay(area, buf, &percentage_text);
            return;
        }
        
        if let Some(label) = self.label {
            self.render_text_overlay(area, buf, label);
        }
    }
}

impl<'a> ProgressBar<'a> {
    fn render_text_overlay(self, area: Rect, buf: &mut Buffer, text: &str) {
        if area.height == 0 {
            return;
        }
        
        let text_y = area.y + area.height / 2;
        let text_x = area.x + (area.width.saturating_sub(text.len() as u16)) / 2;
        
        for (i, ch) in text.chars().enumerate() {
            let x = text_x + i as u16;
            if x >= area.x + area.width {
                break;
            }
            
            let cell = buf.get_mut(x, text_y);
            cell.set_char(ch);
            if x < area.x + ((area.width as f64 * self.progress) as u16) {
                cell.set_fg(Color::Black);
            } else {
                cell.set_fg(Color::White);
            }
        }
    }
}

pub struct StatusIndicator<'a> {
    status: Status,
    label: &'a str,
    show_symbol: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Status {
    Good,
    Warning,
    Error,
    Unknown,
}

impl Status {
    pub fn color(self) -> Color {
        match self {
            Status::Good => Color::Green,
            Status::Warning => Color::Yellow,
            Status::Error => Color::Red,
            Status::Unknown => Color::Gray,
        }
    }
    
    pub fn symbol(self) -> &'static str {
        match self {
            Status::Good => "✓",
            Status::Warning => "⚠",
            Status::Error => "✗",
            Status::Unknown => "?",
        }
    }
    
    pub fn text(self) -> &'static str {
        match self {
            Status::Good => "OK",
            Status::Warning => "WARN",
            Status::Error => "ERROR",
            Status::Unknown => "UNKNOWN",
        }
    }
}

impl<'a> StatusIndicator<'a> {
    pub fn new(status: Status, label: &'a str) -> Self {
        Self {
            status,
            label,
            show_symbol: true,
        }
    }
    
    pub fn show_symbol(mut self, show: bool) -> Self {
        self.show_symbol = show;
        self
    }
}

impl<'a> Widget for StatusIndicator<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        
        let status_text = if self.show_symbol {
            format!("{} {}: {}", self.status.symbol(), self.label, self.status.text())
        } else {
            format!("{}: {}", self.label, self.status.text())
        };
        
        let paragraph = Paragraph::new(status_text)
            .style(Style::default().fg(self.status.color()));
        
        paragraph.render(area, buf);
    }
}

pub struct MiniChart<'a> {
    data: &'a [f64],
    style: Style,
    fill_char: char,
    empty_char: char,
    max_value: Option<f64>,
}

impl<'a> MiniChart<'a> {
    pub fn new(data: &'a [f64]) -> Self {
        Self {
            data,
            style: Style::default(),
            fill_char: '█',
            empty_char: ' ',
            max_value: None,
        }
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    pub fn fill_char(mut self, ch: char) -> Self {
        self.fill_char = ch;
        self
    }
    
    pub fn empty_char(mut self, ch: char) -> Self {
        self.empty_char = ch;
        self
    }
    
    pub fn max_value(mut self, max: f64) -> Self {
        self.max_value = Some(max);
        self
    }
}

impl<'a> Widget for MiniChart<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.data.is_empty() || area.width == 0 || area.height == 0 {
            return;
        }
        
        let max_val = self.max_value.unwrap_or_else(|| {
            self.data.iter().fold(0.0f64, |acc, &x| acc.max(x))
        });
        
        if max_val <= 0.0 {
            return;
        }
        
        let data_len = self.data.len();
        let width = area.width as usize;
        
        for x in 0..width.min(data_len) {
            let data_index = if data_len > width {
                (x * data_len) / width
            } else {
                x
            };
            
            let value = self.data.get(data_index).copied().unwrap_or(0.0);
            let normalized = (value / max_val).clamp(0.0, 1.0);
            let fill_height = (normalized * area.height as f64) as u16;
            
            for y in 0..area.height {
                let cell_y = area.y + area.height - 1 - y;
                let cell_x = area.x + x as u16;
                
                let cell = buf.get_mut(cell_x, cell_y);
                if y < fill_height {
                    cell.set_char(self.fill_char);
                    cell.set_style(self.style);
                } else {
                    cell.set_char(self.empty_char);
                }
            }
        }
    }
}

pub struct ScrollableText<'a> {
    lines: Vec<&'a str>,
    scroll_offset: usize,
    block: Option<Block<'a>>,
    style: Style,
    highlight_style: Option<Style>,
    highlight_line: Option<usize>,
}

impl<'a> ScrollableText<'a> {
    pub fn new(lines: Vec<&'a str>) -> Self {
        Self {
            lines,
            scroll_offset: 0,
            block: None,
            style: Style::default(),
            highlight_style: None,
            highlight_line: None,
        }
    }
    
    pub fn scroll_offset(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }
    
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
    
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
    
    pub fn highlight_line(mut self, line: usize, style: Style) -> Self {
        self.highlight_line = Some(line);
        self.highlight_style = Some(style);
        self
    }
}

impl<'a> Widget for ScrollableText<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text_area = match self.block {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        
        if self.lines.is_empty() || text_area.height == 0 {
            return;
        }
        
        let visible_lines = text_area.height as usize;
        let start_line = self.scroll_offset.min(self.lines.len().saturating_sub(1));
        let end_line = (start_line + visible_lines).min(self.lines.len());
        
        for (i, line_text) in self.lines[start_line..end_line].iter().enumerate() {
            let y = text_area.y + i as u16;
            let line_index = start_line + i;
            
            let line_style = if Some(line_index) == self.highlight_line {
                self.highlight_style.unwrap_or(self.style)
            } else {
                self.style
            };
            
            let max_width = text_area.width as usize;
            let display_text = if line_text.len() > max_width {
                &line_text[..max_width]
            } else {
                line_text
            };
            
            for (j, ch) in display_text.chars().enumerate() {
                let x = text_area.x + j as u16;
                if x >= text_area.x + text_area.width {
                    break;
                }
                
                let cell = buf.get_mut(x, y);
                cell.set_char(ch);
                cell.set_style(line_style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_colors() {
        assert_eq!(Status::Good.color(), Color::Green);
        assert_eq!(Status::Warning.color(), Color::Yellow);
        assert_eq!(Status::Error.color(), Color::Red);
        assert_eq!(Status::Unknown.color(), Color::Gray);
    }
    
    #[test]
    fn test_progress_bar_creation() {
        let bar = ProgressBar::new(0.5);
        assert!((bar.progress - 0.5).abs() < f64::EPSILON);
    }
    
    #[test]
    fn test_progress_bar_clamping() {
        let bar = ProgressBar::new(1.5);
        assert!((bar.progress - 1.0).abs() < f64::EPSILON);
        
        let bar = ProgressBar::new(-0.5);
        assert!(bar.progress.abs() < f64::EPSILON);
    }
}