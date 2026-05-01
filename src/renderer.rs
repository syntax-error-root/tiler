use termion::raw::IntoRawMode;
use termion::screen::{AlternateScreen, IntoAlternateScreen};
use std::io::{self, Write};
use std::os::unix::io::AsRawFd;

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

    pub fn render_cell(&mut self, x: usize, y: usize, cell: &crate::buffer::Cell) {
        self.move_cursor(x as u16, y as u16);
        
        let fg = self.color_to_termion(cell.style.fg_color);
        let bg = self.color_to_termion_bg(cell.style.bg_color);
        
        write!(self.stdout, "{}{}{}", fg, bg, cell.ch).ok();
        if cell.style.bold {
            write!(self.stdout, "{}", termion::style::Reset).ok();
        }
    }

    pub fn render_pane(&mut self, pane: &crate::layout::Pane, is_focused: bool) {
        for y in 0..pane.height {
            for x in 0..pane.width {
                let cell = pane.buffer.get(x, y).unwrap();
                self.move_cursor((pane.x + x) as u16, (pane.y + y) as u16);
                
                let fg = self.color_to_termion(cell.style.fg_color);
                let bg = if is_focused {
                    termion::color::Bg(termion::color::AnsiValue(0))
                } else {
                    self.color_to_termion_bg(cell.style.bg_color)
                };
                
                write!(self.stdout, "{}{}{}", fg, bg, cell.ch).ok();
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
    }

    pub fn flush(&mut self) {
        self.stdout.flush().ok();
    }

    fn color_to_termion(&self, color: crate::buffer::Color) -> termion::color::Fg<termion::color::AnsiValue> {
        use termion::color;
        use crate::buffer::Color;
        
        color::Fg(match color {
            Color::Default => color::AnsiValue(0),
            Color::Black => color::AnsiValue(0),
            Color::Red => color::AnsiValue(1),
            Color::Green => color::AnsiValue(2),
            Color::Yellow => color::AnsiValue(3),
            Color::Blue => color::AnsiValue(4),
            Color::Magenta => color::AnsiValue(5),
            Color::Cyan => color::AnsiValue(6),
            Color::White => color::AnsiValue(7),
        })
    }

    fn color_to_termion_bg(&self, color: crate::buffer::Color) -> termion::color::Bg<termion::color::AnsiValue> {
        use termion::color;
        use crate::buffer::Color;
        
        color::Bg(match color {
            Color::Default => color::AnsiValue(0),
            Color::Black => color::AnsiValue(0),
            Color::Red => color::AnsiValue(1),
            Color::Green => color::AnsiValue(2),
            Color::Yellow => color::AnsiValue(3),
            Color::Blue => color::AnsiValue(4),
            Color::Magenta => color::AnsiValue(5),
            Color::Cyan => color::AnsiValue(6),
            Color::White => color::AnsiValue(7),
        })
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
    fn test_render_cell() {
        let mut renderer = Renderer::new().unwrap();
        let cell = crate::buffer::Cell {
            ch: 'A',
            style: crate::buffer::Style::default(),
        };
        renderer.render_cell(0, 0, &cell);
    }

    #[test]
    fn test_clear_screen() {
        let mut renderer = Renderer::new().unwrap();
        renderer.clear_screen();
    }
}
