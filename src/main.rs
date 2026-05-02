use std::collections::HashMap;
use std::io;
use std::os::unix::io::{AsRawFd, BorrowedFd};

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
    let initial_pty = pty::PTY::new(
        std::env::var("SHELL")
            .unwrap_or_else(|_| "bash".to_string())
            .as_str(),
        &[],
    )?;
    panes.insert(
        focused_pane_id,
        PaneData {
            pty: initial_pty,
            cursor_x: 0,
            cursor_y: 0,
            style: buffer::Style::default(),
        },
    );

    let stdin_fd = io::stdin().as_raw_fd();
    let mut poller = polling::Poller::new().map_err(|e| e.to_string())?;
    let mut stdin_buf = [0u8; 4096];

    unsafe {
        let stdin_borrowed = BorrowedFd::borrow_raw(stdin_fd);
        poller
            .add(&stdin_borrowed, polling::Event::readable(0))
            .map_err(|e| e.to_string())?;
    }

    for (&pane_id, pane_data) in &panes {
        unsafe {
            let pty_borrowed = BorrowedFd::borrow_raw(pane_data.pty.master);
            poller
                .add(&pty_borrowed, polling::Event::readable(pane_id as usize + 1))
                .map_err(|e| e.to_string())?;
        }
    }

    let mut key_buf = Vec::new();

    loop {
        read_pty_output(&mut panes, &mut layout);

        for pane in &layout.panes {
            let is_focused = pane.id == layout.panes[layout.focused].id;
            renderer.render_pane(pane, is_focused);
        }
        renderer.flush();

        let mut events = polling::Events::new();
        poller
            .wait(&mut events, Some(std::time::Duration::from_millis(50)))
            .map_err(|e| e.to_string())?;

        for ev in events.iter() {
            if ev.key == 0 {
                let n = unsafe {
                    libc::read(
                        stdin_fd,
                        stdin_buf.as_mut_ptr() as *mut libc::c_void,
                        stdin_buf.len(),
                    )
                };
                if n > 0 {
                    key_buf.extend_from_slice(&stdin_buf[..n as usize]);
                    if let Some(action) = process_keys(&mut key_buf) {
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
                                    spawn_new_pane(&mut panes);
                                }
                            }
                            input::InputAction::SplitVertical => {
                                let focused_id = layout.panes[layout.focused].id;
                                if layout.split_vertical(focused_id).is_ok() {
                                    spawn_new_pane(&mut panes);
                                }
                            }
                            input::InputAction::Navigate(dir) => {
                                layout.navigate(dir);
                            }
                        }
                    }
                }
                unsafe {
                    let stdin_borrowed = BorrowedFd::borrow_raw(stdin_fd);
                    poller
                        .modify(&stdin_borrowed, polling::Event::readable(0))
                        .map_err(|e| e.to_string())?;
                }
            }
        }
    }
}

fn process_keys(buf: &mut Vec<u8>) -> Option<input::InputAction> {
    if buf.is_empty() {
        return None;
    }

    if buf[0] == 3 {
        buf.clear();
        std::process::exit(0);
    }

    let action = input::handle_input(buf);
    buf.clear();
    action
}

fn spawn_new_pane(panes: &mut HashMap<usize, PaneData>) {
    let new_id = panes.keys().max().copied().unwrap_or(0) + 1;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let new_pty = pty::PTY::new(shell.as_str(), &[]).unwrap();
    panes.insert(new_id, PaneData {
        pty: new_pty,
        cursor_x: 0,
        cursor_y: 0,
        style: buffer::Style::default(),
    });
}

fn read_pty_output(panes: &mut HashMap<usize, PaneData>, layout: &mut layout::Layout) {
    let mut panes_to_remove = Vec::new();

    for pane_id in panes.keys().copied().collect::<Vec<_>>() {
        if let Some(pane_data) = panes.get_mut(&pane_id) {
            if let Ok(output) = pane_data.pty.read() {
                if output.is_empty() && !pane_data.pty.is_alive() {
                    panes_to_remove.push(pane_id);
                    continue;
                }

                if let Some(pane) = layout.panes.iter_mut().find(|p| p.id == pane_id) {
                    let actions = ansi::parse(&String::from_utf8_lossy(&output));
                    process_pty_actions(pane, pane_data, &actions);
                }
            }
        }
    }

    for pane_id in panes_to_remove {
        layout.remove_pane(pane_id);
        panes.remove(&pane_id);
    }
}

fn process_pty_actions(pane: &mut layout::Pane, pane_data: &mut PaneData, actions: &[ansi::Action]) {
    for action in actions {
        match action {
            ansi::Action::Write(ch) => {
                pane.buffer.write(pane_data.cursor_x, pane_data.cursor_y, *ch, pane_data.style);
                pane_data.cursor_x += 1;
                if pane_data.cursor_x >= pane.width {
                    pane_data.cursor_x = 0;
                    pane_data.cursor_y += 1;
                }
            }
            ansi::Action::MoveCursor(x, y) => {
                pane_data.cursor_x = *x;
                pane_data.cursor_y = *y;
            }
            ansi::Action::SetFgColor(color) => {
                pane_data.style.fg_color = buffer_color_to_color(*color);
            }
            ansi::Action::SetBgColor(color) => {
                pane_data.style.bg_color = buffer_color_to_color(*color);
            }
            ansi::Action::Reset => {
                pane_data.cursor_x = 0;
                pane_data.cursor_y = 0;
            }
            ansi::Action::ClearLine => {
                for x in 0..pane.width {
                    pane.buffer.write(x, pane_data.cursor_y, ' ', pane_data.style);
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
