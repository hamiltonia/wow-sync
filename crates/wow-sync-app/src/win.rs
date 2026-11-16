//! The only unsafe code in the shell: window visibility and the single-instance guard.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        fn IsWindowVisible(hwnd: isize) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(attr: *const c_void, owner: i32, name: *const u16) -> isize;
        fn CreateEventW(attr: *const c_void, manual: i32, init: i32, name: *const u16) -> isize;
        fn SetEvent(h: isize) -> i32;
        fn WaitForSingleObject(h: isize, ms: u32) -> u32;
        fn GetLastError() -> u32;
    }

    const SW_HIDE: i32 = 0;
    const SW_RESTORE: i32 = 9;
    const ERROR_ALREADY_EXISTS: u32 = 183;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn set_visible(hwnd: isize, show: bool) {
        // SAFETY: plain Win32 calls on a window handle owned by this process.
        unsafe {
            if show {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            } else {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
    }

    pub fn is_visible(hwnd: isize) -> bool {
        // SAFETY: plain Win32 query.
        unsafe { IsWindowVisible(hwnd) != 0 }
    }

    /// Some(show-event) for the first instance; a later instance signals the first and gets None.
    pub fn single_instance(key: &str) -> Option<isize> {
        // SAFETY: the handles are kept for the process lifetime (never closed).
        unsafe {
            CreateMutexW(
                std::ptr::null(),
                1,
                wide(&format!("Local\\WowSyncSlice{key}")).as_ptr(),
            );
            let existed = GetLastError() == ERROR_ALREADY_EXISTS;
            let ev = CreateEventW(
                std::ptr::null(),
                0,
                0,
                wide(&format!("Local\\WowSyncSliceShow{key}")).as_ptr(),
            );
            if existed {
                SetEvent(ev);
                None
            } else {
                Some(ev)
            }
        }
    }

    pub fn show_requested(ev: isize) -> bool {
        // SAFETY: `ev` is the event handle returned by `single_instance`.
        unsafe { WaitForSingleObject(ev, 0) == 0 }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set_visible(_hwnd: isize, _show: bool) {}
    pub fn is_visible(_hwnd: isize) -> bool {
        false
    }
    pub fn single_instance(_key: &str) -> Option<isize> {
        Some(0)
    }
    pub fn show_requested(_ev: isize) -> bool {
        false
    }
}

pub use imp::{is_visible, set_visible, show_requested, single_instance};
