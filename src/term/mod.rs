pub mod buffer;
pub mod event;
pub mod layout;
pub mod style;
pub mod sys;
pub mod text;
pub mod widgets;

pub use buffer::Terminal;

pub mod symbols {
    pub use crate::term::widgets::Marker;
}

pub mod terminal {
    pub use crate::term::sys::{
        disable_mouse_capture, disable_raw_mode, enable_mouse_capture, enable_raw_mode,
        enter_alternate_screen, leave_alternate_screen, terminal_size as size,
    };
}

pub mod prelude {
    pub use crate::term::buffer::Frame;
    pub use crate::term::layout::{Constraint, Direction, Layout, Rect};
    pub use crate::term::style::{Color, Modifier, Style};
    pub use crate::term::text::{Alignment, Line, Span};
}
