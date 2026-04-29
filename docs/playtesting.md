# Lithos Playtesting Document

**Version:** v0.7.0 (Phase 7 — World of Stone & Steel)  
**Date:** 2026-04-29  
**Target:** Browser-based client connecting to local Dedicated Game Server  
**Prerequisites:** `cargo`, `node` 22+, PostgreSQL running (or Docker stack)

---

## 1. Environment Setup

### 1.1 Start the Server

```bash
cargo build --workspace --release
cargo run --bin lithos-server
```

Expected output:
```
Connecting to database...
Connected to Postgres!
lithos-server starting
WebSocket listener ready
game loop starting
```

### 1.2 Start the Client

```bash
cd client
npm install
npm run dev
```

Open the Vite URL (usually `http://localhost:5173`) in two separate browser windows/tabs.

### 1.3 Join the Game

The client uses a dev-auth fallback: any string entered as a "token" is treated as your username. To test faction features, use the format:

```
playername#123
```

where `123` is your faction ID. Use the **same faction ID** on both clients to test faction chat and shared bases.

---

## 2. Smoke Tests (Must Pass)

### 2.1 Connection & Join

| Step | Action | Expected Result |
|------|--------|-----------------|
| 2.1.1 | Open client in Browser A, enter `alpha#1`, click Connect | Joins Overworld. Player spawns at (0,0). HUD shows Health 100/100, O₂ 100/100, Ammo 50/50. Inventory shows `[mining_laser, scrap, scrap]`. |
| 2.1.2 | Open client in Browser B, enter `bravo#1`, click Connect | Second player spawns. Both players can see each other as blue/purple circles with "YOU" / "Player" labels. |
| 2.1.3 | Check FPS counter in top-left | FPS value updates and stays above 30. |
| 2.1.4 | Check Tick counter | Tick value increments steadily. |

### 2.2 Basic Movement

| Step | Action | Expected Result |
|------|--------|-----------------|
| 2.2.1 | Hold `W` / `A` / `S` / `D` | Player moves smoothly in all 8 directions. Camera follows. |
| 2.2.2 | Release all keys | Player stops moving within one tick. |
| 2.2.3 | Move toward world edge | Player stops at the world border (approx ±2000 units). No jitter or rubber-banding beyond boundary. |
| 2.2.4 | Observe other player moving | Other player's sprite interpolates smoothly (not teleporting). |

---

## 3. Core Gameplay Verification

### 3.1 Combat & Death Loop

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.1.1 | Aim with mouse, Left-Click to fire | Projectile spawns. Ammo counter decreases by 1. Weapon respects cooldown (0.5s). |
| 3.1.2 | Fire until ammo reaches 0 | Ammo text turns **red** (`#ff4444`). Further clicks do nothing. |
| 3.1.3 | Shoot the other player until their health reaches 0 | Target dies. Target's screen shows "DEAD - Inventory Dropped! Press R to Respawn" in red. Their inventory HUD clears to `[]`. Their sprite disappears from your screen. |
| 3.1.4 | Target presses `R` to respawn | Target respawns at origin (0,0) with Health 100/100. Receives Scrapper Dispenser loadout: `[mining_laser, scrap, scrap]`. Inventory HUD repopulates. |
| 3.1.5 | Target dies again within 5 minutes and respawns | This time, **no** Scrapper Dispenser loadout is granted. Inventory remains `[]`. |
| 3.1.6 | Target walks over their own dropped items | Items are picked up automatically. Inventory HUD updates. Items disappear from world. |

### 3.2 Zone Transfers

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.2.1 | Press `SPACE` | `ZoneChanged` message received. Scene transitions to **Asteroid Base**. Background changes to purple (`#1a0a2e`). Title "ASTEROID BASE" appears. |
| 3.2.2 | In Asteroid Base, press `SPACE` again | Transitions back to **Overworld**. Background returns to biome-dependent color. |
| 3.2.3 | Join with a player **without** a faction (e.g. `loner`) and press `SPACE` | System chat message: "You need a faction to enter an Asteroid Base." Player remains in Overworld. |

### 3.3 Mining & Extraction Progression

Resource nodes spawn in veins tied to terrain type and biome:

