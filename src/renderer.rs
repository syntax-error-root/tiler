use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, IntoAlternateScreen};
use std::io::{self, Write};

pub struct Renderer {
    pub stdout: termion::raw::RawTerminal<AlternateScreen<io::Stdout>>,
}

impl Renderer {
    pub fn new() -> Result<Self, String> {
        let stdout = io::stdout().into_alternate_screen()
            .map_err(|e| format!("Failed to create alternate screen: {}", e))?;
        let stdout = stdout.into_raw_mode()
            .map_err(|e| format!("Failed to set raw mode: {}", e))?;

        Ok(Renderer { stdout })
    }

    pub fn clear_screen(&mut self) {
        write!(self.stdout, "{}", termion::clear::All).ok();
        self.stdout.flush().ok();
    }

    pub fn move_cursor(&mut self, x: u16, y: u16) {
        write!(self.stdout, "{}", termion::cursor::Goto(x + 1, y + 1)).ok();
    }

    pub fn hide_cursor(&mut self) {
        write!(self.stdout, "{}", termion::cursor::Hide).ok();
    }

    pub fn show_cursor(&mut self) {
        write!(self.stdout, "{}", termion::cursor::Show).ok();
    }

    pub fn render_pane(&mut self, pane: &crate::layout::Pane, is_focused: bool) {
        for y in 0..pane.height {
            for x in 0..pane.width {
                let cell = pane.buffer.get(x, y).unwrap();
                self.move_cursor((pane.x + x) as u16, (pane.y + y) as u16);

                let fg = fg_escape(cell.style.fg_color);
                let bg = bg_escape(cell.style.bg_color);
                let bold = if cell.style.bold { "\x1B[1m" } else { "" };

                write!(self.stdout, "{}{}{}{}", bold, fg, bg, cell.ch).ok();

                if cell.style.bold || cell.style.fg_color != crate::buffer::Color::Default
                    || cell.style.bg_color != crate::buffer::Color::Default
                {
                    write!(self.stdout, "\x1B[0m").ok();
                }
            }
        }

        if !is_focused {
            self.draw_border(pane);
        }
    }

    fn draw_border(&mut self, pane: &crate::layout::Pane) {
        let border_char = '│';
        for y in pane.y..pane.y + pane.height {
            self.move_cursor(pane.x as u16, y as u16);
            write!(self.stdout, "{}", termion::color::Fg(termion::color::Black)).ok();
            write!(self.stdout, "{}", termion::color::Bg(termion::color::White)).ok();
            write!(self.stdout, "{}", border_char).ok();
        }
        write!(self.stdout, "\x1B[0m").ok();
    }

    pub fn flush(&mut self) {
        self.stdout.flush().ok();
    }
}

fn fg_escape(color: crate::buffer::Color) -> &'static str {
    match color {
        crate::buffer::Color::Default => "",
        crate::buffer::Color::Black => "\x1B[30m",
        crate::buffer::Color::Red => "\x1B[31m",
        crate::buffer::Color::Green => "\x1B[32m",
        crate::buffer::Color::Yellow => "\x1B[33m",
        crate::buffer::Color::Blue => "\x1B[34m",
        crate::buffer::Color::Magenta => "\x1B[35m",
        crate::buffer::Color::Cyan => "\x1B[36m",
        crate::buffer::Color::White => "\x1B[37m",
    }
}

fn bg_escape(color: crate::buffer::Color) -> &'static str {
    match color {
        crate::buffer::Color::Default => "",
        crate::buffer::Color::Black => "\x1B[40m",
        crate::buffer::Color::Red => "\x1B[41m",
        crate::buffer::Color::Green => "\x1B[42m",
        crate::buffer::Color::Yellow => "\x1B[43m",
        crate::buffer::Color::Blue => "\x1B[44m",
        crate::buffer::Color::Magenta => "\x1B[45m",
        crate::buffer::Color::Cyan => "\x1B[46m",
        crate::buffer::Color::White => "\x1B[47m",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new().unwrap();
        assert!(renderer.stdout.as_raw_fd() >= 0);
    }

    #[test]
    fn test_clear_screen() {
        let mut renderer = Renderer::new().unwrap();
        renderer.clear_screen();
    }
}
