# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in CanvasWM, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please email **canvaswm@dev** with:

1. A description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (if any)

### What to Expect

- **Acknowledgement** — within 48 hours
- **Assessment** — we will evaluate the severity and confirm the vulnerability within 7 days
- **Fix** — a patch will be developed and tested
- **Disclosure** — once a fix is available, we will coordinate public disclosure

### Scope

CanvasWM is a compositor — it runs with access to input devices and display hardware. Security-sensitive areas include:

- **IPC socket** — command injection through the Unix socket interface
- **Config parsing** — malicious config file processing
- **Shader compilation** — GLSL shader loading from user-provided paths
- **XWayland** — X11 protocol attack surface
- **Input handling** — keylogger resistance, grab security

### Out of Scope

- Vulnerabilities in upstream dependencies (Smithay, wlroots, Mesa) — please report those to their respective projects
- Denial of service through resource exhaustion on local machine (compositor runs as the user)

## Security Best Practices for Users

- Keep your system and graphics drivers up to date
- Only load shader files from trusted sources
- Restrict IPC socket permissions if exposing to other users

Thank you for helping keep CanvasWM secure.
