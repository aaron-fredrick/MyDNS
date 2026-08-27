# MyDNS Brand Assets

MyDNS is the networking/DNS system in the **My Systems** family.

This directory is the repository-authoritative home for MyDNS visual assets used by the product UI, documentation, screenshots, diagrams, repository presentation, and release material.

## Brand direction

MyDNS follows the My Systems visual language:

- Technical, calm, practical, and infrastructure-oriented.
- Dark-first, with a fully supported light theme.
- Neutral surfaces with restrained blue branding.
- Operational state is communicated with semantic colours rather than decorative accents.
- Visual motifs may use networks, nodes, topology, grids, and relationships.
- Avoid generic cloud/SaaS aesthetics, excessive gradients, glassmorphism, neon effects, and decorative dashboard noise.

## MyDNS identity

- **Family:** My Systems
- **System:** MyDNS
- **Role:** DNS resolver/server, local DNS, custom zones, upstream resolution, filtering, and management.
- **System accent:** Blue / network.
- **Primary mark direction:** modular M / network-node / topology motif.

The shared My Systems family mark should remain the primary identity. MyDNS may use its blue network accent to distinguish the system without becoming a separate visual brand.

## Asset layout

```text
brand/
├── logo/
│   ├── primary/       # Full My Systems / MyDNS lockups
│   ├── monochrome/    # Single-colour variants
│   └── mark/          # Compact application/favicons/repository marks
├── icons/             # Product and system iconography
├── screenshots/       # Approved UI screenshots
├── diagrams/          # Architecture/topology diagrams
├── colors/            # Theme and semantic colour tokens
└── README.md
```

Do not commit generated build output or temporary design exports here. Prefer source-quality SVG for logos and diagrams where practical.

## Typography

- **Inter:** UI, headings, documentation, and general brand communication.
- **JetBrains Mono:** code, commands, configuration, identifiers, and technical metadata.

## UI tokens

Applications should consume semantic tokens rather than hard-code palette values throughout components. The canonical dark and light mappings are documented in:

- [`colors/dark.md`](colors/dark.md)
- [`colors/light.md`](colors/light.md)

Core semantic tokens:

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

Both themes are first-class product themes. Critical state must never be communicated through colour alone. Focus states must remain visible, text must remain readable, and interactive targets must be usable with keyboard and assistive technology.

The browser UI should also respect reduced-motion preferences where animation is introduced.

## Source of truth

The family-level brand definition is maintained in the **My Systems Brand Profile**. This repository copy records the subset required to implement and ship MyDNS consistently. When implementation or accessibility testing exposes a real issue, update the canonical profile and this repository documentation together rather than creating an undocumented local variation.
