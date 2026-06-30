#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClipboardState {
    text: Option<String>,
    owner: Option<String>,
    generation: u64,
}

impl ClipboardState {
    pub fn set_text(&mut self, owner: impl Into<String>, text: impl Into<String>) {
        self.text = Some(text.into());
        self.owner = Some(owner.into());
        self.generation += 1;
    }

    pub fn clear(&mut self) {
        self.text = None;
        self.owner = None;
        self.generation += 1;
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardSmokeReport {
    pub set_ok: bool,
    pub replace_ok: bool,
    pub clear_ok: bool,
    pub generation: u64,
}

impl ClipboardSmokeReport {
    pub fn passed(self) -> bool {
        self.set_ok && self.replace_ok && self.clear_ok && self.generation == 3
    }
}

pub fn run_clipboard_smoke() -> ClipboardSmokeReport {
    let mut clipboard = ClipboardState::default();

    clipboard.set_text("terminal", "hello from terminal");
    let set_ok = clipboard.text() == Some("hello from terminal")
        && clipboard.owner() == Some("terminal")
        && clipboard.generation() == 1;

    clipboard.set_text("browser", "copied url");
    let replace_ok = clipboard.text() == Some("copied url")
        && clipboard.owner() == Some("browser")
        && clipboard.generation() == 2;

    clipboard.clear();
    let clear_ok = clipboard.text().is_none() && clipboard.owner().is_none();

    ClipboardSmokeReport {
        set_ok,
        replace_ok,
        clear_ok,
        generation: clipboard.generation(),
    }
}

#[cfg(test)]
mod tests {
    use super::{run_clipboard_smoke, ClipboardState};

    #[test]
    fn tracks_text_owner_and_generation() {
        let mut clipboard = ClipboardState::default();

        clipboard.set_text("terminal", "hello");
        clipboard.set_text("browser", "url");

        assert_eq!(clipboard.text(), Some("url"));
        assert_eq!(clipboard.owner(), Some("browser"));
        assert_eq!(clipboard.generation(), 2);
    }

    #[test]
    fn clipboard_smoke_passes() {
        let report = run_clipboard_smoke();

        assert!(report.passed(), "{report:?}");
    }
}
