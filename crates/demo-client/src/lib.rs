use std::fs;
use std::io;
use std::path::Path;

use backlit_window_policy::{OutputLayout, Rect, WindowPolicy, WindowState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

const BACKGROUND: Color = Color::rgb(18, 23, 33);
const PANEL: Color = Color::rgb(235, 238, 242);
const PANEL_ACCENT: Color = Color::rgb(44, 101, 141);
const LAUNCHER: Color = Color::rgb(43, 51, 66);
const WINDOW: Color = Color::rgb(248, 248, 246);
const WINDOW_SHADOW: Color = Color::rgb(7, 10, 14);
const TITLE_BAR: Color = Color::rgb(61, 84, 111);
const FOCUSED_TITLE_BAR: Color = Color::rgb(43, 112, 153);
const FOCUS_RING: Color = Color::rgb(78, 211, 119);
const TERMINAL: Color = Color::rgb(23, 27, 30);
const GRAPH: Color = Color::rgb(223, 148, 67);
const POINTER: Color = Color::rgb(255, 255, 255);
const WORKSPACE_ACTIVE: Color = Color::rgb(42, 129, 196);
const WORKSPACE_INACTIVE: Color = Color::rgb(155, 166, 181);
const OVERLAY_SURFACE: Color = Color::rgb(31, 39, 52);
const OVERLAY_SHADOW: Color = Color::rgb(5, 7, 11);
const OVERLAY_FIELD: Color = Color::rgb(237, 241, 244);
const OVERLAY_ROW: Color = Color::rgb(49, 60, 77);
const OVERLAY_SELECTED: Color = Color::rgb(43, 112, 153);
const OVERLAY_TEXT: Color = Color::rgb(224, 232, 238);

pub const DEFAULT_DEMO_WIDTH: u32 = 800;
pub const DEFAULT_DEMO_HEIGHT: u32 = 520;
pub const GOLDEN_DEMO_CHECKSUM: u64 = 5_635_038_614_353_063_225;
pub const SESSION_PREVIEW_CHECKSUM: u64 = 15_888_844_850_457_870_477;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOverlay {
    Launcher,
    AppSwitcher,
}

#[derive(Debug, Clone)]
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<Color>,
}

impl Canvas {
    pub fn new(width: u32, height: u32, color: Color) -> Self {
        let pixels = vec![color; width.saturating_mul(height) as usize];
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let index = (y * self.width + x) as usize;
        self.pixels.get(index).copied()
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }

        let index = (y * self.width + x) as usize;
        self.pixels[index] = color;
        true
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        let max_x = x.saturating_add(width).min(self.width);
        let max_y = y.saturating_add(height).min(self.height);

        for row in y..max_y {
            for column in x..max_x {
                let index = (row * self.width + column) as usize;
                self.pixels[index] = color;
            }
        }
    }

    pub fn write_ppm(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut bytes = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        bytes.reserve(self.pixels.len() * 3);

        for pixel in &self.pixels {
            bytes.extend_from_slice(&[pixel.red, pixel.green, pixel.blue]);
        }

        fs::write(path, bytes)
    }

    pub fn checksum(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325;

        for byte in self
            .width
            .to_le_bytes()
            .into_iter()
            .chain(self.height.to_le_bytes())
        {
            hash = fnv1a(hash, byte);
        }

        for pixel in &self.pixels {
            hash = fnv1a(hash, pixel.red);
            hash = fnv1a(hash, pixel.green);
            hash = fnv1a(hash, pixel.blue);
        }

        hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub non_background_pixels: u64,
    pub checksum: u64,
    pub golden_ok: bool,
    pub panel_ok: bool,
    pub launcher_ok: bool,
    pub window_ok: bool,
    pub pointer_ok: bool,
}

