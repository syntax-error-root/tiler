# Terminal Tiler

A CPU-only terminal emulator with tiling support for old systems.

## Features

- Split terminals horizontally and vertically
- Navigate between panes using arrow keys
- Automatic cleanup when panes exit
- No GPU dependency
- Lightweight and fast

## Keyboard Shortcuts

- `Ctrl+Shift+H` - Split horizontally
- `Ctrl+Shift+V` - Split vertically  
- `Ctrl+Shift+Arrow Keys` - Navigate between panes
- `exit` - Close focused pane
- `Ctrl+C` - Exit emulator

## Installation

```bash
cargo build --release
./target/release/term-tiler
```

## Requirements

- Rust 1.70 or later
- Linux/Unix system (PTY support required)
- Terminal with ANSI support

## Usage

Run the emulator and use your shell as normal. Split your workspace
to run multiple commands simultaneously.
