#[cfg(windows)]
pub use windows::Handoff;

#[cfg(not(windows))]
pub struct Handoff;

#[cfg(not(windows))]
impl Handoff {
    pub fn capture(_: &tauri::AppHandle, _: bool) -> Option<Self> { None }
    pub fn park(self) -> Option<Self> { None }
    pub fn restore(self) {}
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use tauri::{AppHandle, Manager, WebviewWindow};

    type Handle = *mut c_void;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct Point { x: i32, y: i32 }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct Bounds { left: i32, top: i32, right: i32, bottom: i32 }

    impl Bounds {
        fn contains(self, point: Point) -> bool {
            (self.left..self.right).contains(&point.x) && (self.top..self.bottom).contains(&point.y)
        }

        fn intersect(self, other: Self) -> Self {
            Self { left: self.left.max(other.left), top: self.top.max(other.top),
                right: self.right.min(other.right), bottom: self.bottom.min(other.bottom) }
        }

        fn nearest_interior(self, point: Point) -> Option<Point> {
            if self.right <= self.left || self.bottom <= self.top {
                return None;
            }
            let inset_x = 24.min((self.right - self.left - 1) / 2);
            let inset_y = 24.min((self.bottom - self.top - 1) / 2);
            Some(Point {
                x: point.x.clamp(self.left + inset_x, self.right - 1 - inset_x),
                y: point.y.clamp(self.top + inset_y, self.bottom - 1 - inset_y),
            })
        }
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> Handle;
        fn GetAncestor(window: Handle, flag: u32) -> Handle;
        fn GetClientRect(window: Handle, bounds: *mut Bounds) -> i32;
        fn ScreenToClient(window: Handle, point: *mut Point) -> i32;
        fn WindowFromPoint(point: Point) -> Handle;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn SetCursorPos(x: i32, y: i32) -> i32;
        fn GetClipCursor(bounds: *mut Bounds) -> i32;
    }

    fn cursor_position() -> Option<Point> {
        let mut point = Point::default();
        (unsafe { GetCursorPos(&mut point) } != 0).then_some(point)
    }

    fn owns_cursor(hwnd: Handle, point: Point) -> bool {
        let mut client_point = point;
        let mut client = Bounds::default();
        unsafe {
            GetForegroundWindow() == hwnd
                && GetAncestor(WindowFromPoint(point), 2) == hwnd
                && ScreenToClient(hwnd, &mut client_point) != 0
                && GetClientRect(hwnd, &mut client) != 0
                && client.contains(client_point)
        }
    }

    fn parking_position(
        point: Point, lobby: Bounds, clip: Bounds, work_areas: impl Iterator<Item = Bounds>,
    ) -> Option<Point> {
        work_areas
            .filter_map(|area| area.intersect(clip).nearest_interior(point))
            .filter(|&candidate| !lobby.contains(candidate))
            .min_by_key(|candidate| {
                let dx = i64::from(candidate.x) - i64::from(point.x);
                let dy = i64::from(candidate.y) - i64::from(point.y);
                dx * dx + dy * dy
            })
    }

    pub struct Handoff {
        window: WebviewWindow,
        original: Point,
    }

    pub struct Parked {
        hwnd: usize,
        original: Point,
        parked_at: Point,
    }

    impl Handoff {
        pub fn capture(app: &AppHandle, enabled: bool) -> Option<Self> {
            if !enabled { return None; }
            let window = app.get_webview_window("main")?;
            let hwnd = window.hwnd().ok()?.0;
            let original = cursor_position()?;
            owns_cursor(hwnd, original).then_some(Self { window, original })
        }

        pub fn park(self) -> Option<Parked> {
            let hwnd = self.window.hwnd().ok()?.0;
            let monitor = self.window.current_monitor().ok()??;
            let origin = monitor.position();
            let size = monitor.size();
            let lobby = Bounds { left: origin.x, top: origin.y,
                right: origin.x + size.width as i32, bottom: origin.y + size.height as i32 };
            if !lobby.contains(self.original) { return None; }
            let mut clip = Bounds::default();
            if unsafe { GetClipCursor(&mut clip) } == 0 { return None; }
            let monitors = self.window.available_monitors().ok()?;
            let work_areas = monitors.iter().filter(|other| other.position() != origin).map(|other| {
                let area = other.work_area();
                Bounds { left: area.position.x, top: area.position.y,
                    right: area.position.x + area.size.width as i32,
                    bottom: area.position.y + area.size.height as i32 }
            });
            let parked_at = parking_position(self.original, lobby, clip, work_areas)?;

            // The mouse may have moved, or another app may have taken over during launch preparation.
            if cursor_position() != Some(self.original) || !owns_cursor(hwnd, self.original) {
                return None;
            }
            (unsafe { SetCursorPos(parked_at.x, parked_at.y) } != 0).then_some(Parked {
                hwnd: hwnd as usize, original: self.original, parked_at,
            })
        }
    }

    impl Parked {
        pub fn restore(self) {
            if unsafe { GetForegroundWindow() as usize } == self.hwnd && cursor_position() == Some(self.parked_at) {
                unsafe { SetCursorPos(self.original.x, self.original.y); }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const TV: Bounds = Bounds { left: 0, top: 0, right: 3840, bottom: 2160 };
        const LEFT: Bounds = Bounds { left: -1920, top: 0, right: 0, bottom: 1040 };
        const RIGHT: Bounds = Bounds { left: 3840, top: -600, right: 5280, bottom: 1920 };
        const DESKTOP: Bounds = Bounds { left: -1920, top: -600, right: 5280, bottom: 2160 };

        #[test]
        fn parks_inside_the_nearest_secondary_work_area() {
            let destination = parking_position(Point { x: 200, y: 1200 }, TV, DESKTOP, [RIGHT, LEFT].into_iter()).unwrap();
            assert!(LEFT.contains(destination));
            assert_eq!(destination, Point { x: -25, y: 1015 });
        }

        #[test]
        fn preserves_single_display_and_locked_cursor_positions() {
            let point = Point { x: 100, y: 100 };
            assert_eq!(parking_position(point, TV, DESKTOP, std::iter::empty()), None);
            assert_eq!(parking_position(point, TV, TV, [LEFT, RIGHT].into_iter()), None);
            assert_eq!(parking_position(point, TV, DESKTOP, [TV].into_iter()), None);
        }
    }
}
