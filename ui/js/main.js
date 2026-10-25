/**
 * PrismDeck frontend — vanilla JS, no bundler, no framework.
 * All filesystem and process work goes through Tauri commands in Rust.
 */

const $ = (selector, root = document) => root.querySelector(selector);

const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke;

const ui = {
  app: $("#app"),
  loading: $("#loading"),
  onboarding: $("#onboarding"),
  dashboard: $("#dashboard"),
  errorPanel: $("#error-panel"),
  errorMessage: $("#error-message"),
  refreshBtn: $("#refresh-btn"),
  detectBtn: $("#detect-btn"),
  retryDetect: $("#retry-detect"),
  errorRetry: $("#error-retry"),
  demoBtn: $("#demo-btn"),
  errorDemo: $("#error-demo"),
  sourceSelect: $("#source-select"),
  sourceNote: $("#source-note"),
  search: $("#search-input"),
  envMeta: $("#env-meta"),
  groupList: $("#group-list"),
  detectedList: $("#detected-list"),
  deckTitle: $("#deck-title"),
  deckSubtitle: $("#deck-subtitle"),
  statInstances: $("#stat-instances"),
  statGroups: $("#stat-groups"),
  statLast: $("#stat-last"),
  warnings: $("#warnings"),
  resultCount: $("#result-count"),
  activeFilter: $("#active-filter"),
  clearFilter: $("#clear-filter"),
  instanceGrid: $("#instance-grid"),
  emptyDeck: $("#empty-deck"),
  emptyClear: $("#empty-clear"),
  statusVersion: $("#status-version"),
  statusInstall: $("#status-install"),
  statusCount: $("#status-count"),
  themeLabel: $("#theme-label"),
  modal: $("#instance-modal"),
  modalIcon: $("#modal-icon"),
  modalEyebrow: $("#modal-eyebrow"),
  modalTitle: $("#modal-title"),
  modalSubtitle: $("#modal-subtitle"),
  modalDetails: $("#modal-details"),
  modalNotes: $("#modal-notes"),
  modalNotesText: $("#modal-notes-text"),
  modalLaunch: $("#modal-launch"),
  modalLaunchHint: $("#modal-launch-hint"),
  modalConsole: $("#modal-console"),
  modalWrapper: $("#modal-wrapper"),
  modalOffline: $("#modal-offline"),
  modalShowWindow: $("#modal-show-window"),
  modalClose: $(".modal__close"),
  consoleModal: $("#console-modal"),
  consoleTitle: $("#console-title"),
  consoleStatus: $("#console-status"),
  consoleRunId: $("#console-run-id"),
  consoleOutput: $("#console-output"),
  consoleStop: $("#console-stop"),
  consoleClose: $("#console-close"),
  consoleHint: $("#console-hint"),
  themesPanel: $("#themes-panel"),
  themeList: $("#theme-list"),
  themeNote: $("#theme-note"),
  customDataDir: $("#custom-data-dir"),
  customExe: $("#custom-exe"),
  customApply: $("#custom-apply"),
  settingsPanel: $("#settings-panel"),
  settingsTheme: $("#settings-theme"),
  settingsWrapper: $("#settings-wrapper"),
  settingsOffline: $("#settings-offline"),
  settingsShowWindow: $("#settings-show-window"),
  settingsInstall: $("#settings-install"),
  settingsDetect: $("#settings-detect"),
  settingsVersion: $("#settings-version"),
  settingsPlatform: $("#settings-platform"),
  settingsThemeLabel: $("#settings-theme-label"),
  settingsTransparency: $("#settings-transparency"),
  settingsTransparencyValue: $("#settings-transparency-value"),
  settingsSkinChange: $("#settings-skin-change"),
  content: $('[data-region="content"]'),
};

const state = {
  appInfo: null,
  installs: [],
  snapshot: null,
  loading: true,
  error: null,
  query: "",
  /** null = all groups, "" = ungrouped, otherwise group name */
  group: null,
  selectedId: null,
  iconCache: new Map(),
  iconObserver: null,
  iconToken: 0,
  currentRun: null,
  consoleLines: [],
  consoleScrollLock: false,
  view: "instances",
  themes: [],
  activeTheme: null,
  settings: null,
  themeExtension: null,
};

// ── IPC helpers ────────────────────────────────────────────────

async function call(command, args = {}) {
  if (!invoke) {
    throw new Error(
      "Tauri bridge is unavailable. Open PrismDeck through `cargo tauri dev`, not as a plain HTML file."
    );
  }
  return invoke(command, args);
}

function errorMessage(error) {
  if (error instanceof Error) return error.message;
  return String(error);
}

// ── Formatting ─────────────────────────────────────────────────

function plural(count, singular, pluralForm = `${singular}s`) {
  return `${count} ${count === 1 ? singular : pluralForm}`;
}

function initialsFor(name) {
  const words = String(name).trim().split(/\s+/).filter(Boolean);
  if (words.length === 0) return "??";
  if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
  return `${words[0][0]}${words[1][0]}`.toUpperCase();
}

function shortPath(path) {
  const parts = String(path).split(/[\\/]/).filter(Boolean);
  return parts.slice(-2).join("/") || String(path);
}

function formatRelative(ms) {
  if (!ms) return "never";
  const diff = Date.now() - ms;
  const abs = Math.abs(diff);
  const minutes = Math.round(abs / 60_000);

  let value;
  let unit;
  if (minutes < 1) return "just now";
  if (minutes < 60) {
    value = minutes;
    unit = "m";
  } else if (minutes < 1440) {
    value = Math.round(minutes / 60);
    unit = "h";
  } else if (minutes < 43_200) {
    value = Math.round(minutes / 1440);
    unit = "d";
  } else if (minutes < 525_600) {
    value = Math.round(minutes / 43_200);
    unit = "mo";
  } else {
    value = Math.round(minutes / 525_600);
    unit = "y";
  }

  return diff >= 0 ? `${value}${unit} ago` : `in ${value}${unit}`;
}

