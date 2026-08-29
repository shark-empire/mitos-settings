# Icons

This directory is where per-category and per-app icons ship from. Nothing
in this repository loads images at runtime — `Category::icon()` in
`src/categories/*.rs` returns a freedesktop.org icon-theme name (e.g.
`"video-display"`, `"audio-speakers"`) rather than a path, so a real
desktop shell can resolve it against whatever icon theme the user has
selected (`theme.icon_theme`).

Expected layout, if/when actual artwork is added:

```
assets/icons/
├── scalable/ SVGs, preferred
│ ├── video-display.svg
│ ├── audio-speakers.svg
│ └── ...
└── 48x48/ PNG fallback for the SVG-less case
└── ...
```

Naming should match the freedesktop.org icon naming spec so third-party
icon themes can override these without MITOS-specific patching.
