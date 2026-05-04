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
    Indexed(u8),
    Rgb(u8, u8, u8),
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
            Color::Indexed(i) => indexed_to_rgb(*i),
            Color::Rgb(r, g, b) => (*r, *g, *b),
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
            Color::Indexed(i) => indexed_to_rgb(*i),
            Color::Rgb(r, g, b) => (*r, *g, *b),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
    pub wide: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            style: Style::default(),
            wide: false,
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
    scroll_top: usize,
    scroll_bottom: usize,
    saved_main: Option<(Vec<Vec<Cell>>, VecDeque<Vec<Cell>>)>,
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
            scroll_top: 0,
            scroll_bottom: height.saturating_sub(1),
            saved_main: None,
        }
    }

    pub fn write(&mut self, x: usize, y: usize, ch: char, style: Style) {
        if x < self.width && y < self.height {
            self.cells[y][x].ch = ch;
            self.cells[y][x].style = style;
            self.cells[y][x].wide = false;
            if is_wide(ch) && x + 1 < self.width {
                self.cells[y][x + 1].ch = ' ';
                self.cells[y][x + 1].style = style;
                self.cells[y][x + 1].wide = true;
            }
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
        let top = self.scroll_top;
        let bottom = self.scroll_bottom.min(self.height.saturating_sub(1));
        let region_height = bottom.saturating_sub(top) + 1;
        if region_height == 0 || n == 0 {
            return;
        }
        let n = n.min(region_height);
        for _ in 0..n {
            let row = self.cells.remove(top);
            if top == 0 {
                if self.scrollback.len() >= self.scrollback_limit {
                    self.scrollback.pop_front();
                }
                self.scrollback.push_back(row);
            }
            self.cells.insert(bottom, vec![Cell::default(); self.width]);
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
        self.scroll_top = 0;
        self.scroll_bottom = new_height.saturating_sub(1);

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
        let bottom = self.scroll_bottom.min(self.height.saturating_sub(1));
        if at_y > bottom || n == 0 {
            return;
        }
        let n = n.min(bottom - at_y + 1);
        for _ in 0..n {
            self.cells.remove(bottom);
            self.cells.insert(at_y, vec![Cell::default(); self.width]);
        }
    }

    pub fn delete_lines(&mut self, at_y: usize, n: usize) {
        let bottom = self.scroll_bottom.min(self.height.saturating_sub(1));
        if at_y > bottom || n == 0 {
            return;
        }
        let n = n.min(bottom - at_y + 1);
        for _ in 0..n {
            self.cells.remove(at_y);
            self.cells.insert(bottom, vec![Cell::default(); self.width]);
        }
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    pub fn scroll_top(&self) -> usize {
        self.scroll_top
    }

    pub fn scroll_bottom(&self) -> usize {
        self.scroll_bottom
    }

    pub fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        self.scroll_top = top;
        self.scroll_bottom = bottom;
    }

    pub fn save_main_screen(&mut self) {
        let saved_cells = self.cells.clone();
        let saved_scrollback = self.scrollback.clone();
        self.saved_main = Some((saved_cells, saved_scrollback));
        self.cells = vec![vec![Cell::default(); self.width]; self.height];
        self.scrollback = VecDeque::new();
        self.scroll_offset = 0;
    }

    pub fn restore_main_screen(&mut self) {
        if let Some((cells, scrollback)) = self.saved_main.take() {
            self.cells = cells;
            self.scrollback = scrollback;
            self.scroll_offset = 0;
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        let top = self.scroll_top;
        let bottom = self.scroll_bottom.min(self.height.saturating_sub(1));
        let region_height = bottom.saturating_sub(top) + 1;
        if region_height == 0 || n == 0 {
            return;
        }
        let n = n.min(region_height);
        for _ in 0..n {
            self.cells.remove(bottom);
            self.cells.insert(top, vec![Cell::default(); self.width]);
        }
    }

    pub fn insert_chars(&mut self, x: usize, y: usize, n: usize) {
        if y >= self.height || x >= self.width || n == 0 {
            return;
        }
        let row = &mut self.cells[y];
        // Clear wide continuation at boundary
        if x > 0 && row[x - 1].ch != ' ' && !row[x - 1].wide {
            // Check if previous cell is a wide char primary
            if x + 1 < self.width && row[x + 1].wide {
                // Not a wide char primary, ok
            } else if x > 0 && row[x].wide {
                // We're at a continuation cell — back up to the primary
            }
        }
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
        fix_wide_boundaries(row);
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
        fix_wide_boundaries(row);
    }
}

fn indexed_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0..=7 => [
            (0, 0, 0), (128, 0, 0), (0, 128, 0), (128, 128, 0),
            (0, 0, 128), (128, 0, 128), (0, 128, 128), (192, 192, 192),
        ][index as usize],
        8..=15 => [
            (128, 128, 128), (255, 0, 0), (0, 255, 0), (255, 255, 0),
            (0, 0, 255), (255, 0, 255), (0, 255, 255), (255, 255, 255),
        ][(index - 8) as usize],
        16..=231 => {
            let i = (index - 16) as u32;
            let r = color_cube_value(i / 36);
            let g = color_cube_value((i % 36) / 6);
            let b = color_cube_value(i % 6);
            (r, g, b)
        }
        232..=255 => {
            let v = ((index - 232) as u32 * 10 + 8) as u8;
            (v, v, v)
        }
    }
}

