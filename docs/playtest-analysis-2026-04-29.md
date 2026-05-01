# Lithos — Playtest Analysis Report

**Date:** 2026-04-29
**Build:** v0.7.0 (Phase 7 — World of Stone & Steel)

> Note: This report is historical and predates the later GDD-alignment implementation pass. For current execution and verification steps, use `docs/playtesting.md`.

---

## 1. Executive Summary

**Top 3 Findings:**

1. **UI/UX: The game is fully functional but visually primitive.** All entities are colored circles/rectangles. The HUD is informative but cluttered (12 text elements stacked top-left). The game works — movement, combat, crafting, building, trading, chat all function — but lacks visual polish and UI refinement.

2. **Architecture: Phaser-native UI is the main constraint.** Every interactive element (HUD, crafting panel, trade UI, build mode) is rendered through Phaser's canvas. This means no CSS styling, no accessibility, no responsive layout, and no tooltip system. The GDD specifies an HTML overlay for UI — moving to this unlocks significant quality improvements.

3. **Interactive mockups generated for the 4 highest-impact UI improvements:** HUD overlay, crafting panel, inventory system, and main menu. See [`docs/ui-mockups/`](ui-mockups/).

---

## 2. Smoke Test Results

**Automated Playwright test:** 9/10 pass, 0 console errors.

| Test | Result | Notes |
|------|--------|-------|
| Boot → Login transition | ✓ | Scene transition via Phaser API |
| Login form completion | ✓ | DOM `#username` input found and filled |
| Join Overworld | ✓ | OverworldScene active after JoinAck |
| WASD movement | ✓ | Keyboard events sent to game |
| Zone transfer to AsteroidBase | ✗ | Scene remained Overworld — likely headless timing |
| Zone transfer back to Overworld | ✓ | Worked on second Space press |
| Mouse click (fire) | ✓ | Pointer event dispatched |
| Crafting panel toggle (C key) | ✓ | Key event sent |
| Build mode toggle (B key) | ✓ | Key event sent |
| Chat message | ✓ | Enter → type → Enter flow works |

**Console errors:** 0 across entire session.

---

## 3. UX Observations

### 3.1 What Works Well

- **Responsive to keyboard input:** WASD, Space, C, B, Enter all work reliably
- **Scene transitions are smooth:** Overworld ↔ AsteroidBase transitions work correctly
- **WebSocket connection is stable:** No disconnects observed
- **No console errors:** The game runs cleanly in headless Chrome

### 3.2 Friction Points

| Issue | Severity | Details |
|-------|----------|---------|
| **HUD is one vertical stack** | Medium | 12 text elements (Health, O₂, Ammo, FPS, Tick, Zone, Craft/Build hints, XP, Credits, Inventory) are stacked top-left. During combat, information is scattered. See `hud.html` mockup for proposed fix. |
| **Hotbar text abbreviations** | Low | Item names truncated to 2 characters (ML, PP, FE, CU). No icons. Hard to distinguish at a glance. See `inventory.html` mockup. |
| **Crafting panel is a flat list** | Medium | 13 recipes in a single list with no categories, search, or filter. The Fabrication Level gating is shown only via color (no level label per recipe). See `crafting.html` mockup. |
| **No tooltips anywhere** | Medium | No hover tooltips on items, recipes, or HUD elements. Players must memorize item functions. See `inventory.html` mockup for tooltip system. |
| **Build mode has no structure preview** | Low | Ghost grid is a blue rectangle regardless of what's being placed (wall vs. door vs. generator). No visual distinction. |
| **Death feedback is text-only** | Low | "DEAD - Inventory Dropped! Press R to Respawn" in red text. No screen effects, no animation. |
| **No tutorial/onboarding** | Medium | New players spawn in the Overworld with no guidance. The key hints in HUD ([SPACE], [C], [B]) help but don't explain the game loop. |
| **Login flow requires server browser** | Low | Players must click a server listing before they can enter credentials. The server browser is useless for local dev. See `menu.html` mockup for consolidated flow. |
| **No settings menu** | Low | No way to change volume, graphics, or controls. |

### 3.3 Phaser Canvas Limitations

The following features are difficult or impossible to implement well in Phaser's canvas renderer:
- Text selection / copy-paste in chat
- Tooltip system (need raycasting against interactive objects)
- Responsive/scalable UI layouts
- CSS animations and transitions
- Accessibility (screen readers, keyboard navigation)
- Standard form controls (dropdowns, sliders)

**Recommendation:** Move HUD, menus, crafting, inventory, and chat to an HTML/CSS overlay. Keep only the game world (entities, terrain, projectiles) in Phaser canvas.

---

## 4. GDD Alignment Summary

Full report in [`gdd-alignment-report.md`](gdd-alignment-report.md).

| Assessment | Count |
|------------|-------|
| Aligned | 25 / 47 |
| Partial | 5 / 47 |
| Missing | 13 / 47 |
| Not aligned | 4 / 47 |

