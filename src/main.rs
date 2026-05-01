use std::collections::HashMap;
use std::io;
use std::os::unix::io::AsRawFd;

mod buffer;
mod pty;
mod layout;
mod renderer;
mod ansi;
mod input;

struct PaneData {
    pty: pty::PTY,
    cursor_x: usize,
    cursor_y: usize,
    style: buffer::Style,
}

fn main() -> Result<(), String> {
    let mut renderer = renderer::Renderer::new()?;
    renderer.hide_cursor();
    renderer.clear_screen();

    let (terminal_width, terminal_height) = termion::terminal_size()
        .map_err(|e| format!("Failed to get terminal size: {}", e))?;

    let mut layout = layout::Layout::new(terminal_width as usize, terminal_height as usize);
    let mut panes: HashMap<usize, PaneData> = HashMap::new();

    let focused_pane_id = layout.panes[layout.focused].id;
    let initial_pty = pty::PTY::new(std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string()).as_str(), &[])?;
    initial_pty.set_window_size(terminal_width, terminal_height);
    panes.insert(focused_pane_id, PaneData {
        pty: initial_pty,
        cursor_x: 0,
        cursor_y: 0,
        style: buffer::Style::default(),
    });

    let stdin_fd = io::stdin().as_raw_fd();
    let mut stdin_buffer = [0u8; 64];
    let mut sequence_buf: Vec<u8> = Vec::new();

    loop {
        // Build poll fd set: stdin + all PTY master fds
        let mut poll_fds: Vec<libc::pollfd> = Vec::new();
        poll_fds.push(libc::pollfd {
            fd: stdin_fd,
            events: libc::POLLIN,
            revents: 0,
        });
        for pane_data in panes.values() {
            poll_fds.push(libc::pollfd {
                fd: pane_data.pty.master_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
        }

        let poll_result = unsafe {
            libc::poll(poll_fds.as_mut_ptr(), poll_fds.len() as libc::nfds_t, 100)
        };

        if poll_result < 0 {
            break;
        }

        // Check stdin for input
        if poll_fds[0].revents & libc::POLLIN != 0 {
            let n = unsafe {
                libc::read(stdin_fd, stdin_buffer.as_mut_ptr() as *mut libc::c_void, stdin_buffer.len())
            };
            if n <= 0 {
                break;
            }
            sequence_buf.extend_from_slice(&stdin_buffer[..n as usize]);
            while let Some((key, consumed)) = parse_next_key(&sequence_buf) {
                sequence_buf.drain(..consumed);
                let bytes = key_to_bytes(key);
                if handle_key_action(&bytes, &mut panes, &mut layout) {
                    renderer.show_cursor();
                    renderer.clear_screen();
                    return Ok(());
                }
            }
        }

        // Read from any PTY that has data or hung up
        for i in 1..poll_fds.len() {
            let events = poll_fds[i].revents;
            if events & (libc::POLLIN | libc::POLLHUP) != 0 {
                let fd = poll_fds[i].fd;
                if let Some((&pane_id, pane_data)) = panes.iter_mut().find(|(_, pd)| pd.pty.master_fd() == fd) {
                    loop {
                        match pane_data.pty.read_nonblocking() {
                            Ok(Some(output)) => {
                                if !output.is_empty() {
                                    if let Some(pane) = layout.panes.iter_mut().find(|p| p.id == pane_id) {
                                        let actions = ansi::parse(&String::from_utf8_lossy(&output));
                                        process_pty_actions(pane, pane_data, &actions);
                                    }
                                }
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                }
            }
        }

        // Check for dead panes
        let mut panes_to_remove = Vec::new();
        for (&pane_id, pane_data) in panes.iter() {
            if !pane_data.pty.is_alive() {
                panes_to_remove.push(pane_id);
            }
        }
        for pane_id in panes_to_remove {
            layout.remove_pane(pane_id);
            panes.remove(&pane_id);
        }

        // If remove_pane created a fallback pane, spawn a PTY for it
        if panes.is_empty() && !layout.panes.is_empty() {
            let fallback_id = layout.panes[layout.focused].id;
            spawn_new_pane(&mut panes, fallback_id, &layout);
        }

        // Render all panes
        for pane in &layout.panes {
            let is_focused = pane.id == layout.panes[layout.focused].id;
            let cursor = if is_focused {
                panes.get(&pane.id).map(|pd| (pd.cursor_x, pd.cursor_y))
            } else {
                None
            };
            renderer.render_pane(pane, is_focused, cursor);
        }
        renderer.flush();
    }

    renderer.show_cursor();
    renderer.clear_screen();

    Ok(())
}

fn handle_key_action(
    bytes: &[u8],
    panes: &mut HashMap<usize, PaneData>,
    layout: &mut layout::Layout,
) -> bool {
    // Ctrl+C exit
    if bytes.len() == 1 && bytes[0] == 3 {
        for (_, pane_data) in panes.iter_mut() {
            pane_data.pty.close();
        }
        return true;
    }

    if let Some(action) = input::handle_input(bytes) {
        match action {
            input::InputAction::SendToPTY(data) => {
                let focused_id = layout.panes[layout.focused].id;
                if let Some(pane_data) = panes.get_mut(&focused_id) {
                    pane_data.pty.write(&data).ok();
                }
            }
            input::InputAction::SplitHorizontal => {
                let focused_id = layout.panes[layout.focused].id;
                if layout.split_horizontal(focused_id).is_ok() {
                    let new_id = layout.panes[layout.focused].id;
                    spawn_new_pane(panes, new_id, &layout);
                    resize_pty_for_pane(panes, focused_id, &layout);
                }
            }
            input::InputAction::SplitVertical => {
                let focused_id = layout.panes[layout.focused].id;
                if layout.split_vertical(focused_id).is_ok() {
                    let new_id = layout.panes[layout.focused].id;
                    spawn_new_pane(panes, new_id, &layout);
                    resize_pty_for_pane(panes, focused_id, &layout);
                }
            }
            input::InputAction::Navigate(dir) => {
                layout.navigate(dir);
            }
        }
    }
    false
}

fn spawn_new_pane(panes: &mut HashMap<usize, PaneData>, pane_id: usize, layout: &layout::Layout) {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let new_pty = match pty::PTY::new(shell.as_str(), &[]) {
        Ok(pty) => pty,
        Err(e) => {
            eprintln!("Failed to spawn pane: {}", e);
            return;
        }
    };
    if let Some(pane) = layout.panes.iter().find(|p| p.id == pane_id) {
        new_pty.set_window_size(pane.width as u16, pane.height as u16);
    }
    panes.insert(pane_id, PaneData {
        pty: new_pty,
        cursor_x: 0,
        cursor_y: 0,
        style: buffer::Style::default(),
    });
}

fn resize_pty_for_pane(panes: &mut HashMap<usize, PaneData>, pane_id: usize, layout: &layout::Layout) {
    if let (Some(pane), Some(pane_data)) = (
        layout.panes.iter().find(|p| p.id == pane_id),
        panes.get_mut(&pane_id),
    ) {
        pane_data.pty.set_window_size(pane.width as u16, pane.height as u16);
        pane_data.cursor_x = pane_data.cursor_x.min(pane.width.saturating_sub(1));
        pane_data.cursor_y = pane_data.cursor_y.min(pane.height.saturating_sub(1));
    }
}

fn parse_next_key(buf: &[u8]) -> Option<(termion::event::Key, usize)> {
    if buf.is_empty() {
        return None;
    }

    // Escape sequences
    if buf[0] == 27 && buf.len() >= 3 && buf[1] == b'[' {
        match buf[2] {
            b'A' => return Some((termion::event::Key::Up, 3)),
            b'B' => return Some((termion::event::Key::Down, 3)),
            b'C' => return Some((termion::event::Key::Right, 3)),
            b'D' => return Some((termion::event::Key::Left, 3)),
            b'H' => return Some((termion::event::Key::Home, 3)),
            b'F' => return Some((termion::event::Key::End, 3)),
            _ => {}
        }
    }

    // Ctrl+char
    if buf[0] < 32 && buf[0] != 27 && buf[0] != 13 && buf[0] != 10 {
        let c = (buf[0] + 96) as char;
        return Some((termion::event::Key::Ctrl(c), 1));
    }

    // Enter
    if buf[0] == 13 {
        return Some((termion::event::Key::Char('\n'), 1));
    }

    // Backspace
    if buf[0] == 127 {
        return Some((termion::event::Key::Backspace, 1));
    }

    // Alt+char
    if buf[0] == 27 && buf.len() >= 2 {
        // Distinguish Alt+key from incomplete escape sequence
        if buf[1] == b'[' && buf.len() < 4 {
            // Partial escape sequence — need more data
            // But if only \e[ is buffered and no more arrives, treat as stale
            if buf.len() == 2 {
                return None;
            }
        }
        return Some((termion::event::Key::Alt(buf[1] as char), 2));
    }

    // Bare escape key (no follow-up byte)
    if buf[0] == 27 {
        return Some((termion::event::Key::Esc, 1));
    }

    // Plain char (UTF-8)
    if buf[0] >= 32 {
        let len = match buf[0] {
            0..=0x7F => 1,
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF7 => 4,
            _ => 1,
        };
        if buf.len() >= len {
            if let Ok(s) = std::str::from_utf8(&buf[..len]) {
                if let Some(c) = s.chars().next() {
                    return Some((termion::event::Key::Char(c), len));
                }
            }
        }
        return Some((termion::event::Key::Char(buf[0] as char), 1));
    }

    None
}

fn process_pty_actions(pane: &mut layout::Pane, pane_data: &mut PaneData, actions: &[ansi::Action]) {
    for action in actions {
        match action {
            ansi::Action::Write(ch) => {
                ensure_cursor_in_bounds(pane, pane_data);
                pane.buffer.write(pane_data.cursor_x, pane_data.cursor_y, *ch, pane_data.style);
                pane_data.cursor_x += 1;
                if pane_data.cursor_x >= pane.width {
                    pane_data.cursor_x = 0;
                    pane_data.cursor_y += 1;
                }
            }
            ansi::Action::MoveCursor(row, col) => {
                pane_data.cursor_x = (*col).min(pane.width.saturating_sub(1));
                pane_data.cursor_y = (*row).min(pane.height.saturating_sub(1));
            }
            ansi::Action::CursorUp(n) => {
                pane_data.cursor_y = pane_data.cursor_y.saturating_sub(*n);
            }
            ansi::Action::CursorDown(n) => {
                pane_data.cursor_y = (pane_data.cursor_y + n).min(pane.height.saturating_sub(1));
            }
            ansi::Action::CursorForward(n) => {
                pane_data.cursor_x = (pane_data.cursor_x + n).min(pane.width.saturating_sub(1));
            }
            ansi::Action::CursorBack(n) => {
                pane_data.cursor_x = pane_data.cursor_x.saturating_sub(*n);
            }
            ansi::Action::SetFgColor(color) => {
                pane_data.style.fg_color = buffer_color_to_color(*color);
            }
            ansi::Action::SetBgColor(color) => {
                pane_data.style.bg_color = buffer_color_to_color(*color);
            }
            ansi::Action::SetBold(bold) => {
                pane_data.style.bold = *bold;
            }
            ansi::Action::Reset => {
                pane_data.style = buffer::Style::default();
            }
            ansi::Action::Newline => {
                pane_data.cursor_y += 1;
                ensure_cursor_in_bounds(pane, pane_data);
            }
            ansi::Action::CarriageReturn => {
                pane_data.cursor_x = 0;
            }
            ansi::Action::ClearLine => {
                let y = pane_data.cursor_y.min(pane.height - 1);
                for x in 0..pane.width {
                    pane.buffer.write(x, y, ' ', pane_data.style);
                }
                pane_data.cursor_x = 0;
            }
            ansi::Action::ClearScreen => {
                pane.buffer.clear();
                pane_data.cursor_x = 0;
                pane_data.cursor_y = 0;
            }
        }
    }
}

fn ensure_cursor_in_bounds(pane: &mut layout::Pane, pane_data: &mut PaneData) {
    while pane_data.cursor_y >= pane.height {
        pane.buffer.scroll_up(1);
        pane_data.cursor_y = pane.height - 1;
    }
}

fn buffer_color_to_color(color: ansi::Color) -> buffer::Color {
    use buffer::Color as BufferColor;
    use ansi::Color as AnsiColor;
    match color {
        AnsiColor::Default => BufferColor::Default,
        AnsiColor::Black => BufferColor::Black,
        AnsiColor::Red => BufferColor::Red,
        AnsiColor::Green => BufferColor::Green,
        AnsiColor::Yellow => BufferColor::Yellow,
        AnsiColor::Blue => BufferColor::Blue,
        AnsiColor::Magenta => BufferColor::Magenta,
        AnsiColor::Cyan => BufferColor::Cyan,
        AnsiColor::White => BufferColor::White,
    }
}

fn key_to_bytes(key: termion::event::Key) -> Vec<u8> {
    use termion::event::Key;
    match key {
        Key::Char(c) => {
            let mut buf = [0u8; 4];
            c.encode_utf8(&mut buf);
            buf[..c.len_utf8()].to_vec()
        }
        Key::Ctrl(c) => vec![c as u8 - 96],
        Key::Alt(c) => {
            let mut buf = vec![27];
            let mut enc = [0u8; 4];
            c.encode_utf8(&mut enc);
            buf.extend_from_slice(&enc[..c.len_utf8()]);
            buf
        }
        Key::Up => vec![27, 91, 65],
        Key::Down => vec![27, 91, 66],
        Key::Left => vec![27, 91, 68],
        Key::Right => vec![27, 91, 67],
        Key::Backspace => vec![127],
        Key::Home => vec![27, 91, 72],
        Key::End => vec![27, 91, 70],
        _ => vec![],
    }
}
