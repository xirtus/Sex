use sex_pdx::pdx_listen;

/// Termion-compatible Event handling via Sex PDX.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Key(Key),
    Mouse(MouseEvent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,
    Left,
    Right,
    Up,
    Down,
    Ctrl(char),
    Alt(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    Press(u8, u16, u16),
    Release(u16, u16),
    Hold(u16, u16),
}

pub fn next_event() -> Option<Event> {
    let req = pdx_listen(0 /* BLOCKING */);
    
    // Phase 17: sexinput protocol
    // num 1: KEY_EVENT
    // arg0: scancode/char
    // arg1: modifiers
    match req.num {
        1 => {
            let ch = req.arg0 as u8 as char;
            Some(Event::Key(Key::Char(ch)))
        },
        2 => Some(Event::Key(Key::Backspace)),
        3 => { // Arrows (mocked)
            match req.arg0 {
                0 => Some(Event::Key(Key::Up)),
                1 => Some(Event::Key(Key::Down)),
                2 => Some(Event::Key(Key::Left)),
                3 => Some(Event::Key(Key::Right)),
                _ => None,
            }
        },
        _ => None,
    }
}

pub struct RawMode;
impl RawMode {
    pub fn enable() {
        // Call sexinput/sexdisplay to set raw mode
    }
    pub fn disable() {
    }
}
