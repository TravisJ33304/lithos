# Lithos — GDD Alignment Report

**Date:** 2026-04-29
**Version Analyzed:** v0.7.0 (Phase 7 — World of Stone & Steel)

This report compares the current implementation against the [Game Design Document](GDD.md).

---

## Alignment Scorecard

### §1 System Architecture & Networking

| Section | Status | Notes |
|---------|--------|-------|
| §1.1 Central API (Rust, axum, Supabase) | **Implemented** | `lithos-api` crate with axum REST server. Connects to Supabase Postgres. |
| §1.2 Dedicated Game Server (tokio, WebSocket, SQLite) | **Implemented** | `lithos-server` crate uses tokio + bevy_ecs. WebSocket on port 9001. Uses Postgres (not SQLite) for persistence. |
| §1.3 Client Frontend (PixiJS/Phaser) | **Implemented** | Uses Phaser 4 with WebGL rendering. Client-side prediction with server reconciliation. |
| §1.3 UI Layer (React/vanilla HTML) | **Not aligned** | All UI is rendered in Phaser (text/graphics primitives). GDD specifies React or vanilla HTML/CSS overlay. Major UX limitation. |
| Server-authoritative hit reg + lag compensation | **Implemented** | `client_latency_ms` sent with Fire messages. |

**Gap:** The full HTML/React overlay for menus, inventory, chat, and HUD.

---

### §2 World Zones & Instancing

| Section | Status | Notes |
|---------|--------|-------|
| §2.1 Overworld — persistent procedural map | **Implemented** | Perlin noise generation (simplex-noise). Radial difficulty: Core, Mid-Zone, Outer Rim. |
| §2.1 Radial Difficulty — visual differentiation | **Partial** | Background color lerps between zones. No distinct biome tile art — all use terrain-colored rectangles. |
| §2.2 Resources — 6 types + salvage | **Partial** | Iron, Copper, Silica, Uranium, Plutonium, Biomass all spawn. Titanium does NOT spawn as a mineable node. Salvage sites ("rusted_husk", "abandoned_mech") render but are NOT interactable. |
| §2.3 Static POIs (Fabrication Plants, Comms Arrays) | **Partial** | POIs spawn as large colliders. Fabrication Plant tier bonus is NOT wired to crafting. Comms Arrays cannot be hacked. |
| §2.3 Dynamic Events (Meteor Showers, Solar Flares, Crashed Freighters) | **Partial** | Meteor Showers: damage + announcement work. Crashed Freighters: broadcast-only, no physical containers/guards. Solar Flare: minimap disruption only, no electronic weapon disable. |
| §2.4 Asteroid Bases | **Implemented** | Faction-based private zones with persistence across sessions. |

**Gaps:**
- Titanium must be bought from traders (not mineable) — gameplay imbalance
- No salvage/hacking interaction system
- Dynamic events lack physical impact beyond damage broadcasts

---

### §3 Core Mechanics & Controls

| Section | Status | Notes |
|---------|--------|-------|
| WASD movement | **Implemented** | 8-direction with diagonal normalization. Server-authoritative with client prediction. |
| Mouse aim + click fire | **Implemented** | Twin-stick shooter style. Crosshair in canvas. |
| Death penalty (inventory drop) | **Implemented** | Items drop at death location. Pickup on walking over them. |
| Respawn + Scrapper Dispenser (cooldown) | **Implemented** | 5-minute cooldown on free loadout. |
| Right-click alt-fire | **Not implemented** | No alt-fire mechanic exists. |

**Gap:** No right-click interaction (mining is left-click with mining laser selected).

---

### §4 Progression (Action-Based Mastery)

| Section | Status | Notes |
|---------|--------|-------|
| Skill branches — Extraction | **Implemented** | XP gained per mining action. Level text displayed. |
| Skill branches — Fabrication | **Implemented** | XP gained per craft action. Level gates on recipes. |
| Skill branches — Ballistics | **Not implemented** | No Ballistics XP or crafting tree. |
| Skill branches — Cybernetics | **Not implemented** | No Cybernetics XP or crafting tree. |
| No randomized blueprints | **Implemented** | Recipes are fixed per level. |

**Gaps:** Only 2 of 4 skill branches exist. No skill tree visualization — XP is text-only.

---

### §5 Economy & Anti-Exploit

| Section | Status | Notes |
|---------|--------|-------|
| Credits (Faction Vaults) | **Implemented** | Credits tracked per faction. Displayed in HUD and trade UI. |
| NPC Traders with supply/demand | **Implemented** | Price scalars adjust ±0.02 per trade. Green "Trader" NPCs in Overworld. |
| Daily Credit limits | **Not implemented** | No daily limit on trader interaction. |
| Player trading (drop-based) | **Not implemented** | No formal player-to-player trade. Drop-based barter system not wired. |