| Resource | Terrain | Biomes |
|----------|---------|--------|
| `iron` | Rock, DeepRavine | Outer Rim, Mid-Zone |
| `copper` | Rock | Outer Rim, Mid-Zone |
| `silica` | AsteroidField | Mid-Zone |
| `uranium` | DeepRavine, AutomataSpire | Mid-Zone, Core |
| `plutonium` | AutomataSpire | Core |
| `biomass` | Empty | Outer Rim, Mid-Zone |

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.3.1 | Select slot `1` (mining laser) on hotbar. Approach a grey "Ore" node. Left-click. | Node yields 1 unit per click. Inventory gains the node's resource type. XP text updates (e.g. "Extraction Lv.1 (5 XP)"). Green "+5 Extraction XP" flash appears. |
| 3.3.2 | Continue mining until node yield is depleted | Node sprite disappears from world. "ResourceDepleted" handled gracefully (no errors). |
| 3.3.3 | Attempt to mine without mining laser selected (slot 0) | Nothing happens. No server crash. |
| 3.3.4 | Mine 20+ times | Extraction level increases from 1 to 2 once XP threshold is crossed. |

### 3.4 Crafting & Fabrication

| Level | Recipe | Inputs |
|-------|--------|--------|
| 1 | `iron_plate` | `2× iron` |
| 1 | `copper_wire` | `2× copper` |
| 1 | `circuit` | `copper_wire + iron_plate` |
| 1 | `glass` | `2× silica` |
| 1 | `medkit` | `biomass + glass` |
| 1 | `bio_fuel` | `2× biomass` |
| 1 | `wall_segment` | `2× iron_plate` |
| 1 | `door` | `iron_plate + circuit` |
| 1 | `workbench` | `2× iron_plate + circuit` |
| 2 | `generator` | `battery + titanium_plate + circuit` |
| 3 | `titanium_plate` | `2× titanium` |
| 3 | `battery` | `titanium_plate + circuit` |
| 3 | `shield_module` | `titanium_plate + battery + circuit` |
| 5 | `uranium_core` | `2× uranium` |
| 5 | `plutonium_core` | `2× plutonium` |
| 5 | `warp_drive` | `uranium_core + battery + titanium_plate` |
| 5 | `breach_generator` | `plutonium_core + shield_module + warp_drive` |

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.4.1 | Open crafting panel with `C` | Panel shows recipe list with inputs and required levels. |
| 3.4.2 | Click **iron_plate** recipe | If inventory contains `2× iron`, items are consumed and `iron_plate` is added. Inventory updates. XP is gained internally. |
| 3.4.3 | Attempt to craft **titanium_plate** at Fabrication Lv.1 | Red flash: "Craft denied: requires Fabrication level 3". No items consumed. |
| 3.4.4 | Close panel with `[CLOSE]` or `C` | Panel disappears cleanly. |

### 3.5 Base Building

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.5.1 | Craft a `wall_segment` and a `generator`. | Items appear in inventory. |
| 3.5.2 | Press `B` to enter Build Mode | Crosshair hides. Blue 40×40 ghost grid square appears at mouse cursor. |
| 3.5.3 | Click to place a wall | Wall appears as a grey 40×40 square. Inventory loses one `wall_segment`. Structure persists after zone transfer. |
| 3.5.4 | Place a `generator` nearby | Generator is placed. |
| 3.5.5 | Transfer to Asteroid Base and back | Structures remain in place. |
| 3.5.6 | Reconnect client (refresh browser) and rejoin same faction | Previously placed structures are loaded from database and render correctly. |

### 3.6 Power & Life Support

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.6.1 | In Asteroid Base with generator placed, check O₂ text | O₂ remains at or near 100/100 (blue text). |
| 3.6.2 | Remove generator (not yet supported via UI, so test by not placing one) | If no powered life support exists, O₂ begins dropping by 0.5 per tick. When O₂ < 30, text turns **red** (`#ff4444`). |
| 3.6.3 | Let O₂ reach 0 | Player takes 5 damage per tick from asphyxiation. Health text updates. Eventually player dies. Inventory drops. |
| 3.6.4 | Respawn and return to base with generator | O₂ refills. Health stops decreasing. |

### 3.7 Economy & Trading

Trader NPCs (green "Trader" labels) sell the following items with fluctuating prices:

| Item | Base Price |
|------|-----------|
| `iron` | 10 |
| `copper` | 12 |
| `silica` | 15 |
| `biomass` | 8 |
| `titanium` | 22 |
| `uranium` | 60 |
| `plutonium` | 75 |
| `medkit` | 45 |

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.7.1 | Locate a green "Trader" NPC in Overworld. Click on it. | Trade UI opens. Shows BUY/SELL prices for all trader items. Shows faction credits. |
| 3.7.2 | Click **SELL** on an item you own | Item removed from inventory. Faction credits increase. Price scalar adjusts (±0.02). |
| 3.7.3 | Click **BUY** on an item | Item added to inventory. Faction credits decrease. |
| 3.7.4 | Attempt to buy with insufficient faction credits | Red flash: "Trade failed: insufficient faction credits". |
| 3.7.5 | Attempt to sell item you don't have | Red flash: "Trade failed: insufficient items". |

