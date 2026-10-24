/**
 * PrismDeck bootstrap — no framework, no build step.
 * Talks to Rust through Tauri's global API (`app.withGlobalTauri`).
 */

const $ = (selector, root = document) => root.querySelector(selector);

const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke;

const probeButton = $("#probe-btn");
const probeState = $("#probe-state");
const probeOutput = $("#probe-output");
const envMeta = $("#env-meta");
const appEl = $("#app");

function setProbeState(label, tone = "idle") {
  probeState.textContent = label;
  probeState.dataset.tone = tone;
}

function showError(error) {
  const message = error instanceof Error ? error.message : String(error);
  probeOutput.textContent = message;
  setProbeState("error", "error");
}

async function probe() {
  setProbeState("running", "running");
  probeOutput.textContent = "asking the Rust core…";

  try {
    if (!invoke) {
      throw new Error(
        "Tauri bridge is unavailable. Open the app through `cargo tauri dev`, not as a plain file."
      );
    }

    const info = await invoke("app_info");
    probeOutput.textContent = JSON.stringify(info, null, 2);
    envMeta.textContent = `${info.platform}/${info.arch} · ${info.debug ? "debug" : "release"}`;
    setProbeState("ok", "ok");
  } catch (error) {
    showError(error);
  }
}

probeButton?.addEventListener("click", probe);

requestAnimationFrame(() => {
  appEl.dataset.ready = "true";
});

// Auto-probe once so the M0 shell proves the native bridge immediately.
probe();
