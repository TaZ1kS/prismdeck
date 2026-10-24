# PrismDeck

> Ultra-light deck shell for [Prism Launcher](https://prismlauncher.org/).
> Rust + Tauri v2 · vanilla JS · one-file CSS themes · Linux & Windows.

**PrismDeck** does not replace Prism Launcher. It discovers your existing
Prism install, reads your instances, launches them through Prism's CLI, and
adds a fast, themeable deck UI on top. Everything that needs the full Prism
GUI (accounts, modpack installers, Java management) is delegated back to
Prism.

## Status

Early development. Current milestone: **M0 — foundation**.

- [x] Tauri v2 + vanilla frontend scaffold
- [ ] Prism discovery (native / Flatpak / portable / custom)
- [ ] `instance.cfg` + `mmc-pack.json` + groups parsing
- [ ] launch engine with Gamescope / GameMode wrappers
- [ ] live stdout/stderr console
- [ ] one-file CSS themes + hot reload
- [ ] Linux AppImage/deb + Windows MSI release

## Requirements

- Rust 1.77+ and Cargo
- Tauri v2 prerequisites for your OS
- Linux: `webkit2gtk-4.1` (Arch: `sudo pacman -S webkit2gtk-4.1`)
- Prism Launcher installed if you want to launch instances

## Development

```bash
# one-time: Tauri CLI
cargo install tauri-cli --version '^2' --locked

# run
cargo tauri dev
```

The app is a plain static frontend in `ui/` — no Node.js, no bundler, no
runtime framework. Tauri serves it directly.

## Build

```bash
cargo tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`.

## Project layout

```
src-tauri/   Rust backend (discovery, parsing, launching, theme watcher)
ui/          static HTML/CSS/JS frontend
themes/      single-file CSS themes (loaded at runtime)
docs/        architecture and theming notes
fixtures/    fake Prism data for tests (no Prism install required)
```

## Theming

A theme is **one CSS file**. It can change all design tokens and rewrite the
app's named CSS grid areas, so the entire layout can move, skew, rotate or
hide regions without touching JavaScript.

See `docs/THEMES.md` (coming in M5).

## License

MIT. PrismDeck is unofficial and not affiliated with the Prism Launcher
project. No Prism or Minecraft assets are bundled.
