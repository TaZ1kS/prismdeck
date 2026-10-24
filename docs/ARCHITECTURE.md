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

## Threading model (planned)

- Tauri commands are `async` where they touch the filesystem or spawn.
- Blocking work runs on `tauri::async_runtime::spawn_blocking`.
- Each launch gets a run ID, a bounded log ring buffer, and reader tasks for
  stdout/stderr.
- Log lines are batched (≈20–50 lines or 100 ms) before being emitted to the
  webview. This avoids flooding WebKit with thousands of IPC events.
- Themes are watched by a `notify` debouncer (100 ms) and pushed to the
  frontend as a single CSS payload.

## M0 scope

The current commit is a working Tauri v2 + vanilla JS shell with one native
command (`app_info`). Discovery, parsing and launch arrive in M1–M3.
