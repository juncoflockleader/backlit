use backlit_launcher::LaunchTarget;
use backlit_shortcuts::{resolve_shortcut, ShortcutAction};
use backlit_window_policy::{OutputLayout, Rect, WindowId, WindowPolicy, WindowState};

const TITLE_BAR_HEIGHT: i32 = 32;
const RESIZE_GRIP: i32 = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Shortcut(String),
    PointerMotion {
        x: i32,
        y: i32,
    },
    PointerButton {
        button: PointerButton,
        state: ButtonState,
    },
}

impl InputEvent {
    pub fn shortcut(value: impl Into<String>) -> Self {
        Self::Shortcut(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutedAction {
    Ignored,
    PointerMoved {
        x: i32,
        y: i32,
    },
    FocusWindow {
        window: WindowId,
    },
    MoveBegin {
        window: WindowId,
    },
    WindowMoved {
        window: WindowId,
        x: i32,
        y: i32,
    },
    ResizeBegin {
        window: WindowId,
    },
    WindowResized {
        window: WindowId,
        width: i32,
        height: i32,
    },
    PointerGrabEnd,
    Shortcut {
        action: ShortcutAction,
    },
    LaunchTarget {
        target: LaunchTarget,
    },
    AppSwitcher {
        focused: Option<WindowId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PointerPosition {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointerGrab {
    Move {
        window: WindowId,
        offset_x: i32,
        offset_y: i32,
    },
    Resize {
        window: WindowId,
        origin: Rect,
        start: PointerPosition,
    },
}

#[derive(Debug, Clone)]
pub struct InputRouter {
    policy: WindowPolicy,
    layout: OutputLayout,
    pointer: PointerPosition,
    grab: Option<PointerGrab>,
}

impl InputRouter {
    pub fn new(policy: WindowPolicy, layout: OutputLayout) -> Self {
        Self {
            policy,
            layout,
            pointer: PointerPosition { x: 0, y: 0 },
            grab: None,
        }
    }

    pub fn route(&mut self, event: InputEvent) -> RoutedAction {
        match event {
            InputEvent::Shortcut(shortcut) => self.route_shortcut(shortcut.as_str()),
            InputEvent::PointerMotion { x, y } => self.route_pointer_motion(x, y),
            InputEvent::PointerButton { button, state } => self.route_pointer_button(button, state),
        }
    }

    pub fn policy(&self) -> &WindowPolicy {
        &self.policy
    }

    pub fn policy_mut(&mut self) -> &mut WindowPolicy {
        &mut self.policy
    }

    pub fn grab_active(&self) -> bool {
        self.grab.is_some()
    }

    pub fn layout(&self) -> OutputLayout {
        self.layout
    }

    fn route_shortcut(&mut self, shortcut: &str) -> RoutedAction {
        match resolve_shortcut(shortcut) {
            Some(ShortcutAction::Launch(target)) => RoutedAction::LaunchTarget { target },
            Some(ShortcutAction::AppSwitcherNext) => RoutedAction::AppSwitcher {
                focused: self.policy.cycle_focus_forward(),
            },
            Some(ShortcutAction::AppSwitcherPrevious) => RoutedAction::AppSwitcher {
                focused: self.policy.cycle_focus_backward(),
            },
            Some(action) => RoutedAction::Shortcut { action },
            None => RoutedAction::Ignored,
        }
    }

    fn route_pointer_motion(&mut self, x: i32, y: i32) -> RoutedAction {
        self.pointer = PointerPosition { x, y };

        match self.grab {
            Some(PointerGrab::Move {
                window,
                offset_x,
                offset_y,
            }) => {
                let next_x = x - offset_x;
                let next_y = y - offset_y;
                if self.policy.move_window(window, next_x, next_y) {
                    RoutedAction::WindowMoved {
                        window,
                        x: next_x,
                        y: next_y,
                    }
                } else {
                    RoutedAction::Ignored
                }
            }
            Some(PointerGrab::Resize {
                window,
                origin,
                start,
            }) => {
                let width = origin.width + x - start.x;
                let height = origin.height + y - start.y;
                if self.policy.resize_window(window, width, height) {
                    let geometry = self.policy.window(window).map(|window| window.geometry);
                    RoutedAction::WindowResized {
                        window,
                        width: geometry.map(|geometry| geometry.width).unwrap_or(0),
                        height: geometry.map(|geometry| geometry.height).unwrap_or(0),
                    }
                } else {
                    RoutedAction::Ignored
                }
            }
            None => RoutedAction::PointerMoved { x, y },
        }
    }

    fn route_pointer_button(&mut self, button: PointerButton, state: ButtonState) -> RoutedAction {
        match (button, state) {
            (PointerButton::Left, ButtonState::Released) => {
                if self.grab.take().is_some() {
                    RoutedAction::PointerGrabEnd
                } else {
                    RoutedAction::Ignored
                }
            }
            (PointerButton::Left, ButtonState::Pressed) => {
                let Some(window_id) = window_at(&self.policy, self.pointer.x, self.pointer.y)
                else {
                    return RoutedAction::Ignored;
                };
                self.policy.focus(window_id);

                let Some(window) = self.policy.window(window_id) else {
                    return RoutedAction::Ignored;
                };
                let geometry = window.geometry;

                if in_resize_grip(geometry, self.pointer.x, self.pointer.y) {
                    self.grab = Some(PointerGrab::Resize {
                        window: window_id,
                        origin: geometry,
                        start: self.pointer,
                    });
                    RoutedAction::ResizeBegin { window: window_id }
                } else if in_title_bar(geometry, self.pointer.x, self.pointer.y) {
                    self.grab = Some(PointerGrab::Move {
                        window: window_id,
                        offset_x: self.pointer.x - geometry.x,
                        offset_y: self.pointer.y - geometry.y,
                    });
                    RoutedAction::MoveBegin { window: window_id }
                } else {
                    RoutedAction::FocusWindow { window: window_id }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputSmokeReport {
    pub terminal_launch_resolved: bool,
    pub windows_after_terminal_launch: u64,
    pub app_switcher_changed_focus: bool,
    pub pointer_focus_window: bool,
    pub pointer_move_window: bool,
    pub pointer_resize_window: bool,
    pub pointer_grab_ended: bool,
    pub final_focus: u64,
    pub final_width: u64,
    pub final_height: u64,
}

impl InputSmokeReport {
    pub fn passed(self) -> bool {
        self.terminal_launch_resolved
            && self.windows_after_terminal_launch == 4
            && self.app_switcher_changed_focus
            && self.pointer_focus_window
            && self.pointer_move_window
            && self.pointer_resize_window
            && self.pointer_grab_ended
            && self.final_focus != 0
            && self.final_width >= 392
            && self.final_height >= 268
    }
}

pub fn run_input_smoke() -> InputSmokeReport {
    let mut policy = WindowPolicy::default();
    policy.add_window("terminal", (320, 220));
    policy.add_window("settings", (280, 220));
    policy.add_window("browser", (320, 240));

    let layout = OutputLayout::new(800, 520, 42);
    let mut router = InputRouter::new(policy, layout);

    let terminal_launch_resolved = matches!(
        router.route(InputEvent::shortcut("Super+Enter")),
        RoutedAction::LaunchTarget {
            target: LaunchTarget::Terminal,
        }
    );
    let launched_window = if terminal_launch_resolved {
        router.policy_mut().add_window("terminal-2", (320, 220))
    } else {
        WindowId(0)
    };
    let windows_after_terminal_launch = router.policy().windows().len() as u64;

    let focus_before_switcher = router.policy().focused();
    let focus_after_switcher = match router.route(InputEvent::shortcut("Alt+Tab")) {
        RoutedAction::AppSwitcher { focused } => focused,
        _ => None,
    };
    let app_switcher_changed_focus =
        focus_after_switcher.is_some() && focus_after_switcher != focus_before_switcher;

    let original = router
        .policy()
        .window(launched_window)
        .map(|window| window.geometry)
        .unwrap_or(Rect::new(0, 0, 0, 0));
    let title_x = original.x + 18;
    let title_y = original.y + 12;

    router.route(InputEvent::PointerMotion {
        x: title_x,
        y: title_y,
    });
    let pointer_focus_window = matches!(
        router.route(InputEvent::PointerButton {
            button: PointerButton::Left,
            state: ButtonState::Pressed,
        }),
        RoutedAction::MoveBegin { window } if window == launched_window
    ) && router.policy().focused() == Some(launched_window);

    let moved_x = title_x + 44;
    let moved_y = title_y + 36;
    let pointer_move_window = matches!(
        router.route(InputEvent::PointerMotion {
            x: moved_x,
            y: moved_y
        }),
        RoutedAction::WindowMoved { window, x, y }
            if window == launched_window && x == original.x + 44 && y == original.y + 36
    );
    let move_grab_ended = matches!(
        router.route(InputEvent::PointerButton {
            button: PointerButton::Left,
            state: ButtonState::Released,
        }),
        RoutedAction::PointerGrabEnd
    );

    let resized_from = router
        .policy()
        .window(launched_window)
        .map(|window| window.geometry)
        .unwrap_or(original);
    let resize_x = resized_from.x + resized_from.width - 4;
    let resize_y = resized_from.y + resized_from.height - 4;

    router.route(InputEvent::PointerMotion {
        x: resize_x,
        y: resize_y,
    });
    let resize_started = matches!(
        router.route(InputEvent::PointerButton {
            button: PointerButton::Left,
            state: ButtonState::Pressed,
        }),
        RoutedAction::ResizeBegin { window } if window == launched_window
    );

    let pointer_resize_window = resize_started
        && matches!(
            router.route(InputEvent::PointerMotion {
                x: resize_x + 72,
                y: resize_y + 48,
            }),
            RoutedAction::WindowResized {
                window,
                width,
                height,
            } if window == launched_window
                && width == resized_from.width + 72
                && height == resized_from.height + 48
        );
    let resize_grab_ended = matches!(
        router.route(InputEvent::PointerButton {
            button: PointerButton::Left,
            state: ButtonState::Released,
        }),
        RoutedAction::PointerGrabEnd
    ) && !router.grab_active();

    let final_window = router.policy().window(launched_window);
    InputSmokeReport {
        terminal_launch_resolved,
        windows_after_terminal_launch,
        app_switcher_changed_focus,
        pointer_focus_window,
        pointer_move_window,
        pointer_resize_window,
        pointer_grab_ended: move_grab_ended && resize_grab_ended,
        final_focus: router.policy().focused().map(|id| id.0).unwrap_or(0),
        final_width: final_window
            .map(|window| window.geometry.width as u64)
            .unwrap_or(0),
        final_height: final_window
            .map(|window| window.geometry.height as u64)
            .unwrap_or(0),
    }
}

fn window_at(policy: &WindowPolicy, x: i32, y: i32) -> Option<WindowId> {
    policy
        .windows()
        .iter()
        .rev()
        .find(|window| window.state != WindowState::Minimized && contains(window.geometry, x, y))
        .map(|window| window.id)
}

fn contains(rect: Rect, x: i32, y: i32) -> bool {
    x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height
}

fn in_title_bar(rect: Rect, x: i32, y: i32) -> bool {
    contains(
        Rect::new(rect.x, rect.y, rect.width, TITLE_BAR_HEIGHT),
        x,
        y,
    )
}

fn in_resize_grip(rect: Rect, x: i32, y: i32) -> bool {
    contains(
        Rect::new(
            rect.x + rect.width - RESIZE_GRIP,
            rect.y + rect.height - RESIZE_GRIP,
            RESIZE_GRIP,
            RESIZE_GRIP,
        ),
        x,
        y,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        run_input_smoke, ButtonState, InputEvent, InputRouter, PointerButton, RoutedAction,
    };
    use backlit_window_policy::{OutputLayout, WindowPolicy};

    #[test]
    fn input_smoke_passes() {
        let report = run_input_smoke();

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.windows_after_terminal_launch, 4);
    }

    #[test]
    fn keyboard_shortcuts_route_to_window_policy_and_launches() {
        let mut policy = WindowPolicy::default();
        let first = policy.add_window("terminal", (320, 220));
        let second = policy.add_window("browser", (320, 240));
        let mut router = InputRouter::new(policy, OutputLayout::new(800, 520, 42));

        assert!(matches!(
            router.route(InputEvent::shortcut("Super+Enter")),
            RoutedAction::LaunchTarget { .. }
        ));
        assert!(matches!(
            router.route(InputEvent::shortcut("Alt+Tab")),
            RoutedAction::AppSwitcher {
                focused: Some(id)
            } if id == first
        ));
        assert_ne!(first, second);
    }

    #[test]
    fn pointer_drag_moves_and_resizes_a_window() {
        let mut policy = WindowPolicy::default();
        let window = policy.add_window("terminal", (320, 220));
        let mut router = InputRouter::new(policy, OutputLayout::new(800, 520, 42));

        router.route(InputEvent::PointerMotion { x: 82, y: 76 });
        assert_eq!(
            router.route(InputEvent::PointerButton {
                button: PointerButton::Left,
                state: ButtonState::Pressed,
            }),
            RoutedAction::MoveBegin { window }
        );
        assert_eq!(
            router.route(InputEvent::PointerMotion { x: 120, y: 112 }),
            RoutedAction::WindowMoved {
                window,
                x: 102,
                y: 100
            }
        );
        assert_eq!(
            router.route(InputEvent::PointerButton {
                button: PointerButton::Left,
                state: ButtonState::Released,
            }),
            RoutedAction::PointerGrabEnd
        );

        let geometry = router.policy().window(window).unwrap().geometry;
        router.route(InputEvent::PointerMotion {
            x: geometry.x + geometry.width - 4,
            y: geometry.y + geometry.height - 4,
        });
        assert_eq!(
            router.route(InputEvent::PointerButton {
                button: PointerButton::Left,
                state: ButtonState::Pressed,
            }),
            RoutedAction::ResizeBegin { window }
        );
        assert_eq!(
            router.route(InputEvent::PointerMotion {
                x: geometry.x + geometry.width + 36,
                y: geometry.y + geometry.height + 20,
            }),
            RoutedAction::WindowResized {
                window,
                width: 360,
                height: 244
            }
        );
    }
}
