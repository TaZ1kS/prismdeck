/* Prism theme extension — rebuilds the interface into Prism Launcher's
   layout: a toolbar, a grouped icon grid, a right action sidebar and a
   two-layer status bar. Purely frontend; the Rust binary is untouched. */

window.PRISMDECK_THEME = {
  mount(ctx) {
    const S = {
      selectedId: null,
      panelTitle: null,
      panelLogo: null,
      playtime: null,
      version: null,
    };

    ctx.__prism = S;

    // ── 1. Toolbar ──────────────────────────────────────────────
    buildToolbar(ctx);

    // ── 2. Right action sidebar ─────────────────────────────────
    buildSide(ctx, S);

    // ── 3. Status bar ───────────────────────────────────────────
    buildStatus(ctx, S);

    // ── 4. Selection: click a tile to select (not open the modal) ──
    const onContentClick = (event) => {
      const card = event.target.closest(".instance-card");
      if (!card) return;
      event.preventDefault();
      event.stopPropagation();
      selectCard(ctx, S, card);
    };
    ctx.regions.content.addEventListener("click", onContentClick, true);
    ctx.onCleanup(() =>
      ctx.regions.content.removeEventListener("click", onContentClick, true)
    );
  },

  onRender(ctx) {
    groupGrid(ctx);
    refreshSelected(ctx);
    refreshStatus(ctx);
  },
};

/* ── helpers ─────────────────────────────────────────────────────── */

function toolButton(ctx, label, options = {}) {
  const classes = ["prism-tool-btn"];
  if (options.green) classes.push("prism-tool-btn--green");
  const button = ctx.el("button", {
    type: "button",
    className: classes.join(" "),
    title: options.title || "",
  });
  if (options.disabled) button.setAttribute("disabled", "");
  if (options.icon) {
    button.append(ctx.el("span", { text: options.icon }));
  }
  if (label) button.append(document.createTextNode(" " + label));
  if (options.arrow) {
    button.append(ctx.el("span", { className: "prism-tool-btn__arrow", text: " ▾" }));
  }
  if (options.onClick) button.addEventListener("click", options.onClick);
  return button;
}

function buildToolbar(ctx) {
  const topbar = ctx.regions.topbar;
  const search = ctx.$(".search", topbar);
  topbar.replaceChildren();

  const add = toolButton(ctx, "Add Instance", {
    green: true,
    icon: "➕",
    title: "Create a new instance (handled by Prism Launcher)",
    disabled: true,
  });
  const folders = toolButton(ctx, "Folders", {
    icon: "📁",
    arrow: true,
    title: "Open system folders",
    disabled: true,
  });

  const cat = ctx.el("span", { className: "prism-cat", text: "🐱", title: "Meow" });
  const settings = toolButton(ctx, "Settings", {
    icon: "⚙️",
    onClick: () => ctx.actions.setView("settings"),
  });
  const help = toolButton(ctx, "Help", { icon: "❓", disabled: true });
  const accounts = toolButton(ctx, "Accounts", { icon: "👤", disabled: true });

  const spacer1 = ctx.el("span", { className: "prism-tool-spacer" });
  const spacer2 = ctx.el("span", { className: "prism-tool-spacer" });

  topbar.append(
    search,
    add,
    folders,
    spacer1,
    cat,
    spacer2,
    settings,
    help,
    accounts
  );
}

function sideButton(ctx, icon, label, options = {}) {
  const classes = ["prism-side__btn"];
  if (options.launch) classes.push("prism-side__btn--launch");
  if (options.danger) classes.push("prism-side__btn--danger");

  const button = ctx.el("button", {
    type: "button",
    className: classes.join(" "),
    title: options.title || label,
  });
  if (options.disabled) button.setAttribute("disabled", "");
  button.append(ctx.el("span", { className: "ico", text: icon }));
  button.append(ctx.el("span", { text: label }));
  if (options.onClick) button.addEventListener("click", options.onClick);
  return button;
}

function sideLabel(ctx, text) {
  return ctx.el("div", { className: "prism-side__label", text });
}

function buildSide(ctx, S) {
  const sidebar = ctx.regions.sidebar;
  ctx.$(".nav", sidebar)?.remove();
  ctx.$(".sidebar__groups", sidebar)?.remove();
  ctx.$(".source-card", sidebar)?.remove();

  const panel = ctx.el("div", { className: "prism-side" });

  const header = ctx.el("div", { className: "prism-side__header" });
  const logo = ctx.el("div", { className: "prism-side__logo" });
  const headText = ctx.el("div", {});
  S.panelTitle = ctx.el("div", { className: "prism-side__title", text: "No instance selected" });
  const sub = ctx.el("div", { className: "prism-side__sub", text: "Select an instance" });
  headText.append(S.panelTitle, sub);
  header.append(logo, headText);
  S.panelLogo = logo;

  const launch = sideButton(ctx, "▶", "Launch", {
    launch: true,
    onClick: () => {
      if (S.selectedId) ctx.actions.launchInstance(S.selectedId);
    },
  });
  const kill = sideButton(ctx, "❌", "Kill", {
    title: "Stop the running process",
    onClick: () => ctx.actions.stopRun(),
  });
  const edit = sideButton(ctx, "🗂️", "Edit", { disabled: true });
  const changeGroup = sideButton(ctx, "🏷️", "Change Group", { disabled: true });

  const folder = sideButton(ctx, "📂", "Folder", { disabled: true });
  const exportBtn = sideButton(ctx, "📤", "Export", { disabled: true });
  const copy = sideButton(ctx, "📋", "Copy", { disabled: true });
  const del = sideButton(ctx, "🗑️", "Delete", { danger: true, disabled: true });
  const shortcut = sideButton(ctx, "🔗", "Create Shortcut", { disabled: true });

  panel.append(
    header,
    launch,
    kill,
    sideLabel(ctx, "Configure"),
    edit,
    changeGroup,
    sideLabel(ctx, "Files"),
    folder,
    exportBtn,
    copy,
    del,
    shortcut
  );

  sidebar.append(panel);
}