**Biggest gaps:**
- No HTML overlay for UI (GDD §1.3)
- No vector art sprites (GDD §8)
- Only 2 of 4 skill branches (GDD §4)
- Missing enemy retreat/repair AI (GDD §7)
- No power grid visualization (GDD §6)

---

## 5. UI Mockup Gallery

Interactive prototypes in [`docs/ui-mockups/`](ui-mockups/):

| Mockup | File | Key Improvements |
|--------|------|------------------|
| **HUD Overlay** | `hud.html` | Radial health/O₂ bars, leveled hotbar with item labels, info chips (XP, Credits, FPS, Tick), minimap with POI markers, crosshair overlay |
| **Crafting Panel** | `crafting.html` | Search bar, category tabs (ALL, Materials, Structures, Consumables, Tech), recipe cards with craftable/locked indicators, material availability coloring, detail popup with craft button |
| **Inventory + Hotbar** | `inventory.html` | Grid inventory with rarity borders, equipment panel, hover tooltips with stats, right-click context menu (Equip, Use, Drop), quick-equip bar |
| **Login / Main Menu** | `menu.html` | Server browser with ping indicators, login form with faction hint, direct connect field, settings panel (Audio, Graphics, Controls), animated starfield background, loading overlay |

---

## 6. Prioritized Improvement Backlog

### P0 — Must Have (Next Sprint)

| ID | Task | Effort | Impact | Area |
|----|------|--------|--------|------|
| UI-01 | **Implement HTML overlay HUD** — Replace Phaser-native HUD text with DOM-based bars, chips, and hotbar | High | High | Looks |
| UI-02 | **Add item tooltips** — Hover-to-reveal descriptions, stats, and actions | Medium | High | Fun |
| UI-03 | **Redesign crafting panel** — Categories, search, detail popup with craft confirm | Low | High | Fun |
| UI-04 | **Create login/main menu overlay** — Server browser, settings, animated background | Medium | High | Looks |

### P1 — High Impact

| ID | Task | Effort | Impact | Area |
|----|------|--------|--------|------|
| UI-05 | **Replace primitive shapes with vector sprites** — Players, enemies, resources, structures | High | High | Looks |
| UI-06 | **Add Ballistics & Cybernetics skill branches** — Complete the progression system | High | High | Feature |
| UI-07 | **Add tooltip for inventory items** — Item name, type, description, stats | Low | Medium | Fun |
| UI-08 | **Add enemy silhouette differentiation** — Walker types should look distinct | Medium | Medium | Looks |
| UI-09 | **Add sound effects** — Howler.js integration for weapons, mining, UI, ambient | Medium | Medium | Fun |

### P2 — Polish

| ID | Task | Effort | Impact | Area |
|----|------|--------|--------|------|
| UI-10 | **Add death screen effects** — Screen shake, desaturation, respawn countdown | Low | Medium | Fun |
| UI-11 | **Add minimap POI icons** — Different markers for enemies, traders, faction, nodes | Medium | Medium | Fun |
| UI-12 | **Implement right-click interactive mode** — Contextual actions on NPCs/objects | Low | Low | Feature |
| UI-13 | **Add tutorial overlay** — First-join tips overlay with dismiss | Low | Medium | Fun |
| UI-14 | **Add settings panel** — Volume, graphics quality, keybinds | Medium | Low | Feature |
| UI-15 | **Power grid visualization** — Lines connecting generators to consumers | High | Medium | Feature |

### P3 — Nice to Have

| ID | Task | Effort | Impact | Area |
|----|------|--------|--------|------|
| UI-16 | Salvage site interaction system | Medium | Medium | Feature |
| UI-17 | Titanium mineable nodes | Low | Low | Feature |
| UI-18 | Daily trader credit limits | Low | Low | Feature |
| UI-19 | Retreat/Repair AI for advanced enemies | Medium | Medium | Feature |
| UI-20 | Crashed Freighter physical containers | High | Medium | Feature |

---

## 7. Methodology

**Automated Testing:**
- Playwright 1.59.0 with Chromium 147 (headless)
- Fake API server on port 3000 returning server listings
- Phaser game instance exposed via `window.__PHASER_GAME__` for scene API access
- Screenshots at each test step saved to `/tmp/lithos-*.png`
- Game server: Rust (release build), Postgres 16 (Docker), WebSocket on 9001
- Client: Vite dev server on 5173

**Manual Analysis:**
- Source code review of all client scenes and server systems
- Comparison against GDD.md section by section
- UX heuristic evaluation (visibility, feedback, consistency, error prevention)
- 4 interactive HTML mockups generated using `frontend-design` skill

**Skills Used:**
- `webapp-testing` — Playwright automation + screenshot capture
- `frontend-design` — UI mockup generation with industrial utilitarian aesthetic
- `writing-plans` — Plan structure and task decomposition