function formatPlaytime(seconds) {
  const safe = Number.isFinite(seconds) ? Math.max(0, seconds) : 0;
  const totalMinutes = Math.floor(safe / 60);
  if (totalMinutes < 60) return `${totalMinutes}m`;

  const hours = Math.floor(totalMinutes / 60);
  if (hours < 24) return `${hours}h ${totalMinutes % 60}m`;

  const days = Math.floor(hours / 24);
  return `${days}d ${hours % 24}h`;
}

function versionLine(instance) {
  const parts = [];
  if (instance.mc_version) parts.push(`Minecraft ${instance.mc_version}`);
  if (instance.loader) {
    parts.push(
      instance.loader_version
        ? `${instance.loader} ${instance.loader_version}`
        : instance.loader
    );
  }
  return parts.join(" · ") || "Version unknown";
}

function groupLabel(group) {
  return group === "" ? "Ungrouped" : group;
}

function loaderSlug(loader) {
  if (!loader) return "vanilla";
  const value = loader.toLowerCase();
  if (value.includes("fabric")) return "fabric";
  if (value.includes("neoforge")) return "neoforge";
  if (value.includes("forge")) return "forge";
  if (value.includes("quilt")) return "quilt";
  return "unknown";
}

function slugify(value) {
  return String(value)
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "") || "ungrouped";
}

// ── Data loading ───────────────────────────────────────────────

function chooseBestInstall(installs) {
  return [...installs].sort((left, right) => {
    const score = (install) => (install.valid ? 2 : 0) + (install.exe ? 1 : 0);
    return score(right) - score(left) || left.kind.localeCompare(right.kind);
  })[0];
}

function ensureInstall(install) {
  if (!install) return;
  const index = state.installs.findIndex((item) => item.id === install.id);
  if (index === -1) {
    state.installs.push(install);
  } else {
    state.installs[index] = install;
  }
}

async function selectInstall(install) {
  const snapshot = await call("set_prism_install", { install });
  state.snapshot = snapshot;
  state.error = null;
  state.group = null;
  state.query = "";
  if (ui.search) ui.search.value = "";
  ensureInstall(snapshot.install);
}

async function detectAndSelect() {
  const installs = await call("detect_prism");
  state.installs = Array.isArray(installs) ? installs : [];

  if (state.installs.length > 0) {
    await selectInstall(chooseBestInstall(state.installs));
  } else {
    state.snapshot = null;
  }
}

// ── Render: sources, status, groups ────────────────────────────

function renderSources() {
  const active = state.snapshot?.install ?? null;
  const select = ui.sourceSelect;
  select.replaceChildren();

  if (state.installs.length === 0) {
    select.append(new Option("No source detected", ""));
    select.disabled = true;
  } else {
    select.disabled = false;
    for (const install of state.installs) {
      const suffix = install.valid ? "" : " (missing)";
      const option = new Option(
        `${install.label} — ${shortPath(install.data_dir)}${suffix}`,
        install.id
      );
      select.append(option);
    }
  }

  if (active) {
    if (!state.installs.some((install) => install.id === active.id)) {
      select.append(
        new Option(`${active.label} — ${shortPath(active.data_dir)}`, active.id)
      );
    }
    select.value = active.id;
  }

  if (state.loading) {
    ui.sourceNote.textContent = "Scanning Prism data…";
  } else if (active) {
    ui.sourceNote.textContent = `${active.kind} · ${active.source}${
      active.exe ? "" : " · browse only"
    }`;
  } else if (state.installs.length > 0) {
    ui.sourceNote.textContent = `${plural(
      state.installs.length,
      "candidate"
    )} found — select one to scan.`;
  } else {
    ui.sourceNote.textContent = "No Prism installation detected.";
  }
}

function renderStatus() {
  const active = state.snapshot?.install ?? null;
  ui.statusVersion.textContent = state.appInfo
    ? `PrismDeck ${state.appInfo.version}`
    : "PrismDeck";
  ui.statusInstall.textContent = active
    ? `${active.kind}${active.exe ? "" : " · browse only"}`
    : "no source";
  ui.statusCount.textContent = plural(state.snapshot?.instances.length ?? 0, "instance");
  ui.refreshBtn.disabled = state.loading || !active;
  ui.detectBtn.disabled = state.loading;
}

function addGroupButton(label, value, count, hidden = false) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "group-item";
  button.dataset.groupValue = value;

  const isActive =
    (value === "__all" && state.group === null) ||
    (value !== "__all" && state.group === value);
  if (isActive) button.classList.add("is-active");

  const dot = document.createElement("span");
  dot.className = "group-item__dot";
  dot.setAttribute("aria-hidden", "true");

  const name = document.createElement("span");
  name.className = "group-item__name";
  name.textContent = label;

  const counter = document.createElement("span");
  counter.className = "group-item__count";
  counter.textContent = String(count);

  button.append(dot, name);
  if (hidden) {
    const badge = document.createElement("span");
    badge.className = "group-item__hidden";
    badge.textContent = "hidden";
    button.append(badge);
  }
  button.append(counter);

  button.addEventListener("click", () => {
    state.group = value === "__all" ? null : value;
    render();
  });

  ui.groupList.append(button);
}

function renderGroups() {
  const snapshot = state.snapshot;
  if (!snapshot) {
    ui.groupList.replaceChildren();
    return;
  }

  ui.groupList.replaceChildren();
  addGroupButton("All instances", "__all", snapshot.instances.length);
  for (const group of snapshot.groups) {
    addGroupButton(
      groupLabel(group.name),
      group.name,
      group.count,
      group.hidden
    );
  }
}

// ── Filtering ──────────────────────────────────────────────────

