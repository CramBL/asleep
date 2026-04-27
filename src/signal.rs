use std::sync::atomic::{AtomicBool, Ordering};

static STOP_SIGNAL_RECEIVED: AtomicBool = AtomicBool::new(false);

pub fn install_handler() {
    #[cfg(unix)]
    {
        // SAFETY: handle_stop_signal is an extern "C" function that only performs atomic operations.
        // Registering it for SIGINT and SIGTERM is standard and safe on Unix.
        unsafe {
            let handler = handle_stop_signal as *const () as libc::sighandler_t;
            libc::signal(libc::SIGINT, handler);
            libc::signal(libc::SIGTERM, handler);
        }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
        // SAFETY: handle_windows_ctrl is a valid callback and SetConsoleCtrlHandler is safe to call.
        unsafe {
            SetConsoleCtrlHandler(Some(handle_windows_ctrl), 1);
        }
    }
}

pub fn stop_signal_received() -> bool {
    STOP_SIGNAL_RECEIVED.load(Ordering::Relaxed)
}

#[cfg(unix)]
extern "C" fn handle_stop_signal(_: libc::c_int) {
    STOP_SIGNAL_RECEIVED.store(true, Ordering::Relaxed);
}

#[cfg(windows)]
unsafe extern "system" fn handle_windows_ctrl(ctrl_type: u32) -> i32 {
    match ctrl_type {
        0 | 1 => {
            // CTRL_C_EVENT or CTRL_BREAK_EVENT
            STOP_SIGNAL_RECEIVED.store(true, Ordering::Relaxed);
            1 // TRUE
        }
        _ => 0, // FALSE
    }
}
