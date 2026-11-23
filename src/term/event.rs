use std::collections::VecDeque;
use std::io;
use std::sync::Mutex;
use std::time::Duration;

use crate::term::sys;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyModifiers(u8);

impl KeyModifiers {
    pub const NONE: Self = Self(0);
    pub const CONTROL: Self = Self(1 << 0);
    pub const ALT: Self = Self(1 << 1);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    Backspace,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
    BackTab,
    Esc,
    F(u8),
    Char(char),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEventKind {
    Down(MouseButton),
    ScrollDown,
    ScrollUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub column: u16,
    pub row: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
}

static INPUT_BUF: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());

pub fn poll(timeout: Duration) -> io::Result<bool> {
    if sys::peek_resized_flag() {
        return Ok(true);
    }
    if !INPUT_BUF.lock().unwrap().is_empty() {
        return Ok(true);
    }
    let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
    let ready = sys::poll_stdin(ms)?;
    Ok(ready || sys::peek_resized_flag())
}

pub fn read() -> io::Result<Event> {
    loop {
        if sys::take_resized_flag() {
            let (w, h) = sys::terminal_size().unwrap_or((80, 24));
            return Ok(Event::Resize(w, h));
        }

        {
            let buf = INPUT_BUF.lock().unwrap();
            if buf.is_empty() {
                drop(buf);
                let mut raw = [0u8; 64];
                let n = sys::read_stdin_bytes(&mut raw)?;
                if n == 0 {
                    if sys::take_resized_flag() {
                        let (w, h) = sys::terminal_size().unwrap_or((80, 24));
                        return Ok(Event::Resize(w, h));
                    }
                    continue;
                }
                let mut buf = INPUT_BUF.lock().unwrap();
                buf.extend(&raw[..n]);
            }
        }

        // If the buffer currently contains only a lone ESC byte, wait briefly for a possible sequence continuation
        let need_esc_probe = {
            let buf = INPUT_BUF.lock().unwrap();
            buf.len() == 1 && buf[0] == 0x1b
        };
        if need_esc_probe && sys::poll_stdin(12)? {
            let mut raw = [0u8; 64];
            let n = sys::read_stdin_bytes(&mut raw)?;
            if n > 0 {
                let mut buf = INPUT_BUF.lock().unwrap();
                buf.extend(&raw[..n]);
            }
        }

        let parsed = {
            let mut buf = INPUT_BUF.lock().unwrap();
            let slice: Vec<u8> = buf.iter().copied().collect();
            let (ev, consumed) = parse_next_event(&slice);
            for _ in 0..consumed.min(buf.len()) {
                buf.pop_front();
            }
            ev
        };

        if let Some(event) = parsed {
            return Ok(event);
        }
    }
}

pub fn parse_next_event(bytes: &[u8]) -> (Option<Event>, usize) {
    if bytes.is_empty() {
        return (None, 0);
    }

    match bytes[0] {
        0x1b => parse_escape_sequence(bytes),
        b'\r' | b'\n' => (Some(Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))), 1),
        b'\t' => (Some(Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))), 1),
        0x7f | 0x08 => (Some(Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE))), 1),
        b @ 0x01..=0x1a => {
            let ch = (b - 1 + b'a') as char;
            (Some(Event::Key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL))), 1)
        }
        b if b < 0x80 => (
            Some(Event::Key(KeyEvent::new(KeyCode::Char(b as char), KeyModifiers::NONE))),
            1,
        ),
        _ => parse_utf8_char(bytes),
    }
}

fn parse_utf8_char(bytes: &[u8]) -> (Option<Event>, usize) {
    let first = bytes[0];
    let len = if first & 0xE0 == 0xC0 {
        2
    } else if first & 0xF0 == 0xE0 {
        3
    } else if first & 0xF8 == 0xF0 {
        4
    } else {
        return (None, 1);
    };
    if bytes.len() < len {
        return (None, bytes.len());
    }
    if let Ok(s) = std::str::from_utf8(&bytes[..len]) {
        if let Some(ch) = s.chars().next() {
            return (
                Some(Event::Key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE))),
                len,
            );
        }
    }
    (None, 1)
}

