use std::collections::HashMap;
use termion::input::TermRead;
use std::io;

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
    panes.insert(focused_pane_id, PaneData {
        pty: initial_pty,
        cursor_x: 0,
        cursor_y: 0,
        style: buffer::Style::default(),
    });
    
    let stdin = io::stdin();
    for c in stdin.keys() {
        read_pty_output(&mut panes, &mut layout);
        
        for pane in &layout.panes {
            let is_focused = pane.id == layout.panes[layout.focused].id;
            renderer.render_pane(pane, is_focused);
        }
        
        renderer.flush();
        
        match c {
            Ok(termion::event::Key::Ctrl('c')) => {
                for (_, pane_data) in &mut panes {
                    pane_data.pty.close();
                }
                break;
            }
            Ok(key) => {
                let bytes = key_to_bytes(key);
                if let Some(action) = input::handle_input(&bytes) {
                    match action {
                        input::InputAction::SendToPTY(data) => {
                            let focused_id = layout.panes[layout.focused].id;
                            if let Some(pane_data) = panes.get_mut(&focused_id) {
                                pane_data.pty.write(&data).ok();
                            }
                        }
                        input::InputAction::SplitHorizontal => {
                            let focused_id = layout.panes[layout.focused].id;
                            if let Ok(_) = layout.split_horizontal(focused_id) {
                                spawn_new_pane(&mut panes);
                            }
                        }
                        input::InputAction::SplitVertical => {
                            let focused_id = layout.panes[layout.focused].id;
                            if let Ok(_) = layout.split_vertical(focused_id) {
                                spawn_new_pane(&mut panes);
                            }
                        }
                        input::InputAction::Navigate(dir) => {
                            layout.navigate(dir);
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
    
    renderer.show_cursor();
    renderer.clear_screen();
    
    Ok(())
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

fn key_to_bytes(key: termion::event::Key) -> Vec<u8> {
    use termion::event::Key;
    match key {
        Key::Char(c) => vec![c as u8],
        Key::Ctrl(c) => vec![c as u8 - 96],
        Key::Alt(c) => vec![27, c as u8],
        Key::Up => vec![27, 91, 65],
        Key::Down => vec![27, 91, 66],
        Key::Left => vec![27, 91, 68],
        Key::Right => vec![27, 91, 67],
        Key::Backspace => vec![127],
        _ => vec![],
    }
}
