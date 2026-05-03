use sdl2::keyboard::{Keycode, Mod};
use crate::layout::Direction;

#[derive(Debug, PartialEq)]
pub enum InputAction {
    ForwardToPty(Vec<u8>),
    SplitHorizontal,
    SplitVertical,
    Navigate(Direction),
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    ScrollUp(usize),
    ScrollDown(usize),
    Quit,
    Nothing,
}

pub fn handle_key(
    keycode: Option<Keycode>,
    keymod: Mod,
    ctrl_a_pending: bool,
) -> (InputAction, bool) {
    let kmod = keymod;
    let ctrl = kmod.contains(Mod::LCTRLMOD) || kmod.contains(Mod::RCTRLMOD);
    let alt = kmod.contains(Mod::LALTMOD) || kmod.contains(Mod::RALTMOD);
    let shift = kmod.contains(Mod::LSHIFTMOD) || kmod.contains(Mod::RSHIFTMOD);

    if ctrl_a_pending {
        if let Some(kc) = keycode {
            let action = match kc {
                Keycode::A => Some(InputAction::ForwardToPty(vec![1])),
                Keycode::H => Some(InputAction::SplitHorizontal),
                Keycode::V => Some(InputAction::SplitVertical),
                Keycode::J => Some(InputAction::Navigate(Direction::Down)),
                Keycode::K => Some(InputAction::Navigate(Direction::Up)),
                Keycode::L => Some(InputAction::Navigate(Direction::Right)),
                Keycode::P => Some(InputAction::Navigate(Direction::Left)),
                Keycode::T => Some(InputAction::NewTab),
                Keycode::W => Some(InputAction::CloseTab),
                Keycode::N => Some(InputAction::NextTab),
                Keycode::B => Some(InputAction::PrevTab),
                _ => None,
            };
            if let Some(a) = action {
                return (a, false);
            }
        }
        // Unknown prefix command — send Ctrl+A + the key
        let mut bytes = vec![1];
        if let Some(kc) = keycode {
            bytes.extend(key_to_pty_bytes(kc, ctrl, alt, shift));
        }
        return (InputAction::ForwardToPty(bytes), false);
    }

    // Ctrl+A prefix detection
    if ctrl && keycode == Some(Keycode::A) {
        return (InputAction::Nothing, true); // ctrl_a_pending = true
    }

    // Shift+PageUp/PageDown scrolls terminal scrollback
    if shift && !ctrl && !alt {
        if keycode == Some(Keycode::PageUp) {
            return (InputAction::ScrollUp(10), false);
        }
        if keycode == Some(Keycode::PageDown) {
            return (InputAction::ScrollDown(10), false);
        }
    }

    // Ctrl+C quit
    if ctrl && keycode == Some(Keycode::C) {
        return (InputAction::Quit, false);
    }

    // Regular key -> forward to PTY
    if let Some(kc) = keycode {
        let bytes = key_to_pty_bytes(kc, ctrl, alt, shift);
        if !bytes.is_empty() {
            return (InputAction::ForwardToPty(bytes), false);
        }
    }

    (InputAction::Nothing, false)
}

pub fn key_to_pty_bytes(keycode: Keycode, ctrl: bool, alt: bool, shift: bool) -> Vec<u8> {
    if ctrl {
        return ctrl_key_bytes(keycode);
    }

    let bytes = match keycode {
        Keycode::Return | Keycode::KpEnter => vec![13],
        Keycode::Backspace => vec![127],
        Keycode::Tab => vec![9],
        Keycode::Escape => vec![27],
        Keycode::Up => vec![27, 91, 65],
        Keycode::Down => vec![27, 91, 66],
        Keycode::Right => vec![27, 91, 67],
        Keycode::Left => vec![27, 91, 68],
        Keycode::Home => vec![27, 91, 72],
        Keycode::End => vec![27, 91, 70],
        Keycode::PageUp => vec![27, 91, 53, 126],
        Keycode::PageDown => vec![27, 91, 54, 126],
        Keycode::Delete => vec![27, 91, 51, 126],
        Keycode::Insert => vec![27, 91, 50, 126],
        Keycode::F1 => vec![27, 79, 80],
        Keycode::F2 => vec![27, 79, 81],
        Keycode::F3 => vec![27, 79, 82],
        Keycode::F4 => vec![27, 79, 83],
        Keycode::F5 => vec![27, 91, 49, 53, 126],
        Keycode::F6 => vec![27, 91, 49, 55, 126],
        Keycode::F7 => vec![27, 91, 49, 56, 126],
        Keycode::F8 => vec![27, 91, 49, 57, 126],
        Keycode::F9 => vec![27, 91, 50, 48, 126],
        Keycode::F10 => vec![27, 91, 50, 49, 126],
        Keycode::F11 => vec![27, 91, 50, 51, 126],
        Keycode::F12 => vec![27, 91, 50, 52, 126],
        kc => {
            let name = kc.name();
            let ch = if name.len() == 1 {
                let mut c = name.chars().next().unwrap();
                if !shift {
                    c = c.to_lowercase().next().unwrap();
                }
                c
            } else {
                return vec![];
            };
            let mut buf = [0u8; 4];
            let len = ch.encode_utf8(&mut buf).len();
            buf[..len].to_vec()
        }
    };

    if alt && !bytes.is_empty() && bytes[0] != 27 {
        let mut alt_bytes = vec![27];
        alt_bytes.extend_from_slice(&bytes);
        alt_bytes
    } else {
        bytes
    }
}

