# Memory

PrismDeck targets a light footprint, but the honest cost is dominated by the
webview, not by our Rust code. Tauri uses WebKitGTK on Linux, which runs the
page in a separate `WebKitWebProcess`.

Measured on Linux (x86-64, `:release` build, demo deck open, software
rendering under Xvfb — worst case):

| process              | RSS  | PSS  |
| -------------------- | ---- | ---- |
| `prismdeck` (Rust)   | 169 MB | 84 MB |
| `WebKitNetworkProcess` | 66 MB | 32 MB |
| `WebKitWebProcess`   | 405 MB | 316 MB |
| **total (PSS)**      | —    | **~422 MB** |

PSS is the fair number: it splits shared pages between processes, unlike RSS
which double-counts shared WebKit libraries.

Two caveats:

- The `WebKitWebProcess` figure is inflated by `LIBGL_ALWAYS_SOFTWARE=1`
  (no GPU) used for headless screenshots. On a normal desktop with hardware
  acceleration it is typically lower, so expect roughly **250–350 MB** total.
- The Rust backend alone (~84 MB PSS) still links WebKitGTK into the main
  process; the "30–50 MB" figure from early planning applies to the backend
  logic, not the whole app.

Compared to Electron-style shells (typically 400–800 MB for a similar UI),
PrismDeck is roughly half the size or less.

## Build-time memory

The release profile disables LTO and uses `codegen-units = 16`, which trades a
slightly larger binary for a much lower peak memory while compiling. Building
the full dependency tree still peaks around 1–2 GB on the final link; use the
debug profile on very constrained machines.

## Artifact sizes

| artifact | size |
| -------- | ---- |
| `.deb` (depends on system WebKitGTK) | ~3 MB |
| `.AppImage` (bundles WebKitGTK) | ~103 MB |
| release binary | ~7 MB |
