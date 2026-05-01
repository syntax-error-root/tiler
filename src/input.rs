use crate::layout;

#[derive(PartialEq, Debug)]
pub enum InputAction {
    SendToPTY(Vec<u8>),
    SplitHorizontal,
    SplitVertical,
    Navigate(layout::Direction),
}

pub fn handle_input(bytes: &[u8]) -> Option<InputAction> {
    if bytes.len() >= 2 && bytes[0] == 18 {
        match bytes[1] {
            72 => return Some(InputAction::SplitHorizontal),
            86 => return Some(InputAction::SplitVertical),
            65 => return Some(InputAction::Navigate(layout::Direction::Up)),
            66 => return Some(InputAction::Navigate(layout::Direction::Down)),
            68 => return Some(InputAction::Navigate(layout::Direction::Left)),
            67 => return Some(InputAction::Navigate(layout::Direction::Right)),
            _ => return Some(InputAction::SendToPTY(bytes.to_vec())),
        }
    }
    
    Some(InputAction::SendToPTY(bytes.to_vec()))
}

pub fn read_key() -> Option<Vec<u8>> {
    use std::io::{self, Read};
    
    let mut buffer = [0u8; 1];
    let mut stdin = io::stdin();
    if stdin.read_exact(&mut buffer).is_ok() {
        Some(vec![buffer[0]])
    } else {
        None
    }
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
        let result = handle_input(&[18, 72]);
        assert_eq!(result, Some(InputAction::SplitHorizontal));
    }

    #[test]
    fn test_split_vertical() {
        let result = handle_input(&[18, 86]);
        assert_eq!(result, Some(InputAction::SplitVertical));
    }

    #[test]
    fn test_navigate_up() {
        let result = handle_input(&[18, 65]);
        assert_eq!(result, Some(InputAction::Navigate(layout::Direction::Up)));
    }
}