**Gap:** No Credit limits on traders (potential exploit). No player trading UI.

---

### §6 Base Building & Automation

| Section | Status | Notes |
|---------|--------|-------|
| Grid system (structures snap to 2D tile grid) | **Implemented** | 40×40 grid snap. Blue ghost preview in build mode. |
| Wall, Door, Generator, Workbench placement | **Implemented** | Structures persist across sessions and zone transfers via Postgres. |
| Power Grid | **Not implemented** | No power wiring visualization. Generator is placeable but no grid system. |
| Life Support / O₂ | **Implemented** | O₂ depletes without generator. Asphyxiation damage. |
| Hydroponics | **Not implemented** | Stretch goal. |
| Automation (Drones) | **Not implemented** | Stretch goal. |

**Gap:** Power grid is specified in GDD but only O₂ tracking exists. Generator is placeable but doesn't display power connections.

---

### §7 Automata (PvE Threats)

| Section | Status | Notes |
|---------|--------|-------|
| Rovers (light, swarming) | **Implemented** | Red circles, fast movement, melee range ~250. Aggro/chase behavior. |
| Drones (flying, lasers) | **Implemented** | Yellow circles, bypasses obstacles, laser at ~500 range. |
| Walkers (Assault/Sniper/Flamethrower) | **Implemented** | Orange/cyan/dark red variants. Distinct range/damage profiles. |
| Heavy Siege Units (Tanks/Mechs) | **Implemented** | Artillery shells. Mid-Zone/Core spawns. |
| Core Wardens (Boss) | **Implemented** | Massive entity (40 radius, 5000 HP). 3-spread projectiles. Rover adds every ~15s. |
| AI states (Idle/Patrol, Investigate, Combat, Retreat/Repair) | **Partial** | Aggro/Chase/Combat implemented. Retreat/Repair NOT implemented. |
| Distinct visual sprites | **Not aligned** | All enemies are colored circles with labels. No vector art sprites. |

**Gap:** Retreat/Repair AI is missing. All enemies are colored circles — no "Crisp 2D Vector Art" as specified in GDD §8.

---

### §7 (sic) Raid Mechanic

| Section | Status | Notes |
|---------|--------|-------|
| Raid initiation (Breach Generator) | **Not implemented** | Raids are admin/test-only. No breach generator craftable. |
| Raid warning UI | **Implemented** | Warning timer countdown. RaidStarted/RaidEnded messages. |
| Asteroid inaccessibility when offline | **Not implemented** | No offline/online detection for bases. |

**Gap:** Raid mechanic is wired server-side but lacks actual player-facing initiation flow.

---

### §8 Art Style & User Interface

| Section | Status | Notes |
|---------|--------|-------|
| Crisp 2D Vector Art sprites | **Not aligned** | All entities are Phaser primitives (circles, rectangles). No sprite assets. |
| Balanced and clean HUD | **Partial** | All HUD elements present. Layout is stacked vertically top-left — functional but cluttered. |
| Health, Ammo, Hotbar, Minimap, Chat | **Implemented** | All specified elements exist. |
| Event logs integrated into Chat | **Implemented** | System messages go through chat channel. |

**Major Gap:** The entire visual presentation uses geometry primitives. No vector art, no particle effects, no sprite animations.

---

## Summary

| Category | Aligned | Partial | Missing | Not Aligned |
|----------|---------|---------|---------|-------------|
| Architecture | 3 | 0 | 0 | 1 |
| World Zones | 1 | 3 | 3 | 0 |
| Core Mechanics | 4 | 0 | 1 | 0 |
| Progression | 3 | 0 | 2 | 0 |
| Economy | 2 | 0 | 2 | 0 |
| Base Building | 3 | 0 | 3 | 0 |
| PvE Threats | 5 | 1 | 0 | 1 |
| Raids | 1 | 0 | 2 | 0 |
| Art & UI | 3 | 1 | 0 | 2 |

**Totals:** 25 aligned · 5 partial · 13 missing · 4 not aligned

## Critical Gaps (Highest Priority)

1. **HTML/CSS overlay for UI** (§1.3) — All UI is Phaser-native. Should be DOM-based for accessibility, styling, and maintainability.
2. **Vector art sprites instead of primitives** (§8) — Every entity (players, enemies, resources) is a colored circle/rectangle. The GDD specifies "Crisp 2D Vector Art."
3. **Ballistics & Cybernetics skill branches** (§4) — Only 2 of 4 progression paths exist. No skill tree visualization.
4. **Distinct enemy silhouettes** (§7) — Enemy types differ only in color and size. No identifiable shapes.
5. **Power grid visualization** (§6) — Generator placement exists but no wiring/power flow display.