function filteredInstances() {
  const snapshot = state.snapshot;
  if (!snapshot) return [];
  const query = state.query.trim().toLowerCase();

  return snapshot.instances.filter((instance) => {
    if (state.group !== null) {
      if (state.group === "" && instance.group !== "") return false;
      if (state.group !== "" && instance.group !== state.group) return false;
    }
    if (!query) return true;

    const haystack = [
      instance.name,
      instance.id,
      instance.mc_version,
      instance.loader,
      instance.loader_version,
      instance.managed_pack?.name,
      instance.managed_pack?.kind,
      instance.group,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();

    return haystack.includes(query);
  });
}

// ── Icons ──────────────────────────────────────────────────────

async function getInstanceIcon(instanceId) {
  if (state.iconCache.has(instanceId)) {
    return state.iconCache.get(instanceId);
  }

  try {
    const dataUrl = await call("get_instance_icon", { instanceId });
    state.iconCache.set(instanceId, dataUrl || null);
    return dataUrl || null;
  } catch {
    state.iconCache.set(instanceId, null);
    return null;
  }
}

function applyIcon(container, dataUrl) {
  if (!dataUrl) return;
  const initials = container.querySelector(".instance-icon__initials");
  let image = container.querySelector(".instance-icon__image");
  if (!image) {
    image = document.createElement("img");
    image.className = "instance-icon__image";
    image.alt = "";
    image.hidden = true;
    image.addEventListener(
      "load",
      () => {
        image.hidden = false;
        initials?.classList.add("is-hidden");
      },
      { once: true }
    );
    image.addEventListener("error", () => image.remove(), { once: true });
    container.append(image);
  }
  image.src = dataUrl;
}

async function loadCardIcon(instanceId, container) {
  if (container.dataset.iconLoaded === "1") return;
  container.dataset.iconLoaded = "1";
  const dataUrl = await getInstanceIcon(instanceId);
  applyIcon(container, dataUrl);
}

function setupIconObserver() {
  state.iconObserver?.disconnect();
  state.iconObserver = null;

  const icons = [...ui.instanceGrid.querySelectorAll(".instance-icon[data-icon-for]")];
  if (icons.length === 0) return;

  if (!("IntersectionObserver" in window)) {
    for (const icon of icons) {
      loadCardIcon(icon.dataset.iconFor, icon);
    }
    return;
  }

  state.iconObserver = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        state.iconObserver.unobserve(entry.target);
        loadCardIcon(entry.target.dataset.iconFor, entry.target);
      }
    },
    { root: ui.content, rootMargin: "180px 0px" }
  );

  for (const icon of icons) state.iconObserver.observe(icon);
}

// ── Render: onboarding / error / deck ──────────────────────────

function renderError() {
  ui.errorMessage.textContent = state.error || "Unknown error.";
  const demoAvailable = Boolean(state.appInfo?.demo_data_available);
  ui.errorDemo.hidden = !demoAvailable;
  ui.demoBtn.hidden = !demoAvailable;
}

function renderOnboarding() {
  const demoAvailable = Boolean(state.appInfo?.demo_data_available);
  ui.demoBtn.hidden = !demoAvailable;
  ui.detectedList.replaceChildren();

  if (state.installs.length === 0) {
    const hint = document.createElement("p");
    hint.className = "hint";
    hint.textContent =
      "No Prism candidates were found. Install Prism Launcher, then press Detect again.";
    ui.detectedList.append(hint);
    return;
  }

  for (const install of state.installs) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "detected-item";

    const meta = document.createElement("span");
    meta.className = "detected-item__meta";
    const title = document.createElement("strong");
    title.textContent = install.label;
    const path = document.createElement("span");
    path.textContent = install.data_dir;
    meta.append(title, path);

    const kind = document.createElement("span");
    kind.className = "detected-item__kind";
    kind.textContent = install.valid ? install.kind : `${install.kind} · missing`;

    button.append(meta, kind);
    button.addEventListener("click", () => handleInstallChoice(install));
    ui.detectedList.append(button);
  }
}

function addDetail(label, value, wide = false) {
  const row = document.createElement("div");
  row.className = wide ? "detail-row detail-row--wide" : "detail-row";

  const key = document.createElement("span");
  key.className = "detail-label";
  key.textContent = label;

  const text = document.createElement("span");
  text.className = "detail-value";
  text.textContent = value;

  row.append(key, text);
  ui.modalDetails.append(row);
}

function createCard(instance) {
  const card = document.createElement("button");
  card.type = "button";
  card.className = "instance-card";
  card.dataset.instanceId = instance.id;
  card.dataset.loader = loaderSlug(instance.loader);
  card.dataset.group = slugify(instance.group);
  card.dataset.managed = instance.managed_pack ? "true" : "false";
  card.dataset.pack = instance.managed_pack?.kind || "";
  card.dataset.mc = instance.mc_version || "";
  card.dataset.broken =
    !instance.mc_version && !instance.loader && !instance.managed_pack
      ? "true"
      : "false";
  card.setAttribute("aria-label", `Open details for ${instance.name}`);
  card.addEventListener("click", () => openInstance(instance.id));

  const top = document.createElement("div");
  top.className = "instance-card__top";

  const icon = document.createElement("div");
  icon.className = "instance-icon";
  icon.dataset.iconFor = instance.id;
  const initials = document.createElement("span");
  initials.className = "instance-icon__initials";
  initials.textContent = initialsFor(instance.name);
  icon.append(initials);

  const heading = document.createElement("div");
  heading.className = "instance-card__heading";
  const name = document.createElement("span");
  name.className = "instance-card__name";
  name.textContent = instance.name;
  const sub = document.createElement("span");
  sub.className = "instance-card__sub";
  sub.textContent = versionLine(instance);
  heading.append(name, sub);
  top.append(icon, heading);

  const badges = document.createElement("div");
  badges.className = "badge-row";
  if (instance.loader) {
    const loader = document.createElement("span");
    loader.className = "badge badge--loader";
    loader.textContent = instance.loader;
    badges.append(loader);
  }
  if (instance.managed_pack) {
    const pack = document.createElement("span");
    pack.className = "badge badge--pack";
    pack.textContent = instance.managed_pack.name;
    pack.title = `${instance.managed_pack.kind} · ${instance.managed_pack.version}`;
    badges.append(pack);
  }
  if (!instance.loader && !instance.managed_pack) {
    const vanilla = document.createElement("span");
    vanilla.className = "badge";
    vanilla.textContent = "vanilla";
    badges.append(vanilla);
  }

  const footer = document.createElement("div");
  footer.className = "instance-card__footer";
  const lastPlayed = document.createElement("span");
  lastPlayed.textContent = formatRelative(instance.last_launch_ms);
  lastPlayed.title = instance.last_launch_ms
    ? new Date(instance.last_launch_ms).toLocaleString()
    : "Never launched";
  const dot = document.createElement("span");
  dot.className = "dot";
  const playtime = document.createElement("span");
  playtime.textContent = formatPlaytime(instance.total_played_s);
  const group = document.createElement("span");
  group.className = "instance-card__group";
  group.textContent = instance.group ? groupLabel(instance.group) : "";
  footer.append(lastPlayed, dot, playtime, group);

  card.append(top, badges, footer);
  return card;
}

