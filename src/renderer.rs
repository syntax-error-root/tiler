use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use std::collections::HashMap;

use crate::buffer;
use crate::config::Config;
use crate::layout;

const DEFAULT_CELL_WIDTH: usize = 8;
const DEFAULT_CELL_HEIGHT: usize = 16;

fn default_cell() -> buffer::Cell {
    buffer::Cell { ch: ' ', style: buffer::Style::default() }
}

/// Cached rasterized glyph bitmap
struct GlyphBitmap {
    width: usize,
    height: usize,
    /// The x pixel offset from the left edge of the cell where the bitmap starts.
    x_offset: i32,
    /// The y pixel offset from the top of the cell where the bitmap starts.
    /// Computed as: ascent_px - height - ymin
    y_offset: i32,
    /// Alpha coverage bitmap (0–255 per pixel), row-major
    pixels: Vec<u8>,
}

/// Cache key: character + bold flag
#[derive(Hash, PartialEq, Eq)]
struct GlyphKey {
    ch: char,
    bold: bool,
}

pub struct Renderer {
    canvas: Canvas<Window>,
    #[allow(dead_code)]
    texture_creator: TextureCreator<WindowContext>,
    cell_width: usize,
    cell_height: usize,
    /// Font ascent in pixels at the configured font size — distance from
    /// top of cell to the baseline.
    ascent_px: i32,
    font: Option<fontdue::Font>,
    font_size: f32,
    config_bg: (u8, u8, u8),
    config_fg: (u8, u8, u8),
    cursor_style: CursorStyle,
    #[allow(dead_code)]
    cursor_blink: bool,
    /// Per-glyph rasterized bitmap cache (avoids re-rasterizing every frame)
    glyph_cache: HashMap<GlyphKey, GlyphBitmap>,
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

        let mut window = video_subsystem
            .window("tiler", config.render.window_width, config.render.window_height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;

        if let Ok(icon) = create_icon() {
            let _ = window.set_icon(icon);
        }

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

        let texture_creator = canvas.texture_creator();

        let font = Self::load_font(&config.render.font_family);
        let (cell_width, cell_height, ascent_px) =
            Self::compute_cell_metrics(&font, config.render.font_size);

        let cursor_style = match config.render.cursor_style.as_str() {
            "underline" => CursorStyle::Underline,
            "bar" => CursorStyle::Bar,
            _ => CursorStyle::Block,
        };

        Ok(Renderer {
            canvas,
            texture_creator,
            cell_width,
            cell_height,
            ascent_px,
            font,
            font_size: config.render.font_size,
            config_bg: config.render.bg_color,
            config_fg: config.render.fg_color,
            cursor_style,
            cursor_blink: config.render.cursor_blink,
            glyph_cache: HashMap::new(),
        })
    }

    fn load_font(font_family: &str) -> Option<fontdue::Font> {
        let path = crate::config::resolve_font_path(font_family)?;
        let bytes = std::fs::read(&path).ok()?;
        fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
    }

    /// Compute cell dimensions using the font's own line metrics.
    /// Returns (cell_width, cell_height, ascent_px).
    fn compute_cell_metrics(font: &Option<fontdue::Font>, font_size: f32) -> (usize, usize, i32) {
        if let Some(f) = font {
            // Use 'M' advance width for the monospace cell width
            let (m_metrics, _) = f.rasterize('M', font_size);
            let cell_w = (m_metrics.advance_width.ceil() as usize).max(4);

            // Use the font's actual horizontal line metrics for height & ascent.
            if let Some(line_metrics) = f.horizontal_line_metrics(font_size) {
                // ascent: pixels from baseline to top of cell (positive above baseline)
                // descent: pixels from baseline to bottom (negative below baseline)
                let ascent = line_metrics.ascent.ceil() as i32;
                let descent = line_metrics.descent.abs().ceil() as i32;
                // line_gap adds extra space between lines
                let line_gap = line_metrics.line_gap.abs().ceil() as i32;
                let cell_h = (ascent + descent + line_gap).max(12) as usize;
                (cell_w, cell_h, ascent)
            } else {
                // Fallback: rasterize 'M' and estimate
                let glyph_h = m_metrics.height as i32;
                let ascent = (glyph_h as f32 * 0.8).ceil() as i32;
                let cell_h = (glyph_h + 4).max(12) as usize;
                (cell_w, cell_h, ascent)
            }
        } else {
            (DEFAULT_CELL_WIDTH, DEFAULT_CELL_HEIGHT, 13)
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

                // Fill cell background
                self.canvas.set_draw_color(Color::RGB(bg.0, bg.1, bg.2));
                let _ = self.canvas.fill_rect(Rect::new(
                    pixel_x as i32,
                    pixel_y as i32,
                    self.cell_width as u32,
                    self.cell_height as u32,
                ));

                // Underline decoration
                if cell.style.underline {
                    self.canvas.set_draw_color(Color::RGB(fg.0, fg.1, fg.2));
                    let ul_y = (pixel_y + self.cell_height).saturating_sub(2) as i32;
                    let _ = self.canvas.draw_line(
                        sdl2::rect::Point::new(pixel_x as i32, ul_y),
                        sdl2::rect::Point::new((pixel_x + self.cell_width) as i32, ul_y),
                    );
                }

                // Draw glyph
                if cell.ch != ' ' {
                    self.draw_glyph(cell.ch, fg, bg, cell.style.bold, pixel_x, pixel_y);
                }
            }
        }

