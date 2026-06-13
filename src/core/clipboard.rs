use anyhow::Result;
use arboard::Clipboard;

#[allow(dead_code)]
pub struct ClipboardManager {
    clipboard: Clipboard,
}

#[allow(dead_code)]
impl ClipboardManager {
    pub fn new() -> Result<Self> {
        let clipboard = Clipboard::new()?;
        Ok(Self { clipboard })
    }

    pub fn copy(&mut self, text: &str) -> Result<()> {
        self.clipboard.set_text(text)?;
        Ok(())
    }

    pub fn paste(&mut self) -> Result<String> {
        let text = self.clipboard.get_text()?;
        Ok(text)
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize clipboard")
    }
}