function renderGrid() {
  const instances = filteredInstances();
  ui.instanceGrid.replaceChildren();

  ui.instanceGrid.hidden = instances.length === 0;
  ui.emptyDeck.hidden = instances.length !== 0;

  for (const instance of instances) {
    ui.instanceGrid.append(createCard(instance));
  }
  setupIconObserver();
}

function renderDeck() {
  const snapshot = state.snapshot;
  const install = snapshot?.install;
  if (!snapshot || !install) return;

  const total = snapshot.instances.length;
  const visible = filteredInstances();
  const latest = Math.max(
    0,
    ...snapshot.instances.map((instance) => instance.last_launch_ms || 0)
  );

  ui.deckTitle.textContent =
    total === 0
      ? "An empty deck."
      : total === 1
        ? "One instance ready."
        : `${total} instances ready.`;
  ui.deckSubtitle.textContent = `Read from ${install.label} · ${shortPath(
    install.data_dir
  )}`;

  ui.statInstances.textContent = String(total);
  ui.statGroups.textContent = String(
    snapshot.groups.filter((group) => group.name !== "").length
  );
  ui.statLast.textContent = latest ? formatRelative(latest) : "—";

  renderWarnings(snapshot.warnings);
  renderGroups();
  renderGrid();

  ui.resultCount.textContent =
    visible.length === total
      ? plural(total, "instance")
      : `${visible.length} of ${plural(total, "instance")}`;

  const filters = [];
  if (state.query.trim()) filters.push(`search: “${state.query.trim()}”`);
  if (state.group !== null) filters.push(`group: ${groupLabel(state.group)}`);

  if (filters.length > 0) {
    ui.activeFilter.textContent = filters.join(" · ");
    ui.activeFilter.hidden = false;
    ui.clearFilter.hidden = false;
  } else {
    ui.activeFilter.hidden = true;
    ui.clearFilter.hidden = true;
  }
}

function renderWarnings(warnings) {
  ui.warnings.replaceChildren();
  if (!warnings || warnings.length === 0) {
    ui.warnings.hidden = true;
    return;
  }

  for (const warning of warnings) {
    const item = document.createElement("div");
    item.className = "warnings__item";
    item.textContent = warning;
    ui.warnings.append(item);
  }
  ui.warnings.hidden = false;
}

function render() {
  const hasSnapshot = Boolean(state.snapshot?.install);
  const showLoading = state.loading && !hasSnapshot;
  const showError = !state.loading && Boolean(state.error);
  const showOnboarding =
    !state.loading && !showError && !hasSnapshot && state.view === "instances";
  const showDeck = !showError && hasSnapshot && state.view === "instances";
  const showThemes = !state.loading && !showError && state.view === "themes";
  const showSettings = !state.loading && !showError && state.view === "settings";

  ui.loading.hidden = !showLoading;
  ui.errorPanel.hidden = !showError;
  ui.onboarding.hidden = !showOnboarding;
  ui.dashboard.hidden = !showDeck;
  ui.themesPanel.hidden = !showThemes;
  ui.settingsPanel.hidden = !showSettings;

  renderSources();
  renderStatus();
  renderNav();
  syncChrome();

  if (showError) renderError();
  if (showOnboarding) renderOnboarding();
  if (showDeck) renderDeck();
  if (showThemes) renderThemes();
  if (showSettings) renderSettings();

  runThemeRender();

  ui.app.dataset.ready = "true";
}

/**
 * Reflect app state onto #app as data-* attributes, so themes can style by
 * view, platform, install kind or running state without any JS.
 */
function syncChrome() {
  const app = ui.app;
  app.dataset.view = state.view;
  app.dataset.theme = state.activeTheme || "";
  app.dataset.platform = state.appInfo?.platform || "";
  const install = state.snapshot?.install ?? null;
  app.dataset.installKind = install ? install.kind : "none";
  app.dataset.hasInstall = install ? "true" : "false";
  const running =
    state.currentRun &&
    (state.currentRun.status === "running" || state.currentRun.status === "stopping");
  app.dataset.running = running ? "true" : "false";
}

function renderSettings() {
  const select = ui.settingsTheme;
  select.replaceChildren();
  for (const theme of state.themes) {
    select.append(new Option(theme.name, theme.id));
  }
  if (state.activeTheme) select.value = state.activeTheme;

  const install = state.snapshot?.install ?? null;
  ui.settingsInstall.textContent = install
    ? `${install.label} — ${install.data_dir}`
    : "No source selected.";

  ui.settingsVersion.textContent = state.appInfo ? state.appInfo.version : "0.1.0";
  ui.settingsPlatform.textContent = state.appInfo
    ? `${state.appInfo.platform}/${state.appInfo.arch}`
    : "—";
}

// ── Actions ────────────────────────────────────────────────────

async function withLoading(action) {
  state.loading = true;
  state.error = null;
  render();
  try {
    await action();
  } catch (error) {
    state.error = errorMessage(error);
  } finally {
    state.loading = false;
    render();
  }
}

async function handleDetect() {
  await withLoading(async () => {
    await detectAndSelect();
  });
}

async function handleRefresh() {
  if (!state.snapshot?.install) {
    await handleDetect();
    return;
  }

  await withLoading(async () => {
    const snapshot = await call("refresh_instances");
    state.snapshot = snapshot;
    ensureInstall(snapshot.install);
  });
}

