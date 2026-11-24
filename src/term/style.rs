use std::ops::BitOr;
use std::sync::OnceLock;

pub fn supports_truecolor() -> bool {
    static TRUECOLOR: OnceLock<bool> = OnceLock::new();
    *TRUECOLOR.get_or_init(|| {
        if let Ok(ct) = std::env::var("COLORTERM") {
            let l = ct.to_lowercase();
            if l.contains("truecolor") || l.contains("24bit") {
                return true;
            }
        }
        if let Ok(term) = std::env::var("TERM") {
            let l = term.to_lowercase();
            if l.contains("256color") || l.contains("kitty") || l.contains("alacritty") || l.contains("wezterm") || l.contains("direct") {
                return true;
            }
        }
        false
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Color {
    #[default]
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightMagenta,
    LightCyan,
    White,
    Rgb(u8, u8, u8),
}

impl Color {
    pub fn write_fg_ansi(self, out: &mut String) {
        match self {
            Color::Reset => out.push_str("\x1b[39m"),
            Color::Black => out.push_str("\x1b[30m"),
            Color::Red => out.push_str("\x1b[31m"),
            Color::Green => out.push_str("\x1b[32m"),
            Color::Yellow => out.push_str("\x1b[33m"),
            Color::Blue => out.push_str("\x1b[34m"),
            Color::Magenta => out.push_str("\x1b[35m"),
            Color::Cyan => out.push_str("\x1b[36m"),
            Color::Gray => out.push_str("\x1b[37m"),
            Color::DarkGray => out.push_str("\x1b[90m"),
            Color::LightRed => out.push_str("\x1b[91m"),
            Color::LightGreen => out.push_str("\x1b[92m"),
            Color::LightYellow => out.push_str("\x1b[93m"),
            Color::LightMagenta => out.push_str("\x1b[95m"),
            Color::LightCyan => out.push_str("\x1b[96m"),
            Color::White => out.push_str("\x1b[97m"),
            Color::Rgb(r, g, b) => {
                use std::fmt::Write;
                let _ = write!(out, "\x1b[38;2;{};{};{}m", r, g, b);
            }
        }
    }

    pub fn write_bg_ansi(self, out: &mut String) {
        match self {
            Color::Reset => out.push_str("\x1b[49m"),
            Color::Black => {
                if supports_truecolor() {
                    out.push_str("\x1b[48;2;0;0;0m");
                } else {
                    out.push_str("\x1b[40m");
                }
            }
            Color::Red => out.push_str("\x1b[41m"),
            Color::Green => out.push_str("\x1b[42m"),
            Color::Yellow => out.push_str("\x1b[43m"),
            Color::Blue => out.push_str("\x1b[44m"),
            Color::Magenta => out.push_str("\x1b[45m"),
            Color::Cyan => out.push_str("\x1b[46m"),
            Color::Gray => out.push_str("\x1b[47m"),
            Color::DarkGray => out.push_str("\x1b[100m"),
            Color::LightRed => out.push_str("\x1b[101m"),
            Color::LightGreen => out.push_str("\x1b[102m"),
            Color::LightYellow => out.push_str("\x1b[103m"),
            Color::LightMagenta => out.push_str("\x1b[105m"),
            Color::LightCyan => out.push_str("\x1b[106m"),
            Color::White => out.push_str("\x1b[107m"),
            Color::Rgb(r, g, b) => {
                use std::fmt::Write;
                let _ = write!(out, "\x1b[48;2;{};{};{}m", r, g, b);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Modifier(u8);

impl Modifier {
    pub const NONE: Self = Self(0);
    pub const BOLD: Self = Self(1 << 0);
    pub const REVERSED: Self = Self(1 << 1);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl BitOr for Modifier {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub modifier: Modifier,
}

impl Style {
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn add_modifier(mut self, m: Modifier) -> Self {
        self.modifier = self.modifier | m;
        self
    }

    pub fn patch(self, other: Style) -> Style {
        Style {
            fg: other.fg.or(self.fg),
            bg: other.bg.or(self.bg),
            modifier: self.modifier | other.modifier,
        }
    }
}
