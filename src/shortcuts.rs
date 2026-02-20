use macroquad::prelude::KeyCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    SaveChart,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    ToggleHitsound,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 7] = [
        ShortcutAction::SaveChart,
        ShortcutAction::Undo,
        ShortcutAction::Redo,
        ShortcutAction::Cut,
        ShortcutAction::Copy,
        ShortcutAction::Paste,
        ShortcutAction::ToggleHitsound,
    ];

    pub fn is_editable(self) -> bool {
        matches!(self, ShortcutAction::ToggleHitsound)
    }

    pub fn default_chord(self) -> KeyChord {
        match self {
            ShortcutAction::SaveChart => KeyChord::new(true, false, false, ShortcutKey::S),
            ShortcutAction::Undo => KeyChord::new(true, false, false, ShortcutKey::Z),
            ShortcutAction::Redo => KeyChord::new(true, false, false, ShortcutKey::Y),
            ShortcutAction::Cut => KeyChord::new(true, false, false, ShortcutKey::X),
            ShortcutAction::Copy => KeyChord::new(true, false, false, ShortcutKey::C),
            ShortcutAction::Paste => KeyChord::new(true, false, false, ShortcutKey::V),
            ShortcutAction::ToggleHitsound => KeyChord::new(false, false, false, ShortcutKey::H),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyChord {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub shift: bool,
    pub key: ShortcutKey,
}

impl KeyChord {
    pub const fn new(ctrl: bool, alt: bool, shift: bool, key: ShortcutKey) -> Self {
        Self {
            ctrl,
            alt,
            shift,
            key,
        }
    }

    pub fn display(self) -> String {
        let mut parts: Vec<&str> = Vec::with_capacity(4);
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(self.key.display_name());
        parts.join("+")
    }

    pub fn is_pressed(
        self,
        key_pressed: impl Fn(KeyCode) -> bool,
        key_down: impl Fn(KeyCode) -> bool,
    ) -> bool {
        let ctrl_down = key_down(KeyCode::LeftControl) || key_down(KeyCode::RightControl);
        let alt_down = key_down(KeyCode::LeftAlt) || key_down(KeyCode::RightAlt);
        let shift_down = key_down(KeyCode::LeftShift) || key_down(KeyCode::RightShift);

        ctrl_down == self.ctrl
            && alt_down == self.alt
            && shift_down == self.shift
            && key_pressed(self.key.to_key_code())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutKey {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
}

impl ShortcutKey {
    pub const ALL: [ShortcutKey; 26] = [
        ShortcutKey::A,
        ShortcutKey::B,
        ShortcutKey::C,
        ShortcutKey::D,
        ShortcutKey::E,
        ShortcutKey::F,
        ShortcutKey::G,
        ShortcutKey::H,
        ShortcutKey::I,
        ShortcutKey::J,
        ShortcutKey::K,
        ShortcutKey::L,
        ShortcutKey::M,
        ShortcutKey::N,
        ShortcutKey::O,
        ShortcutKey::P,
        ShortcutKey::Q,
        ShortcutKey::R,
        ShortcutKey::S,
        ShortcutKey::T,
        ShortcutKey::U,
        ShortcutKey::V,
        ShortcutKey::W,
        ShortcutKey::X,
        ShortcutKey::Y,
        ShortcutKey::Z,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            ShortcutKey::A => "A",
            ShortcutKey::B => "B",
            ShortcutKey::C => "C",
            ShortcutKey::D => "D",
            ShortcutKey::E => "E",
            ShortcutKey::F => "F",
            ShortcutKey::G => "G",
            ShortcutKey::H => "H",
            ShortcutKey::I => "I",
            ShortcutKey::J => "J",
            ShortcutKey::K => "K",
            ShortcutKey::L => "L",
            ShortcutKey::M => "M",
            ShortcutKey::N => "N",
            ShortcutKey::O => "O",
            ShortcutKey::P => "P",
            ShortcutKey::Q => "Q",
            ShortcutKey::R => "R",
            ShortcutKey::S => "S",
            ShortcutKey::T => "T",
            ShortcutKey::U => "U",
            ShortcutKey::V => "V",
            ShortcutKey::W => "W",
            ShortcutKey::X => "X",
            ShortcutKey::Y => "Y",
            ShortcutKey::Z => "Z",
        }
    }

    pub fn to_key_code(self) -> KeyCode {
        match self {
            ShortcutKey::A => KeyCode::A,
            ShortcutKey::B => KeyCode::B,
            ShortcutKey::C => KeyCode::C,
            ShortcutKey::D => KeyCode::D,
            ShortcutKey::E => KeyCode::E,
            ShortcutKey::F => KeyCode::F,
            ShortcutKey::G => KeyCode::G,
            ShortcutKey::H => KeyCode::H,
            ShortcutKey::I => KeyCode::I,
            ShortcutKey::J => KeyCode::J,
            ShortcutKey::K => KeyCode::K,
            ShortcutKey::L => KeyCode::L,
            ShortcutKey::M => KeyCode::M,
            ShortcutKey::N => KeyCode::N,
            ShortcutKey::O => KeyCode::O,
            ShortcutKey::P => KeyCode::P,
            ShortcutKey::Q => KeyCode::Q,
            ShortcutKey::R => KeyCode::R,
            ShortcutKey::S => KeyCode::S,
            ShortcutKey::T => KeyCode::T,
            ShortcutKey::U => KeyCode::U,
            ShortcutKey::V => KeyCode::V,
            ShortcutKey::W => KeyCode::W,
            ShortcutKey::X => KeyCode::X,
            ShortcutKey::Y => KeyCode::Y,
            ShortcutKey::Z => KeyCode::Z,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShortcutBindings {
    #[serde(default)]
    custom: HashMap<ShortcutAction, KeyChord>,
}

impl ShortcutBindings {
    pub fn chord_for(&self, action: ShortcutAction) -> KeyChord {
        if !action.is_editable() {
            return action.default_chord();
        }
        self.custom
            .get(&action)
            .copied()
            .unwrap_or_else(|| action.default_chord())
    }

    pub fn set_chord(&mut self, action: ShortcutAction, chord: KeyChord) -> bool {
        if !action.is_editable() {
            return false;
        }
        if chord == action.default_chord() {
            self.custom.remove(&action);
        } else {
            self.custom.insert(action, chord);
        }
        true
    }

    pub fn reset_chord(&mut self, action: ShortcutAction) -> bool {
        if !action.is_editable() {
            return false;
        }
        self.custom.remove(&action).is_some()
    }

    pub fn is_pressed(
        &self,
        action: ShortcutAction,
        key_pressed: impl Fn(KeyCode) -> bool,
        key_down: impl Fn(KeyCode) -> bool,
    ) -> bool {
        self.chord_for(action).is_pressed(key_pressed, key_down)
    }
}

pub fn detect_key_chord(
    key_pressed: impl Fn(KeyCode) -> bool,
    key_down: impl Fn(KeyCode) -> bool,
) -> Option<KeyChord> {
    let ctrl = key_down(KeyCode::LeftControl) || key_down(KeyCode::RightControl);
    let alt = key_down(KeyCode::LeftAlt) || key_down(KeyCode::RightAlt);
    let shift = key_down(KeyCode::LeftShift) || key_down(KeyCode::RightShift);

    for key in ShortcutKey::ALL {
        if key_pressed(key.to_key_code()) {
            return Some(KeyChord::new(ctrl, alt, shift, key));
        }
    }
    None
}
