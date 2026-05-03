use tiler::buffer;
use tiler::ansi;
use tiler::layout;
use tiler::config;

#[test]
fn test_config_defaults() {
    let config = config::Config::default();
    assert_eq!(config.render.font_size, 14.0);
    assert_eq!(config.render.scrollback_lines, 10000);
    assert_eq!(config.keybinds.prefix, "CtrlA");
}

#[test]
fn test_buffer_scrollback_integration() {
    let mut buf = buffer::Buffer::new(10, 3);
    buf.scrollback_limit = 5;

    // Fill buffer
    for y in 0..3 {
        buf.write(0, y, ('A' as u8 + y as u8) as char, buffer::Style::default());
    }

    // Scroll multiple times
    buf.scroll_up(1);
    buf.scroll_up(1);
    buf.scroll_up(1);
    assert_eq!(buf.scrollback_len(), 3);

    // Scroll view
    buf.scroll_view_up(2);
    assert_eq!(buf.scroll_offset, 2);
    buf.scroll_view_down(1);
    assert_eq!(buf.scroll_offset, 1);
    buf.reset_scroll();
    assert_eq!(buf.scroll_offset, 0);
}

#[test]
fn test_ansi_to_buffer_flow() {
    let actions = ansi::parse("AB\x1B[31mC\x1B[0mD\nE");
    assert_eq!(actions.len(), 8);
    assert_eq!(actions[0], ansi::Action::Write('A'));
    assert_eq!(actions[1], ansi::Action::Write('B'));
    assert_eq!(actions[2], ansi::Action::SetFgColor(ansi::Color::Red));
    assert_eq!(actions[3], ansi::Action::Write('C'));
    assert_eq!(actions[4], ansi::Action::Reset);
    assert_eq!(actions[5], ansi::Action::Write('D'));
    assert_eq!(actions[6], ansi::Action::Newline);
    assert_eq!(actions[7], ansi::Action::Write('E'));
}

#[test]
fn test_layout_with_tabs() {
    let mut layout = layout::Layout::new(80, 24);
    assert_eq!(layout.tabs.len(), 1);

    layout.split_horizontal(0).unwrap();
    assert_eq!(layout.active_panes().len(), 2);

    layout.new_tab();
    assert_eq!(layout.tabs.len(), 2);
    assert_eq!(layout.active_panes().len(), 1);

    layout.split_vertical(layout.focused_pane_id()).unwrap();
    assert_eq!(layout.active_panes().len(), 2);

    layout.prev_tab();
    assert_eq!(layout.active_tab, 0);
    assert_eq!(layout.active_panes().len(), 2);
}

#[test]
fn test_color_rgb_conversion() {
    assert_eq!(buffer::Color::Red.to_rgb(), (200, 50, 50));
    assert_eq!(buffer::Color::Blue.to_rgb_bg(), (30, 60, 140));
    assert_eq!(buffer::Color::Default.to_rgb(), (220, 220, 220));
    assert_eq!(buffer::Color::Default.to_rgb_bg(), (30, 30, 30));
}

#[test]
fn test_buffer_resize_with_scrollback() {
    let mut buf = buffer::Buffer::new(10, 5);
    buf.write(0, 0, 'X', buffer::Style::default());
    buf.scroll_up(2);
    assert_eq!(buf.scrollback_len(), 2);
    buf.resize(20, 3);
    assert_eq!(buf.width, 20);
    assert_eq!(buf.height, 3);
}
