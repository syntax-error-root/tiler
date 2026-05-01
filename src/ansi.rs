#[derive(PartialEq, Debug)]
pub enum Action {
    Write(char),
    MoveCursor(usize, usize),
    CursorUp(usize),
    CursorDown(usize),
    CursorForward(usize),
    CursorBack(usize),
    Newline,
    CarriageReturn,
    SetFgColor(Color),
    SetBgColor(Color),
    SetBold(bool),
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
                let seq_actions = parse_escape_sequence(&mut chars);
                actions.extend(seq_actions);
            }
        } else if ch == '\n' {
            actions.push(Action::Newline);
        } else if ch == '\r' {
            actions.push(Action::CarriageReturn);
        } else {
            actions.push(Action::Write(ch));
        }
    }

    actions
}

fn parse_escape_sequence(chars: &mut std::iter::Peekable<std::str::Chars>) -> Vec<Action> {
    let mut params = String::new();
    let mut private = false;

    // Handle private mode prefix (?)
    if let Some(&'?') = chars.peek() {
        private = true;
        chars.next();
    }

    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() || ch == ';' {
            params.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    if private {
        // Consume the end character for private sequences (e.g., ?25h, ?25l)
        chars.next();
        return vec![];
    }

    if let Some(end_char) = chars.next() {
        let n: usize = params.parse().unwrap_or(1);
        match end_char {
            'H' | 'f' => {
                let parts: Vec<usize> = params.split(';')
                    .map(|s| s.parse().unwrap_or(1))
                    .collect();
                let row = parts.get(0).copied().unwrap_or(1).saturating_sub(1);
                let col = parts.get(1).copied().unwrap_or(1).saturating_sub(1);
                vec![Action::MoveCursor(row, col)]
            }
            'A' => vec![Action::CursorUp(n.max(1))],
            'B' => vec![Action::CursorDown(n.max(1))],
            'C' => vec![Action::CursorForward(n.max(1))],
            'D' => vec![Action::CursorBack(n.max(1))],
            'K' => vec![Action::ClearLine],
            'J' => vec![Action::ClearScreen],
            'm' => {
                if params.is_empty() {
                    return vec![Action::Reset];
                }
                let codes: Vec<u32> = params.split(';')
                    .filter_map(|s| s.parse().ok())
                    .collect();
                parse_color_codes(&codes)
            }
            _ => vec![],
        }
    } else {
        vec![]
    }
}

fn parse_color_codes(codes: &[u32]) -> Vec<Action> {
    let mut actions = Vec::new();
    for &code in codes {
        match code {
            0 => actions.push(Action::Reset),
            1 => actions.push(Action::SetBold(true)),
            22 => actions.push(Action::SetBold(false)),
            30 => actions.push(Action::SetFgColor(Color::Black)),
            31 => actions.push(Action::SetFgColor(Color::Red)),
            32 => actions.push(Action::SetFgColor(Color::Green)),
            33 => actions.push(Action::SetFgColor(Color::Yellow)),
            34 => actions.push(Action::SetFgColor(Color::Blue)),
            35 => actions.push(Action::SetFgColor(Color::Magenta)),
            36 => actions.push(Action::SetFgColor(Color::Cyan)),
            37 => actions.push(Action::SetFgColor(Color::White)),
            39 => actions.push(Action::SetFgColor(Color::Default)),
            40 => actions.push(Action::SetBgColor(Color::Black)),
            41 => actions.push(Action::SetBgColor(Color::Red)),
            42 => actions.push(Action::SetBgColor(Color::Green)),
            43 => actions.push(Action::SetBgColor(Color::Yellow)),
            44 => actions.push(Action::SetBgColor(Color::Blue)),
            45 => actions.push(Action::SetBgColor(Color::Magenta)),
            46 => actions.push(Action::SetBgColor(Color::Cyan)),
            47 => actions.push(Action::SetBgColor(Color::White)),
            49 => actions.push(Action::SetBgColor(Color::Default)),
            _ => {}
        }
    }
    actions
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

    #[test]
    fn test_compound_colors() {
        let result = parse("\x1B[1;31;42m");
        assert_eq!(result, vec![
            Action::SetBold(true),
            Action::SetFgColor(Color::Red),
            Action::SetBgColor(Color::Green),
        ]);
    }

    #[test]
    fn test_newline() {
        let result = parse("A\nB");
        assert_eq!(result, vec![
            Action::Write('A'),
            Action::Newline,
            Action::Write('B'),
        ]);
    }
}
