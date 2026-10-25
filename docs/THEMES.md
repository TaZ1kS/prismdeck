# Themes

A PrismDeck theme is **one CSS file** — `ui/themes/<id>/theme.css` — plus
optional assets (fonts, images) in the same folder. Because the theme is
applied last, it can rewrite *everything*: colors, typography, card layout,
and the entire grid of regions. Each demo theme looks like a different program
on purpose.

## DOM extensions (structure, not just style)

A theme may also ship an optional **`theme.js`** — a plain script that can
change the *structure* of the interface: add a menu or button, remove or
reorder an element, inject arbitrary HTML. It assigns a global hook:

```js
window.PRISMDECK_THEME = {
  mount(ctx) {
    // remove the Settings nav button
    ctx.$(".nav__item[data-nav='settings']")?.remove();

    // add a custom button from an HTML string
    const button = ctx
      .html('<button class="btn btn--ghost btn--small">⚡ Demo</button>')
      .querySelector("button");
    button.addEventListener("click", () => ctx.actions.setView("themes"));
    ctx.regions.topbar?.append(button);
    ctx.onCleanup(() => button.remove());
  },
};
```

The context (`ctx`) exposes:

- `ctx.root` — the `#app` element
- `ctx.regions` — `{ topbar, sidebar, content, statusbar }`
- `ctx.$(selector, parent?)` / `ctx.$$(selector, parent?)` — query helpers
- `ctx.el(tag, props, ...children)` — element factory
- `ctx.html(htmlString)` — parse an HTML string into a fragment
- `ctx.actions` — `setView`, `openInstance`, `launchInstance`, `refresh`,
  `clearFilters`, `applyTheme`, `stopRun`
- `ctx.onCleanup(fn)` — register teardown run on switch

`mount` runs once when the theme is applied. Prefer injecting into the stable
regions (topbar/sidebar/statusbar/content shell); the instance grid is
re-rendered by the app, so theme it with CSS + `data-*` hooks instead.

`onRender(ctx)` runs after **every** render, on every view. Use
`ctx.state.view` (`instances` / `themes` / `settings`) to transform only the
view you care about. `mount` is the right place for chrome that should exist
for the whole session; `onRender` is the right place for DOM that must follow
re-rendered content.

## Styling every view (not just Instances)

A theme stylesheet loads last, so it can override *any* part of the app, not
only the instance deck. The stable surface map:

| area | selectors |
| -- | -- |
| shell | `#app`, `.topbar`, `.sidebar`, `.content`, `.statusbar` |
| side menu | `.nav`, `.nav__item[data-nav]`, `.nav__item.is-active`, `.group-item`, `.group-item__dot`, `.source-card` |
| Settings | `.settings`, `.settings__head`, `.settings__grid`, `.settings-section`, `.settings-section__row`, `.settings-list`, `.field`, `.field__label`, `.input`, `.input-range`, `.select`, `.check`, `.settings-skin`, `.settings-skin__preview` |
| Themes | `.themes`, `.themes__head`, `.theme-list`, `.theme-card`, `.theme-card__swatch`, `.theme-card__meta`, `.theme-card.is-active` |
| Instances | `.deck-head`, `.deck-toolbar`, `.instance-grid`, `.instance-card` |

Example — restyle the side menu and the Settings cards from a theme:

```css
.nav__item { border-radius: 4px; }
.nav__item.is-active { background: var(--pd-accent-soft); border-color: transparent; }
.settings-section { border-radius: 6px; background: var(--pd-surface-soft); }
.theme-card { border-radius: 6px; }
```

The bundled `prism` theme uses this to style all three views consistently;
`showcase` removes a nav item and injects its own buttons.

## How themes are loaded

- `ui/themes/manifest.json` lists the available themes (id, name, accent,
  scheme, author, description, tags). The frontend fetches it at startup.
- The active theme is applied via a `<link rel="stylesheet">`, so relative
  `url()` and `@font-face` inside the theme resolve against the theme's own
  folder. A theme can ship its own font or background image.