### 3.8 Chat System

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.8.1 | Press `Enter`, type `hello world`, press `Enter` again | Message appears in chat overlay. Other player sees `[E<your_id>] hello world` in global channel (grey). |
| 3.8.2 | Type more than 100 characters | Input is truncated at 100 characters. |
| 3.8.3 | Press `Backspace` while typing | Characters are deleted correctly. |
| 3.8.4 | Press `Enter` with empty input | Chat closes. No empty message sent. |

### 3.9 World Exploration & Points of Interest

| Step | Action | Expected Result |
|------|--------|-----------------|
| 3.9.1 | Explore the Overworld | Terrain tiles render with distinct colors (green=Empty, grey=Rock, brown=DeepRavine, yellow=AsteroidField, purple=AutomataSpire). Height variations visible via brightness. |
| 3.9.2 | Look for POIs in Mid-Zone / Core | Fabrication Plants spawn as large colliders (~30 radius). Comms Arrays spawn on elevated terrain (>150 height). |
| 3.9.3 | Look for Salvage Sites in Mid-Zone / Core | Salvage sites ("rusted_husk" or "abandoned_mech") spawn in Empty / AsteroidField terrain. Not yet interactable. |

---

## 4. Advanced Systems

### 4.1 NPC Automata (PvE)

| Step | Action | Expected Result |
|------|--------|-----------------|
| 4.1.1 | Approach a **Rover** (red, fast) in Outer Rim | Rover enters **Aggro** and chases at high speed. Short attack range (~250). |
| 4.1.2 | Approach a **Drone** (flying, yellow) in Mid-Zone | Drone bypasses ground obstacles. Fires light lasers at ~500 range. |
| 4.1.3 | Approach an **Assault Walker** (large, orange) in Mid-Zone | Aggressive pusher. Moderate range (~400), higher damage. |
| 4.1.4 | Approach a **Sniper Walker** (tall, cyan) in Core | Engages from very long range (~1200). Attempts to maintain distance. |
| 4.1.5 | Approach a **Heavy Flamethrower** (bulky, dark red) in Core | Very short range (~200). Applies **OnFire** DOT (3s, 5 dmg/tick). |
| 4.1.6 | Approach the **Core Warden** boss at (0,0) | Massive entity (40 radius, 5000 HP). Fires 3 spread projectiles. Spawns Rover adds every ~15s. |
| 4.1.7 | Move far away (>1000 units) from any hostile | NPC returns to spawn (Patrol state). |
| 4.1.8 | Let an NPC kill you | You die, inventory drops. NPC continues patrolling. |

### 4.2 Dynamic Events

| Step | Action | Expected Result |
|------|--------|-----------------|
| 4.2.1 | Wait in Overworld for ~45 seconds (900 ticks) | A system chat message announces a dynamic event (Meteor Shower, Solar Flare, or Crashed Freighter). |
| 4.2.2 | During **Meteor Shower**, remain in Overworld | Every ~1.5 seconds all Overworld players take 10 damage from meteor strikes. Health updates. |
| 4.2.3 | Wait another ~15 seconds | Event ends. No crash. |

### 4.3 Raids

| Step | Action | Expected Result |
|------|--------|-----------------|
| 4.3.1 | As a faction member, use the client console or a test harness to send `InitiateRaid { defender_faction_id: 2 }` | Both attacker and defender factions receive `RaidWarning`. Warning timer counts down (approx 6 seconds at 20 TPS). |
| 4.3.2 | Wait for warning to expire | `RaidStarted` message sent. Breach becomes active. |
| 4.3.3 | Wait for breach duration to expire | `RaidEnded` message sent. Raid state cleaned up. |

### 4.4 Tilemap & Pathfinding

| Step | Action | Expected Result |
|------|--------|-----------------|
| 4.4.1 | Explore the Overworld | Terrain tiles render with distinct colors (green=Empty, grey=Rock, brown=DeepRavine, yellow=AsteroidField, purple=AutomataSpire). |
| 4.4.2 | Observe tile borders | Enclosed-ceiling tiles show a dark border. Open tiles have no border. |
| 4.4.3 | Watch NPCs navigate around obstacles | Hostile NPCs pathfind around Rock/DeepRavine tiles using A*. Drones fly over ground obstacles. |

### 4.5 Lag Compensation

| Step | Action | Expected Result |
|------|--------|-----------------|
| 4.5.1 | Have Player A run in a straight line. Player B fires slightly behind Player A's current rendered position. | Hit still registers because server rewinds Player A's position history based on Player B's reported latency. |

---

## 5. Regression & Edge Cases