async function handleInstallChoice(install) {
  await withLoading(async () => {
    await selectInstall(install);
  });
}

async function loadDemoSnapshot() {
  const snapshot = await call("load_demo_data");
  state.snapshot = snapshot;
  state.installs = snapshot.install ? [snapshot.install] : [];
  state.group = null;
  state.query = "";
  ui.search.value = "";
}

async function handleDemo() {
  await withLoading(loadDemoSnapshot);
}

async function handleCustomInstall() {
  const dataDir = ui.customDataDir.value.trim();
  if (!dataDir) {
    ui.customDataDir.focus();
    return;
  }
  const exe = ui.customExe.value.trim() || null;

  await withLoading(async () => {
    const snapshot = await call("set_custom_install", { dataDir, exe });
    state.snapshot = snapshot;
    ensureInstall(snapshot.install);
    state.group = null;
    state.query = "";
    ui.search.value = "";
    ui.customDataDir.value = "";
    ui.customExe.value = "";
  });
}

function clearFilters() {
  state.query = "";
  state.group = null;
  ui.search.value = "";
  render();
}

// ── Themes ─────────────────────────────────────────────────────

const THEME_LINK_ID = "pd-theme-live";

function renderNav() {
  for (const item of document.querySelectorAll(".nav__item")) {
    const name = item.dataset.nav;
    if (!name) continue;
    item.classList.toggle("is-active", state.view === name);
  }
}

/**
 * Apply a theme by swapping a <link rel="stylesheet">. Using a link (rather
 * than an inline <style>) means relative url() and @font-face inside a theme
 * resolve against the theme's own folder, so a theme can ship its own fonts
 * and background images.
 */
function setThemeLink(id) {
  let link = document.getElementById(THEME_LINK_ID);

  if (id === "default") {
    if (link) link.remove();
    return;
  }

  if (!link) {
    link = document.createElement("link");
    link.id = THEME_LINK_ID;
    link.rel = "stylesheet";
    document.head.append(link);
  }
  link.href = `themes/${id}/theme.css?t=${Date.now()}`;
}

/* ── Theme DOM extensions ────────────────────────────────────────
   A theme may ship an optional `theme.js` (a plain script) that assigns
   `window.PRISMDECK_THEME = { mount(ctx), unmount(ctx) }`. mount runs once
   on apply and may inject/remove/reorder DOM in the stable regions (topbar,
   sidebar, content shell, statusbar); `ctx.onCleanup` registers teardown
   that runs on switch. Dynamic lists (instance grid) are still best styled
   via CSS + data-* hooks, because they are re-rendered by the app. */

function el(tag, props = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(props || {})) {
    if (key === "className") node.className = value;
    else if (key === "text") node.textContent = value;
    else if (key === "html") node.innerHTML = value;
    else if (key === "style" && value && typeof value === "object")
      Object.assign(node.style, value);
    else if (key.startsWith("on") && typeof value === "function")
      node.addEventListener(key.slice(2).toLowerCase(), value);
    else if (value !== false && value != null)
      node.setAttribute(key, value === true ? "" : value);
  }
  for (const child of children.flat()) {
    if (child == null) continue;
    node.append(child.nodeType ? child : document.createTextNode(String(child)));
  }
  return node;
}

function makeThemeContext() {
  const cleanups = [];
  return {
    root: ui.app,
    state,
    regions: {
      topbar: document.querySelector('[data-region="topbar"]'),
      sidebar: document.querySelector('[data-region="sidebar"]'),
      content: document.querySelector('[data-region="content"]'),
      statusbar: document.querySelector('[data-region="statusbar"]'),
    },
    $: (selector, parent = document) => parent.querySelector(selector),
    $$: (selector, parent = document) => Array.from(parent.querySelectorAll(selector)),
    el,
    html: (htmlString) => {
      const template = document.createElement("template");
      template.innerHTML = htmlString.trim();
      return template.content;
    },
    actions: {
      setView: (view) => {
        state.view = view;
        render();
      },
      openInstance: (instanceId) => openInstance(instanceId),
      launchInstance: (instanceId) => {
        const instance = state.snapshot?.instances.find((item) => item.id === instanceId);
        if (instance) launchInstance(instance);
      },
      refresh: () => handleRefresh(),
      clearFilters: () => clearFilters(),
      applyTheme: (id) => applyTheme(id),
      stopRun: () => stopRun(),
    },
    onCleanup: (fn) => {
      if (typeof fn === "function") cleanups.push(fn);
    },
    _cleanups: cleanups,
  };
}

function runThemeCleanup() {
  const extension = state.themeExtension;
  if (!extension) return;
  try {
    extension.hook?.unmount?.(extension.ctx);
  } catch {
    /* ignore teardown errors */
  }
  for (const fn of extension.ctx._cleanups) {
    try {
      fn();
    } catch {
      /* ignore teardown errors */
    }
  }
  extension.scriptEl?.remove();
  delete window.PRISMDECK_THEME;
  state.themeExtension = null;
}

function loadThemeExtension(id) {
  return new Promise((resolve) => {
    runThemeCleanup();
    if (id === "default") {
      resolve();
      return;
    }

    const script = document.createElement("script");
    script.src = `themes/${id}/theme.js?t=${Date.now()}`;
    script.onload = () => {
      const hook = window.PRISMDECK_THEME;
      if (hook && typeof hook.mount === "function") {
        const ctx = makeThemeContext();
        state.themeExtension = { hook, ctx, scriptEl: script };
        try {
          hook.mount(ctx);
        } catch (error) {
          console.error(`theme "${id}" mount failed`, error);
        }
      }
      resolve();
    };
    script.onerror = () => resolve();
    document.head.append(script);
  });
}

/** Re-run the active theme's onRender hook after the app re-renders, so a
    theme can transform the dynamic instance grid (grouping, reordering). */
function runThemeRender() {
  const extension = state.themeExtension;
  if (!extension?.hook?.onRender) return;
  try {
    extension.hook.onRender(extension.ctx);
  } catch (error) {
    console.error("theme onRender failed", error);
  }
}