fn parse_escape_sequence(bytes: &[u8]) -> (Option<Event>, usize) {
    if bytes.len() == 1 {
        return (Some(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))), 1);
    }

    match bytes[1] {
        b'[' => parse_csi(bytes),
        b'O' => parse_ss3(bytes),
        b if b >= 0x20 && b < 0x7f => (
            Some(Event::Key(KeyEvent::new(KeyCode::Char(b as char), KeyModifiers::ALT))),
            2,
        ),
        _ => (Some(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))), 1),
    }
}

fn parse_ss3(bytes: &[u8]) -> (Option<Event>, usize) {
    if bytes.len() < 3 {
        return (Some(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))), 1);
    }
    let code = match bytes[2] {
        b'P' => KeyCode::F(1),
        b'Q' => KeyCode::F(2),
        b'R' => KeyCode::F(3),
        b'S' => KeyCode::F(4),
        b'H' => KeyCode::Home,
        b'F' => KeyCode::End,
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        _ => return (None, 3),
    };
    (Some(Event::Key(KeyEvent::new(code, KeyModifiers::NONE))), 3)
}

fn parse_csi(bytes: &[u8]) -> (Option<Event>, usize) {
    if bytes.len() < 3 {
        return (Some(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))), 1);
    }

    // SGR extended mouse: ESC [ < Cb ; Cx ; Cy (M|m)
    if bytes[2] == b'<' {
        return parse_sgr_mouse(bytes);
    }

    // Legacy X10 mouse: ESC [ M Cb Cx Cy
    if bytes[2] == b'M' && bytes.len() >= 6 {
        let cb = bytes[3].saturating_sub(32);
        let cx = bytes[4].saturating_sub(33) as u16;
        let cy = bytes[5].saturating_sub(33) as u16;
        let kind = match cb {
            0 => Some(MouseEventKind::Down(MouseButton::Left)),
            64 => Some(MouseEventKind::ScrollUp),
            65 => Some(MouseEventKind::ScrollDown),
            _ => None,
        };
        return (kind.map(|k| Event::Mouse(MouseEvent { kind: k, column: cx, row: cy })), 6);
    }

    // Scan to final CSI byte (0x40..=0x7E)
    let mut end = 2;
    while end < bytes.len() && !(0x40..=0x7E).contains(&bytes[end]) {
        end += 1;
    }
    if end >= bytes.len() {
        return (None, bytes.len());
    }

    let final_byte = bytes[end];
    let params = std::str::from_utf8(&bytes[2..end]).unwrap_or("");
    let consumed = end + 1;

    let mut parts = params.split(';');
    let first_num = parts.next().and_then(|s| s.parse::<u16>().ok());
    let mod_num = parts.next().and_then(|s| s.parse::<u16>().ok());
    let modifiers = decode_csi_modifier(mod_num);

    let code = match final_byte {
        b'A' => Some(KeyCode::Up),
        b'B' => Some(KeyCode::Down),
        b'C' => Some(KeyCode::Right),
        b'D' => Some(KeyCode::Left),
        b'H' => Some(KeyCode::Home),
        b'F' => Some(KeyCode::End),
        b'Z' => Some(KeyCode::BackTab),
        b'~' => match first_num {
            Some(1) | Some(7) => Some(KeyCode::Home),
            Some(4) | Some(8) => Some(KeyCode::End),
            Some(5) => Some(KeyCode::PageUp),
            Some(6) => Some(KeyCode::PageDown),
            Some(11) => Some(KeyCode::F(1)),
            Some(12) => Some(KeyCode::F(2)),
            Some(13) => Some(KeyCode::F(3)),
            Some(14) => Some(KeyCode::F(4)),
            Some(15) => Some(KeyCode::F(5)),
            Some(17) => Some(KeyCode::F(6)),
            Some(18) => Some(KeyCode::F(7)),
            Some(19) => Some(KeyCode::F(8)),
            Some(20) => Some(KeyCode::F(9)),
            Some(21) => Some(KeyCode::F(10)),
            Some(23) => Some(KeyCode::F(11)),
            Some(24) => Some(KeyCode::F(12)),
            _ => None,
        },
        _ => None,
    };

    (code.map(|c| Event::Key(KeyEvent::new(c, modifiers))), consumed)
}

