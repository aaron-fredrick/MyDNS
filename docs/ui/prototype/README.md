# MyDNS UI Prototype

Static UI/UX concept for the MyDNS web management interface.

## Purpose

This prototype is intentionally non-backend. It preserves the approved MyDNS visual direction and existing prototype interactions while separating markup, styles, and browser-side mock behaviour so the design can later migrate cleanly to React/TypeScript/Vite.

## Structure

- `index.html` — prototype entry point.
- `pages/` — individual application views.
- `css/main.css` — reset, typography, theme tokens, layout primitives.
- `css/components.css` — shared UI components.
- `css/pages.css` — page-specific layout rules.
- `js/app.js` — shared bootstrap.
- `js/navigation.js` — view navigation.
- `js/prototype.js` — mock UI state and interactions only.
- `assets/` — prototype-local assets.

## Brand constraints

MyDNS is dark-first, technical, minimal, practical, and operational. The product mark is the compass. The visual identity uses the canonical My Systems palette with MyDNS blue as the primary identity colour. Network/topology motifs are supporting UI language only and must not replace or alter the compass logo.

Typography: Inter for UI and JetBrains Mono for DNS names, commands, configuration, identifiers, and technical metadata.

Theme tokens must remain semantic and support both dark and light modes. Do not introduce arbitrary cyan, neon effects, glassmorphism, decorative gradients, or generic SaaS dashboard styling.

## Future implementation

The static pages are a visual and interaction reference only. Backend/API, authentication, DNS CRUD, cache management, live logs, and WebSocket behaviour will be implemented during the React/TypeScript/Vite + Rust/Axum integration phase.
