#[derive(PartialEq, Debug)]
pub enum Action {
    Write(char),
    MoveCursor(usize, usize),
    SetFgColor(Color),
    SetBgColor(Color),
    Reset,
    ClearLine,
    ClearScreen,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Color {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

pub fn parse(input: &str) -> Vec<Action> {
    let mut actions = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1B' {
            if let Some(&'[') = chars.peek() {
                chars.next();
                if let Some(seq) = parse_escape_sequence(&mut chars) {
                    actions.push(seq);
                }
            }
        } else if ch != '\r' && ch != '\n' {
            actions.push(Action::Write(ch));
        }
    }

    actions
}

fn parse_escape_sequence(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<Action> {
    let mut params = String::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() || ch == ';' {
            params.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    if let Some(end_char) = chars.next() {
        match end_char {
            'H' => {
                let parts: Vec<usize> = params.split(';')
                    .map(|s| s.parse().unwrap_or(1))
                    .collect();
                let row = parts.get(0).copied().unwrap_or(1).saturating_sub(1);
                let col = parts.get(1).copied().unwrap_or(1).saturating_sub(1);
                Some(Action::MoveCursor(row, col))
            }
            'K' => Some(Action::ClearLine),
            'J' => Some(Action::ClearScreen),
            'm' => {
                let codes: Vec<u32> = params.split(';')
                    .filter_map(|s| s.parse().ok())
                    .collect();
                parse_color_codes(&codes)
            }
            _ => None,
        }
    } else {
        None
    }
}

fn parse_color_codes(codes: &[u32]) -> Option<Action> {
    for &code in codes {
        match code {
            0 => return Some(Action::Reset),
            30 => return Some(Action::SetFgColor(Color::Black)),
            31 => return Some(Action::SetFgColor(Color::Red)),
            32 => return Some(Action::SetFgColor(Color::Green)),
            33 => return Some(Action::SetFgColor(Color::Yellow)),
            34 => return Some(Action::SetFgColor(Color::Blue)),
            35 => return Some(Action::SetFgColor(Color::Magenta)),
            36 => return Some(Action::SetFgColor(Color::Cyan)),
            37 => return Some(Action::SetFgColor(Color::White)),
            40 => return Some(Action::SetBgColor(Color::Black)),
            41 => return Some(Action::SetBgColor(Color::Red)),
            42 => return Some(Action::SetBgColor(Color::Green)),
            43 => return Some(Action::SetBgColor(Color::Yellow)),
            44 => return Some(Action::SetBgColor(Color::Blue)),
            45 => return Some(Action::SetBgColor(Color::Magenta)),
            46 => return Some(Action::SetBgColor(Color::Cyan)),
            47 => return Some(Action::SetBgColor(Color::White)),
            _ => continue,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let result = parse("hello");
        assert_eq!(result, vec![Action::Write('h'), Action::Write('e'), Action::Write('l'), Action::Write('l'), Action::Write('o')]);
    }

    #[test]
    fn test_cursor_movement() {
        let result = parse("\x1B[2;3H");
        assert_eq!(result, vec![Action::MoveCursor(1, 2)]);
    }

    #[test]
    fn test_color_change() {
        let result = parse("\x1B[31m");
        assert_eq!(result, vec![Action::SetFgColor(Color::Red)]);
    }

    #[test]
    fn test_mixed_content() {
        let result = parse("AB\x1B[31mC\x1B[0mD");
        assert_eq!(result, vec![
            Action::Write('A'),
            Action::Write('B'),
            Action::SetFgColor(Color::Red),
            Action::Write('C'),
            Action::Reset,
            Action::Write('D'),
        ]);
    }
}
