use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

/// A single key chord used for an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

/// Resolved keybindings for the TUI (defaults match built-in behavior).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keybinds {
    pub quit: KeyBinding,
    pub delete: KeyBinding,
    pub move_left: KeyBinding,
    pub move_right: KeyBinding,
    pub move_up: KeyBinding,
    pub move_down: KeyBinding,
    pub enter: KeyBinding,
    pub parent: KeyBinding,
    pub zoom_in: KeyBinding,
    pub zoom_out: KeyBinding,
    pub reset_zoom: KeyBinding,
    pub confirm: KeyBinding,
    pub cancel: KeyBinding,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            quit: KeyBinding::char('q'),
            delete: KeyBinding::char('d'),
            move_left: KeyBinding::char('h'),
            move_right: KeyBinding::char('l'),
            move_up: KeyBinding::char('k'),
            move_down: KeyBinding::char('j'),
            enter: KeyBinding::key(KeyCode::Enter),
            parent: KeyBinding::key(KeyCode::Esc),
            zoom_in: KeyBinding::char('+'),
            zoom_out: KeyBinding::char('-'),
            reset_zoom: KeyBinding::char('0'),
            confirm: KeyBinding::char('y'),
            cancel: KeyBinding::char('n'),
        }
    }
}

impl KeyBinding {
    pub fn char(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
        }
    }

    pub fn key(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::NONE,
        }
    }

    pub fn ctrl_char(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
        }
    }

    pub fn shift_char(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::SHIFT,
        }
    }

    /// Parses a key string from config, e.g. `"d"`, `"enter"`, `"ctrl+c"`, `"left"`.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("key cannot be empty".into());
        }

        let lower = s.to_ascii_lowercase();
        let rest_after = |prefix: &str| lower.starts_with(prefix).then(|| &s[prefix.len()..]);
        if let Some(rest) = rest_after("ctrl+").or_else(|| rest_after("control+")) {
            // Terminals send Ctrl+C as Char('c') with CONTROL.
            let c = single_char(rest)?.to_ascii_lowercase();
            return Ok(Self::ctrl_char(c));
        }
        if let Some(rest) = rest_after("shift+") {
            // Shift+a arrives as 'A': the shift is in the character.
            let c = single_char(rest)?;
            return Ok(Self::char(c.to_uppercase().next().unwrap_or(c)));
        }

        if s.chars().count() == 1 {
            let c = s.chars().next().unwrap();
            return Ok(Self::char(c));
        }

        let code = match s.to_ascii_lowercase().as_str() {
            "enter" | "return" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "backspace" => KeyCode::Backspace,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "tab" => KeyCode::Tab,
            "space" => KeyCode::Char(' '),
            other => return Err(format!("unknown key name '{other}'")),
        };
        Ok(Self::key(code))
    }

    /// Whether `evt` is this chord. For characters SHIFT is ignored: it is
    /// already in the character (`D` arrives as Char('D') with SHIFT, `?`
    /// with SHIFT on most layouts), so a binding to "D" or "?" must match it.
    pub fn matches_event(&self, evt: &Event) -> bool {
        let ignored = match self.code {
            KeyCode::Char(_) => KeyModifiers::SHIFT,
            _ => KeyModifiers::NONE,
        };
        matches!(
            evt,
            Event::Key(KeyEvent {
                code,
                modifiers,
                ..
            }) if *code == self.code
                && modifiers.difference(ignored) == self.modifiers.difference(ignored)
        )
    }
}

/// How a binding is shown on screen: `d`, `D`, `ENTER`, `Ctrl+c`.
impl std::fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            f.write_str("Ctrl+")?;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            f.write_str("Alt+")?;
        }
        match self.code {
            KeyCode::Char(' ') => f.write_str("SPACE"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::Enter => f.write_str("ENTER"),
            KeyCode::Esc => f.write_str("ESC"),
            KeyCode::Backspace => f.write_str("BACKSPACE"),
            KeyCode::Tab => f.write_str("TAB"),
            KeyCode::Left => f.write_str("←"),
            KeyCode::Right => f.write_str("→"),
            KeyCode::Up => f.write_str("↑"),
            KeyCode::Down => f.write_str("↓"),
            other => write!(f, "{other:?}"),
        }
    }
}

impl Keybinds {
    pub fn is_quit(&self, evt: &Event) -> bool {
        self.quit.matches_event(evt) || KeyBinding::ctrl_char('c').matches_event(evt)
    }

    pub fn is_confirm(&self, evt: &Event) -> bool {
        self.confirm.matches_event(evt)
    }

    pub fn is_cancel(&self, evt: &Event) -> bool {
        self.cancel.matches_event(evt) || self.parent.matches_event(evt)
    }

    pub fn is_move_left(&self, evt: &Event) -> bool {
        self.move_left.matches_event(evt)
            || KeyBinding::key(KeyCode::Left).matches_event(evt)
            || KeyBinding::ctrl_char('b').matches_event(evt)
    }

    pub fn is_move_right(&self, evt: &Event) -> bool {
        self.move_right.matches_event(evt)
            || KeyBinding::key(KeyCode::Right).matches_event(evt)
            || KeyBinding::ctrl_char('f').matches_event(evt)
    }

    pub fn is_move_up(&self, evt: &Event) -> bool {
        self.move_up.matches_event(evt)
            || KeyBinding::key(KeyCode::Up).matches_event(evt)
            || KeyBinding::ctrl_char('p').matches_event(evt)
    }

    pub fn is_move_down(&self, evt: &Event) -> bool {
        self.move_down.matches_event(evt)
            || KeyBinding::key(KeyCode::Down).matches_event(evt)
            || KeyBinding::ctrl_char('n').matches_event(evt)
    }

    pub fn is_zoom_in(&self, evt: &Event) -> bool {
        self.zoom_in.matches_event(evt) || KeyBinding::shift_char('+').matches_event(evt)
    }

    pub fn is_enter(&self, evt: &Event) -> bool {
        self.enter.matches_event(evt) || KeyBinding::char('\n').matches_event(evt)
    }
}

fn single_char(s: &str) -> Result<char, String> {
    let mut chars = s.chars();
    let c = chars
        .next()
        .ok_or_else(|| "expected a single character".to_string())?;
    if chars.next().is_some() {
        return Err(format!("expected a single character, got '{s}'"));
    }
    Ok(c)
}
