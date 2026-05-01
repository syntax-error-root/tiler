#[derive(Clone, Copy, PartialEq)]
pub struct Style {
    pub fg_color: Color,
    pub bg_color: Color,
    pub bold: bool,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            fg_color: Color::Default,
            bg_color: Color::Default,
            bold: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
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

#[derive(Clone)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            style: Style::default(),
        }
    }
}

pub struct Buffer {
    cells: Vec<Vec<Cell>>,
    pub width: usize,
    pub height: usize,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![Cell::default(); width]; height];
        Buffer { cells, width, height }
    }

    pub fn write(&mut self, x: usize, y: usize, ch: char, style: Style) {
        if x < self.width && y < self.height {
            self.cells[y][x].ch = ch;
            self.cells[y][x].style = style;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&Cell> {
        if x < self.width && y < self.height {
            Some(&self.cells[y][x])
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row {
                *cell = Cell::default();
            }
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        if n >= self.height {
            self.clear();
            return;
        }
        for y in 0..self.height - n {
            self.cells[y] = self.cells[y + n].clone();
        }
        for y in self.height - n..self.height {
            for x in 0..self.width {
                self.cells[y][x] = Cell::default();
            }
        }
    }

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        let mut new_cells = vec![vec![Cell::default(); new_width]; new_height];
        for y in 0..new_height.min(self.height) {
            for x in 0..new_width.min(self.width) {
                new_cells[y][x] = self.cells[y][x].clone();
            }
        }
        self.cells = new_cells;
        self.width = new_width;
        self.height = new_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = Buffer::new(10, 20);
        assert_eq!(buffer.width, 10);
        assert_eq!(buffer.height, 20);
    }

    #[test]
    fn test_cell_write() {
        let mut buffer = Buffer::new(5, 5);
        buffer.write(2, 3, 'A', Style::default());
        assert_eq!(buffer.get(2, 3).unwrap().ch, 'A');
    }

    #[test]
    fn test_clear() {
        let mut buffer = Buffer::new(5, 5);
        buffer.write(1, 1, 'X', Style::default());
        buffer.clear();
        assert_eq!(buffer.get(1, 1).unwrap().ch, ' ');
    }

    #[test]
    fn test_scroll_up() {
        let mut buffer = Buffer::new(5, 3);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(0, 1, 'B', Style::default());
        buffer.write(0, 2, 'C', Style::default());
        buffer.scroll_up(1);
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'B');
        assert_eq!(buffer.get(0, 1).unwrap().ch, 'C');
        assert_eq!(buffer.get(0, 2).unwrap().ch, ' ');
    }
}
