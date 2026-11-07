# SodaM Design System

## Direction

Dark, cover-led desktop music player for evening listening on Ubuntu GNOME. The neutral shell stays near-black; the current artwork supplies a restrained hue wash and the accent is reserved for active playback and selection.

## Color roles

- Background: near-black neutral with a subtle ambient cover wash.
- Surface: slightly lifted charcoal for content and navigation.
- Elevated surface: queue, menus, and controls.
- Text: high-contrast white and cool gray secondary text.
- Accent: current cover-derived hue for play, progress, selection, and status.

## Typography

Use one system sans family with a compact scale. Titles use weight and spacing for hierarchy; body copy stays readable at the user's configured font size.

## Components

- Sidebar: compact navigation rows with a single active surface and accent icon.
- Artwork: 6px list radius, 12px card radius, soft hover lift without oversized rounding.
- Player bar: persistent three-zone layout, strong center play control, thin progress rail, clear queue affordance.
- Lists: dense, keyboard-friendly rows with artwork, metadata, like/cache/download actions.
- Queue drawer: elevated panel with active-track emphasis and direct reordering.

## Motion

Use short ease-out transitions for hover, selection, queue reveal, and player state changes. Artwork and content may crossfade or lift slightly, but motion must not block playback. Respect reduced motion in settings.
