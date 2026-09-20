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

    pub fn dracula() -> Self {
        Self {
            primary: Color::Magenta,
            secondary: Color::Cyan,
            accent: Color::LightMagenta,
            background: Color::Reset,
            text: Color::White,
            text_secondary: Color::Gray,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            border: Color::DarkGray,
            highlight: Color::LightMagenta,
        }
    }

    pub fn solarized() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Cyan,
            accent: Color::Yellow,
            background: Color::Reset,
            text: Color::White,
            text_secondary: Color::DarkGray,
            success: Color::Green,
            warning: Color::Rgb(255, 140, 0),
            error: Color::Red,
            info: Color::Blue,
            border: Color::DarkGray,
            highlight: Color::Cyan,
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            primary: Color::Yellow,
            secondary: Color::White,
            accent: Color::Cyan,
            background: Color::Reset,
            text: Color::White,
            text_secondary: Color::Yellow,
            success: Color::LightGreen,
            warning: Color::LightYellow,
            error: Color::LightRed,
            info: Color::LightCyan,
            border: Color::White,
            highlight: Color::Yellow,
        }
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

pub const THEME_COUNT: usize = 6;

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
        let theme = match index % THEME_COUNT {
            0 => ColorScheme::nord(),
            1 => ColorScheme::dark(),
            2 => ColorScheme::light(),
            3 => ColorScheme::dracula(),
            4 => ColorScheme::solarized(),
            5 => ColorScheme::high_contrast(),
            _ => ColorScheme::nord(),
        };
        Self { current_theme: theme }
    }
    
    pub fn get_theme(&self) -> &ColorScheme {
        &self.current_theme
    }
    
    pub fn theme_name(index: usize) -> &'static str {
        match index % THEME_COUNT {
            0 => "Nord",
            1 => "Dark",
            2 => "Light",
            3 => "Dracula",
            4 => "Solarized",
            5 => "High Contrast",
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