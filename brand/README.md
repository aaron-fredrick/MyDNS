# MyDNS Brand Assets

MyDNS is the DNS/networking system in the **My Systems** family.

This directory is the repository-authoritative home for MyDNS visual assets used by the product UI, documentation, screenshots, diagrams, repository presentation, and release material.

## Brand direction

MyDNS follows the My Systems visual language defined by the canonical My Systems Brand Profile:

- Technical, calm, practical, self-reliant, modular, minimal, open, and reliable.
- Dark-first, with a fully supported light theme.
- Neutral surfaces with restrained brand colour.
- Semantic colours communicate operational state and are not decorative branding.
- Infrastructure, engineering, topology, grids, and system relationships may be used as supporting visual motifs.
- Avoid excessive gradients, glassmorphism, neon effects, generic AI/cloud/SaaS aesthetics, and decorative dashboard noise.

## MyDNS identity

- **Family:** My Systems
- **System:** MyDNS
- **Role:** DNS resolver/server, local DNS, custom zones, upstream resolution, filtering, and management.
- **System identity:** Blue / network.
- **Logo concept:** **Compass** — a precise geometric directional mark representing navigation, routing, and control of the DNS namespace.

The compass is the MyDNS product mark. It is not a network-node diagram and must not be replaced with an abstract M, cloud, or generic topology symbol.

The MyDNS identity remains part of the My Systems family: use the shared family visual language and the MyDNS blue system accent rather than creating an unrelated visual identity.

## Logo rules

The approved logo family consists of:

```text
brand/logo/
├── primary/       # Full-colour MyDNS lockup
├── monochrome/    # Single-colour lockup
└── mark/          # Compact compass mark for app/favicons/repository use
```

The compass geometry should remain clean, legible, and recognisable at small sizes. Do not add cyan, gradients, glow, node dots, or per-screen variants.

The primary logo uses the MyDNS brand blue. The monochrome version is intended for contexts where a single colour is required. Select the appropriate variant for the background rather than modifying the artwork.

## Asset layout

```text
brand/
├── logo/
│   ├── primary/
│   ├── monochrome/
│   └── mark/
├── icons/
├── screenshots/
├── diagrams/
└── README.md
```

Theme token documentation lives in `brand/colors/`. Do not commit generated build output or temporary design exports. Prefer source-quality SVG for logos and diagrams where practical.

## Canonical colour relationship

MyDNS uses the **My Systems** core palette. The MyDNS system identity is blue/network; it does not introduce cyan as a logo colour.

- Dark brand primary: `#5B8CFF`
- Light brand primary: `#315EDE`
- Dark accent: `#22C7A5`
- Light accent: `#0D9F82`

The green/teal accent is a secondary brand accent and should not be confused with the MyDNS primary blue or used as a replacement for it.

## Typography

- **Inter:** UI, headings, documentation, and general brand communication.
- **JetBrains Mono:** code, commands, configuration, identifiers, DNS names, and technical metadata.

## UI tokens

Applications should consume semantic tokens rather than hard-code palette values throughout components. Canonical dark and light mappings are documented in:

- `colors/dark.md`
- `colors/light.md`

Core tokens:

```text
color.bg.0
color.bg.1
color.bg.2
color.border
color.text.primary
color.text.secondary
color.text.muted
color.brand.primary
color.brand.hover
color.accent
color.success
color.warning
color.error
color.info
color.neutral
```

## Accessibility

Both themes are first-class product themes. Critical state must never be communicated through colour alone. Focus states must remain visible, text must remain readable, and interactive targets must be usable with keyboard and assistive technology. Respect reduced-motion preferences where animation is introduced.

## Source of truth

The family-level brand definition is maintained in the **My Systems Brand Profile**. This repository copy records the subset required to implement and ship MyDNS consistently. If the canonical profile changes, update this documentation and the corresponding assets together.