| ID | Test | Steps | Expected Result |
|----|------|-------|-----------------|
| R1 | Server full rejection | Connect 100+ mock clients (use `tools/bot_runner.js`) | 101st client receives `Disconnect { reason: "server full" }`. |
| R2 | Invalid token rejection | Join with empty string or malformed JWT when `SUPABASE_JWKS_URL` is set | `Disconnect { reason: "invalid token" }`. |
| R3 | Rapid zone transfer spam | Press `SPACE` 10 times in 1 second | Player toggles between zones without desync. No duplicate entities. |
| R4 | Rapid fire spam | Auto-click fire button | Weapon respects cooldown. Ammo does not go negative. |
| R5 | Inventory overflow | Mine/pick up hundreds of items | Server stores all items. Client JSON parse handles large arrays. |
| R6 | Disconnect persistence | Pick up items, move far from spawn, close browser. Reconnect with same username. | Player spawns at last position with last inventory and health. |
| R7 | Concurrent base editing | Two faction members place structures simultaneously | Both structures saved to DB. No deadlock. On reload, both structures render. |
| R8 | Chat empty/whitespace | Press Enter, type spaces, press Enter | Message is not sent. |
| R9 | Mine depleted node | Click on a node that another player just depleted | No item gained. No crash. |
| R10 | Respawn while alive | Press `R` while alive | Nothing happens. Respawn only works when dead. |
| R11 | NPC pathfinding stress | Lure 5+ hostile NPCs around Rock/DeepRavine tiles | NPCs pathfind around obstacles using A*. No server lag spike. |
| R12 | OnFire DOT persistence | Get hit by Heavy Flamethrower, then die and respawn | OnFire component is removed on death. No damage after respawn. |
| R13 | Boss add spawning | Engage Core Warden for >15 seconds | Rover adds spawn near the boss. Adds are registered in EntityRegistry and appear in snapshots. |

---

## 6. Performance Checks

| Check | Method | Acceptable Threshold |
|-------|--------|---------------------|
| Server tick budget | Observe server logs | `elapsed_ms` should stay below 50 ms consistently. |
| Client FPS | Observe FPS counter | > 45 FPS on modern hardware. |
| Memory growth | Run server for 30 minutes with 2 players | RSS growth < 50 MB. No OOM. |
| Snapshot size | Connect 2 clients, observe tick text | Snapshot processing does not cause client stutter. |
| Automated stress test | `cargo test --workspace` runs `test_simulation_tick_stress` | 1000 ticks with 110 entities completes in < 5 seconds in debug mode. |

---

## 7. Known Limitations / Not Yet Implemented

The following features are **not** in the current build. Do **not** report them as bugs:

- **Breach Generator crafting & warp signature scanning** (raid initiation is currently admin/test-only).
- **Faction management UI** (faction assignment is done via dev token `username#faction_id`).
- **Sound effects** (Howler.js integration planned for polish phase).
- **Hydroponics / drones** (stretch goals).
- **Asteroid base denial without faction** is implemented server-side but UI messaging is minimal.
- **Crashed Freighter loot spawning** (event is broadcast-only; no physical container or guards spawn yet).
- **Solar Flare gameplay effects** (client minimap disruption only; no server-side electronic weapon disable yet).
- **Titanium mining** — titanium is sold by traders but does not spawn as a mineable resource node. Must be purchased.
- **Salvage Site interaction** — salvage sites spawn in the world but cannot yet be harvested (requires `salvage_torch` tool and interaction system).
- **Loot Containers & Hacking** — `LootContainer` and `HackingTarget` components exist but no entities spawn with them yet.
- **Fabrication Plant bonus crafting** — POIs spawn but the `tier_bonus` crafting override is not yet wired to the crafting system.
- **Comms Array hacking** — arrays spawn but cannot be interacted with yet.

---

## 8. Bug Reporting Template

If you find an issue, please file a ticket with:

```
**Category:** [Crash / Desync / UI / Balance / Other]
**Repro Steps:**
1. ...
2. ...
**Expected:** ...
**Actual:** ...
**Environment:** [Browser, OS, server commit hash]
**Logs:** [Paste server/client console output]
**Screenshot:** [If UI-related]
```

---

## 9. Sign-Off

| Tester | Date | Result |
|--------|------|--------|
|        |      | [ ] Pass / [ ] Fail |

**Critical blockers for release:**
- [ ] No server crashes during 30-minute session.
- [ ] Zone transfer works reliably for 2+ players.
- [ ] Death, inventory drop, and respawn cycle is coherent.
- [ ] Base structures persist across reconnects.
- [ ] Tilemap generates deterministically and renders correctly for all connected clients.
- [ ] NPC pathfinding does not cause tick budget overflow with 100+ entities.
- [ ] All 6 resource types spawn in correct biomes and are mineable.
