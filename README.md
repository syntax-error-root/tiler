<div align="center">

# tiler

*A lightweight, CPU-only terminal emulator with tiling support.*

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 2024](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

</div>

## Overview

Tiler is a minimal terminal emulator written in Rust, inspired by [Ghostty](https://ghostty.org/). It renders entirely on the CPU via SDL2 and [fontdue](https://github.com/mooman219/fontdue) — no GPU required. It supports tiling panes, tabs, scrollback, and a broad set of VT100/VT220 escape sequences.

## Features

- **Tiling** — Split panes horizontally and vertically
- **Tabs** — Multiple tab workspaces with tab bar
- **Scrollback** — Up to 10,000 lines (configurable)
- **ANSI Colors** — 8-color foreground/background, 256-color indexed, 24-bit RGB (SGR)
- **Text Styles** — Bold, italic, underline, reverse video
- **Cursor Styles** — Block, underline, or bar (configurable)
- **VT100/VT220 Support** — Scroll regions, origin mode, alternate screen buffer, bracketed paste, autowrap mode, custom tab stops, device attributes, cursor position reporting
- **Mouse Support** — Click to focus panes, scroll wheel for scrollback
- **Configurable** — TOML config file for fonts, colors, keybinds
- **No GPU Required** — Pure software rendering via SDL2 + fontdue

## Keyboard Shortcuts

All shortcuts use a prefix key (default: `Ctrl+A`):

| Shortcut | Action |
|---|---|
| `Ctrl+A`, `H` | Split pane horizontally |
| `Ctrl+A`, `V` | Split pane vertically |
| `Ctrl+A`, `J` | Navigate down |
| `Ctrl+A`, `K` | Navigate up |
| `Ctrl+A`, `L` | Navigate right |
| `Ctrl+A`, `P` | Navigate left |
| `Ctrl+A`, `T` | New tab |
| `Ctrl+A`, `W` | Close tab |
| `Ctrl+A`, `N` | Next tab |
| `Ctrl+A`, `B` | Previous tab |
| `Ctrl+A`, `A` | Send literal `Ctrl+A` |
| `Ctrl+C` | Quit |
| Mouse wheel | Scroll scrollback |
| Mouse click | Focus pane |

## Installation

### Prerequisites

- Rust 1.70+ (edition 2024)
- SDL2 development library
- Linux (PTY support required)
- A monospace font (defaults to system monospace)

### Build

```bash
# Install SDL2
sudo apt-get install libsdl2-dev

# Build
cargo build --release

# Run
./target/release/tiler
```

## Configuration

Config file: `~/.config/tiler/config.toml`

All fields are optional — unspecified values use defaults.

```toml
[render]
font_family = "JetBrains Mono"
font_size = 14.0
cursor_style = "block"       # block | underline | bar
cursor_blink = true
scrollback_lines = 10000
window_width = 1024
window_height = 768
bg_color = [30, 30, 30]
fg_color = [220, 220, 220]

[keybinds]
prefix = "CtrlA"
split_horizontal = "h"
split_vertical = "v"
new_tab = "t"
close_tab = "w"
next_tab = "n"
prev_tab = "b"
```

## Architecture

```
src/
  main.rs       — SDL2 event loop, PTY I/O, ANSI action dispatch
  ansi.rs       — ANSI/VT100 escape sequence parser
  buffer.rs     — Terminal buffer with scrollback (VecDeque-based)
  renderer.rs   — SDL2 canvas rendering, glyph rasterization (fontdue)
  layout.rs     — Pane tiling tree + tab management
  input.rs      — SDL2 keyboard → PTY byte sequence mapping
  pty.rs        — PTY wrapper (raw libc fork/exec)
  config.rs     — TOML configuration loader
  lib.rs        — Module re-exports
```

## Supported Escape Sequences

| Sequence | Description |
|---|---|
| CSI n A / B / C / D | Cursor up/down/forward/back |
| CSI n ; m H | Cursor position |
| CSI n J | Erase in display |
| CSI n K | Erase in line |
| CSI n L / M | Insert/delete lines |
| CSI n @ / P | Insert/delete characters |
| CSI n S / T | Scroll up/down |
| CSI r | Set scroll region (DECSTBM) |
| CSI s / u | Save/restore cursor |
| CSI ? 1 h/l | Cursor key mode (DECCKM) |
| CSI ? 25 h/l | Cursor visibility (DECTCEM) |
| CSI ? 7 h/l | Autowrap mode (DECAWM) |
| CSI ? 1049 h/l | Alternate screen buffer |
| CSI ? 2004 h/l | Bracketed paste mode |
| CSI ? 1 h/l | Origin mode (DECOM) |
| CSI 6n | Device status report (DSR) |
| CSI c | Device attributes (DA1) |
| CSI n m | SGR — colors and styles |
| CSI n q | Set cursor style (DECSCUSR) |
| ESC 7 / 8 | Save/restore cursor (DECSC/DECRC) |
| HTS / TBC | Set/clear tab stops |

## Uninstall

```bash
cargo clean
rm -rf ~/.config/tiler
# Optionally remove SDL2
sudo apt-get remove libsdl2-dev
```

## License

[MIT](LICENSE)
