# Lithos Playtesting Guide

**Version:** v0.8.0 (Post GDD-alignment implementation pass)  
**Date:** 2026-05-01  
**Target:** Browser client + local dedicated server

---

## 1. Purpose

This guide validates the current non-sprite feature-complete gameplay loop:

- Boot/menu/login -> Overworld -> Asteroid Base transitions.
- DOM overlay UX (HUD, inventory, crafting, progression, chat, onboarding, death).
- Interaction and progression systems (mining, salvage, hacking, crafting, item use/drop/equip).
- Economy, power/base systems, dynamic events, raids, and NPC behavior.

---

## 2. Prerequisites

- Rust toolchain (`cargo`)
- Node.js 22+
- PostgreSQL available for server runtime
- Browser for manual tests

Install client dependencies:

```bash
cd client
npm install
```

---

## 3. Environment Setup

### 3.1 Start the server

```bash
cargo run --bin lithos-server
```

Expected behavior:

- Server boots without panic.
- WebSocket endpoint is available at `ws://localhost:9001`.

### 3.2 Start the client

```bash
cd client
npm run dev
```

Open the URL reported by Vite (default `http://localhost:5173`).

### 3.3 Join format

Dev join token is username text. Use:

```text
username#faction_id
```

Example: `alpha#1`, `bravo#1` for same-faction testing.

---

## 4. Automated End-to-End Testing (Playwright)

Playwright coverage is in `client/e2e`.

### 4.1 Install browser binary (first run only)

```bash
cd client
npx playwright install chromium
```

### 4.2 Run E2E tests

```bash
cd client
npm run test:e2e
```

### 4.3 Current suites

- `ui-smoke.spec.ts`
  - Validates menu UI render and Boot -> Login transition through DOM join controls.
- `live-integration.spec.ts`
  - Validates local-server join and core loop checks: Overworld HUD visibility, crafting catalog hydration, chat send/echo, and zone transfer.
  - This test is opt-in. Set `LITHOS_RUN_LIVE_E2E=1` with a healthy `lithos-server` listening on `localhost:9001`.

### 4.4 Debug run

```bash
cd client
npm run test:e2e:headed
```

Use Playwright HTML report:

```bash
cd client
npx playwright show-report
```

---

## 5. Manual Integration Checklist

Run these in addition to Playwright to validate server-authoritative gameplay behavior.

## 5.1 Boot, Menu, and Login

- [ ] Menu renders with server list and editable endpoint.
- [ ] Choosing a server row updates endpoint input.
- [ ] Join enters `LoginScene`, then successful join enters `OverworldScene`.
- [ ] Onboarding overlay appears on first Overworld load and auto-hides.

## 5.2 Core Movement, Combat, and Respawn

- [ ] WASD movement is smooth and bounded.
- [ ] Left-click fire/mining behavior responds to equipped context.
- [ ] Death shows DOM death overlay and respawn prompt behavior.
- [ ] Respawn clears death overlay and restores play state.

## 5.3 DOM Gameplay UI

- [ ] HUD values update: health, oxygen, ammo, credits, fps, tick.
- [ ] Inventory panel reflects structured stack entries.
- [ ] Crafting summary and modal recipe list populate from `CraftingCatalog`.
- [ ] Progression panel updates branch/level/xp.
- [ ] Chat input sends messages and log updates.
- [ ] Hotbar, crosshair, minimap, trader panel, base status, flashes, onboarding, and death screens render through the DOM overlay rather than Phaser scene text.

## 5.4 Interaction Layer

- [ ] Right-click interact path works on nearby interactables.
- [ ] Salvage interaction produces inventory updates when tool requirements are met.
- [ ] Hacking interactions produce expected rewards/state changes.
- [ ] Fabrication plant interactions affect relevant crafting state/feedback.

## 5.5 Resources, Events, and Economy

- [ ] Titanium nodes can be found/mined in intended zones.
- [ ] Dynamic events produce physical gameplay effects (not chat-only).
- [ ] Trader quotes display daily limit usage fields.
- [ ] Buy/sell updates faction credits and inventory correctly.

## 5.6 Base and Power Systems

- [ ] Build placement supports updated structures (including hydroponics/drone bay).
- [ ] Power state panel reflects network count/load.
- [ ] Powered vs unpowered behavior affects hydroponics and drone bay runtime state.

## 5.7 Raids and NPC Behavior

- [ ] Raid target discovery surfaces online defender factions.
- [ ] Raid initiation requires breach generator and enforces defender-online rules.
- [ ] NPC retreat/repair behavior triggers under health thresholds and recovers correctly.

---

## 6. Full Validation Commands

From `client`:

```bash
npm run test:e2e
npx @biomejs/biome ci src e2e playwright.config.ts
npx tsc --noEmit
npx vitest run
npm run build
```

From repo root:

```bash
cargo check
cargo test --workspace
```

---

## 7. Known Testing Constraints

- Final vector sprite asset validation is out of scope for this pass.
- Live-integration Playwright coverage depends on `LITHOS_RUN_LIVE_E2E=1` and a protocol-ready local server at `ws://localhost:9001`.
- Some advanced multiplayer/raid scenarios still require manual multi-client playtests.

---

## 8. Bug Report Template

```text
Category: Crash / Desync / Gameplay / UI / Economy / Other
Repro Steps:
1.
2.
Expected:
Actual:
Environment: browser + OS + server commit
Logs: server/client console snippets
```
