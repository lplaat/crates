/*
 * Copyright (c) 2025 Bastiaan van der Plaat
 *
 * SPDX-License-Identifier: MIT
 */

//! A minimal replacement for the [terminal_size](https://crates.io/crates/terminal_size) crate

/// Represents the width of a terminal in characters.
pub struct Width(pub u16);

/// Represents the height of a terminal in characters.
pub struct Height(pub u16);

/// Returns the terminal size as a tuple.
pub fn terminal_size() -> Option<(Width, Height)> {
    #[cfg(unix)]
    {
        use libc::{TIOCGWINSZ, ioctl, winsize};
        let fd: std::os::fd::RawFd = 0;
        let mut size: winsize = unsafe { std::mem::zeroed() };
        if unsafe { ioctl(fd, TIOCGWINSZ, &mut size) } == -1 {
            return None;
        }
        Some((Width(size.ws_col), Height(size.ws_row)))
    }

    #[cfg(windows)]
    #[allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]
    unsafe {
        #[repr(C)]
        struct COORD {
            X: i16,
            Y: i16,
        }
        #[repr(C)]
        struct SMALL_RECT {
            Left: i16,
            Top: i16,
            Right: i16,
            Bottom: i16,
        }
        #[repr(C)]
        struct CONSOLE_SCREEN_BUFFER_INFO {
            dwSize: COORD,
            dwCursorPosition: COORD,
            wAttributes: u16,
            srWindow: SMALL_RECT,
            dwMaximumWindowSize: COORD,
        }
        const STD_OUTPUT_HANDLE: i32 = -11;
        #[link(name = "kernel32")]
        unsafe extern "C" {
            unsafe fn GetStdHandle(nStdHandle: i32) -> *mut std::ffi::c_void;
            unsafe fn GetConsoleScreenBufferInfo(
                hConsoleOutput: *mut std::ffi::c_void,
                lpConsoleScreenBufferInfo: *mut CONSOLE_SCREEN_BUFFER_INFO,
            ) -> i32;
        }

        let h_stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        _ = GetConsoleScreenBufferInfo(h_stdout, &mut csbi);
        Some((
            Width((csbi.srWindow.Right - csbi.srWindow.Left + 1) as u16),
            Height((csbi.srWindow.Bottom - csbi.srWindow.Top + 1) as u16),
        ))
    }

    #[cfg(not(any(unix, windows)))]
    compile_error!("Unsupported platform");
}
