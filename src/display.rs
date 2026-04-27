use crate::duration::Seconds;
use std::io::{self, Write};

const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
const SHOW_CURSOR: &[u8] = b"\x1b[?25h";
const CLEAR_LINE: &[u8] = b"\r\x1b[K";

pub struct TerminalGuard;

impl Default for TerminalGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalGuard {
    pub fn new() -> Self {
        let mut stdout = io::stdout().lock();
        let _ = stdout.write_all(HIDE_CURSOR);
        let _ = stdout.flush();
        TerminalGuard
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout().lock();
        let _ = stdout.write_all(SHOW_CURSOR);
        let _ = stdout.write_all(CLEAR_LINE);
        let _ = stdout.flush();
    }
}

pub fn update_progress_seconds(s: Seconds) {
    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(CLEAR_LINE);
    let _ = s.write_human(&mut stdout);
    let _ = stdout.flush();
}