        // Draw cursor
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
                        // Draw glyph with inverted colors on cursor
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

    /// Rasterize a glyph and cache it.
    fn get_or_rasterize(&mut self, ch: char, bold: bool) -> Option<GlyphBitmap> {
        let key = GlyphKey { ch, bold };
        if self.glyph_cache.contains_key(&key) {
            return None; // Signal: use cache
        }

        if let Some(font) = &self.font {
            let (metrics, pixels) = font.rasterize(ch, self.font_size);

            if metrics.width == 0 || metrics.height == 0 {
                return None;
            }

            // ── Industry-standard fontdue placement ──────────────────────────
            //
            // fontdue coordinate system (same as OpenType):
            //   - origin is at the baseline, pen position
            //   - Y increases UPWARD (opposite of screen)
            //   - xmin: pixels from pen_x to left edge of bitmap (can be negative)
            //   - ymin: pixels from baseline to BOTTOM of bitmap (negative = descender)
            //   - height: bitmap height in pixels
            //
            // Screen coordinate system (SDL2 top-left = 0,0, Y increases DOWN):
            //   - pen_x = left edge of cell
            //   - pen_y = top edge of cell
            //   - baseline_screen_y = pen_y + ascent_px
            //
            // To find where the TOP of the glyph bitmap lands on screen:
            //   top_of_bitmap_in_font = ymin + height    (above baseline, in font coords)
            //   screen_y = baseline_screen_y - top_of_bitmap_in_font
            //            = pen_y + ascent_px - (metrics.ymin + metrics.height)
            //
            let x_off = metrics.xmin;
            let y_off = self.ascent_px - metrics.ymin - metrics.height as i32;

            let bm = GlyphBitmap {
                width: metrics.width,
                height: metrics.height,
                x_offset: x_off,
                y_offset: y_off,
                pixels,
            };
            self.glyph_cache.insert(key, bm);
        }
        None
    }

