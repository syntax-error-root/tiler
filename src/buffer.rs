use std::collections::HashSet;

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
    dirty: HashSet<(usize, usize)>,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![Cell::default(); width]; height];
        Buffer {
            cells,
            width,
            height,
            dirty: HashSet::new(),
        }
    }

    pub fn write(&mut self, x: usize, y: usize, ch: char, style: Style) {
        if x < self.width && y < self.height {
            self.cells[y][x].ch = ch;
            self.cells[y][x].style = style;
            self.dirty.insert((x, y));
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
        self.dirty.clear();
    }

    pub fn is_dirty(&self, x: usize, y: usize) -> bool {
        self.dirty.contains(&(x, y))
    }

    pub fn clear_dirty(&mut self) {
        self.dirty.clear();
    }

    pub fn get_dirty_cells(&self) -> impl Iterator<Item = &(usize, usize)> {
        self.dirty.iter()
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
    fn test_dirty_tracking() {
        let mut buffer = Buffer::new(5, 5);
        assert!(!buffer.is_dirty(2, 2));
        buffer.write(2, 2, 'Y', Style::default());
        assert!(buffer.is_dirty(2, 2));
    }
}
