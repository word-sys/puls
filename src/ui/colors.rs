use ratatui::style::Color;

/// Color scheme for the application
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
    /// Default dark theme
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
    
    /// Light theme variant
    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::DarkGray,
            accent: Color::Magenta,
            background: Color::White,
            text: Color::Black,
            text_secondary: Color::DarkGray,
            success: Color::Green,
            warning: Color::Rgb(255, 165, 0), // Orange
            error: Color::Red,
            info: Color::Blue,
            border: Color::DarkGray,
            highlight: Color::Blue,
        }
    }
    
    /// Matrix-style green theme
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
    
    /// High contrast theme for accessibility
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
    
    /// Solarized dark theme
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

/// Get color for CPU usage based on percentage
pub fn cpu_usage_color(usage: f32) -> Color {
    match usage {
        x if x >= 90.0 => Color::Red,
        x if x >= 70.0 => Color::Yellow,
        x if x >= 50.0 => Color::Rgb(255, 165, 0), // Orange
        x if x >= 30.0 => Color::Cyan,
        _ => Color::Green,
    }
}

/// Get color for memory usage based on percentage
pub fn memory_usage_color(usage: f32) -> Color {
    match usage {
        x if x >= 95.0 => Color::Red,
        x if x >= 85.0 => Color::Yellow,
        x if x >= 70.0 => Color::Rgb(255, 165, 0), // Orange
        x if x >= 50.0 => Color::Cyan,
        _ => Color::Green,
    }
}

/// Get color for disk usage based on percentage
pub fn disk_usage_color(usage: f32) -> Color {
    match usage {
        x if x >= 95.0 => Color::Red,
        x if x >= 90.0 => Color::Yellow,
        x if x >= 80.0 => Color::Rgb(255, 165, 0), // Orange
        x if x >= 60.0 => Color::Cyan,
        _ => Color::Green,
    }
}

/// Get color for temperature based on Celsius
pub fn temperature_color(temp: f32) -> Color {
    match temp {
        x if x >= 80.0 => Color::Red,
        x if x >= 70.0 => Color::Yellow,
        x if x >= 60.0 => Color::Rgb(255, 165, 0), // Orange
        x if x >= 45.0 => Color::Cyan,
        _ => Color::Green,
    }
}

/// Get color for network activity (based on activity level)
pub fn network_activity_color(rate_mbps: f64) -> Color {
    match rate_mbps {
        x if x >= 100.0 => Color::Red,
        x if x >= 50.0 => Color::Yellow,
        x if x >= 10.0 => Color::Rgb(255, 165, 0), // Orange
        x if x >= 1.0 => Color::Cyan,
        _ => Color::Green,
    }
}

/// Get color for process status
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

/// Get color for container status
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

/// Color gradients for sparklines and charts
pub struct ColorGradient;

impl ColorGradient {
    /// Get a color from a gradient based on value (0.0 to 1.0)
    pub fn heat_map(value: f32) -> Color {
        let value = value.clamp(0.0, 1.0);
        match value {
            x if x >= 0.8 => Color::Red,
            x if x >= 0.6 => Color::Rgb(255, 165, 0), // Orange
            x if x >= 0.4 => Color::Yellow,
            x if x >= 0.2 => Color::Cyan,
            _ => Color::Blue,
        }
    }
    
    /// Rainbow gradient
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
    
    /// Blue to red gradient
    pub fn blue_to_red(value: f32) -> Color {
        let value = value.clamp(0.0, 1.0);
        let red = (255.0 * value) as u8;
        let blue = (255.0 * (1.0 - value)) as u8;
        Color::Rgb(red, 0, blue)
    }
    
    /// Green to red gradient (good to bad)
    pub fn green_to_red(value: f32) -> Color {
        let value = value.clamp(0.0, 1.0);
        match value {
            x if x >= 0.8 => Color::Red,
            x if x >= 0.6 => Color::Rgb(255, 100, 0), // Red-orange
            x if x >= 0.4 => Color::Rgb(255, 200, 0), // Orange-yellow
            x if x >= 0.2 => Color::Yellow,
            _ => Color::Green,
        }
    }
}

/// Theme manager for switching between themes
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
    
    /// Cycle to next theme
    pub fn next_theme(&mut self) {
        // This is a simplified implementation
        // In a real app, you'd track which theme is current
        self.current_theme = ColorScheme::matrix();
    }
    
    /// Apply theme-aware colors
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

/// Utility functions for color manipulation
pub mod utils {
    use super::*;
    
    /// Darken a color by a factor (0.0 = black, 1.0 = original)
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
            // For named colors, approximate with RGB
            Color::Red => Color::Rgb((255.0 * factor) as u8, 0, 0),
            Color::Green => Color::Rgb(0, (255.0 * factor) as u8, 0),
            Color::Blue => Color::Rgb(0, 0, (255.0 * factor) as u8),
            Color::Yellow => Color::Rgb((255.0 * factor) as u8, (255.0 * factor) as u8, 0),
            Color::Cyan => Color::Rgb(0, (255.0 * factor) as u8, (255.0 * factor) as u8),
            Color::Magenta => Color::Rgb((255.0 * factor) as u8, 0, (255.0 * factor) as u8),
            Color::White => Color::Rgb((255.0 * factor) as u8, (255.0 * factor) as u8, (255.0 * factor) as u8),
            // For other colors, return as-is
            _ => color,
        }
    }
    
    /// Lighten a color by a factor (1.0 = original, 2.0 = white)
    pub fn lighten_color(color: Color, factor: f32) -> Color {
        let factor = factor.clamp(1.0, 2.0);
        match color {
            Color::Rgb(r, g, b) => {
                let new_r = ((r as f32) + (255.0 - r as f32) * (factor - 1.0)).min(255.0) as u8;
                let new_g = ((g as f32) + (255.0 - g as f32) * (factor - 1.0)).min(255.0) as u8;
                let new_b = ((b as f32) + (255.0 - b as f32) * (factor - 1.0)).min(255.0) as u8;
                Color::Rgb(new_r, new_g, new_b)
            }
            _ => color, // Return as-is for named colors
        }
    }
    
    /// Get contrasting color for text on given background
    pub fn contrasting_text_color(background: Color) -> Color {
        match background {
            Color::Black | Color::DarkGray | Color::Blue | Color::Red | Color::Magenta => Color::White,
            Color::White | Color::Gray | Color::Yellow | Color::Cyan | Color::Green => Color::Black,
            Color::Rgb(r, g, b) => {
                // Calculate luminance
                let luminance = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;
                if luminance > 0.5 {
                    Color::Black
                } else {
                    Color::White
                }
            }
            _ => Color::White, // Default to white for other colors
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_colors() {
        assert_eq!(cpu_usage_color(95.0), Color::Red);
        assert_eq!(cpu_usage_color(75.0), Color::Yellow);
        assert_eq!(cpu_usage_color(25.0), Color::Green);
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