function renderThemes() {
  ui.themeList.replaceChildren();

  if (state.themes.length === 0) {
    ui.themeNote.textContent =
      "No themes found. Expected a ui/themes directory next to the app.";
    return;
  }

  ui.themeNote.textContent = `Live reload is on — edit any theme.css and it re-applies. ${plural(
    state.themes.length,
    "theme"
  )} found.`;

  for (const theme of state.themes) {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "theme-card";
    card.dataset.themeId = theme.id;
    if (state.activeTheme === theme.id) card.classList.add("is-active");

    const swatch = document.createElement("span");
    swatch.className = "theme-card__swatch";
    if (theme.accent) {
      swatch.style.background = `linear-gradient(135deg, ${theme.accent}, var(--pd-accent-2))`;
    }

    const meta = document.createElement("span");
    meta.className = "theme-card__meta";
    const name = document.createElement("span");
    name.className = "theme-card__name";
    name.textContent = theme.name;
    const sub = document.createElement("span");
    sub.className = "theme-card__id";
    sub.textContent = theme.description
      ? `${theme.id} · ${theme.description}`
      : theme.id;
    meta.append(name, sub);

    card.append(swatch, meta);

    if (theme.scheme && theme.scheme !== "dark") {
      const scheme = document.createElement("span");
      scheme.className = "theme-card__scheme";
      scheme.textContent = theme.scheme;
      card.append(scheme);
    }

    if (state.activeTheme === theme.id) {
      const active = document.createElement("span");
      active.className = "theme-card__active";
      active.textContent = "active";
      card.append(active);
    }
    card.addEventListener("click", () => applyTheme(theme.id));
    ui.themeList.append(card);
  }
}

async function applyTheme(id) {
  state.activeTheme = id;
  setThemeLink(id);
  await loadThemeExtension(id);
  // mount() only appends the theme's chrome; let onRender() sync it with the
  // current view immediately (e.g. Prism hides its deck chrome off-instances).
  runThemeRender();
  localStorage.setItem("prismdeck.theme", id);
  ui.themeLabel.textContent = `theme: ${id}`;
  ui.settingsThemeLabel.textContent = id;
  if (state.settings) {
    state.settings.theme = id;
    await persistSettings();
  }
  renderThemes();
}

const FALLBACK_THEMES = [
  { id: "default", name: "Default", accent: "#ff7a45", scheme: "dark", description: "Card grid with a left sidebar" },
  { id: "arcade", name: "Arcade", accent: "#22d3ee", scheme: "dark", description: "Neon tiles, tilted, no bottom bar" },
  { id: "terminal", name: "Terminal", accent: "#4ade80", scheme: "dark", description: "Monospaced list, nav on top" },
  { id: "paper", name: "Paper", accent: "#c2672c", scheme: "light", description: "Light, centered, soft dividers" },
  { id: "split", name: "Split", accent: "#ffd23f", scheme: "dark", description: "Dock bottom, sidebar right, wide rows" },
  { id: "showcase", name: "Showcase", accent: "#ff3d81", scheme: "dark", description: "Exercises every customization feature" },
  { id: "prism", name: "Prism", accent: "#4caf50", scheme: "dark", description: "Mimics Prism Launcher's dark UI" },
  { id: "translucent", name: "Translucent", accent: "#7c5cff", scheme: "dark", description: "Frosted glass over a colorful background" },
];

async function loadThemes() {
  try {
    const response = await fetch("themes/manifest.json");
    const text = await response.text();
    const manifest = JSON.parse(text);
    state.themes = Array.isArray(manifest) ? manifest : FALLBACK_THEMES;
  } catch {
    state.themes = FALLBACK_THEMES;
  }

  const stored = state.settings?.theme || localStorage.getItem("prismdeck.theme");
  const id = state.themes.some((theme) => theme.id === stored) ? stored : "default";
  if (state.themes.some((theme) => theme.id === id)) {
    await applyTheme(id);
  } else {
    state.activeTheme = "default";
    setThemeLink("default");
    ui.themeLabel.textContent = "theme: default";
    ui.settingsThemeLabel.textContent = "default";
  }
}

async function loadSettings() {
  try {
    state.settings = await call("get_settings");
  } catch {
    state.settings = {
      theme: null,
      wrapper: null,
      offline: false,
      showWindow: false,
      server: null,
      world: null,
      skin: null,
      transparency: null,
    };
  }

  ui.modalWrapper.value = state.settings.wrapper || "";
  ui.modalOffline.checked = Boolean(state.settings.offline);
  ui.modalShowWindow.checked = Boolean(state.settings.showWindow);
  ui.settingsWrapper.value = state.settings.wrapper || "";
  ui.settingsOffline.checked = Boolean(state.settings.offline);
  ui.settingsShowWindow.checked = Boolean(state.settings.showWindow);

  const transparency = state.settings.transparency ?? 1;
  ui.settingsTransparency.value = String(transparency);
  applyTransparency(transparency);
}

function applyTransparency(value) {
  const t = Math.min(1, Math.max(0.3, Number(value) || 1));
  document.documentElement.style.setProperty("--pd-surface-alpha", String(0.72 * t));
  document.documentElement.style.setProperty("--pd-surface-strong-alpha", String(0.92 * t));
  if (ui.settingsTransparencyValue) {
    ui.settingsTransparencyValue.textContent = `${Math.round(t * 100)}%`;
  }
}

async function persistSettings() {
  if (!state.settings) return;
  try {
    state.settings = await call("save_settings", { settings: state.settings });
  } catch {
    /* persistence is best-effort */
  }
}

function registerThemeEvents() {
  const events = tauri?.event;
  if (!events?.listen) return;

  events.listen("theme-changed", (event) => {
    const id = event.payload?.id;
    if (!id || !state.activeTheme || id !== state.activeTheme) return;
    setThemeLink(id);
    loadThemeExtension(id);
  });
}

// ── Instance modal ─────────────────────────────────────────────

