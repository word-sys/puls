#![allow(dead_code)]

use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    pub background: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub border: Color,
    pub highlight: Color,
}

impl ColorScheme {
    pub fn nord() -> Self {
        Self {
            primary: Color::Cyan, 
            secondary: Color::Blue,
            accent: Color::Magenta,
            background: Color::Reset,
            text: Color::White,
            text_secondary: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            border: Color::White,
            highlight: Color::LightCyan,
        }
    }

    pub fn dark() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            accent: Color::Magenta,
            background: Color::Reset,
            text: Color::White,
            text_secondary: Color::Gray,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            border: Color::DarkGray,
            highlight: Color::Cyan,
        }
    }
    
    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::DarkGray,
            accent: Color::Magenta,
            background: Color::White,
            text: Color::Black,
            text_secondary: Color::DarkGray,
            success: Color::Green,
            warning: Color::Rgb(255, 165, 0),
            error: Color::Red,
            info: Color::Blue,
            border: Color::DarkGray,
            highlight: Color::Blue,
        }
    }
}

pub fn cpu_usage_color(usage: f32) -> Color {
    match usage {
        x if x >= 85.0 => Color::Red,
        x if x >= 65.0 => Color::Yellow,
        x if x >= 40.0 => Color::Rgb(255, 165, 0), 
        x if x >= 20.0 => Color::Cyan,
        _ => Color::Green,
    }
}

pub fn memory_usage_color(usage: f32) -> Color {
    match usage {
        x if x >= 90.0 => Color::Red,
        x if x >= 75.0 => Color::Yellow,
        x if x >= 60.0 => Color::Rgb(255, 165, 0), 
        x if x >= 40.0 => Color::Cyan,
        _ => Color::Green,
    }
}

pub fn disk_usage_color(usage: f32) -> Color {
    match usage {
        x if x >= 90.0 => Color::Red,
        x if x >= 80.0 => Color::Yellow,
        x if x >= 70.0 => Color::Rgb(255, 165, 0), 
        x if x >= 50.0 => Color::Cyan,
        _ => Color::Green,
    }
}

pub fn temperature_color(temp: f32) -> Color {
    match temp {
        x if x >= 90.0 => Color::Red,
        x if x >= 75.0 => Color::Yellow,
        x if x >= 60.0 => Color::Rgb(255, 165, 0), 
        x if x >= 45.0 => Color::Cyan,
        _ => Color::Green,
    }
}

pub fn network_activity_color(rate_mbps: f64) -> Color {
    match rate_mbps {
        x if x >= 100.0 => Color::Red,
        x if x >= 50.0 => Color::Yellow,
        x if x >= 10.0 => Color::Rgb(255, 165, 0),
        x if x >= 1.0 => Color::Cyan,
        _ => Color::Green,
    }
}

pub fn process_status_color(status: &str) -> Color {
    match status.to_lowercase().as_str() {
        "running" | "r" => Color::Green,
        "sleeping" | "s" => Color::Blue,
        "waiting" | "w" => Color::Yellow,
        "zombie" | "z" => Color::Red,
        "stopped" | "t" => Color::Gray,
        "dead" | "x" => Color::DarkGray,
        "idle" | "i" => Color::Cyan,
        _ => Color::White,
    }
}

pub fn container_status_color(status: &str) -> Color {
    if status.to_lowercase().contains("up") || status.to_lowercase().contains("running") {
        Color::Green
    } else if status.to_lowercase().contains("exit") || status.to_lowercase().contains("dead") {
        Color::Red
    } else if status.to_lowercase().contains("pause") {
        Color::Yellow
    } else if status.to_lowercase().contains("restart") {
        Color::Cyan
    } else {
        Color::Gray
    }
}

pub struct ThemeManager {
    current_theme: ColorScheme,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            current_theme: ColorScheme::nord(),
        }
    }
    
    pub fn from_index(index: usize) -> Self {
        let theme = match index % 3 {
            0 => ColorScheme::nord(),
            1 => ColorScheme::dark(),
            2 => ColorScheme::light(),
            _ => ColorScheme::nord(),
        };
        Self { current_theme: theme }
    }
    
    pub fn get_theme(&self) -> &ColorScheme {
        &self.current_theme
    }
    
    pub fn theme_name(index: usize) -> &'static str {
        match index % 3 {
            0 => "Nord",
            1 => "Dark",
            2 => "Light",
            _ => "Nord",
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_colors() {
        assert_eq!(cpu_usage_color(95.0), Color::Red);
        assert_eq!(cpu_usage_color(70.0), Color::Yellow);
        assert_eq!(cpu_usage_color(15.0), Color::Green);
    }
    
    #[test]
    fn test_color_schemes() {
        let dark = ColorScheme::dark();
        assert_eq!(dark.primary, Color::Cyan);
        assert_eq!(dark.background, Color::Reset);
        
        let light = ColorScheme::light();
        assert_eq!(light.primary, Color::Blue);
        assert_eq!(light.background, Color::White);
    }
    
    #[test]
    fn test_process_status_colors() {
        assert_eq!(process_status_color("running"), Color::Green);
        assert_eq!(process_status_color("zombie"), Color::Red);
        assert_eq!(process_status_color("sleeping"), Color::Blue);
    }
}