fn ctrl_key_bytes(keycode: Keycode) -> Vec<u8> {
    let name = keycode.name();
    if name.len() == 1 {
        if let Some(c) = name.chars().next() {
            let lower = c.to_ascii_lowercase();
            if lower >= 'a' && lower <= 'z' {
                return vec![(lower as u8) - b'a' + 1];
            }
        }
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctrl_a_enters_prefix_mode() {
        let (action, pending) = handle_key(
            Some(Keycode::A),
            Mod::LCTRLMOD,
            false,
        );
        assert_eq!(action, InputAction::Nothing);
        assert!(pending);
    }

    #[test]
    fn test_prefix_then_split_h() {
        let (_, pending) = handle_key(Some(Keycode::A), Mod::LCTRLMOD, false);
        assert!(pending);
        let (action, pending) = handle_key(Some(Keycode::H), Mod::empty(), pending);
        assert_eq!(action, InputAction::SplitHorizontal);
        assert!(!pending);
    }

    #[test]
    fn test_prefix_then_split_v() {
        let (_, pending) = handle_key(Some(Keycode::A), Mod::LCTRLMOD, false);
        let (action, _) = handle_key(Some(Keycode::V), Mod::empty(), pending);
        assert_eq!(action, InputAction::SplitVertical);
    }

    #[test]
    fn test_prefix_then_new_tab() {
        let (_, pending) = handle_key(Some(Keycode::A), Mod::LCTRLMOD, false);
        let (action, _) = handle_key(Some(Keycode::T), Mod::empty(), pending);
        assert_eq!(action, InputAction::NewTab);
    }

    #[test]
    fn test_prefix_then_literal_a() {
        let (_, pending) = handle_key(Some(Keycode::A), Mod::LCTRLMOD, false);
        let (action, _) = handle_key(Some(Keycode::A), Mod::empty(), pending);
        assert_eq!(action, InputAction::ForwardToPty(vec![1]));
    }

    #[test]
    fn test_ctrl_c_quits() {
        let (action, _) = handle_key(Some(Keycode::C), Mod::LCTRLMOD, false);
        assert_eq!(action, InputAction::Quit);
    }

    #[test]
    fn test_regular_char_forwarded() {
        let (action, _) = handle_key(Some(Keycode::X), Mod::empty(), false);
        match action {
            InputAction::ForwardToPty(bytes) => assert_eq!(bytes, b"x"),
            _ => panic!("Expected ForwardToPty"),
        }
    }

    #[test]
    fn test_shift_char() {
        let (action, _) = handle_key(Some(Keycode::X), Mod::LSHIFTMOD, false);
        match action {
            InputAction::ForwardToPty(bytes) => assert_eq!(bytes, b"X"),
            _ => panic!("Expected ForwardToPty"),
        }
    }

    #[test]
    fn test_arrow_keys() {
        let (action, _) = handle_key(Some(Keycode::Up), Mod::empty(), false);
        match action {
            InputAction::ForwardToPty(bytes) => assert_eq!(bytes, vec![27, 91, 65]),
            _ => panic!("Expected ForwardToPty"),
        }
    }

    #[test]
    fn test_enter_key() {
        let (action, _) = handle_key(Some(Keycode::Return), Mod::empty(), false);
        match action {
            InputAction::ForwardToPty(bytes) => assert_eq!(bytes, vec![13]),
            _ => panic!("Expected ForwardToPty"),
        }
    }

    #[test]
    fn test_alt_char() {
        let (action, _) = handle_key(Some(Keycode::X), Mod::LALTMOD, false);
        match action {
            InputAction::ForwardToPty(bytes) => assert_eq!(bytes, vec![27, b'x']),
            _ => panic!("Expected ForwardToPty"),
        }
    }

    #[test]
    fn test_ctrl_char_byte() {
        let bytes = key_to_pty_bytes(Keycode::D, true, false, false);
        assert_eq!(bytes, vec![4]); // Ctrl+D = EOT
    }

    #[test]
    fn test_prefix_then_unknown_key() {
        let (_, pending) = handle_key(Some(Keycode::A), Mod::LCTRLMOD, false);
        let (action, _) = handle_key(Some(Keycode::X), Mod::empty(), pending);
        // Should send Ctrl+A byte + 'x'
        match action {
            InputAction::ForwardToPty(bytes) => {
                assert_eq!(bytes[0], 1);
                assert_eq!(bytes[1], b'x');
            }
            _ => panic!("Expected ForwardToPty"),
        }
    }

    #[test]
    fn test_prefix_navigation() {
        let (_, pending) = handle_key(Some(Keycode::A), Mod::LCTRLMOD, false);
        let (action, _) = handle_key(Some(Keycode::J), Mod::empty(), pending);
        assert_eq!(action, InputAction::Navigate(Direction::Down));

        let (_, pending) = handle_key(Some(Keycode::A), Mod::LCTRLMOD, false);
        let (action, _) = handle_key(Some(Keycode::K), Mod::empty(), pending);
        assert_eq!(action, InputAction::Navigate(Direction::Up));
    }

    #[test]
    fn test_shift_pageup_scrolls() {
        let (action, _) = handle_key(Some(Keycode::PageUp), Mod::LSHIFTMOD, false);
        assert_eq!(action, InputAction::ScrollUp(10));
    }

    #[test]
    fn test_shift_pagedown_scrolls() {
        let (action, _) = handle_key(Some(Keycode::PageDown), Mod::LSHIFTMOD, false);
        assert_eq!(action, InputAction::ScrollDown(10));
    }

    #[test]
    fn test_plain_pageup_forwards_to_pty() {
        let (action, _) = handle_key(Some(Keycode::PageUp), Mod::empty(), false);
        match action {
            InputAction::ForwardToPty(bytes) => assert_eq!(bytes, vec![27, 91, 53, 126]),
            _ => panic!("Expected ForwardToPty"),
        }
    }
}