fn decode_csi_modifier(mod_num: Option<u16>) -> KeyModifiers {
    match mod_num {
        Some(3) => KeyModifiers::ALT,
        Some(5) => KeyModifiers::CONTROL,
        Some(7) => KeyModifiers(KeyModifiers::CONTROL.0 | KeyModifiers::ALT.0),
        _ => KeyModifiers::NONE,
    }
}

fn parse_sgr_mouse(bytes: &[u8]) -> (Option<Event>, usize) {
    let mut end = 3;
    while end < bytes.len() && bytes[end] != b'M' && bytes[end] != b'm' {
        end += 1;
    }
    if end >= bytes.len() {
        return (None, bytes.len());
    }
    let is_press = bytes[end] == b'M';
    let body = std::str::from_utf8(&bytes[3..end]).unwrap_or("");
    let consumed = end + 1;

    let mut nums = body.split(';').filter_map(|s| s.parse::<u16>().ok());
    let (Some(cb), Some(cx), Some(cy)) = (nums.next(), nums.next(), nums.next()) else {
        return (None, consumed);
    };

    if !is_press {
        return (None, consumed);
    }

    let kind = match cb & 0b1100_0011 {
        0 => Some(MouseEventKind::Down(MouseButton::Left)),
        64 => Some(MouseEventKind::ScrollUp),
        65 => Some(MouseEventKind::ScrollDown),
        _ => None,
    };

    let event = kind.map(|k| {
        Event::Mouse(MouseEvent {
            kind: k,
            column: cx.saturating_sub(1),
            row: cy.saturating_sub(1),
        })
    });
    (event, consumed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keyboard_and_ctrl_sequences() {
        assert_eq!(
            parse_next_event(b"q"),
            (Some(Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))), 1)
        );
        assert_eq!(
            parse_next_event(b"\x03"),
            (Some(Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL))), 1)
        );
        assert_eq!(
            parse_next_event(b"\x1b[A"),
            (Some(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))), 3)
        );
        assert_eq!(
            parse_next_event(b"\x1b[Z"),
            (Some(Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE))), 3)
        );
        assert_eq!(
            parse_next_event(b"\x1b[5~"),
            (Some(Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE))), 4)
        );
        assert_eq!(
            parse_next_event(b"\x1b[20~"),
            (Some(Event::Key(KeyEvent::new(KeyCode::F(9), KeyModifiers::NONE))), 5)
        );
        assert_eq!(
            parse_next_event(b"\x1bOQ"),
            (Some(Event::Key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE))), 3)
        );
        assert_eq!(
            parse_next_event("ş".as_bytes()),
            (Some(Event::Key(KeyEvent::new(KeyCode::Char('ş'), KeyModifiers::NONE))), 2)
        );
    }

    #[test]
    fn test_parse_sgr_mouse_events() {
        assert_eq!(
            parse_next_event(b"\x1b[<0;12;4M"),
            (
                Some(Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 11,
                    row: 3,
                })),
                10
            )
        );
        assert_eq!(
            parse_next_event(b"\x1b[<64;20;10M"),
            (
                Some(Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollUp,
                    column: 19,
                    row: 9,
                })),
                12
            )
        );
        assert_eq!(
            parse_next_event(b"\x1b[<65;20;10M"),
            (
                Some(Event::Mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 19,
                    row: 9,
                })),
                12
            )
        );
        // Mouse release ignored
        assert_eq!(parse_next_event(b"\x1b[<0;12;4m"), (None, 10));
    }
}
