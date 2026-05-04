use std::collections::HashMap;

use sdl2::event::{Event, WindowEvent};

use tiler::ansi;
use tiler::buffer;
use tiler::config;
use tiler::input;
use tiler::layout;
use tiler::pty;
use tiler::renderer::{self, PaneData};

struct PaneState {
    pty: pty::PTY,
    cursor_x: usize,
    cursor_y: usize,
    saved_cursor: Option<(usize, usize, bool)>,
    style: buffer::Style,
    wrap_pending: bool,
    pending_utf8: Vec<u8>,
    cursor_visible: bool,
    origin_mode: bool,
    bracketed_paste: bool,
}

fn main() -> Result<(), String> {
    let cfg = config::load_config();

    let sdl_context = sdl2::init()?;
    let mut renderer = renderer::Renderer::new(&sdl_context, &cfg)?;

    let (cols, rows) = renderer.grid_size();
    let mut layout = layout::Layout::with_scrollback(cols, rows, cfg.render.scrollback_lines);
    let mut panes: HashMap<usize, PaneState> = HashMap::new();

    // Spawn initial PTY
    let initial_id = layout.focused_pane_id();
    spawn_pane(&mut panes, initial_id, &layout)?;

    let mut event_pump = sdl_context.event_pump()?;
    let key_config = input::KeyConfig::from_config(&cfg.keybinds);
    let mut ctrl_a_pending = false;
    let mut cursor_blink_counter: u32 = 0;
    let mut cursor_visible = true;
    let blink_interval = 30; // frames (~0.5s at 60fps)

    'main_loop: loop {
        // --- SDL2 Events ---
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main_loop,

                Event::KeyDown { keycode, keymod, .. } => {
                    let (action, new_pending) = input::handle_key(keycode, keymod, ctrl_a_pending, &key_config);
                    ctrl_a_pending = new_pending;

                    match action {
                        input::InputAction::Quit => break 'main_loop,

                        input::InputAction::SplitHorizontal => {
                            let focused_id = layout.focused_pane_id();
                            if layout.split_horizontal(focused_id).is_ok() {
                                let new_id = layout.focused_pane_id();
                                if spawn_pane(&mut panes, new_id, &layout).is_err() {
                                    layout.remove_pane(new_id);
                                } else {
                                    resize_pty(&mut panes, focused_id, &layout);
                                }
                            }
                        }

                        input::InputAction::SplitVertical => {
                            let focused_id = layout.focused_pane_id();
                            if layout.split_vertical(focused_id).is_ok() {
                                let new_id = layout.focused_pane_id();
                                if spawn_pane(&mut panes, new_id, &layout).is_err() {
                                    layout.remove_pane(new_id);
                                } else {
                                    resize_pty(&mut panes, focused_id, &layout);
                                }
                            }
                        }

                        input::InputAction::Navigate(dir) => {
                            layout.navigate(dir);
                        }

                        input::InputAction::NewTab => {
                            let tab_pane_id = layout.new_tab();
                            handle_resize(&renderer, &mut layout, &mut panes);
                            if spawn_pane(&mut panes, tab_pane_id, &layout).is_err() {
                                layout.close_tab();
                                handle_resize(&renderer, &mut layout, &mut panes);
                            }
                        }

                        input::InputAction::CloseTab => {
                            let tab = &layout.tabs[layout.active_tab];
                            let pane_ids: Vec<usize> = tab.panes.iter().map(|p| p.id).collect();
                            for id in pane_ids {
                                if let Some(ps) = panes.get_mut(&id) {
                                    ps.pty.close();
                                }
                                panes.remove(&id);
                            }
                            layout.close_tab();
                            handle_resize(&renderer, &mut layout, &mut panes);
                        }

                        input::InputAction::NextTab => {
                            layout.next_tab();
                        }

                        input::InputAction::PrevTab => {
                            layout.prev_tab();
                        }

                        input::InputAction::ScrollUp(n) => {
                            let focused_id = layout.focused_pane_id();
                            if let Some(pane) = layout.active_panes_mut().iter_mut().find(|p| p.id == focused_id) {
                                pane.buffer.scroll_view_up(n);
                            }
                        }

                        input::InputAction::ScrollDown(n) => {
                            let focused_id = layout.focused_pane_id();
                            if let Some(pane) = layout.active_panes_mut().iter_mut().find(|p| p.id == focused_id) {
                                pane.buffer.scroll_view_down(n);
                            }
                        }

                        input::InputAction::ForwardToPty(bytes) => {
                            let focused_id = layout.focused_pane_id();
                            if let Some(ps) = panes.get_mut(&focused_id) {
                                let _ = ps.pty.write(&bytes);
                            }
                        }

                        input::InputAction::Nothing => {}
                    }
                }

                Event::MouseWheel { y, .. } => {
                    let focused_id = layout.focused_pane_id();
                    if y > 0 {
                        if let Some(pane) = layout.active_panes_mut().iter_mut().find(|p| p.id == focused_id) {
                            pane.buffer.scroll_view_up(3);
                        }
                    } else if y < 0 {
                        if let Some(pane) = layout.active_panes_mut().iter_mut().find(|p| p.id == focused_id) {
                            pane.buffer.scroll_view_down(3);
                        }
                    }
                }

                Event::MouseButtonDown { x, y, .. } => {
                    let (cell_w, cell_h) = renderer.cell_size();
                    let col = x as usize / cell_w;
                    let row = y as usize / cell_h;
                    // Find which pane was clicked
                    let tab_bar_rows = if layout.tabs.len() > 1 { 1 } else { 0 };
                    let adj_row = row.saturating_sub(tab_bar_rows);
                    for (i, pane) in layout.active_panes().iter().enumerate() {
                        if col >= pane.x && col < pane.x + pane.width
                            && adj_row >= pane.y && adj_row < pane.y + pane.height
                        {
                            layout.active_tab_mut().focused = i;
                            break;
                        }
                    }
                }

                Event::Window { win_event: WindowEvent::Resized(_, _), .. } => {
                    handle_resize(&renderer, &mut layout, &mut panes);
                }

                _ => {}
            }
        }

        // --- Read PTY output (non-blocking) ---
        for (&pane_id, pane_state) in panes.iter_mut() {
            loop {
                match pane_state.pty.read_nonblocking() {
                    Ok(Some(output)) => {
                        if output.is_empty() {
                            break;
                        }
                        if let Some(pane) = layout.find_pane_mut(pane_id) {
                            // Prepend any leftover bytes from previous read
                            let mut combined = std::mem::take(&mut pane_state.pending_utf8);
                            combined.extend_from_slice(&output);

                            // Find the last complete UTF-8 boundary
                            let text = match std::str::from_utf8(&combined) {
                                Ok(s) => {
                                    pane_state.pending_utf8.clear();
                                    s.to_string()
                                }
                                Err(e) => {
                                    let valid_up_to = e.valid_up_to();
                                    if valid_up_to == 0 {
                                        // Entire buffer is incomplete — keep buffering
                                        // Cap at 4 bytes (max UTF-8 sequence) to prevent unbounded growth
                                        if combined.len() > 4 {
                                            combined.drain(..combined.len() - 4);
                                        }
                                        pane_state.pending_utf8 = combined;
                                        break;
                                    }
                                    let text = unsafe {
                                        std::str::from_utf8_unchecked(&combined[..valid_up_to])
                                    }.to_string();
                                    pane_state.pending_utf8 = combined[valid_up_to..].to_vec();
                                    text
                                }
                            };

                            let actions = ansi::parse(&text);
                            process_pty_actions(pane, pane_state, &actions);
                            pane.buffer.reset_scroll();
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        }

        // --- Cleanup dead panes ---
        let mut panes_to_remove = Vec::new();
        for (&pane_id, pane_state) in panes.iter_mut() {
            if !pane_state.pty.is_alive() {
                panes_to_remove.push(pane_id);
            }
        }
        for pane_id in panes_to_remove {
            layout.remove_pane(pane_id);
            panes.remove(&pane_id);
            // Resize surviving panes after neighbor absorption
            update_all_pane_sizes(&layout, &mut panes);
        }

        // If layout created a fallback pane, spawn a PTY for it
        if panes.is_empty() && !layout.active_panes().is_empty() {
            let fallback_id = layout.focused_pane_id();
            if spawn_pane(&mut panes, fallback_id, &layout).is_err() {
                layout.remove_pane(fallback_id);
            }
        }

        // Handle tabs with no living PTYs (all shells exited)
        loop {
            let dead_tab = layout.tabs.iter().enumerate().find(|(_, tab)| {
                !tab.panes.is_empty()
                    && !tab.panes.iter().any(|p| panes.contains_key(&p.id))
            });

            match dead_tab {
                None => break,
                Some((tab_idx, _)) => {
                    if layout.tabs.len() > 1 {
                        let pane_ids: Vec<usize> =
                            layout.tabs[tab_idx].panes.iter().map(|p| p.id).collect();
                        for id in pane_ids {
                            panes.remove(&id);
                        }
                        layout.active_tab = tab_idx;
                        layout.close_tab();
                    } else {
                        let pane_id = layout.focused_pane_id();
                        if spawn_pane(&mut panes, pane_id, &layout).is_err() {
                            layout.remove_pane(pane_id);
                        }
                        break;
                    }
                }
            }
        }

        // --- Cursor blink ---
        cursor_blink_counter += 1;
        if cursor_blink_counter >= blink_interval {
            cursor_blink_counter = 0;
            cursor_visible = !cursor_visible;
        }

        // --- Build PaneData for rendering ---
        let render_panes: HashMap<usize, PaneData> = panes.iter().map(|(&id, ps)| {
            (id, PaneData {
                cursor_x: ps.cursor_x,
                cursor_y: ps.cursor_y,
                cursor_visible: ps.cursor_visible,
            })
        }).collect();

        // --- Render ---
        renderer.render(&layout, &render_panes, cursor_visible);

        // --- Frame rate cap ~60fps ---
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    Ok(())
}

fn spawn_pane(
    panes: &mut HashMap<usize, PaneState>,
    pane_id: usize,
    layout: &layout::Layout,
) -> Result<(), String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let new_pty = pty::PTY::new(shell.as_str(), &[])?;
    if let Some(pane) = layout.find_pane(pane_id) {
        new_pty.set_window_size(pane.width as u16, pane.height as u16);
    }
    panes.insert(pane_id, PaneState {
        pty: new_pty,
        cursor_x: 0,
        cursor_y: 0,
        saved_cursor: None,
        style: buffer::Style::default(),
        wrap_pending: false,
        pending_utf8: Vec::new(),
        cursor_visible: true,
        origin_mode: false,
        bracketed_paste: false,
    });
    Ok(())
}

fn resize_pty(panes: &mut HashMap<usize, PaneState>, pane_id: usize, layout: &layout::Layout) {
    if let (Some(pane), Some(ps)) = (
        layout.find_pane(pane_id),
        panes.get_mut(&pane_id),
    ) {
        ps.pty.set_window_size(pane.width as u16, pane.height as u16);
        ps.cursor_x = ps.cursor_x.min(pane.width.saturating_sub(1));
        ps.cursor_y = ps.cursor_y.min(pane.height.saturating_sub(1));
    }
}

fn process_pty_actions(pane: &mut layout::Pane, ps: &mut PaneState, actions: &[ansi::Action]) {
    for action in actions {
        match action {
            ansi::Action::Write(ch) => {
                ensure_cursor_in_bounds(pane, ps);
                if ps.wrap_pending {
                    ps.wrap_pending = false;
                    ps.cursor_x = 0;
                    ps.cursor_y += 1;
                    ensure_cursor_in_bounds(pane, ps);
                }
                pane.buffer.write(ps.cursor_x, ps.cursor_y, *ch, ps.style);
                ps.cursor_x += 1;
                if ps.cursor_x >= pane.width {
                    ps.cursor_x = pane.width - 1;
                    ps.wrap_pending = true;
                }
            }
            ansi::Action::MoveCursor(row, col) => {
                ps.wrap_pending = false;
                ps.cursor_x = (*col).min(pane.width.saturating_sub(1));
                ps.cursor_y = (*row).min(pane.height.saturating_sub(1));
            }
            ansi::Action::CursorUp(n) => {
                ps.wrap_pending = false;
                ps.cursor_y = ps.cursor_y.saturating_sub(*n);
            }
            ansi::Action::CursorDown(n) => {
                ps.wrap_pending = false;
                ps.cursor_y = (ps.cursor_y + n).min(pane.height.saturating_sub(1));
            }
            ansi::Action::CursorForward(n) => {
                ps.wrap_pending = false;
                ps.cursor_x = (ps.cursor_x + n).min(pane.width.saturating_sub(1));
            }
            ansi::Action::CursorBack(n) => {
                ps.wrap_pending = false;
                ps.cursor_x = ps.cursor_x.saturating_sub(*n);
            }
            ansi::Action::SetFgColor(color) => {
                ps.style.fg_color = ansi_color_to_buffer(*color);
            }
            ansi::Action::SetBgColor(color) => {
                ps.style.bg_color = ansi_color_to_buffer(*color);
            }
            ansi::Action::SetBold(bold) => {
                ps.style.bold = *bold;
            }
            ansi::Action::SetItalic(italic) => {
                ps.style.italic = *italic;
            }
            ansi::Action::SetUnderline(underline) => {
                ps.style.underline = *underline;
            }
            ansi::Action::SetReverse(reverse) => {
                ps.style.reverse = *reverse;
            }
            ansi::Action::Reset => {
                ps.style = buffer::Style::default();
            }
            ansi::Action::Newline => {
                ps.wrap_pending = false;
                ps.cursor_y += 1;
                ensure_cursor_in_bounds(pane, ps);
            }
            ansi::Action::CarriageReturn => {
                ps.wrap_pending = false;
                ps.cursor_x = 0;
            }
            ansi::Action::SaveCursor => {
                ps.saved_cursor = Some((ps.cursor_x, ps.cursor_y, ps.wrap_pending));
            }
            ansi::Action::RestoreCursor => {
                if let Some((sx, sy, sw)) = ps.saved_cursor {
                    ps.cursor_x = sx;
                    ps.cursor_y = sy;
                    ps.wrap_pending = sw;
                } else {
                    ps.wrap_pending = false;
                }
            }
            ansi::Action::ClearLine(mode) => {
                ps.wrap_pending = false;
                let y = ps.cursor_y.min(pane.height.saturating_sub(1));
                match mode {
                    ansi::ClearMode::ToEnd => {
                        for x in ps.cursor_x..pane.width {
                            pane.buffer.write(x, y, ' ', ps.style);
                        }
                    }
                    ansi::ClearMode::ToStart => {
                        for x in 0..=ps.cursor_x.min(pane.width.saturating_sub(1)) {
                            pane.buffer.write(x, y, ' ', ps.style);
                        }
                    }
                    ansi::ClearMode::All => {
                        for x in 0..pane.width {
                            pane.buffer.write(x, y, ' ', ps.style);
                        }
                    }
                }
            }
            ansi::Action::ClearScreen(mode) => {
                ps.wrap_pending = false;
                match mode {
                    ansi::ClearMode::ToEnd => {
                        let y = ps.cursor_y.min(pane.height.saturating_sub(1));
                        for x in ps.cursor_x..pane.width {
                            pane.buffer.write(x, y, ' ', ps.style);
                        }
                        for row in (y + 1)..pane.height {
                            for x in 0..pane.width {
                                pane.buffer.write(x, row, ' ', ps.style);
                            }
                        }
                    }
                    ansi::ClearMode::ToStart => {
                        let y = ps.cursor_y.min(pane.height.saturating_sub(1));
                        for row in 0..y {
                            for x in 0..pane.width {
                                pane.buffer.write(x, row, ' ', ps.style);
                            }
                        }
                        for x in 0..=ps.cursor_x.min(pane.width.saturating_sub(1)) {
                            pane.buffer.write(x, y, ' ', ps.style);
                        }
                    }
                    ansi::ClearMode::All => {
                        for y in 0..pane.height {
                            for x in 0..pane.width {
                                pane.buffer.write(x, y, ' ', ps.style);
                            }
                        }
                    }
                }
            }
            ansi::Action::InsertLines(n) => {
                ps.wrap_pending = false;
                pane.buffer.insert_lines(ps.cursor_y, *n);
            }
            ansi::Action::DeleteLines(n) => {
                ps.wrap_pending = false;
                pane.buffer.delete_lines(ps.cursor_y, *n);
            }
            ansi::Action::InsertChars(n) => {
                ps.wrap_pending = false;
                pane.buffer.insert_chars(ps.cursor_x, ps.cursor_y, *n);
            }
            ansi::Action::DeleteChars(n) => {
                ps.wrap_pending = false;
                pane.buffer.delete_chars(ps.cursor_x, ps.cursor_y, *n);
            }
            ansi::Action::ScrollUp(n) => {
                ps.wrap_pending = false;
                pane.buffer.scroll_up(*n);
            }
            ansi::Action::ScrollDown(n) => {
                ps.wrap_pending = false;
                pane.buffer.scroll_down(*n);
            }
            ansi::Action::Tab => {
                ps.wrap_pending = false;
                let tab_width = 8;
                let next_tab = ((ps.cursor_x / tab_width) + 1) * tab_width;
                ps.cursor_x = next_tab.min(pane.width.saturating_sub(1));
            }
            ansi::Action::CursorUpAbsolute(row) => {
                ps.wrap_pending = false;
                let effective_row = if ps.origin_mode {
                    let top = pane.buffer.scroll_top();
                    (*row).min(pane.buffer.scroll_bottom().saturating_sub(top)) + top
                } else {
                    *row
                };
                ps.cursor_y = effective_row.min(pane.height.saturating_sub(1));
            }
            ansi::Action::CursorRightAbsolute(col) => {
                ps.wrap_pending = false;
                let effective_col = if ps.origin_mode {
                    let left = pane.buffer.scroll_top();
                    (*col).min(pane.width.saturating_sub(1)) + left
                } else {
                    *col
                };
                ps.cursor_x = effective_col.min(pane.width.saturating_sub(1));
            }
            ansi::Action::CursorNextLine(n) => {
                ps.wrap_pending = false;
                ps.cursor_x = 0;
                ps.cursor_y = (ps.cursor_y + n).min(pane.height.saturating_sub(1));
            }
            ansi::Action::CursorPrevLine(n) => {
                ps.wrap_pending = false;
                ps.cursor_x = 0;
                ps.cursor_y = ps.cursor_y.saturating_sub(*n);
            }
            ansi::Action::SetScrollRegion(top, bottom) => {
                ps.wrap_pending = false;
                if *top == 0 && *bottom == 0 {
                    pane.buffer.set_scroll_region(0, pane.height.saturating_sub(1));
                } else {
                    pane.buffer.set_scroll_region(*top, (*bottom).min(pane.height.saturating_sub(1)));
                }
                ps.cursor_x = 0;
                ps.cursor_y = 0;
            }
            ansi::Action::SetPrivateMode(mode, set) => {
                match mode {
                    ansi::PrivateMode::CursorVisibility => {
                        ps.cursor_visible = *set;
                    }
                    ansi::PrivateMode::OriginMode => {
                        ps.origin_mode = *set;
                        if *set {
                            let top = pane.buffer.scroll_top();
                            ps.cursor_x = 0;
                            ps.cursor_y = top;
                        } else {
                            ps.cursor_x = 0;
                            ps.cursor_y = 0;
                        }
                    }
                    ansi::PrivateMode::AlternateScreen => {
                        if *set {
                            pane.buffer.save_main_screen();
                        } else {
                            pane.buffer.restore_main_screen();
                        }
                        ps.cursor_x = 0;
                        ps.cursor_y = 0;
                    }
                    ansi::PrivateMode::BracketedPaste => {
                        ps.bracketed_paste = *set;
                    }
                }
            }
            ansi::Action::DeviceStatusReport => {
                let row = (ps.cursor_y + 1).min(pane.height);
                let col = (ps.cursor_x + 1).min(pane.width);
                let response = format!("\x1B[{};{}R", row, col);
                let _ = ps.pty.write(response.as_bytes());
            }
            ansi::Action::SetCursorStyle(_style) => {
                // Cursor style tracked but rendering not yet implemented
            }
        }
    }
}

fn ensure_cursor_in_bounds(pane: &mut layout::Pane, ps: &mut PaneState) {
    if pane.height == 0 { return; }
    while ps.cursor_y >= pane.height {
        pane.buffer.scroll_up(1);
        ps.cursor_y = pane.height - 1;
    }
}

fn ansi_color_to_buffer(color: ansi::Color) -> buffer::Color {
    use ansi::Color as AnsiColor;
    use buffer::Color as BufferColor;
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
        AnsiColor::Indexed(i) => BufferColor::Indexed(i),
        AnsiColor::Rgb(r, g, b) => BufferColor::Rgb(r, g, b),
    }
}

fn handle_resize(
    renderer: &renderer::Renderer,
    layout: &mut layout::Layout,
    panes: &mut HashMap<usize, PaneState>,
) {
    let (new_cols, new_rows) = renderer.grid_size();
    let tab_bar_rows = if layout.tabs.len() > 1 { 1 } else { 0 };
    let usable_rows = new_rows.saturating_sub(tab_bar_rows);

    if new_cols != layout.width || usable_rows != layout.height {
        layout.resize(new_cols, usable_rows);
        update_all_pane_sizes(layout, panes);
    }
}

fn update_all_pane_sizes(
    layout: &layout::Layout,
    panes: &mut HashMap<usize, PaneState>,
) {
    for tab in &layout.tabs {
        for pane in &tab.panes {
            if let Some(ps) = panes.get_mut(&pane.id) {
                ps.pty.set_window_size(pane.width as u16, pane.height as u16);
                ps.cursor_x = ps.cursor_x.min(pane.width.saturating_sub(1));
                ps.cursor_y = ps.cursor_y.min(pane.height.saturating_sub(1));
            }
        }
    }
}