    /// Draw a single glyph at (pixel_x, pixel_y) = top-left of its cell.
    fn draw_glyph(
        &mut self,
        ch: char,
        fg: (u8, u8, u8),
        bg: (u8, u8, u8),
        bold: bool,
        pixel_x: usize,
        pixel_y: usize,
    ) {
        // Ensure glyph is cached
        self.get_or_rasterize(ch, bold);

        let key = GlyphKey { ch, bold };
        let bm = match self.glyph_cache.get(&key) {
            Some(b) => b,
            None => return,
        };

        let cw = self.cell_width as i32;
        let ch_h = self.cell_height as i32;
        let glyph_w = bm.width;
        let glyph_h = bm.height;
        let x_off = bm.x_offset;
        let y_off = bm.y_offset;

        // Build a temporary ARGB surface for this glyph and blit to canvas.
        // This gives us proper per-pixel alpha compositing against the background.
        let surf_w = (glyph_w as u32).max(1);
        let surf_h = (glyph_h as u32).max(1);

        // We'll render directly pixel by pixel using the already-cleared background.
        // For each covered pixel, we alpha-composite fg over bg.
        let pixels_ref = bm.pixels.clone(); // clone to avoid borrow conflict

        for gy in 0..glyph_h {
            let screen_y = pixel_y as i32 + y_off + gy as i32;
            if screen_y < 0 || screen_y >= pixel_y as i32 + ch_h {
                continue;
            }
            for gx in 0..glyph_w {
                let screen_x = pixel_x as i32 + x_off + gx as i32;
                if screen_x < 0 || screen_x >= pixel_x as i32 + cw {
                    continue;
                }

                let alpha = pixels_ref[gy * glyph_w + gx];
                if alpha == 0 {
                    continue;
                }

                // Alpha-composite fg over bg using coverage as alpha
                // out = alpha * fg + (1 - alpha) * bg
                let a = alpha as u32;
                let inv_a = 255 - a;
                let r = ((a * fg.0 as u32 + inv_a * bg.0 as u32) / 255) as u8;
                let g = ((a * fg.1 as u32 + inv_a * bg.1 as u32) / 255) as u8;
                let b = ((a * fg.2 as u32 + inv_a * bg.2 as u32) / 255) as u8;

                self.canvas.set_draw_color(Color::RGB(r, g, b));
                let _ = self.canvas.draw_point(sdl2::rect::Point::new(screen_x, screen_y));
            }
        }

        // Suppress unused variable warnings
        let _ = surf_w;
        let _ = surf_h;
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
        let bar_y = 0usize;
        let bar_height = self.cell_height;
        let tab_width = self.cell_width * 12;

        for (i, _tab) in layout.tabs.iter().enumerate() {
            let x = i * tab_width;
            let is_active = i == layout.active_tab;

            let bg_color = if is_active {
                Color::RGB(60, 60, 80)
            } else {
                Color::RGB(40, 40, 50)
            };

            self.canvas.set_draw_color(bg_color);
            let _ = self.canvas.fill_rect(Rect::new(
                x as i32,
                bar_y as i32,
                tab_width as u32,
                bar_height as u32,
            ));

            let label = format!("Tab {}", i + 1);
            let fg = if is_active { (220, 220, 255) } else { (160, 160, 180) };
            let bg_tuple = if is_active { (60u8, 60u8, 80u8) } else { (40u8, 40u8, 50u8) };

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

/// Generate the app icon: dark rounded tile with `>_` prompt and subtle grid.
fn create_icon() -> Result<Surface<'static>, String> {
    const S: u32 = 64;
    let mut surface = Surface::new(S, S, sdl2::pixels::PixelFormatEnum::ARGB8888)
        .map_err(|e| e.to_string())?;

    let bg = Color::RGBA(0x1e, 0x1e, 0x2e, 0xff);
    let border = Color::RGBA(0x45, 0x47, 0x5a, 0xff);
    let grid = Color::RGBA(0x31, 0x32, 0x44, 0xff);
    let blue = Color::RGBA(0x89, 0xb4, 0xfa, 0xff);
    let green = Color::RGBA(0xa6, 0xe3, 0xa1, 0xff);

    // Background
    surface.fill_rect(None, bg).map_err(|e| e.to_string())?;

    // Dashed grid lines at center
    for i in (8..56).step_by(4) {
        surface.fill_rect(Some(Rect::new(31, i, 2, 2)), grid).ok();
        surface.fill_rect(Some(Rect::new(i, 31, 2, 2)), grid).ok();
    }

    // > prompt — top stroke: (14,20) → (30,32)
    draw_thick_line(&mut surface, 14, 20, 30, 32, blue, 4)?;
    // > prompt — bottom stroke: (30,32) → (14,44)
    draw_thick_line(&mut surface, 30, 32, 14, 44, blue, 4)?;

    // _ underscore
    surface.fill_rect(Some(Rect::new(33, 42, 16, 3)), green).ok();

    // Cursor block
    surface.fill_rect(Some(Rect::new(52, 34, 4, 10)), blue).ok();

    // Border (1px on each edge)
    surface.fill_rect(Some(Rect::new(0, 0, S, 1)), border).ok();
    surface.fill_rect(Some(Rect::new(0, (S - 1) as i32, S, 1)), border).ok();
    surface.fill_rect(Some(Rect::new(0, 0, 1, S)), border).ok();
    surface.fill_rect(Some(Rect::new((S - 1) as i32, 0, 1, S)), border).ok();

    Ok(surface)
}

/// Bresenham thick line drawing on a surface.
fn draw_thick_line(
    surface: &mut Surface,
    x0: i32, y0: i32, x1: i32, y1: i32,
    color: Color, thickness: i32,
) -> Result<(), String> {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;
    let half = thickness / 2;

    loop {
        surface.fill_rect(
            Some(Rect::new(x - half, y - half, thickness as u32, thickness as u32)),
            color,
        ).ok();

        if x == x1 && y == y1 { break; }
        let e2 = 2 * err;
        if e2 > -dy { err -= dy; x += sx; }
        if e2 < dx { err += dx; y += sy; }
    }
    Ok(())
}