function buildStatus(ctx, S) {
  const statusbar = ctx.regions.statusbar;

  // top layer: news
  const news = ctx.el("div", { className: "statusbar__news" });
  news.append(ctx.el("span", { className: "dot" }));
  news.append(ctx.el("span", { text: "Prism Launcher Release 6.1, now available" }));
  const more = ctx.el("a", { text: "More news" });
  news.append(more);

  // bottom layer: version (left) + playtime (right) + resize grip
  const row = ctx.el("div", { className: "statusbar__row" });

  S.version = ctx.el("span", { text: "Minecraft —" });
  S.playtime = ctx.el("span", { text: "Total playtime: 0s" });
  const spacer = ctx.el("span", { className: "spacer" });
  const grip = ctx.el("span", { className: "prism-grip" });

  row.append(S.version, spacer, S.playtime, grip);

  statusbar.replaceChildren(news, row);
}

function titleFor(slug) {
  if (!slug || slug === "ungrouped") return "Ungrouped";
  return slug
    .split("-")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

function groupGrid(ctx) {
  const grid = ctx.$("#instance-grid");
  if (!grid) return;
  const cards = Array.from(grid.querySelectorAll(".instance-card"));
  if (cards.length === 0) return;

  const groups = new Map();
  for (const card of cards) {
    const slug = card.dataset.group || "ungrouped";
    if (!groups.has(slug)) groups.set(slug, []);
    groups.get(slug).push(card);
  }

  grid.replaceChildren();
  for (const [slug, list] of groups) {
    const section = ctx.el("section", { className: "prism-group" });
    const header = ctx.el("div", { className: "prism-group__header" });

    const toggle = ctx.el("button", {
      type: "button",
      className: "prism-group__toggle",
      text: "−",
    });
    toggle.addEventListener("click", () => {
      const collapsed = section.classList.toggle("is-collapsed");
      toggle.textContent = collapsed ? "+" : "−";
    });

    const title = ctx.el("span", {
      className: "prism-group__title",
      text: titleFor(slug),
    });
    const count = ctx.el("span", {
      className: "prism-group__count",
      text: `(${list.length})`,
    });

    header.append(toggle, title, count);

    const groupGrid = ctx.el("div", { className: "prism-group__grid" });
    for (const card of list) groupGrid.append(card);

    section.append(header, groupGrid);
    grid.append(section);
  }
}

function selectCard(ctx, S, card) {
  S.selectedId = card.dataset.instanceId || null;
  document.querySelectorAll(".instance-card").forEach((c) => c.classList.remove("is-selected"));
  card.classList.add("is-selected");

  const name = card.querySelector(".instance-card__name")?.textContent || "Instance";
  S.panelTitle.textContent = name;

  const icon = card.querySelector(".instance-icon")?.cloneNode(true);
  S.panelLogo.replaceChildren();
  if (icon) S.panelLogo.append(icon);

  refreshStatus(ctx);
}

function refreshSelected(ctx) {
  const S = ctx.__prism;
  if (!S) return;
  document.querySelectorAll(".instance-card").forEach((c) => {
    c.classList.toggle("is-selected", c.dataset.instanceId === S.selectedId);
  });
  const card = document.querySelector(
    `.instance-card[data-instance-id="${CSS.escape(S.selectedId || "")}"]`
  );
  if (card) {
    S.panelTitle.textContent =
      card.querySelector(".instance-card__name")?.textContent || "Instance";
  }
}

function formatPlaytime(seconds) {
  const total = Math.floor((Number(seconds) || 0) / 60);
  if (total < 60) return `${total}s`;
  const hours = Math.floor(total / 60);
  if (hours < 24) return `${hours}h ${total % 60}m`;
  return `${Math.floor(hours / 24)}d ${hours % 24}h`;
}

function refreshStatus(ctx) {
  const S = ctx.__prism;
  if (!S) return;
  const snapshot = ctx.state.snapshot;
  if (snapshot?.instances) {
    const total = snapshot.instances.reduce((sum, i) => sum + (i.total_played_s || 0), 0);
    S.playtime.textContent = `Total playtime: ${formatPlaytime(total)}`;
  }
  const selected = snapshot?.instances?.find((i) => i.id === S.selectedId);
  S.version.textContent = selected?.mc_version
    ? `Minecraft ${selected.mc_version}`
    : "Minecraft —";
}