async function openInstance(instanceId) {
  const instance = state.snapshot?.instances.find((item) => item.id === instanceId);
  if (!instance) return;

  state.selectedId = instanceId;
  state.iconToken += 1;
  const token = state.iconToken;

  ui.modalEyebrow.textContent = instance.group
    ? `instance · ${groupLabel(instance.group)}`
    : "instance";
  ui.modalTitle.textContent = instance.name;
  ui.modalSubtitle.textContent = versionLine(instance);
  ui.modalDetails.replaceChildren();
  ui.modalNotes.hidden = !instance.notes;
  ui.modalNotesText.textContent = instance.notes;
  updateLaunchButton();

  addDetail("Minecraft", instance.mc_version || "Unknown");
  addDetail(
    "Loader",
    instance.loader
      ? `${instance.loader}${instance.loader_version ? ` ${instance.loader_version}` : ""}`
      : "Vanilla / unknown"
  );
  addDetail(
    "Managed pack",
    instance.managed_pack
      ? `${instance.managed_pack.name} · ${instance.managed_pack.version}`
      : "None",
    true
  );
  addDetail("Group", instance.group ? groupLabel(instance.group) : "Ungrouped");
  addDetail("Last played", formatRelative(instance.last_launch_ms));
  addDetail("Playtime", formatPlaytime(instance.total_played_s));
  addDetail("Instance ID", instance.id, true);

  ui.modalIcon.replaceChildren();
  const initials = document.createElement("span");
  initials.className = "instance-icon__initials";
  initials.textContent = initialsFor(instance.name);
  ui.modalIcon.append(initials);

  const dataUrl = await getInstanceIcon(instanceId);
  if (token === state.iconToken && state.selectedId === instanceId) {
    applyIcon(ui.modalIcon, dataUrl);
  }

  ui.modal.hidden = false;
  ui.modalClose.focus();
}

function closeModal() {
  ui.modal.hidden = true;
  state.selectedId = null;
  state.iconToken += 1;
}

// ── Launch / console ───────────────────────────────────────────

function selectedInstance() {
  return state.snapshot?.instances.find((item) => item.id === state.selectedId);
}

function updateLaunchButton() {
  const instance = selectedInstance();
  const canLaunch = Boolean(state.snapshot?.install?.exe) && Boolean(instance);
  const status = state.currentRun?.status;
  const busy = status === "running" || status === "stopping";

  ui.modalLaunch.disabled = !canLaunch || busy;
  if (!canLaunch) {
    ui.modalLaunch.title = "No Prism executable configured";
    ui.modalLaunchHint.textContent = "no executable configured";
  } else if (busy) {
    ui.modalLaunch.title = "An instance is already running";
    ui.modalLaunchHint.textContent = "stop the current run first";
  } else {
    ui.modalLaunch.title = "Launch through Prism";
    ui.modalLaunchHint.textContent = "starts Prism via CLI";
  }

  ui.modalConsole.hidden = !state.currentRun;
}

async function launchInstance(instanceArg) {
  const instance = instanceArg || selectedInstance();
  if (!instance) return;

  const options = {
    wrapper: ui.modalWrapper.value.trim() || null,
    showWindow: ui.modalShowWindow.checked,
    offline: ui.modalOffline.checked,
  };

  if (state.settings) {
    state.settings.wrapper = options.wrapper;
    state.settings.offline = options.offline;
    state.settings.showWindow = options.showWindow;
    await persistSettings();
  }

  ui.modalLaunch.disabled = true;
  ui.modalLaunch.textContent = "Launching…";
  try {
    const run = await call("launch_instance", { instanceId: instance.id, options });
    if (!state.currentRun || state.currentRun.run_id !== run.run_id) {
      state.currentRun = {
        run_id: run.run_id,
        instance_id: run.instance_id,
        status: "running",
        exit_code: null,
      };
    }
    state.consoleLines = [];
    openConsole(instance);
  } catch (error) {
    state.consoleLines = [];
    openConsole(instance);
    appendConsoleLine("system", `Launch failed: ${errorMessage(error)}`);
    state.currentRun = null;
  } finally {
    ui.modalLaunch.textContent = "Launch";
    updateLaunchButton();
  }
}

function openConsole(instance) {
  if (instance) ui.consoleTitle.textContent = instance.name;
  ui.consoleModal.hidden = false;
  renderConsoleStatus();
  renderConsoleOutput();
  if (!ui.consoleStop.disabled) ui.consoleStop.focus();
}

async function openConsoleForCurrentRun() {
  if (!state.currentRun) return;
  try {
    const result = await call("get_run_logs", { runId: state.currentRun.run_id });
    if (Array.isArray(result.lines)) state.consoleLines = result.lines;
  } catch {
    /* keep the lines already buffered */
  }
  openConsole(selectedInstance());
}

function closeConsole() {
  ui.consoleModal.hidden = true;
}

function appendConsoleLine(stream, text) {
  state.consoleLines.push({ stream, text, seq: state.consoleLines.length + 1 });
}

function renderConsoleStatus() {
  if (!state.currentRun) {
    ui.consoleStatus.textContent = "idle";
    ui.consoleStatus.dataset.tone = "error";
    ui.consoleRunId.textContent = "";
    ui.consoleStop.disabled = true;
    return;
  }

  const status = state.currentRun.status;
  ui.consoleStatus.textContent = status;
  ui.consoleStatus.dataset.tone =
    status === "running" || status === "stopping" ? "running" : "error";
  ui.consoleRunId.textContent = state.currentRun.run_id;
  ui.consoleStop.disabled = !(status === "running" || status === "stopping");
  ui.consoleStop.textContent = status === "stopping" ? "Force stop" : "Stop";
  ui.consoleHint.textContent =
    status === "running"
      ? "Live output from Prism"
      : status === "stopping"
        ? "Stopping process tree…"
        : "Run finished";
}

function renderConsoleOutput() {
  if (ui.consoleModal.hidden) return;
  ui.consoleOutput.textContent = state.consoleLines
    .map((line) => line.text)
    .join("\n");
  ui.consoleOutput.scrollTop = ui.consoleOutput.scrollHeight;
}

