# MyDNS UI Brand Integration

This document translates the My Systems Brand Profile into implementation rules for the MyDNS web UI.

## Product identity

MyDNS is the networking/DNS product in the My Systems family. Its system accent is **blue / network**. The shared My Systems identity remains primary; MyDNS should not become a visually independent brand.

## Visual direction

MyDNS is **dark-first**, technical, minimal, and operational. Use neutral surfaces and one restrained primary accent. Network topology, nodes, grids, and system relationships are appropriate visual motifs.

Avoid excessive gradients, glassmorphism, neon effects, generic cloud/SaaS styling, and decorative dashboard elements that do not communicate operational information.

## Theme tokens

The canonical theme mappings live in `brand/colors/`:

- `brand/colors/dark.md`
- `brand/colors/light.md`

The frontend should map these values to semantic CSS variables/design tokens rather than hard-coding hex values inside components.

## Typography

- Inter for UI, headings, documentation, and general communication.
- JetBrains Mono for code, DNS names, commands, configuration, identifiers, and technical metadata.

## Component rules

Core interactive components must represent:

- Default
- Hover
- Focus
- Active/selected
- Disabled
- Loading
- Success
- Warning
- Error

System health, DNS resolution results, cache state, WebSocket state, and alerts must use semantic state tokens. State must never be communicated by colour alone.

## Operational hierarchy

The dashboard hierarchy is:

1. System state and failures.
2. Primary operational actions.
3. Important DNS/cache information.
4. Supporting metadata.
5. Decorative detail.

Information-dense screens should remain visually calm.

## Layout

Use the shared spacing scale:

`4 / 8 / 12 / 16 / 24 / 32 / 48 / 64 px`

Prefer clear grouping and whitespace over excessive borders. Use a restrained, consistent radius scale.

## Logo usage

Approved repository logo sources are under `brand/logo/`.

- `primary/` — full colour product lockup.
- `monochrome/` — single-colour use on constrained surfaces.
- `mark/` — compact application/repository/favicon identity.

The supplied MyDNS mark follows the approved modular M/network-node direction. Treat it as a source asset; do not redraw or create per-screen variants.

## Accessibility

Both themes are supported product themes. Maintain readable contrast, visible keyboard focus, usable interactive targets, and reduced-motion support where applicable. Critical operational state requires a non-colour indicator such as text, icon, shape, or status label.

## Relationship to the production plan

Brand implementation is part of the V1 UI specification and should be verified alongside the React/TypeScript/Vite frontend, responsive layouts, browser smoke tests, and accessibility checks. Figma is optional and must not become a release dependency.