impl VerificationReport {
    pub fn passed(&self) -> bool {
        self.non_background_pixels > 10_000
            && self.panel_ok
            && self.launcher_ok
            && self.window_ok
            && self.pointer_ok
            && self.golden_ok
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyPreviewReport {
    pub non_background_pixels: u64,
    pub checksum: u64,
    pub golden_ok: bool,
    pub panel_ok: bool,
    pub launcher_ok: bool,
    pub window_ok: bool,
    pub pointer_ok: bool,
    pub policy_windows: u64,
    pub visible_windows: u64,
    pub focused_window_visible: bool,
    pub focused_title_bar_ok: bool,
    pub workspace_indicator_ok: bool,
}

impl PolicyPreviewReport {
    pub fn passed(&self) -> bool {
        self.non_background_pixels > 10_000
            && self.panel_ok
            && self.launcher_ok
            && self.window_ok
            && self.pointer_ok
            && self.policy_windows >= self.visible_windows
            && self.visible_windows > 0
            && self.focused_window_visible
            && self.focused_title_bar_ok
            && self.workspace_indicator_ok
            && self.golden_ok
    }
}

pub fn render_demo_gui(width: u32, height: u32) -> Canvas {
    let width = width.max(320);
    let height = height.max(220);
    let mut canvas = Canvas::new(width, height, BACKGROUND);

    draw_panel(&mut canvas);
    draw_launcher(&mut canvas);
    draw_window(&mut canvas, 132, 74, 310, 178, WINDOW, "terminal");
    draw_terminal(&mut canvas, 154, 116, 260, 96);
    draw_window(&mut canvas, 390, 132, 280, 170, WINDOW, "settings");
    draw_metrics(&mut canvas, 414, 178, 226, 68);
    draw_window(&mut canvas, 214, 260, 374, 188, WINDOW, "browser");
    draw_browser_content(&mut canvas, 238, 306, 322, 96);
    draw_pointer(
        &mut canvas,
        width.saturating_sub(168),
        height.saturating_sub(116),
    );

    canvas
}

pub fn render_policy_gui(
    width: u32,
    height: u32,
    policy: &WindowPolicy,
    layout: OutputLayout,
) -> Canvas {
    render_policy_gui_with_overlay(width, height, policy, layout, None)
}

pub fn render_policy_gui_with_overlay(
    width: u32,
    height: u32,
    policy: &WindowPolicy,
    layout: OutputLayout,
    overlay: Option<SessionOverlay>,
) -> Canvas {
    let width = width.max(320);
    let height = height.max(220);
    let mut canvas = Canvas::new(width, height, BACKGROUND);

    draw_panel(&mut canvas);
    draw_workspace_indicator(&mut canvas, policy);
    draw_launcher(&mut canvas);

    for window in policy
        .visible_windows()
        .filter(|window| window.state != WindowState::Minimized)
    {
        draw_policy_window(
            &mut canvas,
            window.geometry,
            window.title.as_str(),
            policy.focused() == Some(window.id),
            layout,
        );
    }

    match overlay {
        Some(SessionOverlay::Launcher) => draw_launcher_overlay(&mut canvas),
        Some(SessionOverlay::AppSwitcher) => draw_app_switcher_overlay(&mut canvas, policy),
        None => {}
    }

    draw_pointer(
        &mut canvas,
        width.saturating_sub(168),
        height.saturating_sub(116),
    );

    canvas
}

pub fn verify_demo_gui(canvas: &Canvas) -> VerificationReport {
    let non_background_pixels = canvas
        .pixels
        .iter()
        .filter(|pixel| **pixel != BACKGROUND)
        .count() as u64;

    let checksum = canvas.checksum();
    let golden_ok = if canvas.width == DEFAULT_DEMO_WIDTH && canvas.height == DEFAULT_DEMO_HEIGHT {
        checksum == GOLDEN_DEMO_CHECKSUM
    } else {
        true
    };

    VerificationReport {
        non_background_pixels,
        checksum,
        golden_ok,
        panel_ok: canvas.pixel(104, 18) == Some(PANEL),
        launcher_ok: canvas.pixel(10, 78) == Some(LAUNCHER),
        window_ok: canvas.pixel(364, 86) == Some(TITLE_BAR),
        pointer_ok: canvas.pixel(
            canvas.width.saturating_sub(168),
            canvas.height.saturating_sub(116),
        ) == Some(POINTER),
    }
}

pub fn verify_policy_gui(
    canvas: &Canvas,
    policy: &WindowPolicy,
    _layout: OutputLayout,
) -> PolicyPreviewReport {
    let non_background_pixels = canvas
        .pixels
        .iter()
        .filter(|pixel| **pixel != BACKGROUND)
        .count() as u64;

    let checksum = canvas.checksum();
    let golden_ok = if canvas.width == DEFAULT_DEMO_WIDTH && canvas.height == DEFAULT_DEMO_HEIGHT {
        checksum == SESSION_PREVIEW_CHECKSUM
    } else {
        true
    };
    let visible_windows = policy
        .visible_windows()
        .filter(|window| window.state != WindowState::Minimized)
        .count() as u64;
    let focused_window = policy
        .focused()
        .and_then(|focused| policy.window(focused))
        .filter(|window| window.state != WindowState::Minimized)
        .filter(|window| window.workspace == policy.active_workspace());
    let focused_window_visible = focused_window.is_some();
    let focused_title_bar_ok = focused_window
        .and_then(|window| sample_title_pixel(canvas, window.geometry))
        == Some(FOCUSED_TITLE_BAR);

    PolicyPreviewReport {
        non_background_pixels,
        checksum,
        golden_ok,
        panel_ok: canvas.pixel(104, 18) == Some(PANEL),
        launcher_ok: canvas.pixel(10, 78) == Some(LAUNCHER),
        window_ok: focused_window
            .and_then(|window| sample_window_body_pixel(canvas, window.geometry))
            == Some(WINDOW),
        pointer_ok: canvas.pixel(
            canvas.width.saturating_sub(168),
            canvas.height.saturating_sub(116),
        ) == Some(POINTER),
        policy_windows: policy.windows().len() as u64,
        visible_windows,
        focused_window_visible,
        focused_title_bar_ok,
        workspace_indicator_ok: active_workspace_pixel(canvas, policy) == Some(WORKSPACE_ACTIVE),
    }
}

pub fn verify_session_overlay(canvas: &Canvas, overlay: SessionOverlay) -> bool {
    match overlay {
        SessionOverlay::Launcher => {
            canvas.pixel(100, 68) == Some(OVERLAY_SURFACE)
                && canvas.pixel(120, 144) == Some(OVERLAY_SELECTED)
                && canvas.pixel(124, 88) == Some(OVERLAY_FIELD)
        }
        SessionOverlay::AppSwitcher => {
            let x = centered_x(canvas, 360);
            let y = 72;
            canvas.pixel(x + 10, y + 10) == Some(OVERLAY_SURFACE)
                && canvas.pixel(x + 28, y + 50) == Some(OVERLAY_SELECTED)
                && canvas.pixel(x + 48, y + 64) == Some(OVERLAY_TEXT)
        }
    }
}

fn fnv1a(hash: u64, byte: u8) -> u64 {
    (hash ^ byte as u64).wrapping_mul(0x00000100000001b3)
}

fn draw_panel(canvas: &mut Canvas) {
    canvas.fill_rect(0, 0, canvas.width(), 42, PANEL);
    canvas.fill_rect(16, 13, 74, 16, PANEL_ACCENT);

    let right = canvas.width().saturating_sub(164);
    canvas.fill_rect(right, 13, 38, 16, Color::rgb(60, 143, 83));
    canvas.fill_rect(right + 50, 13, 38, 16, Color::rgb(203, 80, 73));
    canvas.fill_rect(right + 100, 13, 48, 16, Color::rgb(32, 38, 50));
}

fn draw_workspace_indicator(canvas: &mut Canvas, policy: &WindowPolicy) {
    for index in 0..policy.workspace_count() {
        let color = if index + 1 == policy.active_workspace().0 {
            WORKSPACE_ACTIVE
        } else {
            WORKSPACE_INACTIVE
        };
        canvas.fill_rect(112 + index * 18, 16, 10, 10, color);
    }
}

fn draw_launcher(canvas: &mut Canvas) {
    canvas.fill_rect(0, 42, 74, canvas.height().saturating_sub(42), LAUNCHER);

    for index in 0..6 {
        let y = 68 + index * 42;
        canvas.fill_rect(19, y, 36, 28, Color::rgb(88, 101, 124));
        canvas.fill_rect(26, y + 7, 22, 14, Color::rgb(236, 238, 241));
    }
}

fn draw_launcher_overlay(canvas: &mut Canvas) {
    let rect = Rect::new(96, 64, 304, 224);
    fill_rect_i32(
        canvas,
        Rect::new(rect.x + 8, rect.y + 10, rect.width, rect.height),
        OVERLAY_SHADOW,
    );
    fill_rect_i32(canvas, rect, OVERLAY_SURFACE);
    fill_rect_i32(canvas, Rect::new(rect.x, rect.y, rect.width, 4), FOCUS_RING);
    fill_rect_i32(
        canvas,
        Rect::new(rect.x + 24, rect.y + 20, rect.width - 48, 32),
        OVERLAY_FIELD,
    );
    fill_rect_i32(
        canvas,
        Rect::new(rect.x + 40, rect.y + 33, rect.width - 112, 6),
        Color::rgb(93, 106, 122),
    );

    for index in 0..3 {
        let y = rect.y + 70 + index * 46;
        let color = if index == 0 {
            OVERLAY_SELECTED
        } else {
            OVERLAY_ROW
        };
        fill_rect_i32(
            canvas,
            Rect::new(rect.x + 18, y, rect.width - 36, 34),
            color,
        );
        fill_rect_i32(canvas, Rect::new(rect.x + 34, y + 10, 18, 14), OVERLAY_TEXT);
        draw_overlay_text_bars(canvas, rect.x + 68, y + 11, 8 + index as u32);
    }
}

fn draw_app_switcher_overlay(canvas: &mut Canvas, policy: &WindowPolicy) {
    let width = 360;
    let rect = Rect::new(centered_x(canvas, width) as i32, 72, width as i32, 126);
    fill_rect_i32(
        canvas,
        Rect::new(rect.x + 8, rect.y + 10, rect.width, rect.height),
        OVERLAY_SHADOW,
    );
    fill_rect_i32(canvas, rect, OVERLAY_SURFACE);
    fill_rect_i32(canvas, Rect::new(rect.x, rect.y, rect.width, 4), FOCUS_RING);
    draw_overlay_text_bars(canvas, rect.x + 18, rect.y + 18, 10);

    let focused = policy.focused();
    for (index, window) in policy.visible_windows().take(3).enumerate() {
        let x = rect.x + 18 + index as i32 * 108;
        let color = if focused == Some(window.id) {
            OVERLAY_SELECTED
        } else {
            OVERLAY_ROW
        };
        fill_rect_i32(canvas, Rect::new(x, rect.y + 48, 92, 58), color);
        fill_rect_i32(canvas, Rect::new(x + 12, rect.y + 60, 24, 18), OVERLAY_TEXT);
        draw_overlay_text_bars(canvas, x + 12, rect.y + 86, window.title.len() as u32);
    }
}

fn draw_policy_window(
    canvas: &mut Canvas,
    geometry: Rect,
    title: &str,
    focused: bool,
    layout: OutputLayout,
) {
    if geometry.width <= 0 || geometry.height <= 0 {
        return;
    }

    let clipped = clip_rect(canvas, geometry);
    if clipped.is_none() {
        return;
    }

    if geometry != layout.output {
        fill_rect_i32(
            canvas,
            Rect::new(
                geometry.x + 6,
                geometry.y + 8,
                geometry.width,
                geometry.height,
            ),
            WINDOW_SHADOW,
        );
    }

    fill_rect_i32(canvas, geometry, WINDOW);

    let title_height = geometry.height.min(32);
    let title_color = if focused {
        FOCUSED_TITLE_BAR
    } else {
        TITLE_BAR
    };
    fill_rect_i32(
        canvas,
        Rect::new(geometry.x, geometry.y, geometry.width, title_height),
        title_color,
    );

    if focused {
        draw_focus_ring(canvas, geometry);
    }

    fill_rect_i32(
        canvas,
        Rect::new(geometry.x + 12, geometry.y + 11, 10, 10),
        Color::rgb(232, 87, 74),
    );
    fill_rect_i32(
        canvas,
        Rect::new(geometry.x + 29, geometry.y + 11, 10, 10),
        Color::rgb(235, 181, 82),
    );
    fill_rect_i32(
        canvas,
        Rect::new(geometry.x + 46, geometry.y + 11, 10, 10),
        Color::rgb(82, 168, 93),
    );
    draw_label_bars_i32(canvas, geometry.x + 72, geometry.y + 12, title.len() as u32);
    draw_policy_window_content(canvas, geometry, title);
}

fn draw_focus_ring(canvas: &mut Canvas, geometry: Rect) {
    fill_rect_i32(
        canvas,
        Rect::new(geometry.x, geometry.y, geometry.width, 3),
        FOCUS_RING,
    );
    fill_rect_i32(
        canvas,
        Rect::new(geometry.x, geometry.y, 3, geometry.height),
        FOCUS_RING,
    );
    fill_rect_i32(
        canvas,
        Rect::new(
            geometry.x,
            geometry.y + geometry.height - 3,
            geometry.width,
            3,
        ),
        FOCUS_RING,
    );
    fill_rect_i32(
        canvas,
        Rect::new(
            geometry.x + geometry.width - 3,
            geometry.y,
            3,
            geometry.height,
        ),
        FOCUS_RING,
    );
}

fn draw_policy_window_content(canvas: &mut Canvas, geometry: Rect, title: &str) {
    let content = Rect::new(
        geometry.x + 22,
        geometry.y + 50,
        geometry.width - 44,
        geometry.height - 70,
    );

    if content.width <= 0 || content.height <= 0 {
        return;
    }

    if title.contains("terminal") {
        draw_terminal_content(canvas, content);
    } else if title.contains("settings") {
        draw_settings_content(canvas, content);
    } else {
        draw_browser_content_i32(canvas, content);
    }
}

fn draw_terminal_content(canvas: &mut Canvas, content: Rect) {
    fill_rect_i32(canvas, content, TERMINAL);

    for index in 0..5 {
        let y = content.y + 14 + index * 15;
        if y + 5 > content.y + content.height {
            break;
        }
        fill_rect_i32(
            canvas,
            Rect::new(content.x + 14, y, 12, 5),
            Color::rgb(82, 213, 112),
        );
        fill_rect_i32(
            canvas,
            Rect::new(content.x + 34, y, 90 + index * 18, 5),
            Color::rgb(217, 225, 220),
        );
    }
}

fn draw_settings_content(canvas: &mut Canvas, content: Rect) {
    fill_rect_i32(canvas, content, Color::rgb(234, 236, 231));

    for index in 0..6 {
        let bar_height = 12 + index * 6;
        let bar_x = content.x + 18 + index * 28;
        fill_rect_i32(
            canvas,
            Rect::new(
                bar_x,
                content.y + content.height - 12 - bar_height,
                15,
                bar_height,
            ),
            GRAPH,
        );
    }
}

fn draw_browser_content_i32(canvas: &mut Canvas, content: Rect) {
    fill_rect_i32(canvas, content, Color::rgb(236, 241, 244));
    fill_rect_i32(
        canvas,
        Rect::new(content.x + 18, content.y + 18, content.width - 36, 16),
        PANEL_ACCENT,
    );

    for index in 0..4 {
        fill_rect_i32(
            canvas,
            Rect::new(
                content.x + 20,
                content.y + 48 + index * 14,
                content.width - 44 - index * 24,
                6,
            ),
            Color::rgb(99, 110, 122),
        );
    }
}

fn draw_window(
    canvas: &mut Canvas,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    body: Color,
    label: &str,
) {
    canvas.fill_rect(x + 6, y + 8, width, height, WINDOW_SHADOW);
    canvas.fill_rect(x, y, width, height, body);
    canvas.fill_rect(x, y, width, 32, TITLE_BAR);
    canvas.fill_rect(x + 12, y + 11, 10, 10, Color::rgb(232, 87, 74));
    canvas.fill_rect(x + 29, y + 11, 10, 10, Color::rgb(235, 181, 82));
    canvas.fill_rect(x + 46, y + 11, 10, 10, Color::rgb(82, 168, 93));
    draw_label_bars(canvas, x + 72, y + 12, label.len() as u32);
}

fn draw_label_bars(canvas: &mut Canvas, x: u32, y: u32, count: u32) {
    for index in 0..count.min(12) {
        let width = 4 + (index % 3) * 3;
        canvas.fill_rect(x + index * 10, y, width, 8, Color::rgb(232, 238, 246));
    }
}

fn draw_terminal(canvas: &mut Canvas, x: u32, y: u32, width: u32, height: u32) {
    canvas.fill_rect(x, y, width, height, TERMINAL);

    for index in 0..5 {
        let y = y + 14 + index * 15;
        canvas.fill_rect(x + 14, y, 12, 5, Color::rgb(82, 213, 112));
        canvas.fill_rect(x + 34, y, 118 + index * 19, 5, Color::rgb(217, 225, 220));
    }
}

fn draw_metrics(canvas: &mut Canvas, x: u32, y: u32, width: u32, height: u32) {
    canvas.fill_rect(x, y, width, height, Color::rgb(234, 236, 231));

    for index in 0..7 {
        let bar_height = 12 + index * 6;
        let bar_x = x + 18 + index * 28;
        canvas.fill_rect(bar_x, y + height - 12 - bar_height, 15, bar_height, GRAPH);
    }
}

fn draw_browser_content(canvas: &mut Canvas, x: u32, y: u32, width: u32, height: u32) {
    canvas.fill_rect(x, y, width, height, Color::rgb(236, 241, 244));
    canvas.fill_rect(x + 18, y + 18, width.saturating_sub(36), 16, PANEL_ACCENT);

    for index in 0..4 {
        canvas.fill_rect(
            x + 20,
            y + 48 + index * 14,
            width.saturating_sub(44 + index * 24),
            6,
            Color::rgb(99, 110, 122),
        );
    }
}

fn draw_pointer(canvas: &mut Canvas, x: u32, y: u32) {
    for row in 0..28 {
        let width = (row / 2).max(1);
        canvas.fill_rect(x, y + row, width, 1, POINTER);
    }

    canvas.fill_rect(x + 9, y + 20, 12, 5, Color::rgb(30, 34, 41));
}

fn draw_label_bars_i32(canvas: &mut Canvas, x: i32, y: i32, count: u32) {
    for index in 0..count.min(12) {
        let width = 4 + (index % 3) * 3;
        fill_rect_i32(
            canvas,
            Rect::new(x + (index * 10) as i32, y, width as i32, 8),
            Color::rgb(232, 238, 246),
        );
    }
}

fn draw_overlay_text_bars(canvas: &mut Canvas, x: i32, y: i32, count: u32) {
    for index in 0..count.min(12) {
        let width = 6 + (index % 2) * 5;
        fill_rect_i32(
            canvas,
            Rect::new(x + (index * 12) as i32, y, width as i32, 7),
            OVERLAY_TEXT,
        );
    }
}

fn fill_rect_i32(canvas: &mut Canvas, rect: Rect, color: Color) {
    let Some((x, y, width, height)) = clip_rect(canvas, rect) else {
        return;
    };
    canvas.fill_rect(x, y, width, height, color);
}

fn clip_rect(canvas: &Canvas, rect: Rect) -> Option<(u32, u32, u32, u32)> {
    let min_x = rect.x.max(0).min(canvas.width() as i32);
    let min_y = rect.y.max(0).min(canvas.height() as i32);
    let max_x = rect
        .x
        .saturating_add(rect.width)
        .max(0)
        .min(canvas.width() as i32);
    let max_y = rect
        .y
        .saturating_add(rect.height)
        .max(0)
        .min(canvas.height() as i32);

    if max_x <= min_x || max_y <= min_y {
        return None;
    }

    Some((
        min_x as u32,
        min_y as u32,
        (max_x - min_x) as u32,
        (max_y - min_y) as u32,
    ))
}

fn sample_title_pixel(canvas: &Canvas, geometry: Rect) -> Option<Color> {
    sample_rect_pixel(canvas, Rect::new(geometry.x + 8, geometry.y + 8, 1, 1))
}

fn sample_window_body_pixel(canvas: &Canvas, geometry: Rect) -> Option<Color> {
    sample_rect_pixel(canvas, Rect::new(geometry.x + 8, geometry.y + 40, 1, 1))
}

fn active_workspace_pixel(canvas: &Canvas, policy: &WindowPolicy) -> Option<Color> {
    let workspace_index = policy.active_workspace().0.saturating_sub(1);
    canvas.pixel(114 + workspace_index * 18, 18)
}

fn centered_x(canvas: &Canvas, width: u32) -> u32 {
    canvas.width().saturating_sub(width) / 2
}

fn sample_rect_pixel(canvas: &Canvas, rect: Rect) -> Option<Color> {
    let (x, y, _, _) = clip_rect(canvas, rect)?;
    canvas.pixel(x, y)
}

#[cfg(test)]
mod tests {
    use super::{
        render_demo_gui, render_policy_gui, verify_demo_gui, verify_policy_gui, Color, BACKGROUND,
        FOCUSED_TITLE_BAR, GOLDEN_DEMO_CHECKSUM, SESSION_PREVIEW_CHECKSUM,
    };
    use super::{render_policy_gui_with_overlay, verify_session_overlay, SessionOverlay};
    use backlit_window_policy::{OutputLayout, WindowPolicy, WorkspaceId};

    #[test]
    fn renders_minimum_size_preview() {
        let canvas = render_demo_gui(100, 100);

        assert_eq!(canvas.width(), 320);
        assert_eq!(canvas.height(), 220);
    }

    #[test]
    fn verifies_expected_gui_regions() {
        let canvas = render_demo_gui(800, 520);
        let report = verify_demo_gui(&canvas);

        assert!(report.passed(), "{report:?}");
        assert!(report.golden_ok, "{report:?}");
        assert_eq!(report.checksum, GOLDEN_DEMO_CHECKSUM);
        assert_eq!(canvas.pixel(104, 18), Some(Color::rgb(235, 238, 242)));
    }

    #[test]
    fn renders_policy_preview_from_visible_workspace() {
        let layout = OutputLayout::new(800, 520, 42);
        let mut policy = WindowPolicy::default();
        let terminal = policy.add_window("terminal", (310, 178));
        let settings = policy.add_window("settings", (280, 170));
        let browser = policy.add_window("browser", (374, 188));
        assert!(policy.move_window(terminal, 132, 74));
        assert!(policy.move_window(settings, 390, 132));
        assert!(policy.move_window(browser, 214, 260));

        let canvas = render_policy_gui(800, 520, &policy, layout);
        let report = verify_policy_gui(&canvas, &policy, layout);

        assert!(report.passed(), "{report:?}");
        assert_eq!(report.visible_windows, 3);
        assert_eq!(report.policy_windows, 3);
        assert!(report.focused_window_visible);
        assert_eq!(canvas.pixel(222, 268), Some(FOCUSED_TITLE_BAR));
        assert_eq!(report.checksum, SESSION_PREVIEW_CHECKSUM);
    }

    #[test]
    fn hides_windows_from_inactive_workspaces() {
        let layout = OutputLayout::new(800, 520, 42);
        let mut policy = WindowPolicy::default();
        let hidden = policy.add_window("hidden", (180, 120));
        assert!(policy.move_window(hidden, 520, 360));
        assert!(policy.move_window_to_workspace(hidden, WorkspaceId(2)));

        let canvas = render_policy_gui(800, 520, &policy, layout);

        assert_eq!(policy.visible_windows().count(), 0);
        assert_eq!(canvas.pixel(528, 368), Some(BACKGROUND));
    }

    #[test]
    fn renders_launcher_overlay() {
        let layout = OutputLayout::new(800, 520, 42);
        let mut policy = WindowPolicy::default();
        policy.add_window("terminal", (310, 178));

        let canvas = render_policy_gui_with_overlay(
            800,
            520,
            &policy,
            layout,
            Some(SessionOverlay::Launcher),
        );

        assert!(verify_session_overlay(&canvas, SessionOverlay::Launcher));
    }

    #[test]
    fn renders_app_switcher_overlay() {
        let layout = OutputLayout::new(800, 520, 42);
        let mut policy = WindowPolicy::default();
        policy.add_window("terminal", (310, 178));
        policy.add_window("settings", (280, 170));
        policy.add_window("browser", (374, 188));
        assert!(policy.focus(policy.windows()[0].id));

        let canvas = render_policy_gui_with_overlay(
            800,
            520,
            &policy,
            layout,
            Some(SessionOverlay::AppSwitcher),
        );

        assert!(verify_session_overlay(&canvas, SessionOverlay::AppSwitcher));
    }
}
