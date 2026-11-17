//! The only unsafe code in the prototype: hiding and showing the window for the tray.

#[cfg(windows)]
mod imp {
    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
    }

    pub fn set_visible(hwnd: isize, show: bool) {
        // SAFETY: plain Win32 calls on a window handle owned by this process.
        unsafe {
            if show {
                ShowWindow(hwnd, 9);
                SetForegroundWindow(hwnd);
            } else {
                ShowWindow(hwnd, 0);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set_visible(_hwnd: isize, _show: bool) {}
}

pub use imp::set_visible;