async function stopRun() {
  if (!state.currentRun) return;
  const runId = state.currentRun.run_id;
  state.currentRun.status = "stopping";
  renderConsoleStatus();
  try {
    const info = await call("stop_run", { runId });
    state.currentRun.status = info.status;
  } catch (error) {
    appendConsoleLine("system", `Stop failed: ${errorMessage(error)}`);
  }
  renderConsoleStatus();
  renderConsoleOutput();
}

function exitText(exit) {
  if (exit.status === "stopped") return "Process stopped.";
  if (exit.status === "exited") return "Process exited with code 0.";
  return `Process exited (${exit.exit_code ?? "signal"}).`;
}

function registerRunEvents() {
  const events = tauri?.event;
  if (!events?.listen) return;

  events.listen("launch-started", (event) => {
    const info = event.payload;
    state.currentRun = {
      run_id: info.run_id,
      instance_id: info.instance_id,
      status: "running",
      exit_code: null,
    };
    renderConsoleStatus();
    updateLaunchButton();
  });

  events.listen("launch-log", (event) => {
    const batch = event.payload;
    if (!state.currentRun || batch.run_id !== state.currentRun.run_id) return;
    for (const line of batch.lines) state.consoleLines.push(line);
    if (state.consoleLines.length > 2000) {
      state.consoleLines = state.consoleLines.slice(state.consoleLines.length - 2000);
    }
    renderConsoleOutput();
  });

  events.listen("launch-exit", (event) => {
    const exit = event.payload;
    if (!state.currentRun || exit.run_id !== state.currentRun.run_id) return;
    state.currentRun.status = exit.status;
    state.currentRun.exit_code = exit.exit_code;
    appendConsoleLine("system", exitText(exit));
    renderConsoleStatus();
    renderConsoleOutput();
    updateLaunchButton();
  });
}

// ── Events ─────────────────────────────────────────────────────

ui.refreshBtn.addEventListener("click", handleRefresh);
ui.detectBtn.addEventListener("click", handleDetect);
ui.retryDetect.addEventListener("click", handleDetect);
ui.errorRetry.addEventListener("click", handleDetect);
ui.demoBtn.addEventListener("click", handleDemo);
ui.errorDemo.addEventListener("click", handleDemo);
ui.customApply.addEventListener("click", handleCustomInstall);
ui.settingsTheme.addEventListener("change", () => {
  if (ui.settingsTheme.value) applyTheme(ui.settingsTheme.value);
});
ui.settingsDetect.addEventListener("click", handleDetect);
ui.settingsWrapper.addEventListener("input", () => {
  state.settings.wrapper = ui.settingsWrapper.value.trim() || null;
  ui.modalWrapper.value = ui.settingsWrapper.value;
  persistSettings();
});
ui.settingsOffline.addEventListener("change", () => {
  state.settings.offline = ui.settingsOffline.checked;
  ui.modalOffline.checked = ui.settingsOffline.checked;
  persistSettings();
});
ui.settingsShowWindow.addEventListener("change", () => {
  state.settings.showWindow = ui.settingsShowWindow.checked;
  ui.modalShowWindow.checked = ui.settingsShowWindow.checked;
  persistSettings();
});
ui.settingsTransparency.addEventListener("input", () => {
  const value = Number(ui.settingsTransparency.value);
  applyTransparency(value);
  state.settings.transparency = value;
  persistSettings();
});
ui.clearFilter.addEventListener("click", clearFilters);
ui.emptyClear.addEventListener("click", clearFilters);
ui.modalLaunch.addEventListener("click", launchInstance);
ui.modalConsole.addEventListener("click", openConsoleForCurrentRun);
ui.consoleStop.addEventListener("click", stopRun);

for (const item of document.querySelectorAll(".nav__item:not(:disabled)")) {
  item.addEventListener("click", () => {
    state.view = item.dataset.nav || "instances";
    render();
  });
}

ui.sourceSelect.addEventListener("change", () => {
  const install = state.installs.find((item) => item.id === ui.sourceSelect.value);
  if (install) handleInstallChoice(install);
});

ui.search.addEventListener("input", () => {
  state.query = ui.search.value;
  render();
});

for (const element of document.querySelectorAll("[data-close-modal]")) {
  element.addEventListener("click", closeModal);
}

for (const element of document.querySelectorAll("[data-close-console]")) {
  element.addEventListener("click", closeConsole);
}

document.addEventListener("keydown", (event) => {
  const target = event.target;
  const typing =
    target instanceof HTMLElement &&
    (target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT" ||
      target.isContentEditable);

  if (event.key === "Escape") {
    if (!ui.consoleModal.hidden) {
      closeConsole();
    } else if (!ui.modal.hidden) {
      closeModal();
    } else if (state.query) {
      clearFilters();
    }
    return;
  }

  if (event.key === "/" && !typing) {
    event.preventDefault();
    ui.search.focus();
    ui.search.select();
  }
});

// ── Boot ───────────────────────────────────────────────────────

async function boot() {
  registerRunEvents();
  registerThemeEvents();
  state.loading = true;
  render();

  try {
    state.appInfo = await call("app_info");
    ui.envMeta.textContent = `${state.appInfo.platform}/${state.appInfo.arch} · ${
      state.appInfo.debug ? "debug" : "release"
    }`;
    ui.demoBtn.hidden = !state.appInfo.demo_data_available;
    ui.errorDemo.hidden = !state.appInfo.demo_data_available;
    await loadSettings();
    await loadThemes();
    if (state.appInfo.demo_auto && state.appInfo.demo_data_available) {
      await loadDemoSnapshot();
    } else if (state.settings?.install) {
      try {
        await selectInstall(state.settings.install);
      } catch {
        await detectAndSelect();
      }
    } else {
      await detectAndSelect();
    }

    if (
      state.appInfo.auto_launch &&
      state.snapshot?.install?.exe &&
      state.snapshot.instances.length > 0
    ) {
      const instance = state.snapshot.instances[0];
      state.selectedId = instance.id;
      await launchInstance(instance);
    }
  } catch (error) {
    state.error = errorMessage(error);
  } finally {
    state.loading = false;
    render();
  }
}

requestAnimationFrame(() => {
  ui.app.dataset.ready = "true";
});

boot();
