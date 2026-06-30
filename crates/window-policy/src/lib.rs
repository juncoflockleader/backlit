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
        });
        self.focused = Some(id);
        id
    }

    pub fn remove_window(&mut self, id: WindowId) -> Option<Window> {
        let index = self.windows.iter().position(|window| window.id == id)?;
        let removed = self.windows.remove(index);

        if self.focused == Some(id) {
            self.focused = self.windows.last().map(|window| window.id);
        }

        Some(removed)
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
        if self.windows.is_empty() {
            self.focused = None;
            return None;
        }

        let next_index = match self.focused {
            Some(id) => self
                .windows
                .iter()
                .position(|window| window.id == id)
                .map(|index| (index + 1) % self.windows.len())
                .unwrap_or(0),
            None => 0,
        };

        let id = self.windows[next_index].id;
        self.focused = Some(id);
        Some(id)
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

    pub fn focused(&self) -> Option<WindowId> {
        self.focused
    }

    pub fn windows(&self) -> &[Window] {
        &self.windows
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowPolicy, WindowState};

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
    fn cycles_focus_in_window_order() {
        let mut policy = WindowPolicy::default();
        let first = policy.add_window("terminal", (800, 600));
        let second = policy.add_window("browser", (1200, 800));

        assert_eq!(policy.cycle_focus_forward(), Some(first));
        assert_eq!(policy.cycle_focus_forward(), Some(second));
    }

    #[test]
    fn updates_window_state() {
        let mut policy = WindowPolicy::default();
        let id = policy.add_window("video", (1280, 720));

        assert!(policy.set_state(id, WindowState::Fullscreen));
        assert_eq!(policy.windows()[0].state, WindowState::Fullscreen);
    }
}
