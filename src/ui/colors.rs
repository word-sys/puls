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
    pub fn dark() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            accent: Color::Yellow,
            background: Color::Black,
            text: Color::White,
            text_secondary: Color::Gray,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            border: Color::Gray,
            highlight: Color::Yellow,
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
    
    pub fn matrix() -> Self {
        Self {
            primary: Color::Green,
            secondary: Color::Rgb(0, 100, 0),
            accent: Color::Rgb(0, 255, 0),
            background: Color::Black,
            text: Color::Green,
            text_secondary: Color::Rgb(0, 150, 0),
            success: Color::Rgb(0, 255, 0),
            warning: Color::Rgb(255, 255, 0),
            error: Color::Red,
            info: Color::Green,
            border: Color::Green,
            highlight: Color::Rgb(0, 255, 0),
        }
    }
    
    pub fn high_contrast() -> Self {
        Self {
            primary: Color::White,
            secondary: Color::Yellow,
            accent: Color::Magenta,
            background: Color::Black,
            text: Color::White,
            text_secondary: Color::White,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            border: Color::White,
            highlight: Color::Yellow,
        }
    }
    
    pub fn solarized_dark() -> Self {
        Self {
            primary: Color::Rgb(131, 148, 150),   // base0
            secondary: Color::Rgb(88, 110, 117),  // base01
            accent: Color::Rgb(42, 161, 152),     // cyan
            background: Color::Rgb(0, 43, 54),    // base03
            text: Color::Rgb(131, 148, 150),      // base0
            text_secondary: Color::Rgb(101, 123, 131), // base00
            success: Color::Rgb(133, 153, 0),     // green
            warning: Color::Rgb(181, 137, 0),     // yellow
            error: Color::Rgb(220, 50, 47),       // red
            info: Color::Rgb(38, 139, 210),       // blue
            border: Color::Rgb(88, 110, 117),     // base01
            highlight: Color::Rgb(42, 161, 152),  // cyan
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

pub struct ColorGradient;

impl ColorGradient {
    pub fn heat_map(value: f32) -> Color {
        let value = value.clamp(0.0, 1.0);
        match value {
            x if x >= 0.8 => Color::Red,
            x if x >= 0.6 => Color::Rgb(255, 165, 0), 
            x if x >= 0.4 => Color::Yellow,
            x if x >= 0.2 => Color::Cyan,
            _ => Color::Blue,
        }
    }
    
    pub fn rainbow(value: f32) -> Color {
        let value = value.clamp(0.0, 1.0);
        match value {
            x if x >= 0.83 => Color::Magenta,
            x if x >= 0.67 => Color::Blue,
            x if x >= 0.5 => Color::Cyan,
            x if x >= 0.33 => Color::Green,
            x if x >= 0.17 => Color::Yellow,
            _ => Color::Red,
        }
    }
    
    pub fn blue_to_red(value: f32) -> Color {
        let value = value.clamp(0.0, 1.0);
        let red = (255.0 * value) as u8;
        let blue = (255.0 * (1.0 - value)) as u8;
        Color::Rgb(red, 0, blue)
    }
    
    pub fn green_to_red(value: f32) -> Color {
        let value = value.clamp(0.0, 1.0);
        match value {
            x if x >= 0.8 => Color::Red,
            x if x >= 0.6 => Color::Rgb(255, 100, 0), 
            x if x >= 0.4 => Color::Rgb(255, 200, 0), 
            x if x >= 0.2 => Color::Yellow,
            _ => Color::Green,
        }
    }
}

pub struct ThemeManager {
    current_theme: ColorScheme,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            current_theme: ColorScheme::dark(),
        }
    }
    
    pub fn set_theme(&mut self, theme: ColorScheme) {
        self.current_theme = theme;
    }
    
    pub fn get_theme(&self) -> &ColorScheme {
        &self.current_theme
    }
    
    pub fn next_theme(&mut self) {
        self.current_theme = ColorScheme::matrix();
    }
    
    pub fn usage_color(&self, usage: f32, metric_type: &str) -> Color {
        match metric_type {
            "cpu" => cpu_usage_color(usage),
            "memory" => memory_usage_color(usage),
            "disk" => disk_usage_color(usage),
            _ => {
                if usage >= 90.0 {
                    self.current_theme.error
                } else if usage >= 70.0 {
                    self.current_theme.warning
                } else {
                    self.current_theme.success
                }
            }
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

pub mod utils {
    use super::*;
    
    pub fn darken_color(color: Color, factor: f32) -> Color {
        let factor = factor.clamp(0.0, 1.0);
        match color {
            Color::Rgb(r, g, b) => {
                Color::Rgb(
                    (r as f32 * factor) as u8,
                    (g as f32 * factor) as u8,
                    (b as f32 * factor) as u8,
                )
            }
            Color::Red => Color::Rgb((255.0 * factor) as u8, 0, 0),
            Color::Green => Color::Rgb(0, (255.0 * factor) as u8, 0),
            Color::Blue => Color::Rgb(0, 0, (255.0 * factor) as u8),
            Color::Yellow => Color::Rgb((255.0 * factor) as u8, (255.0 * factor) as u8, 0),
            Color::Cyan => Color::Rgb(0, (255.0 * factor) as u8, (255.0 * factor) as u8),
            Color::Magenta => Color::Rgb((255.0 * factor) as u8, 0, (255.0 * factor) as u8),
            Color::White => Color::Rgb((255.0 * factor) as u8, (255.0 * factor) as u8, (255.0 * factor) as u8),
            _ => color,
        }
    }
    
    pub fn lighten_color(color: Color, factor: f32) -> Color {
        let factor = factor.clamp(1.0, 2.0);
        match color {
            Color::Rgb(r, g, b) => {
                let new_r = ((r as f32) + (255.0 - r as f32) * (factor - 1.0)).min(255.0) as u8;
                let new_g = ((g as f32) + (255.0 - g as f32) * (factor - 1.0)).min(255.0) as u8;
                let new_b = ((b as f32) + (255.0 - b as f32) * (factor - 1.0)).min(255.0) as u8;
                Color::Rgb(new_r, new_g, new_b)
            }
            _ => color,
        }
    }
    
    pub fn contrasting_text_color(background: Color) -> Color {
        match background {
            Color::Black | Color::DarkGray | Color::Blue | Color::Red | Color::Magenta => Color::White,
            Color::White | Color::Gray | Color::Yellow | Color::Cyan | Color::Green => Color::Black,
            Color::Rgb(r, g, b) => {
                let luminance = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;
                if luminance > 0.5 {
                    Color::Black
                } else {
                    Color::White
                }
            }
            _ => Color::White,
        }
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
        assert_eq!(dark.background, Color::Black);
        
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
    
    #[test]
    fn test_color_gradient() {
        assert_eq!(ColorGradient::heat_map(1.0), Color::Red);
        assert_eq!(ColorGradient::heat_map(0.0), Color::Blue);
    }
}