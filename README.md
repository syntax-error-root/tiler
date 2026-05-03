# term-tiler

A lightweight, CPU-only terminal emulator with tiling support. Inspired by Ghostty, but without any GPU dependency — pure software rendering via SDL2.

## Features

- **Tiling** — Split terminals horizontally and vertically
- **Tabs** — Multiple tab workspaces
- **Scrollback** — Up to 10,000 lines (configurable)
- **ANSI colors** — Full 8-color foreground/background, bold, italic, underline
- **Cursor styles** — Block, underline, or bar (configurable)
- **Mouse support** — Click to focus panes, scroll wheel for scrollback
- **Configurable** — TOML config file for fonts, colors, keybinds
- **No GPU required** — CPU-only rendering via SDL2 + fontdue

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+A, H` | Split horizontally |
| `Ctrl+A, V` | Split vertically |
| `Ctrl+A, J/K/L/P` | Navigate down/up/right/left |
| `Ctrl+A, T` | New tab |
| `Ctrl+A, W` | Close tab |
| `Ctrl+A, N` | Next tab |
| `Ctrl+A, A` | Send literal Ctrl+A |
| `Ctrl+C` | Quit |
| Mouse wheel | Scroll scrollback |
| Mouse click | Focus pane |

## Installation

```bash
# Install SDL2 development library
sudo apt-get install libsdl2-dev

# Build
cargo build --release

# Run
./target/release/term-tiler
```

## Configuration

Config file location: `~/.config/term-tiler/config.toml`

```toml
[render]
font_family = "JetBrains Mono"
font_size = 14.0
cursor_style = "block"       # block | underline | bar
cursor_blink = true
scrollback_lines = 10000
window_width = 1200
window_height = 800

[render.colors]  # coming soon
background = [30, 30, 30]
foreground = [220, 220, 220]

[keybinds]
prefix = "CtrlA"
split_horizontal = "h"
split_vertical = "v"
new_tab = "t"
close_tab = "w"
next_tab = "n"
prev_tab = "p"
```

All fields are optional — unspecified values use sensible defaults.

## Uninstall

```bash
# Remove the compiled binary and build artifacts
cargo clean

# Remove the config directory
rm -rf ~/.config/term-tiler

# (Optional) Uninstall SDL2 if no longer needed
sudo apt-get remove libsdl2-dev
```

## Requirements

- Rust 1.70+
- SDL2 (`libsdl2-dev`)
- Linux (PTY support required)
- A monospace font (defaults to system monospace)

## Architecture

```
src/
  main.rs       — SDL2 event loop, PTY I/O, ANSI processing
  renderer.rs   — SDL2 canvas rendering, glyph rasterization (fontdue)
  input.rs      — SDL2 keyboard → PTY byte sequence mapping
  ansi.rs       — ANSI escape sequence parser
  buffer.rs     — Terminal buffer with scrollback (VecDeque)
  layout.rs     — Pane tiling + tab management
  pty.rs        — PTY wrapper (raw libc fork/exec)
  config.rs     — TOML configuration loader
```

## License

MIT
