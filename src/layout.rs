use crate::buffer;

#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct Pane {
    pub id: usize,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub buffer: buffer::Buffer,
    pub style: buffer::Style,
}

pub struct Layout {
    pub panes: Vec<Pane>,
    pub focused: usize,
    pub width: usize,
    pub height: usize,
    next_id: usize,
}

impl Layout {
    pub fn new(width: usize, height: usize) -> Self {
        let initial_pane = Pane {
            id: 0,
            x: 0,
            y: 0,
            width,
            height,
            buffer: buffer::Buffer::new(width, height),
            style: buffer::Style::default(),
        };

        Layout {
            panes: vec![initial_pane],
            focused: 0,
            width,
            height,
            next_id: 1,
        }
    }

    pub fn split_horizontal(&mut self, pane_id: usize) -> Result<(), String> {
        let pane_index = self.panes.iter().position(|p| p.id == pane_id)
            .ok_or("Pane not found")?;

        let pane = &mut self.panes[pane_index];
        if pane.height < 4 {
            return Err("Pane too small to split".to_string());
        }

        let new_height = pane.height / 2;
        pane.height = new_height;

        let new_pane = Pane {
            id: self.next_id,
            x: pane.x,
            y: pane.y + new_height,
            width: pane.width,
            height: pane.height,
            buffer: buffer::Buffer::new(pane.width, pane.height),
            style: buffer::Style::default(),
        };

        self.next_id += 1;
        self.panes.push(new_pane);
        self.focused = self.panes.len() - 1;

        Ok(())
    }

    pub fn split_vertical(&mut self, pane_id: usize) -> Result<(), String> {
        let pane_index = self.panes.iter().position(|p| p.id == pane_id)
            .ok_or("Pane not found")?;

        let pane = &mut self.panes[pane_index];
        if pane.width < 4 {
            return Err("Pane too small to split".to_string());
        }

        let new_width = pane.width / 2;
        pane.width = new_width;

        let new_pane = Pane {
            id: self.next_id,
            x: pane.x + new_width,
            y: pane.y,
            width: pane.width,
            height: pane.height,
            buffer: buffer::Buffer::new(pane.width, pane.height),
            style: buffer::Style::default(),
        };

        self.next_id += 1;
        self.panes.push(new_pane);
        self.focused = self.panes.len() - 1;

        Ok(())
    }

    pub fn navigate(&mut self, direction: Direction) {
        let current = &self.panes[self.focused];
        let target = self.find_adjacent_pane(current, direction);

        if let Some(target_id) = target {
            self.focused = self.panes.iter().position(|p| p.id == target_id).unwrap();
        }
    }

    fn find_adjacent_pane(&self, pane: &Pane, direction: Direction) -> Option<usize> {
        let mut best_candidate: Option<(usize, f64)> = None;

        for other in &self.panes {
            if other.id == pane.id {
                continue;
            }

            let distance = match direction {
                Direction::Up if other.y + other.height == pane.y => {
                    Some(Self::horizontal_overlap(pane, other) as f64)
                }
                Direction::Down if other.y == pane.y + pane.height => {
                    Some(Self::horizontal_overlap(pane, other) as f64)
                }
                Direction::Left if other.x + other.width == pane.x => {
                    Some(Self::vertical_overlap(pane, other) as f64)
                }
                Direction::Right if other.x == pane.x + pane.width => {
                    Some(Self::vertical_overlap(pane, other) as f64)
                }
                _ => None,
            };

            if let Some(overlap) = distance {
                if overlap > 0.0 {
                    if best_candidate.is_none() || overlap > best_candidate.unwrap().1 {
                        best_candidate = Some((other.id, overlap));
                    }
                }
            }
        }

        best_candidate.map(|(id, _)| id)
    }

    fn horizontal_overlap(a: &Pane, b: &Pane) -> usize {
        let start = a.x.max(b.x);
        let end = (a.x + a.width).min(b.x + b.width);
        end.saturating_sub(start)
    }

    fn vertical_overlap(a: &Pane, b: &Pane) -> usize {
        let start = a.y.max(b.y);
        let end = (a.y + a.height).min(b.y + b.height);
        end.saturating_sub(start)
    }

    pub fn get_focused_pane(&mut self) -> &mut Pane {
        &mut self.panes[self.focused]
    }

    pub fn remove_pane(&mut self, pane_id: usize) {
        self.panes.retain(|p| p.id != pane_id);
        if self.panes.is_empty() {
            let initial_pane = Pane {
                id: self.next_id,
                x: 0,
                y: 0,
                width: self.width,
                height: self.height,
                buffer: buffer::Buffer::new(self.width, self.height),
                style: buffer::Style::default(),
            };
            self.next_id += 1;
            self.panes.push(initial_pane);
        }
        if self.focused >= self.panes.len() {
            self.focused = self.panes.len() - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_creation() {
        let layout = Layout::new(80, 24);
        assert_eq!(layout.panes.len(), 1);
        assert_eq!(layout.focused, 0);
    }

    #[test]
    fn test_horizontal_split() {
        let mut layout = Layout::new(80, 24);
        layout.split_horizontal(0).unwrap();
        assert_eq!(layout.panes.len(), 2);
    }

    #[test]
    fn test_vertical_split() {
        let mut layout = Layout::new(80, 24);
        layout.split_vertical(0).unwrap();
        assert_eq!(layout.panes.len(), 2);
    }

    #[test]
    fn test_navigation() {
        let mut layout = Layout::new(80, 24);
        layout.split_horizontal(0).unwrap();
        layout.navigate(Direction::Down);
        assert_eq!(layout.focused, 1);
    }

    #[test]
    fn test_boundary_navigation() {
        let mut layout = Layout::new(80, 24);
        let original = layout.focused;
        layout.navigate(Direction::Up);
        assert_eq!(layout.focused, original);
    }
}
