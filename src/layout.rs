use crate::buffer;

#[derive(Clone, Copy, PartialEq, Debug)]
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
}

pub struct Tab {
    pub id: usize,
    pub panes: Vec<Pane>,
    pub focused: usize,
}

pub struct Layout {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub width: usize,
    pub height: usize,
    next_pane_id: usize,
    next_tab_id: usize,
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
        };

        Layout {
            tabs: vec![Tab {
                id: 0,
                panes: vec![initial_pane],
                focused: 0,
            }],
            active_tab: 0,
            width,
            height,
            next_pane_id: 1,
            next_tab_id: 1,
        }
    }

    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    pub fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    pub fn active_panes(&self) -> &[Pane] {
        &self.tabs[self.active_tab].panes
    }

    pub fn active_panes_mut(&mut self) -> &mut Vec<Pane> {
        &mut self.tabs[self.active_tab].panes
    }

    pub fn focused_pane(&self) -> &Pane {
        &self.tabs[self.active_tab].panes[self.tabs[self.active_tab].focused]
    }

    pub fn focused_pane_mut(&mut self) -> &mut Pane {
        let focused = self.tabs[self.active_tab].focused;
        &mut self.tabs[self.active_tab].panes[focused]
    }

    pub fn focused_pane_id(&self) -> usize {
        self.focused_pane().id
    }

    pub fn focused_idx(&self) -> usize {
        self.tabs[self.active_tab].focused
    }

    pub fn find_pane(&self, pane_id: usize) -> Option<&Pane> {
        for tab in &self.tabs {
            if let Some(pane) = tab.panes.iter().find(|p| p.id == pane_id) {
                return Some(pane);
            }
        }
        None
    }

    pub fn find_pane_mut(&mut self, pane_id: usize) -> Option<&mut Pane> {
        for tab in &mut self.tabs {
            if let Some(pane) = tab.panes.iter_mut().find(|p| p.id == pane_id) {
                return Some(pane);
            }
        }
        None
    }

    pub fn split_horizontal(&mut self, pane_id: usize) -> Result<(), String> {
        let tab = &mut self.tabs[self.active_tab];
        let pane_index = tab.panes.iter().position(|p| p.id == pane_id)
            .ok_or("Pane not found")?;

        let pane = &mut tab.panes[pane_index];
        if pane.height < 4 {
            return Err("Pane too small to split".to_string());
        }

        let original_height = pane.height;
        let top_height = original_height / 2;
        let bottom_height = original_height - top_height;
        pane.height = top_height;
        pane.buffer.resize(pane.width, top_height);

        let new_pane = Pane {
            id: self.next_pane_id,
            x: pane.x,
            y: pane.y + top_height,
            width: pane.width,
            height: bottom_height,
            buffer: buffer::Buffer::new(pane.width, bottom_height),
        };

        self.next_pane_id += 1;
        tab.panes.push(new_pane);
        tab.focused = tab.panes.len() - 1;

        Ok(())
    }

    pub fn split_vertical(&mut self, pane_id: usize) -> Result<(), String> {
        let tab = &mut self.tabs[self.active_tab];
        let pane_index = tab.panes.iter().position(|p| p.id == pane_id)
            .ok_or("Pane not found")?;

        let pane = &mut tab.panes[pane_index];
        if pane.width < 4 {
            return Err("Pane too small to split".to_string());
        }

        let original_width = pane.width;
        let left_width = original_width / 2;
        let right_width = original_width - left_width;
        pane.width = left_width;
        pane.buffer.resize(left_width, pane.height);

        let new_pane = Pane {
            id: self.next_pane_id,
            x: pane.x + left_width,
            y: pane.y,
            width: right_width,
            height: pane.height,
            buffer: buffer::Buffer::new(right_width, pane.height),
        };

        self.next_pane_id += 1;
        tab.panes.push(new_pane);
        tab.focused = tab.panes.len() - 1;

        Ok(())
    }

    pub fn navigate(&mut self, direction: Direction) {
        let tab = &self.tabs[self.active_tab];
        let current = &tab.panes[tab.focused];
        let target = Self::find_adjacent_pane(&tab.panes, current, direction);

        if let Some(target_id) = target {
            let tab = &mut self.tabs[self.active_tab];
            if let Some(idx) = tab.panes.iter().position(|p| p.id == target_id) {
                tab.focused = idx;
            }
        }
    }

    fn find_adjacent_pane(panes: &[Pane], pane: &Pane, direction: Direction) -> Option<usize> {
        let mut best_candidate: Option<(usize, f64)> = None;

        for other in panes {
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

    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        let old_width = self.width;
        let old_height = self.height;

        self.width = new_width;
        self.height = new_height;

        if old_width == 0 || old_height == 0 {
            for tab in &mut self.tabs {
                for pane in &mut tab.panes {
                    pane.width = new_width;
                    pane.height = new_height;
                    pane.buffer.resize(new_width, new_height);
                }
            }
            return;
        }

        let w_ratio = new_width as f64 / old_width as f64;
        let h_ratio = new_height as f64 / old_height as f64;

        for tab in &mut self.tabs {
            for pane in &mut tab.panes {
                let new_x = (pane.x as f64 * w_ratio).round() as usize;
                let new_y = (pane.y as f64 * h_ratio).round() as usize;
                let new_right = ((pane.x + pane.width) as f64 * w_ratio).round() as usize;
                let new_bottom = ((pane.y + pane.height) as f64 * h_ratio).round() as usize;

                pane.x = new_x;
                pane.y = new_y;
                pane.width = (new_right - new_x).max(2);
                pane.height = (new_bottom - new_y).max(2);
                pane.buffer.resize(pane.width, pane.height);
            }
        }
    }

    pub fn remove_pane(&mut self, pane_id: usize) {
        for tab in &mut self.tabs {
            let had_pane = tab.panes.iter().any(|p| p.id == pane_id);
            if !had_pane {
                continue;
            }
            tab.panes.retain(|p| p.id != pane_id);
            if tab.panes.is_empty() {
                let initial_pane = Pane {
                    id: self.next_pane_id,
                    x: 0,
                    y: 0,
                    width: self.width,
                    height: self.height,
                    buffer: buffer::Buffer::new(self.width, self.height),
                };
                self.next_pane_id += 1;
                tab.panes.push(initial_pane);
            }
            if tab.focused >= tab.panes.len() {
                tab.focused = tab.panes.len() - 1;
            }
        }
    }

    pub fn new_tab(&mut self) -> usize {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let initial_pane = Pane {
            id: self.next_pane_id,
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
            buffer: buffer::Buffer::new(self.width, self.height),
        };
        self.next_pane_id += 1;

        self.tabs.push(Tab {
            id: tab_id,
            panes: vec![initial_pane],
            focused: 0,
        });
        self.active_tab = self.tabs.len() - 1;
        tab_id
    }

    pub fn close_tab(&mut self) {
        if self.tabs.len() <= 1 {
            return;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_creation() {
        let layout = Layout::new(80, 24);
        assert_eq!(layout.tabs.len(), 1);
        assert_eq!(layout.tabs[0].panes.len(), 1);
        assert_eq!(layout.active_tab, 0);
    }

    #[test]
    fn test_horizontal_split() {
        let mut layout = Layout::new(80, 24);
        layout.split_horizontal(0).unwrap();
        assert_eq!(layout.tabs[0].panes.len(), 2);
    }

    #[test]
    fn test_vertical_split() {
        let mut layout = Layout::new(80, 24);
        layout.split_vertical(0).unwrap();
        assert_eq!(layout.tabs[0].panes.len(), 2);
    }

    #[test]
    fn test_navigation() {
        let mut layout = Layout::new(80, 24);
        layout.split_horizontal(0).unwrap();
        layout.navigate(Direction::Down);
        assert_eq!(layout.tabs[0].focused, 1);
    }

    #[test]
    fn test_boundary_navigation() {
        let mut layout = Layout::new(80, 24);
        let original = layout.tabs[0].focused;
        layout.navigate(Direction::Up);
        assert_eq!(layout.tabs[0].focused, original);
    }

    #[test]
    fn test_new_tab() {
        let mut layout = Layout::new(80, 24);
        layout.new_tab();
        assert_eq!(layout.tabs.len(), 2);
        assert_eq!(layout.active_tab, 1);
    }

    #[test]
    fn test_close_tab() {
        let mut layout = Layout::new(80, 24);
        layout.new_tab();
        layout.close_tab();
        assert_eq!(layout.tabs.len(), 1);
        assert_eq!(layout.active_tab, 0);
    }

    #[test]
    fn test_close_last_tab_does_nothing() {
        let mut layout = Layout::new(80, 24);
        layout.close_tab();
        assert_eq!(layout.tabs.len(), 1);
    }

    #[test]
    fn test_next_prev_tab() {
        let mut layout = Layout::new(80, 24);
        layout.new_tab();
        layout.new_tab();
        assert_eq!(layout.active_tab, 2);
        layout.prev_tab();
        assert_eq!(layout.active_tab, 1);
        layout.next_tab();
        assert_eq!(layout.active_tab, 2);
        layout.next_tab();
        assert_eq!(layout.active_tab, 0); // wraps
    }

    #[test]
    fn test_split_in_different_tabs() {
        let mut layout = Layout::new(80, 24);
        layout.split_horizontal(0).unwrap();
        assert_eq!(layout.tabs[0].panes.len(), 2);
        layout.new_tab();
        assert_eq!(layout.tabs[1].panes.len(), 1);
        layout.split_vertical(layout.focused_pane_id()).unwrap();
        assert_eq!(layout.tabs[1].panes.len(), 2);
    }

    #[test]
    fn test_active_panes_accessors() {
        let mut layout = Layout::new(80, 24);
        assert_eq!(layout.active_panes().len(), 1);
        assert_eq!(layout.focused_pane_id(), 0);
        layout.split_horizontal(0).unwrap();
        assert_eq!(layout.active_panes().len(), 2);
    }
}
