//! Key names shown on screen, from the configured keybinds.

use crate::config::Keybinds;

pub struct KeyHints {
    pub enter: String,
    pub parent: String,
    pub delete: String,
    pub zoom: String,
    pub quit: String,
    pub confirm: String,
    pub cancel: String,
}

impl KeyHints {
    pub fn new(keybinds: &Keybinds) -> Self {
        Self {
            enter: keybinds.enter.to_string(),
            parent: keybinds.parent.to_string(),
            delete: keybinds.delete.to_string(),
            zoom: format!(
                "{}/{}/{}",
                keybinds.zoom_in, keybinds.zoom_out, keybinds.reset_zoom
            ),
            quit: keybinds.quit.to_string(),
            confirm: keybinds.confirm.to_string(),
            cancel: keybinds.cancel.to_string(),
        }
    }

    /// `(y/n)`, with the configured keys.
    pub fn yes_no(&self) -> String {
        format!("({}/{})", self.confirm, self.cancel)
    }
}
