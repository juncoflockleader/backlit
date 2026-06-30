#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputLayout {
    pub output: Rect,
    pub panel_height: i32,
}

impl OutputLayout {
    pub const fn new(width: i32, height: i32, panel_height: i32) -> Self {
        Self {
            output: Rect::new(0, 0, width, height),
            panel_height,
        }
    }

    pub const fn work_area(self) -> Rect {
        Rect::new(
            self.output.x,
            self.output.y + self.panel_height,
            self.output.width,
            self.output.height - self.panel_height,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Maximized,
    Fullscreen,
    Minimized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub id: WindowId,
    pub title: String,
    pub geometry: Rect,
    pub state: WindowState,
    restore_geometry: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct WindowPolicy {
    windows: Vec<Window>,
    focused: Option<WindowId>,
    next_id: u64,
    next_offset: i32,
}

impl Default for WindowPolicy {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            focused: None,
            next_id: 1,
            next_offset: 0,
        }
    }
}

impl WindowPolicy {
    pub fn add_window(&mut self, title: impl Into<String>, size: (i32, i32)) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;

        let geometry = Rect::new(64 + self.next_offset, 64 + self.next_offset, size.0, size.1);
        self.next_offset = (self.next_offset + 28) % 168;

        self.windows.push(Window {
            id,
            title: title.into(),
            geometry,
            state: WindowState::Normal,
            restore_geometry: None,
        });
        self.focused = Some(id);
        id
    }

    pub fn remove_window(&mut self, id: WindowId) -> Option<Window> {
        let index = self.windows.iter().position(|window| window.id == id)?;
        let removed = self.windows.remove(index);

        if self.focused == Some(id) {
            self.focused = self.last_focusable_window();
        }

        Some(removed)
    }

    pub fn close_focused_window(&mut self) -> Option<Window> {
        let id = self.focused?;
        self.remove_window(id)
    }

    pub fn close_all_windows(&mut self) -> usize {
        let closed = self.windows.len();
        self.windows.clear();
        self.focused = None;
        closed
    }

    pub fn focus(&mut self, id: WindowId) -> bool {
        if self.windows.iter().any(|window| window.id == id) {
            self.focused = Some(id);
            true
        } else {
            false
        }
    }

    pub fn cycle_focus_forward(&mut self) -> Option<WindowId> {
        self.cycle_focus(1)
    }

    pub fn cycle_focus_backward(&mut self) -> Option<WindowId> {
        self.cycle_focus(-1)
    }

    fn cycle_focus(&mut self, direction: i32) -> Option<WindowId> {
        if self.windows.is_empty() {
            self.focused = None;
            return None;
        }

        let len = self.windows.len();
        let forward = direction >= 0;
        let start_index = self
            .focused
            .and_then(|id| self.windows.iter().position(|window| window.id == id))
            .unwrap_or(if forward { len - 1 } else { 0 });

        for offset in 1..=len {
            let next_index = if forward {
                (start_index + offset) % len
            } else {
                (start_index + len - offset) % len
            };

            if self.windows[next_index].state != WindowState::Minimized {
                let id = self.windows[next_index].id;
                self.focused = Some(id);
                return Some(id);
            }
        }

        self.focused = None;
        None
    }

    pub fn set_state(&mut self, id: WindowId, state: WindowState) -> bool {
        match self.windows.iter_mut().find(|window| window.id == id) {
            Some(window) => {
                window.state = state;
                true
            }
            None => false,
        }
    }

    pub fn move_window(&mut self, id: WindowId, x: i32, y: i32) -> bool {
        match self.window_mut(id) {
            Some(window) => {
                window.geometry.x = x;
                window.geometry.y = y;
                window.state = WindowState::Normal;
                window.restore_geometry = None;
                true
            }
            None => false,
        }
    }

    pub fn resize_window(&mut self, id: WindowId, width: i32, height: i32) -> bool {
        match self.window_mut(id) {
            Some(window) => {
                window.geometry.width = width.max(64);
                window.geometry.height = height.max(48);
                window.state = WindowState::Normal;
                window.restore_geometry = None;
                true
            }
            None => false,
        }
    }

    pub fn maximize_window(&mut self, id: WindowId, work_area: Rect) -> bool {
        self.place_window_in_state(id, work_area, WindowState::Maximized)
    }

    pub fn fullscreen_window(&mut self, id: WindowId, output_area: Rect) -> bool {
        self.place_window_in_state(id, output_area, WindowState::Fullscreen)
    }

    pub fn minimize_window(&mut self, id: WindowId) -> bool {
        match self.window_mut(id) {
            Some(window) => {
                window.state = WindowState::Minimized;
                if self.focused == Some(id) {
                    self.cycle_focus_forward();
                }
                true
            }
            None => false,
        }
    }

    pub fn restore_window(&mut self, id: WindowId) -> bool {
        match self.window_mut(id) {
            Some(window) => {
                if let Some(geometry) = window.restore_geometry.take() {
                    window.geometry = geometry;
                }
                window.state = WindowState::Normal;
                true
            }
            None => false,
        }
    }

    pub fn focused(&self) -> Option<WindowId> {
        self.focused
    }

    pub fn window(&self, id: WindowId) -> Option<&Window> {
        self.windows.iter().find(|window| window.id == id)
    }

    pub fn windows(&self) -> &[Window] {
        &self.windows
    }