fn color_cube_value(component: u32) -> u8 {
    match component {
        0 => 0,
        v => ((v - 1) * 40 + 55) as u8,
    }
}

pub fn is_wide(ch: char) -> bool {
    let cp = ch as u32;
    // CJK Unified Ideographs
    (0x4E00..=0x9FFF).contains(&cp)
    // CJK Extension A
    || (0x3400..=0x4DBF).contains(&cp)
    // CJK Compatibility Ideographs
    || (0xF900..=0xFAFF).contains(&cp)
    // Hiragana, Katakana
    || (0x3040..=0x309F).contains(&cp)
    || (0x30A0..=0x30FF).contains(&cp)
    // Hangul Syllables
    || (0xAC00..=0xD7AF).contains(&cp)
    // Fullwidth forms
    || (0xFF01..=0xFF60).contains(&cp)
    // CJK punctuation
    || (0x3000..=0x303F).contains(&cp)
}

fn fix_wide_boundaries(row: &mut Vec<Cell>) {
    let width = row.len();
    let mut x = 0;
    while x < width {
        if is_wide(row[x].ch) && x + 1 < width {
            row[x].wide = false;
            row[x + 1].wide = true;
            row[x + 1].ch = ' ';
            x += 2;
        } else {
            row[x].wide = false;
            x += 1;
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
        // IL does not push to scrollback (not a scroll operation)
        assert_eq!(buffer.scrollback_len(), 0);
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

    #[test]
    fn test_scroll_down() {
        let mut buffer = Buffer::new(3, 4);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(0, 1, 'B', Style::default());
        buffer.write(0, 2, 'C', Style::default());
        buffer.write(0, 3, 'D', Style::default());
        buffer.scroll_down(2);
        assert_eq!(buffer.get(0, 0).unwrap().ch, ' ');
        assert_eq!(buffer.get(0, 1).unwrap().ch, ' ');
        assert_eq!(buffer.get(0, 2).unwrap().ch, 'A');
        assert_eq!(buffer.get(0, 3).unwrap().ch, 'B');
    }

    #[test]
    fn test_scroll_region() {
        let mut buffer = Buffer::new(3, 6);
        // Fill rows: A B C D E F
        for (i, ch) in "ABCDEF".chars().enumerate() {
            buffer.write(0, i, ch, Style::default());
        }
        // Set scroll region to rows 1-4 (B C D E)
        buffer.set_scroll_region(1, 4);
        buffer.scroll_up(1);
        // Row 0 (A) untouched, rows 1-4 shifted up, row 5 (F) untouched
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'A'); // outside region
        assert_eq!(buffer.get(0, 1).unwrap().ch, 'C'); // B scrolled out
        assert_eq!(buffer.get(0, 2).unwrap().ch, 'D');
        assert_eq!(buffer.get(0, 3).unwrap().ch, 'E');
        assert_eq!(buffer.get(0, 4).unwrap().ch, ' '); // blank line in
        assert_eq!(buffer.get(0, 5).unwrap().ch, 'F'); // outside region
    }

    #[test]
    fn test_scroll_region_down() {
        let mut buffer = Buffer::new(3, 6);
        for (i, ch) in "ABCDEF".chars().enumerate() {
            buffer.write(0, i, ch, Style::default());
        }
        buffer.set_scroll_region(1, 4);
        buffer.scroll_down(1);
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buffer.get(0, 1).unwrap().ch, ' '); // blank inserted
        assert_eq!(buffer.get(0, 2).unwrap().ch, 'B');
        assert_eq!(buffer.get(0, 3).unwrap().ch, 'C');
        assert_eq!(buffer.get(0, 4).unwrap().ch, 'D'); // E scrolled out
        assert_eq!(buffer.get(0, 5).unwrap().ch, 'F');
    }

    #[test]
    fn test_scroll_region_reset() {
        let mut buffer = Buffer::new(3, 4);
        buffer.set_scroll_region(1, 3);
        assert_eq!(buffer.scroll_top(), 1);
        assert_eq!(buffer.scroll_bottom(), 3);
        buffer.set_scroll_region(0, 3);
        assert_eq!(buffer.scroll_top(), 0);
    }

    #[test]
    fn test_alt_screen() {
        let mut buffer = Buffer::new(5, 3);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(0, 1, 'B', Style::default());
        buffer.save_main_screen();
        // Alt screen should be blank
        assert_eq!(buffer.get(0, 0).unwrap().ch, ' ');
        // Write to alt screen
        buffer.write(0, 0, 'X', Style::default());
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'X');
        // Restore main screen
        buffer.restore_main_screen();
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'A');
        assert_eq!(buffer.get(0, 1).unwrap().ch, 'B');
    }

    #[test]
    fn test_wide_char_marking() {
        let mut buffer = Buffer::new(10, 1);
        // Write CJK character (wide)
        buffer.write(0, 0, '\u{4E00}', Style::default());
        assert_eq!(buffer.get(0, 0).unwrap().ch, '\u{4E00}');
        assert_eq!(buffer.get(0, 0).unwrap().wide, false); // primary cell
        assert_eq!(buffer.get(1, 0).unwrap().wide, true);  // continuation cell
        assert_eq!(buffer.get(1, 0).unwrap().ch, ' ');
    }

    #[test]
    fn test_wide_char_at_edge() {
        let mut buffer = Buffer::new(3, 1);
        // Wide char at position 2 (last position) — no room for continuation
        buffer.write(2, 0, '\u{4E00}', Style::default());
        assert_eq!(buffer.get(2, 0).unwrap().ch, '\u{4E00}');
        assert_eq!(buffer.get(2, 0).unwrap().wide, false);
    }

    #[test]
    fn test_delete_chars_wide_boundary() {
        let mut buffer = Buffer::new(6, 1);
        buffer.write(0, 0, 'A', Style::default());
        buffer.write(1, 0, '\u{4E00}', Style::default()); // wide at pos 1-2
        buffer.write(3, 0, 'B', Style::default());
        // After write: [A, 一, ' '(wide), B, ' ', ' ']
        // Delete 1 char at position 1 removes the wide char primary
        // Shifts left: [A, ' '(was wide), B, ' ', ' ', ' ']
        // fix_wide_boundaries clears orphaned continuation
        buffer.delete_chars(1, 0, 1);
        assert_eq!(buffer.get(0, 0).unwrap().ch, 'A');
        // B moved to position 2 after the orphaned continuation was cleared
        assert_eq!(buffer.get(2, 0).unwrap().ch, 'B');
        // No stale wide flags
        assert!(!buffer.get(1, 0).unwrap().wide);
    }
}
