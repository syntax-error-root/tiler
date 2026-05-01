use crate::layout;

#[derive(PartialEq, Debug)]
pub enum InputAction {
    SendToPTY(Vec<u8>),
    SplitHorizontal,
    SplitVertical,
    Navigate(layout::Direction),
}

pub fn handle_input(bytes: &[u8]) -> Option<InputAction> {
    // Ctrl+A prefix (byte 1) for tiling commands, like tmux
    if bytes.len() == 2 && bytes[0] == 1 {
        match bytes[1] {
            // Ctrl+A then Ctrl+H = split horizontal
            8 => return Some(InputAction::SplitHorizontal),
            // Ctrl+A then Ctrl+V (synthetic) = split vertical
            // Use Ctrl+A then % (0x25) for vertical split
            0x25 => return Some(InputAction::SplitVertical),
            // Ctrl+A then arrow escapes use second byte
            b'h' | b'H' => return Some(InputAction::SplitHorizontal),
            b'v' | b'V' => return Some(InputAction::SplitVertical),
            // Ctrl+A then direction chars
            b'k' | b'K' => return Some(InputAction::Navigate(layout::Direction::Up)),
            b'j' | b'J' => return Some(InputAction::Navigate(layout::Direction::Down)),
            b'l' | b'L' => return Some(InputAction::Navigate(layout::Direction::Right)),
            // Ctrl+A, Ctrl+A = send literal Ctrl+A
            1 => return Some(InputAction::SendToPTY(vec![1])),
            // Ctrl+A then any other ctrl char = navigate
            16 => return Some(InputAction::Navigate(layout::Direction::Left)),
            _ => return Some(InputAction::SendToPTY(bytes.to_vec())),
        }
    }

    Some(InputAction::SendToPTY(bytes.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_key() {
        let result = handle_input(&[b'a']).unwrap();
        assert_eq!(result, InputAction::SendToPTY(vec![b'a']));
    }

    #[test]
    fn test_split_horizontal() {
        let result = handle_input(&[1, b'h']);
        assert_eq!(result, Some(InputAction::SplitHorizontal));
    }

    #[test]
    fn test_split_vertical() {
        let result = handle_input(&[1, b'v']);
        assert_eq!(result, Some(InputAction::SplitVertical));
    }

    #[test]
    fn test_navigate_up() {
        let result = handle_input(&[1, b'k']);
        assert_eq!(result, Some(InputAction::Navigate(layout::Direction::Up)));
    }

    #[test]
    fn test_literal_ctrl_a() {
        let result = handle_input(&[1, 1]);
        assert_eq!(result, Some(InputAction::SendToPTY(vec![1])));
    }
}
