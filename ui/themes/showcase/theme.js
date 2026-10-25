/* Showcase theme extension — demonstrates DOM-level customization.
   This plain script sets window.PRISMDECK_THEME. The app runs mount() when
   the theme is applied and runs unmount()/cleanups when it switches away. */

window.PRISMDECK_THEME = {
  mount(ctx) {
    // 1) delete the "Settings" nav button entirely (structural removal)
    ctx.$(".nav__item[data-nav='settings']")?.remove();

    // 2) inject a custom button into the topbar using an HTML string
    const html = ctx.html(
      '<button class="btn btn--ghost btn--small" data-showcase-btn>⚡ Demo</button>'
    );
    const button = html.querySelector("[data-showcase-btn]");
    button.title = "Injected by the showcase theme";
    button.style.marginLeft = "auto";
    button.addEventListener("click", () => ctx.actions.setView("themes"));

    ctx.regions.topbar?.append(button);
    ctx.onCleanup(() => button.remove());

    // 3) add a custom chip to the statusbar
    const badge = ctx.el("span", {
      text: "theme: showcase",
      style: { color: "var(--pd-accent)" },
    });
    ctx.regions.statusbar?.append(badge);
    ctx.onCleanup(() => badge.remove());
  },
};
