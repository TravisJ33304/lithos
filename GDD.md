# Lithos - Software Requirements Specification (SRS)
**Genre:** Top-down multiplayer survival crafting (Rust meets Rimworld)
**Perspective:** 2D Vector Art
**Core Loop:** Factions build persistent bases on private asteroids, venturing into a shared, procedurally generated overworld to gather resources, fight PvE/PvP threats, and extract loot back home.

## 1. System Architecture & Networking
The backend is split into two distinct tiers to ensure stability and scalability.

### 1.1 Central Orchestration API (Global)
*   **Tech Stack:** Rust (`axum` or `actix-web`), PostgreSQL/CockroachDB.
*   **Responsibilities:**
    *   Player authentication and account metadata.
    *   Global faction registry and cross-server Faction Wealth Leaderboards.
    *   Server browser and matchmaking (directing clients to specific Dedicated Game Servers).

### 1.2 Dedicated Game Servers (Per-Shard/Instance)
*   **Tech Stack:** Rust (`tokio` async runtime, `tokio-tungstenite` for WebSockets), SQLite.
*   **Capacity:** Max 100 concurrent players per server.
*   **Responsibilities:**
    *   Maintains the Authoritative Game State in-memory (Entities, Projectiles, Inventories, Asteroid grids).
    *   Executes the main game loop at a fixed tick rate (e.g., 20-30 TPS).
    *   Calculates physics, collisions, and state changes, broadcasting snapshots to connected WebSocket clients.
    *   Periodically flushes state (player inventories, base layouts) to the local SQLite database to avoid blocking the main thread.
    *   Reports health, population, and Faction Wealth metrics back to the Central API.

### 1.3 Client Frontend
*   **Tech Stack:** HTML5 Canvas powered by **PixiJS** or **Phaser 3** for 60+ FPS WebGL rendering. 
*   **UI Layer:** React or vanilla HTML/CSS overlaid on the canvas for menus, inventory, chat, and HUD.
*   **Networking Strategy:** Client-side prediction for local player movement with server reconciliation. Server-authoritative hit registration and lag compensation for projectiles.

## 2. World Zones & Instancing
### 2.1 The Overworld (Zone 0)
*   A single, persistent map procedurally generated once per server wipe using Perlin noise for biomes.
*   **Radial Difficulty:** 
    *   *Outer Rim:* Safe arrival points, low-tier resources, basic Automata threats.
    *   *Mid-Zone:* Contested resources, Scrapper Colonies (NPC hubs).
    *   *The Core:* Highest-value loot, massive PvE bosses (Core Wardens), major PvP choke points.
*   **Dynamic Events:** Weather events (Meteor Showers, Solar Flares) and dynamic POIs (Crashed Freighters) spawn organically to drive conflict.

### 2.2 Asteroid Bases (Zones 1-100)
*   Each faction is assigned a private grid coordinate space representing their asteroid.
*   The server dynamically loads/unloads these zones into memory based on faction member presence.

## 3. Core Mechanics & Controls
*   **Movement:** WASD for omnidirectional movement.
*   **Aiming:** Mouse cursor determines facing/aiming (Twin-stick shooter style). Left Click fires, Right Click interacts/alt-fire.
*   **Death Penalty:** Dying in the Overworld results in a total inventory/gear drop at the death location.
*   **Respawn:** Players respawn in their Asteroid's Medbay. A "Scrapper Dispenser" (on a cooldown) provides a basic, non-tradable loadout (plasma pistol, mining laser) to prevent soft-locking.

## 4. Progression (Action-Based Mastery)
*   No randomized blueprints. Players level up skill branches (Fabrication, Extraction, Ballistics, Cybernetics) by performing related actions.
*   Leveling up unlocks the ability to craft higher-tier items and structures within that branch.

## 5. Economy & Anti-Exploit
*   **Currency:** "Credits" stored in Faction Vaults.
*   **NPC Traders (Scrapper Colonies):** 
    *   Simulated localized supply and demand. Traders have daily Credit limits and will crash prices if flooded with a single resource type.
*   **Player Trading:** No formal UI for currency exchange between players. Players must drop items on the ground to barter, preventing automated market manipulation.

## 6. Base Building & Automation
*   **Grid System:** Structures (Walls, Doors, Workbenches) snap to a 2D tile grid.
*   **Resource Management:**
    *   *Power Grid:* Generators (Solar, Nuclear) must be wired to defenses and workbenches.
    *   *Life Support:* If power is cut, bases lose Oxygen. Players inside without spacesuits suffer asphyxiation damage.
    *   *Hydroponics:* Used to grow biological materials for crafting.
*   **Automation (Stretch Goal):** Craftable drones (Logistics, Agri, Defense) to automate base tasks and sorting.

## 7. The Raid Mechanic (Online-Only)
*   Asteroids are completely inaccessible when a faction is offline.
*   When online, attackers can craft an expensive "Breach Generator" in the overworld to scan for active Warp Signatures.
*   Initiating a breach triggers a 5-10 minute warning UI for the defenders (*"WARP BREACH DETECTED"*), giving them time to recall from the overworld and prepare base defenses before the attackers teleport in.

## 8. Art Style & User Interface
*   **Visuals:** Crisp, 2D Vector Art sprites.
*   **HUD Layout:** Balanced and clean. Includes Health, Ammo, Hotbar, Minimap, and a Chat window. Event logs and status readouts are integrated into the Chat window rather than cluttering the screen.

## 9. Minimum Viable Product (MVP) Milestone
The first development deliverable must prove the core technical loop:
1.  Full client-server WebSocket communication and state synchronization.
2.  Basic WASD movement and collision detection in a simple, flat Overworld.
3.  The ability to seamlessly teleport between the Overworld and a private Asteroid base instance.
