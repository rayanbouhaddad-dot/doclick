//! Window organizer: tile or stack the tracked Dofus windows.
//!
//! Layouts target the work area (taskbar excluded) of the monitor hosting
//! the first window in display order, so a multi-monitor user organizes onto
//! whichever screen their team currently lives on.

use serde::Deserialize;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    IsIconic, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOZORDER, SW_RESTORE,
};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Tile side by side in a near-square grid.
    Grid,
    /// All windows at the exact same full-work-area rect. Combined with the
    /// focus shortcuts this gives "one screen, N accounts" flipping, and
    /// makes the proportional click translation exactly 1:1.
    Stack,
}

/// Arrange `hwnds` (already in display order) on the work area of the
/// monitor hosting the first one. Returns how many windows were moved.
pub fn organize(hwnds: &[isize], layout: Layout) -> usize {
    let Some(&first) = hwnds.first() else {
        return 0;
    };
    let Some(work) = monitor_work_area(first) else {
        return 0;
    };
    let (left, top) = (work.0, work.1);
    let (width, height) = (work.2.max(1), work.3.max(1));

    let n = hwnds.len();
    let mut moved = 0usize;
    for (i, &hwnd) in hwnds.iter().enumerate() {
        let (x, y, w, h) = match layout {
            Layout::Stack => (left, top, width, height),
            Layout::Grid => {
                // Near-square grid, filled row by row. The last row stretches
                // its cells so no work-area sliver is left unused.
                let cols = (n as f64).sqrt().ceil() as usize;
                let rows = n.div_ceil(cols);
                let row = i / cols;
                let cols_in_row = if row == rows - 1 {
                    n - row * cols
                } else {
                    cols
                };
                let col = i % cols;
                let cell_w = width / cols_in_row as i32;
                let cell_h = height / rows as i32;
                (
                    left + col as i32 * cell_w,
                    top + row as i32 * cell_h,
                    cell_w,
                    cell_h,
                )
            }
        };
        if place_window(hwnd, x, y, w, h) {
            moved += 1;
        }
    }
    moved
}

fn place_window(hwnd: isize, x: i32, y: i32, w: i32, h: i32) -> bool {
    unsafe {
        let handle = HWND(hwnd as *mut _);
        if IsIconic(handle).as_bool() {
            let _ = ShowWindow(handle, SW_RESTORE);
        }
        SetWindowPos(handle, None, x, y, w, h, SWP_NOACTIVATE | SWP_NOZORDER).is_ok()
    }
}

/// `(left, top, width, height)` of the work area of the monitor hosting `hwnd`.
fn monitor_work_area(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let monitor = MonitorFromWindow(HWND(hwnd as *mut _), MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let r = info.rcWork;
        Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
    }
}
