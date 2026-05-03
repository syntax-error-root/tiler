#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ClearMode {
    ToEnd = 0,
    ToStart = 1,
    All = 2,
}

#[derive(PartialEq, Debug)]
pub enum Action {
    Write(char),
    MoveCursor(usize, usize),
    CursorUp(usize),
    CursorDown(usize),
    CursorForward(usize),
    CursorBack(usize),
    SaveCursor,
    RestoreCursor,
    Newline,
    CarriageReturn,
    SetFgColor(Color),
    SetBgColor(Color),
    SetBold(bool),
    SetItalic(bool),
    SetUnderline(bool),
    Reset,
    ClearLine(ClearMode),
    ClearScreen(ClearMode),
    InsertLines(usize),
    DeleteLines(usize),
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
            } else if let Some(&']') = chars.peek() {
                chars.next();
                consume_osc(&mut chars);
            } else if let Some(&next) = chars.peek() {
                match next {
                    '7' => { chars.next(); actions.push(Action::SaveCursor); }
                    '8' => { chars.next(); actions.push(Action::RestoreCursor); }
                    _ => {}
                }
            }
        } else if ch == '\n' {
            actions.push(Action::Newline);
        } else if ch == '\r' {
            actions.push(Action::CarriageReturn);
        } else if ch == '\x08' {
            actions.push(Action::CursorBack(1));
        } else if ch.is_control() {
            // Ignore other control characters (BEL, NUL, etc.)
        } else {
            actions.push(Action::Write(ch));
        }
    }

    actions
}

/// Consume an OSC (Operating System Command) sequence.
/// OSC sequences start after \x1B] and end with BEL (\x07) or ST (\x1B\\).
/// Used for window titles, hyperlinks, etc. — silently ignored.
fn consume_osc(chars: &mut std::iter::Peekable<std::str::Chars>) {
    loop {
        match chars.next() {
            None | Some('\x07') => break,
            Some('\x1B') => {
                if let Some(&'\\') = chars.peek() {
                    chars.next();
                    break;
                }
            }
            Some(_) => continue,
        }
    }
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
            'K' => {
                let mode_n = if params.is_empty() { 0 } else { n };
                vec![Action::ClearLine(match mode_n {
                    0 => ClearMode::ToEnd,
                    1 => ClearMode::ToStart,
                    _ => ClearMode::All,
                })]
            }
            'J' => {
                let mode_n = if params.is_empty() { 0 } else { n };
                vec![Action::ClearScreen(match mode_n {
                    0 => ClearMode::ToEnd,
                    1 => ClearMode::ToStart,
                    _ => ClearMode::All,
                })]
            }
            's' => vec![Action::SaveCursor],
            'u' => vec![Action::RestoreCursor],
            'L' => vec![Action::InsertLines(n.max(1))],
            'M' => vec![Action::DeleteLines(n.max(1))],
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
            3 => actions.push(Action::SetItalic(true)),
            4 => actions.push(Action::SetUnderline(true)),
            22 => actions.push(Action::SetBold(false)),
            23 => actions.push(Action::SetItalic(false)),
            24 => actions.push(Action::SetUnderline(false)),
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
    fn test_clear_line_modes() {
        assert_eq!(parse("\x1B[K"), vec![Action::ClearLine(ClearMode::ToEnd)]);
        assert_eq!(parse("\x1B[0K"), vec![Action::ClearLine(ClearMode::ToEnd)]);
        assert_eq!(parse("\x1B[1K"), vec![Action::ClearLine(ClearMode::ToStart)]);
        assert_eq!(parse("\x1B[2K"), vec![Action::ClearLine(ClearMode::All)]);
    }

    #[test]
    fn test_clear_screen_modes() {
        assert_eq!(parse("\x1B[J"), vec![Action::ClearScreen(ClearMode::ToEnd)]);
        assert_eq!(parse("\x1B[0J"), vec![Action::ClearScreen(ClearMode::ToEnd)]);
        assert_eq!(parse("\x1B[1J"), vec![Action::ClearScreen(ClearMode::ToStart)]);
        assert_eq!(parse("\x1B[2J"), vec![Action::ClearScreen(ClearMode::All)]);
    }

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

    #[test]
    fn test_osc_title_bel() {
        let result = parse("\x1B]0;my title\x07text");
        assert_eq!(result, vec![Action::Write('t'), Action::Write('e'), Action::Write('x'), Action::Write('t')]);
    }

    #[test]
    fn test_osc_title_st() {
        let result = parse("\x1B]2;title\x1B\\ok");
        assert_eq!(result, vec![Action::Write('o'), Action::Write('k')]);
    }

    #[test]
    fn test_osc_ignored_mid_text() {
        let result = parse("before\x1B]0;title\x07after");
        assert_eq!(result, vec![
            Action::Write('b'), Action::Write('e'), Action::Write('f'), Action::Write('o'), Action::Write('r'), Action::Write('e'),
            Action::Write('a'), Action::Write('f'), Action::Write('t'), Action::Write('e'), Action::Write('r'),
        ]);
    }

    #[test]
    fn test_decsc_decrc() {
        assert_eq!(parse("\x1B7"), vec![Action::SaveCursor]);
        assert_eq!(parse("\x1B8"), vec![Action::RestoreCursor]);
    }

    #[test]
    fn test_decsc_mid_text() {
        let result = parse("A\x1B7B\x1B8C");
        assert_eq!(result, vec![
            Action::Write('A'),
            Action::SaveCursor,
            Action::Write('B'),
            Action::RestoreCursor,
            Action::Write('C'),
        ]);
    }

    #[test]
    fn test_insert_delete_lines() {
        assert_eq!(parse("\x1B[2L"), vec![Action::InsertLines(2)]);
        assert_eq!(parse("\x1B[3M"), vec![Action::DeleteLines(3)]);
        assert_eq!(parse("\x1B[L"), vec![Action::InsertLines(1)]);
        assert_eq!(parse("\x1B[M"), vec![Action::DeleteLines(1)]);
    }
}
