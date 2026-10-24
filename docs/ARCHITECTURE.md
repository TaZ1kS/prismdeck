# Architecture

## Roles

**PrismDeck is a shell, not a launcher replacement.** Prism Launcher owns:

- accounts and authentication
- Java runtimes
- downloading Minecraft, loaders and modpacks
- instance editing and mod management
- game launching internals

PrismDeck owns:

- discovering an existing Prism install and its data directory
- reading instance metadata (read-only)
- presenting a fast, themable deck UI
- invoking Prism's CLI with optional wrapper commands
- showing process output and launch state

## Process boundaries

```
┌─────────────────────────┐        invoke/event        ┌──────────────────────┐
│ Vanilla frontend (ui/)  │ ◄────────────────────────► │ Rust core (Tauri v2) │
└─────────────────────────┘                            └──────────┬───────────┘
                                                                  │ std::process
                                                                  ▼
                                                       ┌──────────────────────┐
                                                       │ Prism Launcher CLI    │
                                                       │ (native / Flatpak)    │
                                                       └──────────────────────┘
```

The frontend never touches the filesystem or spawns processes directly.
All privileged operations go through explicit Tauri commands.

## Read-only Prism data

PrismDeck reads (never writes):

- `<data>/instances/<id>/instance.cfg`
- `<data>/instances/<id>/mmc-pack.json`
- `<data>/instgroups.json`
- `<data>/icons/*`
- `<data>/prismlauncher.cfg` (only for custom folder settings)

Wrapper-specific state (selected install, theme, wrapper command) lives in
PrismDeck's own app config directory.

## Threading model

- Tauri commands are `async` where they touch the filesystem or spawn.
- Blocking work runs on `tauri::async_runtime::spawn_blocking`.
- Each launch gets a run ID, a bounded log ring buffer (2000 lines), and
  reader threads for stdout/stderr.
- Log lines are batched (40 lines or 60 ms) before being emitted to the
  webview. This avoids flooding WebKit with thousands of IPC events.
- On Linux each launch starts a new session (`setsid`) so the whole process
  tree can be signalled with `kill(-pid, …)`. Windows uses `taskkill /T`.
- Themes are watched by a `notify` debouncer (120 ms) and pushed to the
  frontend via a `theme-changed` event; the frontend re-fetches the active
  theme's CSS and re-applies it.

## Current scope

M0–M5 are implemented: Tauri shell, Linux/Windows discovery, instance
parsing, the launch engine with wrapper support and a live console, the deck
UI, and the one-file theme engine with hot reload. Remaining: settings (M6),
packaging (M7) and release (M8).
