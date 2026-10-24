# Themes

A PrismDeck theme is **one CSS file** — `ui/themes/<id>/theme.css`. It can:

- override any design token in `ui/css/tokens.css` (colors, fonts, radii,
  spacing, region sizes);
- re-map the named grid areas in `ui/css/layout.css` to move whole regions;
- skew, rotate or scale cards via the transform tokens;
- hide or restyle any component with ordinary CSS.

## Design tokens

Tokens are plain CSS custom properties on `:root`. A theme overrides them by
re-declaring the same properties:

```css
:root {
  --pd-accent: #22d3ee;
  --pd-card-skew: -1.5deg;
  --pd-card-rotate: 0.6deg;
  --pd-card-scale: 1.02;
}
```

Key transform tokens (see `tokens.css` for the full set):

| token              | effect                          |
| ------------------ | ------------------------------- |
| `--pd-card-skew`   | horizontal skew of instance cards |
| `--pd-card-rotate` | rotation of instance cards      |
| `--pd-card-scale`  | scale of instance cards         |
| `--pd-sidebar-w`   | sidebar width                   |
| `--pd-topbar-h`    | topbar height                   |

## Rearranging the layout

Regions are placed with CSS grid and named areas. A theme can move the
sidebar to the right and the topbar to the bottom:

```css
#app {
  grid-template-columns: minmax(0, 1fr) var(--pd-sidebar-w);
  grid-template-areas:
    "statusbar statusbar"
    "content sidebar"
    "topbar topbar";
}
```

`[data-region="topbar"]`, `[data-region="sidebar"]`,
`[data-region="content"]` and `[data-region="statusbar"]` are the four named
areas.

## Adding a theme

1. Create `ui/themes/<id>/theme.css`.
2. The theme id is the folder name; the display name is derived from it.
3. The first `--pd-accent` value is used as the swatch in the theme picker.

## Hot reload

The backend watches `ui/themes` with a debounced filesystem watcher. When a
`theme.css` changes, the frontend re-fetches and re-applies the active theme
without a restart.
