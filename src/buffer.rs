use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Style {
    pub fg_color: Color,
    pub bg_color: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            fg_color: Color::Default,
            bg_color: Color::Default,
            bold: false,
            italic: false,
            underline: false,
            reverse: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
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

impl Color {
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            Color::Default => (220, 220, 220),
            Color::Black => (40, 40, 40),
            Color::Red => (200, 50, 50),
            Color::Green => (50, 200, 50),
            Color::Yellow => (200, 200, 50),
            Color::Blue => (50, 100, 200),
            Color::Magenta => (200, 50, 200),
            Color::Cyan => (50, 200, 200),
            Color::White => (220, 220, 220),
        }
    }

    pub fn to_rgb_bg(&self) -> (u8, u8, u8) {
        match self {
            Color::Default => (30, 30, 30),
            Color::Black => (0, 0, 0),
            Color::Red => (140, 30, 30),
            Color::Green => (30, 140, 30),
            Color::Yellow => (140, 140, 30),
            Color::Blue => (30, 60, 140),
            Color::Magenta => (140, 30, 140),
            Color::Cyan => (30, 140, 140),
            Color::White => (180, 180, 180),
        }
    }
}

#[derive(Clone, Copy)]
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
    scrollback: VecDeque<Vec<Cell>>,
    pub scrollback_limit: usize,
    pub scroll_offset: usize,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        let cells = vec![vec![Cell::default(); width]; height];
        Buffer {
            cells,
            width,
            height,
            scrollback: VecDeque::new(),
            scrollback_limit: 10000,
            scroll_offset: 0,
        }
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
            // Push all rows to scrollback
            for row in self.cells.drain(..) {
                if self.scrollback.len() >= self.scrollback_limit {
                    self.scrollback.pop_front();
                }
                self.scrollback.push_back(row);
            }
            self.cells = vec![vec![Cell::default(); self.width]; self.height];
            return;
        }
        for _ in 0..n {
            let row = self.cells.remove(0);
            if self.scrollback.len() >= self.scrollback_limit {
                self.scrollback.pop_front();
            }
            self.scrollback.push_back(row);
            self.cells.push(vec![Cell::default(); self.width]);
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

        // Resize scrollback rows to match new width
        for row in &mut self.scrollback {
            let mut new_row = vec![Cell::default(); new_width];
            let copy_len = new_width.min(row.len());
            for x in 0..copy_len {
                new_row[x] = row[x].clone();
            }
            *row = new_row;
        }
    }

    pub fn get_render_row(&self, y: usize) -> Option<&[Cell]> {
        if self.scroll_offset == 0 {
            self.cells.get(y).map(|r| r.as_slice())
        } else {
            let scrolled_y = y;
            if scrolled_y < self.scrollback.len() {
                // Show from scrollback — map y=0 to oldest visible scrollback line
                let sb_start = self.scrollback.len().saturating_sub(self.scroll_offset);
                let idx = sb_start + scrolled_y;
                if idx < self.scrollback.len() {
                    Some(self.scrollback.get(idx).unwrap().as_slice())
                } else {
                    let visible_y = idx - self.scrollback.len();
                    self.cells.get(visible_y).map(|r| r.as_slice())
                }
            } else {
                let visible_y = scrolled_y - self.scrollback.len();
                self.cells.get(visible_y).map(|r| r.as_slice())
            }
        }
    }

    pub fn scroll_view_up(&mut self, n: usize) {
        self.scroll_offset = (self.scroll_offset + n).min(self.scrollback.len());
    }

    pub fn scroll_view_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn insert_lines(&mut self, at_y: usize, n: usize) {
        if at_y >= self.height || n == 0 {
            return;
        }
        let n = n.min(self.height - at_y);
        for _ in 0..n {
            if at_y + n < self.height {
                self.cells.pop();
            }
        }
        for _ in 0..n {
            self.cells.insert(at_y, vec![Cell::default(); self.width]);
        }
        self.cells.truncate(self.height);
    }

    pub fn delete_lines(&mut self, at_y: usize, n: usize) {
        if at_y >= self.height || n == 0 {
            return;
        }
        let n = n.min(self.height - at_y);
        for _ in 0..n {
            self.cells.remove(at_y);
        }
        for _ in 0..n {
            self.cells.push(vec![Cell::default(); self.width]);
        }
        self.cells.truncate(self.height);
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    pub fn insert_chars(&mut self, x: usize, y: usize, n: usize) {
        if y >= self.height || x >= self.width || n == 0 {
            return;
        }
        let row = &mut self.cells[y];
        let remaining = self.width - x;
        let n = n.min(remaining);
        for i in (x..self.width).rev() {
            if i >= n {
                row[i] = row[i - n].clone();
            } else {
                break;
            }
        }
        for i in x..x + n {
            row[i] = Cell::default();
        }
    }

    pub fn delete_chars(&mut self, x: usize, y: usize, n: usize) {
        if y >= self.height || x >= self.width || n == 0 {
            return;
        }
        let row = &mut self.cells[y];
        let remaining = self.width - x;
        let n = n.min(remaining);
        for i in x..self.width {
            if i + n < self.width {
                row[i] = row[i + n].clone();
            } else {
                row[i] = Cell::default();
            }
        }
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
        assert_eq!(buffer.scrollback_len(), 1);
    }

    #[test]
    fn test_scrollback_push() {
        let mut buffer = Buffer::new(5, 2);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(0, 1, 'B', Style::default());
        buffer.scroll_up(1);
        assert_eq!(buffer.scrollback_len(), 1);
        buffer.scroll_up(1);
        assert_eq!(buffer.scrollback_len(), 2);
    }

    #[test]
    fn test_scrollback_limit() {
        let mut buffer = Buffer::new(5, 2);
        buffer.scrollback_limit = 3;
        buffer.write(0, 0, 'A', Style::default());
        buffer.scroll_up(1);
        buffer.write(0, 0, 'B', Style::default());
        buffer.scroll_up(1);
        buffer.write(0, 0, 'C', Style::default());
        buffer.scroll_up(1);
        buffer.write(0, 0, 'D', Style::default());
        buffer.scroll_up(1);
        assert_eq!(buffer.scrollback_len(), 3);
        // Oldest (A) was evicted
        assert_eq!(buffer.scrollback[0][0].ch, 'B');
    }

    #[test]
    fn test_scroll_view() {
        let mut buffer = Buffer::new(5, 2);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(0, 1, 'B', Style::default());
        buffer.scroll_up(1);
        buffer.write(0, 1, 'C', Style::default());
        // Now scrollback has ['A' row], visible has [' ', 'C']
        buffer.scroll_view_up(1);
        assert_eq!(buffer.scroll_offset, 1);
        buffer.scroll_view_down(1);
        assert_eq!(buffer.scroll_offset, 0);
    }

    #[test]
    fn test_reset_scroll() {
        let mut buffer = Buffer::new(5, 2);
        buffer.scroll_up(1);
        buffer.scroll_view_up(1);
        assert_eq!(buffer.scroll_offset, 1);
        buffer.reset_scroll();
        assert_eq!(buffer.scroll_offset, 0);
    }

    #[test]
    fn test_style_default() {
        let style = Style::default();
        assert_eq!(style.fg_color, Color::Default);
        assert_eq!(style.bg_color, Color::Default);
        assert!(!style.bold);
        assert!(!style.italic);
        assert!(!style.underline);
        assert!(!style.reverse);
    }

    #[test]
    fn test_color_to_rgb() {
        assert_eq!(Color::Red.to_rgb(), (200, 50, 50));
        assert_eq!(Color::Blue.to_rgb(), (50, 100, 200));
        assert_eq!(Color::Default.to_rgb(), (220, 220, 220));
    }

    #[test]
    fn test_color_to_rgb_bg() {
        assert_eq!(Color::Red.to_rgb_bg(), (140, 30, 30));
        assert_eq!(Color::Default.to_rgb_bg(), (30, 30, 30));
    }

    #[test]
    fn test_resize_preserves_content() {
        let mut buffer = Buffer::new(4, 3);
        buffer.write(1, 1, 'X', Style::default());
        buffer.resize(4, 5);
        assert_eq!(buffer.width, 4);
        assert_eq!(buffer.height, 5);
        assert_eq!(buffer.get(1, 1).unwrap().ch, 'X');
    }

    #[test]
    fn test_insert_lines() {
        let mut buffer = Buffer::new(3, 4);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(0, 1, 'B', Style::default());
        buffer.write(0, 2, 'C', Style::default());
        buffer.write(0, 3, 'D', Style::default());
        buffer.insert_lines(1, 2);
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buffer.get(0, 1).unwrap().ch, ' ');
        assert_eq!(buffer.get(0, 2).unwrap().ch, ' ');
        assert_eq!(buffer.get(0, 3).unwrap().ch, 'B');
    }

    #[test]
    fn test_delete_lines() {
        let mut buffer = Buffer::new(3, 4);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(0, 1, 'B', Style::default());
        buffer.write(0, 2, 'C', Style::default());
        buffer.write(0, 3, 'D', Style::default());
        buffer.delete_lines(1, 2);
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buffer.get(0, 1).unwrap().ch, 'D');
        assert_eq!(buffer.get(0, 2).unwrap().ch, ' ');
        assert_eq!(buffer.get(0, 3).unwrap().ch, ' ');
    }

    #[test]
    fn test_insert_chars() {
        let mut buffer = Buffer::new(5, 1);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(1, 0, 'B', Style::default());
        buffer.write(2, 0, 'C', Style::default());
        buffer.write(3, 0, 'D', Style::default());
        buffer.insert_chars(1, 0, 2);
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buffer.get(1, 0).unwrap().ch, ' ');
        assert_eq!(buffer.get(2, 0).unwrap().ch, ' ');
        assert_eq!(buffer.get(3, 0).unwrap().ch, 'B');
        assert_eq!(buffer.get(4, 0).unwrap().ch, 'C');
    }

    #[test]
    fn test_delete_chars() {
        let mut buffer = Buffer::new(5, 1);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(1, 0, 'B', Style::default());
        buffer.write(2, 0, 'C', Style::default());
        buffer.write(3, 0, 'D', Style::default());
        buffer.delete_chars(1, 0, 2);
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buffer.get(1, 0).unwrap().ch, 'D');
        assert_eq!(buffer.get(2, 0).unwrap().ch, ' ');
        assert_eq!(buffer.get(3, 0).unwrap().ch, ' ');
    }
}
