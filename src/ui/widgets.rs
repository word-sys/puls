use ratatui::{
    prelude::*,
    widgets::{Block, Gauge, Paragraph},
};

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



#[cfg(test)]
mod tests {
    use super::*;

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