# Security Policy

## Supported Versions

Only the latest release is supported with security updates.

| Version | Supported |
| ------- | --------- |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x: |

## Reporting a Vulnerability

Report security vulnerabilities by opening a [private security advisory](https://github.com/syntax-error-root/tiler/security/advisories/new) on GitHub, or contact the maintainer directly via email.

- You can expect an acknowledgment within 48 hours.
- If the vulnerability is accepted, a fix will be prioritized and a new release published.
- If declined, you will receive an explanation of the reasoning.

## Security Considerations

Tiler spawns shell processes via PTY (`fork`/`exec`). The following areas are security-sensitive:

- **PTY handling** — Process spawning in `src/pty.rs` uses raw libc calls. Input is never evaluated beyond writing bytes to the PTY.
- **Escape sequence parsing** — `src/ansi.rs` parses incoming PTY output. Malformed or oversized sequences are handled gracefully without unsafe operations.
- **Configuration** — `src/config.rs` reads `~/.config/tiler/config.toml` via `serde`. Malformed TOML falls back to defaults rather than panicking.
- **No network access** — Tiler does not make outbound network connections or expose any network services.

## Scope

Tiler is a local terminal emulator intended for single-user Linux desktops. It is not designed for multi-user, sandboxed, or containerized environments.

The following are **out of scope**:

- Vulnerabilities in dependencies (SDL2, fontdue, libc, serde, toml) — report those upstream
- Issues requiring physical access or already-compromised user accounts
- Denial-of-service via shell commands run inside the terminal