    fn place_window_in_state(&mut self, id: WindowId, geometry: Rect, state: WindowState) -> bool {
        match self.window_mut(id) {
            Some(window) => {
                if window.state == WindowState::Normal {
                    window.restore_geometry = Some(window.geometry);
                }
                window.geometry = geometry;
                window.state = state;
                true
            }
            None => false,
        }
    }

    fn window_mut(&mut self, id: WindowId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|window| window.id == id)
    }

    fn last_focusable_window(&self) -> Option<WindowId> {
        self.windows
            .iter()
            .rev()
            .find(|window| window.state != WindowState::Minimized)
            .map(|window| window.id)
    }
}

#[cfg(test)]
mod tests {
    use super::{OutputLayout, WindowPolicy, WindowState};

    #[test]
    fn new_windows_take_focus() {
        let mut policy = WindowPolicy::default();
        let first = policy.add_window("terminal", (800, 600));
        let second = policy.add_window("browser", (1200, 800));

        assert_eq!(policy.focused(), Some(second));
        assert_ne!(first, second);
    }

    #[test]
    fn removing_focused_window_falls_back_to_last_window() {
        let mut policy = WindowPolicy::default();
        let first = policy.add_window("terminal", (800, 600));
        let second = policy.add_window("browser", (1200, 800));

        assert_eq!(policy.remove_window(second).unwrap().title, "browser");
        assert_eq!(policy.focused(), Some(first));
    }

    #[test]
    fn closing_all_windows_clears_focus() {
        let mut policy = WindowPolicy::default();
        policy.add_window("terminal", (800, 600));
        policy.add_window("browser", (1200, 800));

        assert_eq!(policy.close_all_windows(), 2);
        assert!(policy.windows().is_empty());
        assert_eq!(policy.focused(), None);
    }

    #[test]
    fn close_focused_window_skips_minimized_fallbacks() {
        let mut policy = WindowPolicy::default();
        let first = policy.add_window("terminal", (800, 600));
        let second = policy.add_window("settings", (720, 560));
        let third = policy.add_window("browser", (1200, 800));

        assert!(policy.minimize_window(third));
        assert!(policy.focus(second));
        assert_eq!(policy.close_focused_window().unwrap().id, second);
        assert_eq!(policy.focused(), Some(first));
    }

    #[test]
    fn cycles_focus_in_window_order() {
        let mut policy = WindowPolicy::default();
        let first = policy.add_window("terminal", (800, 600));
        let second = policy.add_window("browser", (1200, 800));

        assert_eq!(policy.cycle_focus_forward(), Some(first));
        assert_eq!(policy.cycle_focus_forward(), Some(second));
        assert_eq!(policy.cycle_focus_backward(), Some(first));
    }

    #[test]
    fn updates_window_state() {
        let mut policy = WindowPolicy::default();
        let id = policy.add_window("video", (1280, 720));

        assert!(policy.set_state(id, WindowState::Fullscreen));
        assert_eq!(policy.windows()[0].state, WindowState::Fullscreen);
    }

    #[test]
    fn moves_and_resizes_normal_windows() {
        let mut policy = WindowPolicy::default();
        let id = policy.add_window("terminal", (800, 600));

        assert!(policy.move_window(id, 120, 96));
        assert!(policy.resize_window(id, 20, 20));

        let window = policy.window(id).unwrap();
        assert_eq!(window.geometry, super::Rect::new(120, 96, 64, 48));
        assert_eq!(window.state, WindowState::Normal);
    }

    #[test]
    fn maximizes_and_restores_windows() {
        let mut policy = WindowPolicy::default();
        let id = policy.add_window("browser", (1024, 768));
        let original = policy.window(id).unwrap().geometry;
        let work_area = super::Rect::new(0, 42, 1920, 1038);

        assert!(policy.maximize_window(id, work_area));
        assert_eq!(policy.window(id).unwrap().geometry, work_area);
        assert_eq!(policy.window(id).unwrap().state, WindowState::Maximized);

        assert!(policy.restore_window(id));
        assert_eq!(policy.window(id).unwrap().geometry, original);
        assert_eq!(policy.window(id).unwrap().state, WindowState::Normal);
    }

    #[test]
    fn fullscreen_uses_output_area() {
        let mut policy = WindowPolicy::default();
        let id = policy.add_window("video", (1280, 720));
        let output = super::Rect::new(0, 0, 2560, 1440);

        assert!(policy.fullscreen_window(id, output));

        let window = policy.window(id).unwrap();
        assert_eq!(window.geometry, output);
        assert_eq!(window.state, WindowState::Fullscreen);
    }

    #[test]
    fn output_layout_reserves_panel_work_area() {
        let layout = OutputLayout::new(1920, 1080, 42);

        assert_eq!(layout.output, super::Rect::new(0, 0, 1920, 1080));
        assert_eq!(layout.work_area(), super::Rect::new(0, 42, 1920, 1038));
    }

    #[test]
    fn minimized_windows_are_skipped_by_focus_cycle() {
        let mut policy = WindowPolicy::default();
        let first = policy.add_window("terminal", (800, 600));
        let second = policy.add_window("settings", (720, 560));
        let third = policy.add_window("browser", (1200, 800));

        assert!(policy.minimize_window(third));
        assert_eq!(policy.focused(), Some(first));
        assert!(policy.minimize_window(second));
        assert_eq!(policy.cycle_focus_forward(), Some(first));

        assert!(policy.restore_window(second));
        assert!(policy.focus(second));
        assert_eq!(policy.window(second).unwrap().state, WindowState::Normal);
    }
}
