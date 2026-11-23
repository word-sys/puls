use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub const STDIN_FILENO: i32 = 0;
pub const STDOUT_FILENO: i32 = 1;
const TCSANOW: i32 = 0;
const TIOCGWINSZ: u64 = 0x5413;
const SIGWINCH: i32 = 28;
pub const POLLIN: i16 = 0x0001;

#[repr(C)]
#[derive(Clone, Copy)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; 32],
    c_ispeed: u32,
    c_ospeed: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct WinSize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

type SigHandler = extern "C" fn(i32);

unsafe extern "C" {
    fn tcgetattr(fd: i32, termios_p: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, optional_actions: i32, termios_p: *const Termios) -> i32;
    fn cfmakeraw(termios_p: *mut Termios);
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn signal(signum: i32, handler: SigHandler) -> usize;
}

static ORIG_TERMIOS: Mutex<Option<Termios>> = Mutex::new(None);
static RESIZED_FLAG: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigwinch(_sig: i32) {
    RESIZED_FLAG.store(true, Ordering::SeqCst);
}

pub fn take_resized_flag() -> bool {
    RESIZED_FLAG.swap(false, Ordering::SeqCst)
}

pub fn peek_resized_flag() -> bool {
    RESIZED_FLAG.load(Ordering::SeqCst)
}

pub fn enable_raw_mode() -> io::Result<()> {
    let mut guard = ORIG_TERMIOS.lock().unwrap();
    if guard.is_some() {
        return Ok(());
    }

    let mut orig = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; 32],
        c_ispeed: 0,
        c_ospeed: 0,
    };

    if unsafe { tcgetattr(STDIN_FILENO, &mut orig) } != 0 {
        return Err(io::Error::last_os_error());
    }

    let mut raw = orig;
    unsafe {
        cfmakeraw(&mut raw);
        // Preserve ONLCR off (raw), VMIN=1, VTIME=0 for responsive poll()+read()
        raw.c_cc[6] = 1; // VMIN
        raw.c_cc[5] = 0; // VTIME
    }

    if unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &raw) } != 0 {
        return Err(io::Error::last_os_error());
    }

    unsafe {
        signal(SIGWINCH, handle_sigwinch);
    }

    *guard = Some(orig);
    Ok(())
}

pub fn disable_raw_mode() -> io::Result<()> {
    let mut guard = ORIG_TERMIOS.lock().unwrap();
    if let Some(orig) = guard.take() {
        if unsafe { tcsetattr(STDIN_FILENO, TCSANOW, &orig) } != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

pub fn terminal_size() -> io::Result<(u16, u16)> {
    let mut ws = WinSize::default();
    let ok = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut ws) == 0 }
        || unsafe { ioctl(STDIN_FILENO, TIOCGWINSZ, &mut ws) == 0 };
    if ok && ws.ws_col > 0 && ws.ws_row > 0 {
        Ok((ws.ws_col, ws.ws_row))
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to query terminal window size",
        ))
    }
}

pub fn enter_alternate_screen(w: &mut impl Write) -> io::Result<()> {
    w.write_all(b"\x1b[?1049h\x1b[?25l")?;
    w.flush()
}

pub fn leave_alternate_screen(w: &mut impl Write) -> io::Result<()> {
    w.write_all(b"\x1b[0m\x1b[?25h\x1b[?1049l")?;
    w.flush()
}

pub fn enable_mouse_capture(w: &mut impl Write) -> io::Result<()> {
    // Normal tracking + SGR extended coordinates
    w.write_all(b"\x1b[?1000h\x1b[?1006h")?;
    w.flush()
}

pub fn disable_mouse_capture(w: &mut impl Write) -> io::Result<()> {
    w.write_all(b"\x1b[?1006l\x1b[?1000l")?;
    w.flush()
}

pub fn show_cursor(w: &mut impl Write) -> io::Result<()> {
    w.write_all(b"\x1b[?25h")?;
    w.flush()
}

pub fn poll_stdin(timeout_ms: i32) -> io::Result<bool> {
    let mut pfd = PollFd {
        fd: STDIN_FILENO,
        events: POLLIN,
        revents: 0,
    };
    let ret = unsafe { poll(&mut pfd, 1, timeout_ms) };
    if ret < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            return Ok(false);
        }
        return Err(err);
    }
    Ok(ret > 0 && (pfd.revents & POLLIN) != 0)
}

pub fn read_stdin_bytes(buf: &mut [u8]) -> io::Result<usize> {
    let n = unsafe { read(STDIN_FILENO, buf.as_mut_ptr(), buf.len()) };
    if n < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            return Ok(0);
        }
        return Err(err);
    }
    Ok(n as usize)
}
