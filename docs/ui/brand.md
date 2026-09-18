# MyDNS UI Brand Integration

This document translates the canonical **My Systems Brand Profile** into implementation rules for the MyDNS web UI.

## Product identity

MyDNS is the DNS/networking product in the My Systems family. Its system identity is **blue / network**. The product remains recognisably part of the My Systems family rather than becoming a visually independent brand.

## Visual direction

MyDNS is **dark-first**, technical, minimal, practical, and operational. Use neutral surfaces with restrained brand colour. Infrastructure, topology, grids, and system relationships are appropriate supporting motifs, but the logo itself is a **compass**.

Avoid excessive gradients, glassmorphism, neon effects, generic AI/cloud/SaaS styling, and decorative dashboard elements that do not communicate operational information.

## Theme tokens

The canonical mappings live in `brand/colors/`:

- `brand/colors/dark.md`
- `brand/colors/light.md`

The frontend must map these values to semantic CSS variables/design tokens rather than scattering hard-coded hex values through components.

## Typography

- **Inter** for UI, headings, documentation, and general communication.
- **JetBrains Mono** for code, DNS names, commands, configuration, identifiers, and technical metadata.

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

- `primary/` — full-colour MyDNS lockup using the MyDNS blue identity.
- `monochrome/` — single-colour lockup for constrained contexts.
- `mark/` — compact geometric compass for application, favicon, and repository identity.

The compass is the MyDNS product mark. Do **not** redraw it as an M/network-node symbol, add cyan, add node dots, add glow/gradients, or create screen-specific variants.

The My Systems Brand Profile allows topology/network motifs in the wider visual language; that does not change the MyDNS logo concept.

## Colour discipline

Use the canonical My Systems palette:

- Dark primary: `#5B8CFF`
- Light primary: `#315EDE`
- Dark accent: `#22C7A5`
- Light accent: `#0D9F82`

The green/teal accent is a secondary family accent. It is not the MyDNS logo colour. Semantic colours are reserved for operational meaning.

## Accessibility

Both themes are supported product themes. Maintain readable contrast, visible keyboard focus, usable interactive targets, and reduced-motion support where applicable. Critical operational state requires a non-colour indicator such as text, icon, shape, or status label.

## Relationship to the production plan

Brand implementation is part of the V1 UI specification and should be verified alongside the React/TypeScript/Vite frontend, responsive layouts, browser smoke tests, and accessibility checks. Figma is optional and must not become a release dependency.