- In development a filesystem watcher watches `ui/themes` and re-links the
  active theme on every edit — hot reload, no restart.

## State hooks (data-* attributes)

The app exposes its state so themes can style by it without JavaScript:

On `#app`:

- `data-theme` — active theme id
- `data-view` — `instances` / `themes` / `settings`
- `data-platform` — `linux` / `windows`
- `data-install-kind` — `native` / `flatpak` / `portable` / `appimage` /
  `custom` / `data_only` / `none`
- `data-has-install` — `true` / `false`
- `data-running` — `true` when a launch is running

On each `.instance-card`:

- `data-loader` — `fabric` / `forge` / `neoforge` / `quilt` / `vanilla` / `unknown`
- `data-group` — slug of the group name (`ungrouped` when none)
- `data-managed` — `true` / `false`
- `data-pack` — `curseforge` / `modrinth` / …
- `data-mc` — e.g. `1.20.1`
- `data-broken` — `true` when metadata is missing

Example — recolor only Fabric instances:

```css
.instance-card[data-loader="fabric"] { border-color: #a8b0ff; }
```

## Design tokens

Tokens are plain CSS custom properties on `:root`. See `tokens.css` for the
full set. In addition to colors/fonts/radii/spacing, themes can override the
effect and motion tokens:

```css
:root {
  --pd-accent: #22d3ee;
  --pd-card-skew: -1.2deg;
  --pd-card-rotate: 0.8deg;
  --pd-backdrop-blur: 8px;
  --pd-icon-filter: saturate(1.4);
  --pd-dur-base: 320ms;
  --pd-ease: cubic-bezier(0.2, 0.8, 0.2, 1);
  --pd-scrollbar-thumb: rgba(34, 211, 238, 0.4);
  --pd-selection-bg: rgba(34, 211, 238, 0.3);
}
```

## Re-arranging the whole interface

Regions are placed with CSS grid and named areas (`topbar`, `sidebar`,
`content`, `statusbar`). A theme can move them:

```css
#app {
  grid-template-columns: minmax(0, 1fr) var(--pd-sidebar-w);
  grid-template-areas:
    "statusbar statusbar"
    "content sidebar"
    "topbar topbar";
}
```

…or switch from a card grid to a dense list:

```css
.instance-grid { grid-template-columns: minmax(0, 1fr); gap: 0; }
.instance-card {
  flex-direction: row; align-items: center;
  border: 0; border-bottom: 1px solid var(--pd-line);
  border-radius: 0; background: transparent; transform: none;
}
.instance-icon { width: 34px; height: 34px; border-radius: 0; }
.badge-row { display: none; }
```

…or hide a region entirely:

```css
.statusbar { display: none; }
#app { grid-template-rows: var(--pd-topbar-h) minmax(0, 1fr); grid-template-areas: "topbar topbar" "sidebar content"; }
```

## Custom fonts and images

Because themes load as a linked stylesheet, relative assets work:

```
ui/themes/<id>/theme.css
ui/themes/<id>/my-font.woff2
ui/themes/<id>/bg.png
```

```css
@font-face {
  font-family: "My Font";
  src: url("my-font.woff2") format("woff2");
}
:root { --pd-font-sans: "My Font", system-ui, sans-serif; }
body { background-image: url("bg.png"); }
```

## Bundled themes

| id | feel |
| -- | ---- |
| `default` | card grid, left sidebar |
| `arcade` | neon tiles, tilted, no bottom bar |
| `terminal` | monospaced list, nav across the top |
| `paper` | light, centered, soft-divided list |
| `split` | dock on the bottom, sidebar right, wide rows |
| `prism` | 1:1 Prism Launcher layout: toolbar, grouped tiles, right action panel |
| `translucent` | frosted glass panels over a colorful gradient |
| `showcase` | exercises every lever: hooks, transforms, fonts, texture, animations |

## Adding a theme

1. Create `ui/themes/<id>/theme.css` (and any assets).
2. Add an entry to `ui/themes/manifest.json`.
