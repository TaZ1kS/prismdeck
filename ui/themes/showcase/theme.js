/* Showcase theme extension — demonstrates DOM-level customization across the
   whole app, not only the instances view. This plain script sets
   window.PRISMDECK_THEME. The app runs mount() when the theme is applied and
   runs unmount()/cleanups when it switches away. Everything it changes is
   restored on cleanup. */

window.PRISMDECK_THEME = {
  mount(ctx) {
    // 1) remove the stock "Settings" nav button (structural removal)…
    const settingsNav = ctx.$(".nav__item[data-nav='settings']");
    const navParent = settingsNav?.parentElement ?? null;
    const navNext = settingsNav?.nextSibling ?? null;
    settingsNav?.remove();
    ctx.onCleanup(() => {
      if (settingsNav && navParent) navParent.insertBefore(settingsNav, navNext);
    });

    // 2) …and put our own item in the side menu that opens Settings instead
    const navItem = ctx.html(
      '<button class="nav__item" type="button">✦ Preferences</button>'
    ).querySelector("button");
    navItem.addEventListener("click", () => ctx.actions.setView("settings"));
    ctx.$(".nav")?.append(navItem);
    ctx.onCleanup(() => navItem.remove());

    // 3) inject a custom button into the topbar using an HTML string
    const html = ctx.html(
      '<button class="btn btn--ghost btn--small" data-showcase-btn>⚡ Demo</button>'
    );
    const button = html.querySelector("[data-showcase-btn]");
    button.title = "Injected by the showcase theme";
    button.style.marginLeft = "auto";
    button.addEventListener("click", () => ctx.actions.setView("themes"));

    ctx.regions.topbar?.append(button);
    ctx.onCleanup(() => button.remove());

    // 4) inject a custom card into the Settings grid (static markup, so it
    //    survives re-renders; removed again on cleanup)
    const settingsGrid = ctx.$(".settings__grid");
    const settingsCard = ctx.html(`
      <section class="settings-section" data-showcase-card>
        <h2>Showcase</h2>
        <p class="settings-section__value">
          This card was injected into Settings by theme.js — themes can
          restructure every view, not just instances.
        </p>
      </section>
    `).querySelector("[data-showcase-card]");
    settingsGrid?.append(settingsCard);
    ctx.onCleanup(() => settingsCard?.remove());

    // 5) add a custom chip to the statusbar
    const badge = ctx.el("span", {
      text: "theme: showcase",
      style: { color: "var(--pd-accent)" },
    });
    ctx.regions.statusbar?.append(badge);
    ctx.onCleanup(() => badge.remove());
  },

  // Runs on every render; ctx.state.view tells us which view is visible.
  onRender(ctx) {
    ctx.root.dataset.showcaseView = ctx.state.view;
  },
};
