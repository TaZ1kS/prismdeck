# Themes

A PrismDeck theme is **one CSS file** — `ui/themes/<id>/theme.css`. Because the
theme is applied last, it can rewrite *everything*: colors, typography, card
layout, and the entire grid of regions. Each demo theme looks like a different
program on purpose.

## How themes are loaded

- `ui/themes/manifest.json` lists the available themes (id, display name,
  swatch accent). The frontend fetches it at startup.
- The active theme's `theme.css` is fetched and injected as a `<style>` tag
  after the base styles, so it always wins the cascade.
- In development a filesystem watcher watches `ui/themes` and re-fetches the
  active theme on every edit — hot reload, no restart.

## What a theme can change

| layer | how |
| ----- | --- |
| colors / fonts / radii | override `:root` design tokens |
| region layout | re-map `#app` grid areas (see below) |
| cards | restyle `.instance-grid`, `.instance-card`, `.instance-icon`, … |
| chrome | restyle/hide `.topbar`, `.sidebar`, `.statusbar`, `.nav` |

## Design tokens

Tokens are plain CSS custom properties on `:root`. See `tokens.css` for the
full set. The transform tokens drive the "deck" feel:

```css
:root {
  --pd-accent: #22d3ee;
  --pd-card-skew: -1.2deg;
  --pd-card-rotate: 0.8deg;
}
```

## Re-arranging the whole interface

Regions are placed with CSS grid and named areas. The four areas are
`topbar`, `sidebar`, `content` and `statusbar`. A theme can move them:

```css
#app {
  grid-template-columns: minmax(0, 1fr) var(--pd-sidebar-w);
  grid-template-areas:
    "statusbar statusbar"
    "content sidebar"
    "topbar topbar";
}
```

…or switch from a card grid to a dense list by restyling the card markup:

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

## Bundled themes

| id | feel |
| -- | ---- |
| `default` | card grid, left sidebar |
| `arcade` | neon tiles, tilted, no bottom bar |
| `terminal` | monospaced list, nav across the top |
| `paper` | light, centered, soft-divided list |
| `split` | dock on the bottom, sidebar right, wide rows |

## Adding a theme

1. Create `ui/themes/<id>/theme.css`.
2. Add an entry to `ui/themes/manifest.json`.
