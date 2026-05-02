use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::buffer;
use crate::config::Config;
use crate::layout;

const DEFAULT_CELL_WIDTH: usize = 8;
const DEFAULT_CELL_HEIGHT: usize = 16;

fn default_cell() -> buffer::Cell {
    buffer::Cell { ch: ' ', style: buffer::Style::default() }
}

pub struct Renderer {
    canvas: Canvas<Window>,
    cell_width: usize,
    cell_height: usize,
    font: Option<fontdue::Font>,
    font_size: f32,
    config_bg: (u8, u8, u8),
    config_fg: (u8, u8, u8),
    cursor_style: CursorStyle,
    #[allow(dead_code)]
    cursor_blink: bool,
}

#[derive(Clone, Copy)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

impl Renderer {
    pub fn new(sdl_context: &sdl2::Sdl, config: &Config) -> Result<Self, String> {
        let video_subsystem = sdl_context.video().map_err(|e| e.to_string())?;

        let window = video_subsystem
            .window("term-tiler", config.render.window_width, config.render.window_height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;

        let mut canvas = window
            .into_canvas()
            .software()
            .build()
            .map_err(|e| e.to_string())?;

        canvas.set_draw_color(Color::RGB(
            config.render.bg_color.0,
            config.render.bg_color.1,
            config.render.bg_color.2,
        ));
        canvas.clear();
        canvas.present();

        let font = Self::load_font(&config.render.font_family);
        let (cell_width, cell_height) = Self::compute_cell_size(&font, config.render.font_size);

        let cursor_style = match config.render.cursor_style.as_str() {
            "underline" => CursorStyle::Underline,
            "bar" => CursorStyle::Bar,
            _ => CursorStyle::Block,
        };

        Ok(Renderer {
            canvas,
            cell_width,
            cell_height,
            font,
            font_size: config.render.font_size,
            config_bg: config.render.bg_color,
            config_fg: config.render.fg_color,
            cursor_style,
            cursor_blink: config.render.cursor_blink,
        })
    }

    fn load_font(font_family: &str) -> Option<fontdue::Font> {
        let path = crate::config::resolve_font_path(font_family)?;
        let bytes = std::fs::read(&path).ok()?;
        fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
    }

    fn compute_cell_size(font: &Option<fontdue::Font>, font_size: f32) -> (usize, usize) {
        if let Some(f) = font {
            let (metrics, _) = f.rasterize('M', font_size);
            let w = (metrics.advance_width.ceil() as usize).max(4);
            let line_h = metrics.advance_height.ceil() as usize;
            let glyph_h = metrics.bounds.height.ceil() as usize;
            let h = line_h.max(glyph_h).max(8);
            (w, h)
        } else {
            (DEFAULT_CELL_WIDTH, DEFAULT_CELL_HEIGHT)
        }
    }

    pub fn grid_size(&self) -> (usize, usize) {
        let (w, h) = self.canvas.window().size();
        let cols = w as usize / self.cell_width;
        let rows = h as usize / self.cell_height;
        (cols.max(1), rows.max(1))
    }

    pub fn render(
        &mut self,
        layout: &layout::Layout,
        panes: &std::collections::HashMap<usize, PaneData>,
        cursor_visible: bool,
    ) {
        self.canvas.set_draw_color(Color::RGB(
            self.config_bg.0,
            self.config_bg.1,
            self.config_bg.2,
        ));
        self.canvas.clear();

        let tab_bar_pixels = if layout.tabs.len() > 1 {
            self.cell_height
        } else {
            0
        };

        let active_tab = &layout.tabs[layout.active_tab];
        for (i, pane) in active_tab.panes.iter().enumerate() {
            let is_focused = i == active_tab.focused;
            let pane_data = panes.get(&pane.id);
            self.render_pane(pane, pane_data, is_focused, cursor_visible, tab_bar_pixels);
        }

        for (i, pane) in active_tab.panes.iter().enumerate() {
            if i != active_tab.focused {
                self.draw_border(pane, tab_bar_pixels);
            }
        }

        if layout.tabs.len() > 1 {
            self.draw_tab_bar(layout);
        }

        self.canvas.present();
    }

    fn render_pane(
        &mut self,
        pane: &layout::Pane,
        pane_data: Option<&PaneData>,
        is_focused: bool,
        cursor_visible: bool,
        y_offset: usize,
    ) {
        let pd = match pane_data {
            Some(d) => d,
            None => return,
        };

        for y in 0..pane.height {
            for x in 0..pane.width {
                let cell = pane.buffer.get_render_row(y)
                    .and_then(|r| r.get(x).copied())
                    .unwrap_or_else(default_cell);

                let pixel_x = (pane.x + x) * self.cell_width;
                let pixel_y = y_offset + (pane.y + y) * self.cell_height;

                let fg = self.resolve_fg(&cell.style);
                let bg = self.resolve_bg(&cell.style);

                self.canvas.set_draw_color(Color::RGB(bg.0, bg.1, bg.2));
                let _ = self.canvas.fill_rect(Rect::new(
                    pixel_x as i32,
                    pixel_y as i32,
                    self.cell_width as u32,
                    self.cell_height as u32,
                ));

                if cell.style.underline {
                    self.canvas.set_draw_color(Color::RGB(fg.0, fg.1, fg.2));
                    let _ = self.canvas.draw_line(
                        sdl2::rect::Point::new(pixel_x as i32, (pixel_y + self.cell_height - 1) as i32),
                        sdl2::rect::Point::new((pixel_x + self.cell_width) as i32, (pixel_y + self.cell_height - 1) as i32),
                    );
                }

                if cell.ch != ' ' {
                    self.draw_glyph(cell.ch, fg, bg, cell.style.bold, pixel_x, pixel_y);
                }
            }
        }

        if is_focused && cursor_visible {
            let cx = pd.cursor_x.min(pane.width.saturating_sub(1));
            let cy = pd.cursor_y.min(pane.height.saturating_sub(1));
            let pixel_x = (pane.x + cx) * self.cell_width;
            let pixel_y = y_offset + (pane.y + cy) * self.cell_height;

            let cell = pane.buffer.get(cx, cy).copied().unwrap_or_else(default_cell);
            let fg = self.resolve_fg(&cell.style);
            let bg = self.resolve_bg(&cell.style);

            match self.cursor_style {
                CursorStyle::Block => {
                    self.canvas.set_draw_color(Color::RGB(fg.0, fg.1, fg.2));
                    let _ = self.canvas.fill_rect(Rect::new(
                        pixel_x as i32,
                        pixel_y as i32,
                        self.cell_width as u32,
                        self.cell_height as u32,
                    ));
                    if cell.ch != ' ' {
                        self.draw_glyph(cell.ch, bg, fg, cell.style.bold, pixel_x, pixel_y);
                    }
                }
                CursorStyle::Underline => {
                    self.canvas.set_draw_color(Color::RGB(fg.0, fg.1, fg.2));
                    let _ = self.canvas.fill_rect(Rect::new(
                        pixel_x as i32,
                        (pixel_y + self.cell_height - 2) as i32,
                        self.cell_width as u32,
                        2,
                    ));
                }
                CursorStyle::Bar => {
                    self.canvas.set_draw_color(Color::RGB(fg.0, fg.1, fg.2));
                    let _ = self.canvas.fill_rect(Rect::new(
                        pixel_x as i32,
                        pixel_y as i32,
                        2,
                        self.cell_height as u32,
                    ));
                }
            }
        }
    }

    fn draw_glyph(
        &mut self,
        ch: char,
        fg: (u8, u8, u8),
        bg: (u8, u8, u8),
        bold: bool,
        pixel_x: usize,
        pixel_y: usize,
    ) {
        let cw = self.cell_width;
        let cell_h = self.cell_height;

        if let Some(font) = &self.font {
            let (metrics, bitmap) = font.rasterize(ch, self.font_size);
            let glyph_w = metrics.bounds.width as usize;
            let glyph_h = metrics.bounds.height as usize;
            let x_start = if metrics.bounds.xmin < 0.0 {
                0
            } else {
                metrics.bounds.xmin as usize
            };
            let baseline = cell_h as f32 * 0.8;
            let y_start = (baseline as usize)
                .saturating_sub(glyph_h)
                .saturating_add(metrics.bounds.ymin.abs() as usize);

            for gy in 0..glyph_h {
                for gx in 0..glyph_w {
                    let coverage = bitmap[gy * glyph_w + gx];
                    if coverage == 0 {
                        continue;
                    }
                    let sx = x_start + gx;
                    let sy = y_start + gy;
                    if sx >= cw || sy >= cell_h {
                        continue;
                    }

                    let alpha = coverage as f32 / 255.0;
                    let r = (fg.0 as f32 * alpha + bg.0 as f32 * (1.0 - alpha)) as u8;
                    let g = (fg.1 as f32 * alpha + bg.1 as f32 * (1.0 - alpha)) as u8;
                    let b = (fg.2 as f32 * alpha + bg.2 as f32 * (1.0 - alpha)) as u8;

                    self.canvas.set_draw_color(Color::RGB(r, g, b));
                    let _ = self.canvas.draw_line(
                        sdl2::rect::Point::new((pixel_x + sx) as i32, (pixel_y + sy) as i32),
                        sdl2::rect::Point::new((pixel_x + sx) as i32, (pixel_y + sy) as i32),
                    );

                    // Fake bold: also draw 1px to the right
                    if bold && sx + 1 < cw {
                        let _ = self.canvas.draw_line(
                            sdl2::rect::Point::new((pixel_x + sx + 1) as i32, (pixel_y + sy) as i32),
                            sdl2::rect::Point::new((pixel_x + sx + 1) as i32, (pixel_y + sy) as i32),
                        );
                    }
                }
            }
        }
    }

    fn draw_border(&mut self, pane: &layout::Pane, y_offset: usize) {
        let border_color = Color::RGB(80, 80, 80);
        self.canvas.set_draw_color(border_color);
        let px = pane.x * self.cell_width;
        let py = y_offset + pane.y * self.cell_height;
        let pw = pane.width * self.cell_width;
        let ph = pane.height * self.cell_height;

        let _ = self.canvas.draw_line(
            sdl2::rect::Point::new(px as i32, py.saturating_sub(1) as i32),
            sdl2::rect::Point::new((px + pw) as i32, py.saturating_sub(1) as i32),
        );
        let _ = self.canvas.draw_line(
            sdl2::rect::Point::new(px as i32, (py + ph) as i32),
            sdl2::rect::Point::new((px + pw) as i32, (py + ph) as i32),
        );
        let _ = self.canvas.draw_line(
            sdl2::rect::Point::new(px.saturating_sub(1) as i32, py as i32),
            sdl2::rect::Point::new(px.saturating_sub(1) as i32, (py + ph) as i32),
        );
        let _ = self.canvas.draw_line(
            sdl2::rect::Point::new((px + pw) as i32, py as i32),
            sdl2::rect::Point::new((px + pw) as i32, (py + ph) as i32),
        );
    }

    fn draw_tab_bar(&mut self, layout: &layout::Layout) {
        let bar_y = 0;
        let bar_height = self.cell_height;
        let tab_width = self.cell_width * 12;

        for (i, _tab) in layout.tabs.iter().enumerate() {
            let x = i * tab_width;
            let is_active = i == layout.active_tab;

            let bg = if is_active {
                Color::RGB(60, 60, 80)
            } else {
                Color::RGB(40, 40, 50)
            };

            self.canvas.set_draw_color(bg);
            let _ = self.canvas.fill_rect(Rect::new(
                x as i32,
                bar_y as i32,
                tab_width as u32,
                bar_height as u32,
            ));

            let label = format!("Tab {}", i + 1);
            let fg = if is_active { (220, 220, 255) } else { (160, 160, 180) };
            let bg_tuple = if is_active { (60, 60, 80) } else { (40, 40, 50) };
            for (ci, ch) in label.chars().enumerate() {
                if ci >= 12 {
                    break;
                }
                self.draw_glyph(ch, fg, bg_tuple, false, x + ci * self.cell_width, bar_y);
            }

            self.canvas.set_draw_color(Color::RGB(80, 80, 80));
            let _ = self.canvas.draw_line(
                sdl2::rect::Point::new((x + tab_width) as i32, bar_y as i32),
                sdl2::rect::Point::new((x + tab_width) as i32, (bar_y + bar_height) as i32),
            );
        }
    }

    fn resolve_fg(&self, style: &buffer::Style) -> (u8, u8, u8) {
        if style.reverse {
            match style.bg_color {
                buffer::Color::Default => self.config_bg,
                c => c.to_rgb_bg(),
            }
        } else {
            match style.fg_color {
                buffer::Color::Default => self.config_fg,
                c => c.to_rgb(),
            }
        }
    }

    fn resolve_bg(&self, style: &buffer::Style) -> (u8, u8, u8) {
        if style.reverse {
            match style.fg_color {
                buffer::Color::Default => self.config_fg,
                c => c.to_rgb(),
            }
        } else {
            match style.bg_color {
                buffer::Color::Default => self.config_bg,
                c => c.to_rgb_bg(),
            }
        }
    }

    pub fn cell_size(&self) -> (usize, usize) {
        (self.cell_width, self.cell_height)
    }
}

pub struct PaneData {
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub saved_cursor: Option<(usize, usize)>,
    pub style: buffer::Style,
}
