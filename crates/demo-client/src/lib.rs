use std::fs;
use std::io;
use std::path::Path;

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
const TERMINAL: Color = Color::rgb(23, 27, 30);
const GRAPH: Color = Color::rgb(223, 148, 67);
const POINTER: Color = Color::rgb(255, 255, 255);

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub non_background_pixels: u64,
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

pub fn verify_demo_gui(canvas: &Canvas) -> VerificationReport {
    let non_background_pixels = canvas
        .pixels
        .iter()
        .filter(|pixel| **pixel != BACKGROUND)
        .count() as u64;

    VerificationReport {
        non_background_pixels,
        panel_ok: canvas.pixel(104, 18) == Some(PANEL),
        launcher_ok: canvas.pixel(10, 78) == Some(LAUNCHER),
        window_ok: canvas.pixel(364, 86) == Some(TITLE_BAR),
        pointer_ok: canvas.pixel(
            canvas.width.saturating_sub(168),
            canvas.height.saturating_sub(116),
        ) == Some(POINTER),
    }
}

fn draw_panel(canvas: &mut Canvas) {
    canvas.fill_rect(0, 0, canvas.width(), 42, PANEL);
    canvas.fill_rect(16, 13, 74, 16, PANEL_ACCENT);

    let right = canvas.width().saturating_sub(164);
    canvas.fill_rect(right, 13, 38, 16, Color::rgb(60, 143, 83));
    canvas.fill_rect(right + 50, 13, 38, 16, Color::rgb(203, 80, 73));
    canvas.fill_rect(right + 100, 13, 48, 16, Color::rgb(32, 38, 50));
}

fn draw_launcher(canvas: &mut Canvas) {
    canvas.fill_rect(0, 42, 74, canvas.height().saturating_sub(42), LAUNCHER);

    for index in 0..6 {
        let y = 68 + index * 42;
        canvas.fill_rect(19, y, 36, 28, Color::rgb(88, 101, 124));
        canvas.fill_rect(26, y + 7, 22, 14, Color::rgb(236, 238, 241));
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

#[cfg(test)]
mod tests {
    use super::{render_demo_gui, verify_demo_gui, Color};

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
        assert_eq!(canvas.pixel(104, 18), Some(Color::rgb(235, 238, 242)));
    }
}
