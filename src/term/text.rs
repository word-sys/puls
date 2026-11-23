use std::borrow::Cow;

use crate::term::style::Style;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}

pub fn char_width(ch: char) -> usize {
    let cp = ch as u32;
    if cp == 0 || cp < 0x20 || (0x7f..0xa0).contains(&cp) {
        return 0;
    }
    // Combining diacritical marks
    if (0x0300..=0x036F).contains(&cp)
        || (0x1AB0..=0x1AFF).contains(&cp)
        || (0x1DC0..=0x1DFF).contains(&cp)
        || (0x20D0..=0x20FF).contains(&cp)
        || (0xFE20..=0xFE2F).contains(&cp)
    {
        return 0;
    }
    // Wide East Asian & common emoji ranges
    if (0x1100..=0x115F).contains(&cp)
        || (0x2E80..=0xA4CF).contains(&cp)
        || (0xAC00..=0xD7A3).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0xFE10..=0xFE19).contains(&cp)
        || (0xFE30..=0xFE6F).contains(&cp)
        || (0xFF00..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp)
        || (0x1F300..=0x1FAFF).contains(&cp)
    {
        return 2;
    }
    1
}

pub fn str_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span<'a> {
    pub content: Cow<'a, str>,
    pub style: Style,
}

impl<'a> Span<'a> {
    pub fn raw<T: Into<Cow<'a, str>>>(content: T) -> Self {
        Self {
            content: content.into(),
            style: Style::default(),
        }
    }

    pub fn styled<T: Into<Cow<'a, str>>>(content: T, style: Style) -> Self {
        Self {
            content: content.into(),
            style,
        }
    }

    pub fn width(&self) -> usize {
        str_width(&self.content)
    }
}

impl<'a> From<&'a str> for Span<'a> {
    fn from(s: &'a str) -> Self {
        Self::raw(s)
    }
}

impl<'a> From<String> for Span<'a> {
    fn from(s: String) -> Self {
        Self::raw(s)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line<'a> {
    pub spans: Vec<Span<'a>>,
}

impl<'a> Line<'a> {
    pub fn raw<T: Into<Cow<'a, str>>>(content: T) -> Self {
        Self {
            spans: vec![Span::raw(content)],
        }
    }

    pub fn width(&self) -> usize {
        self.spans.iter().map(Span::width).sum()
    }
}

impl<'a> From<&'a str> for Line<'a> {
    fn from(s: &'a str) -> Self {
        Self::raw(s)
    }
}

impl<'a> From<String> for Line<'a> {
    fn from(s: String) -> Self {
        Self::raw(s)
    }
}

impl<'a> From<Span<'a>> for Line<'a> {
    fn from(span: Span<'a>) -> Self {
        Self { spans: vec![span] }
    }
}

impl<'a> From<Vec<Span<'a>>> for Line<'a> {
    fn from(spans: Vec<Span<'a>>) -> Self {
        Self { spans }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text<'a> {
    pub lines: Vec<Line<'a>>,
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(s: &'a str) -> Self {
        Self {
            lines: s.lines().map(Line::from).collect(),
        }
    }
}

impl<'a> From<String> for Text<'a> {
    fn from(s: String) -> Self {
        Self {
            lines: s.lines().map(|l| Line::from(l.to_string())).collect(),
        }
    }
}

impl<'a> From<Span<'a>> for Text<'a> {
    fn from(span: Span<'a>) -> Self {
        Self {
            lines: vec![Line::from(span)],
        }
    }
}

impl<'a> From<Line<'a>> for Text<'a> {
    fn from(line: Line<'a>) -> Self {
        Self { lines: vec![line] }
    }
}

impl<'a> From<Vec<Line<'a>>> for Text<'a> {
    fn from(lines: Vec<Line<'a>>) -> Self {
        Self { lines }
    }
}
