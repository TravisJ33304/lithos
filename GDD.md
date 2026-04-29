# Lithos - Game Design Document (GDD)

**Genre:** Top-down multiplayer survival crafting
**Perspective:** 2D Vector Art
**Core Loop:** Factions build persistent bases on private asteroids, venturing into a shared, procedurally generated overworld to gather resources, fight PvE/PvP threats, and extract loot back home.

## 1. System Architecture & Networking

The backend is split into two distinct tiers to ensure stability and scalability.

### 1.1 Central Orchestration API (Global)

* **Tech Stack:** Rust (`axum` or `actix-web`), PostgreSQL/CockroachDB.
* **Responsibilities:**
  * Player authentication and account metadata.
  * Global faction registry and cross-server Faction Wealth Leaderboards.
  * Server browser and matchmaking (directing clients to specific Dedicated Game Servers).

### 1.2 Dedicated Game Servers (Per-Shard/Instance)

* **Tech Stack:** Rust (`tokio` async runtime, `tokio-tungstenite` for WebSockets), SQLite.
* **Capacity:** Max 100 concurrent players per server.
* **Responsibilities:**
  * Maintains the Authoritative Game State in-memory (Entities, Projectiles, Inventories, Asteroid grids).
  * Executes the main game loop at a fixed tick rate (e.g., 20-30 TPS).
  * Calculates physics, collisions, and state changes, broadcasting snapshots to connected WebSocket clients.
  * Periodically flushes state (player inventories, base layouts) to the local SQLite database to avoid blocking the main thread.
  * Reports health, population, and Faction Wealth metrics back to the Central API.

### 1.3 Client Frontend

* **Tech Stack:** HTML5 Canvas powered by **PixiJS** or **Phaser 3** for 60+ FPS WebGL rendering.
* **UI Layer:** React or vanilla HTML/CSS overlaid on the canvas for menus, inventory, chat, and HUD.
* **Networking Strategy:** Client-side prediction for local player movement with server reconciliation. Server-authoritative hit registration and lag compensation for projectiles.

## 2. World Zones & Instancing

### 2.1 The Overworld (Zone 0)

* A single, persistent map procedurally generated once per server wipe using Perlin noise for biomes.
* **Radial Difficulty:**
  * *Outer Rim:* Safe arrival points, low-tier resources, basic Automata threats.
  * *Mid-Zone:* Contested resources, Scrapper Colonies (NPC hubs).
  * *The Core:* Highest-value loot, massive PvE bosses (Core Wardens), major PvP choke points.
* **Dynamic Events:** Weather events (Meteor Showers, Solar Flares) and dynamic POIs (Crashed Freighters) spawn organically to drive conflict.

### 2.2 Overworld Resources & Ecology
To drive the core gathering loop, the Overworld contains dynamically respawning nodes and interactive objects:
*   **Raw Resource Nodes (Renewable):**
    *   *Iron/Copper Outcroppings:* Found everywhere; used for basic structures and ammo. Mined with lasers/drills.
    *   *Titanium Nodes:* Found in rocky biomes; used for mid-tier structures and weapons.
    *   *Silica Deposits:* Found in arid biomes; refined into glass and basic electronics.
    *   *Uranium/Plutonium Veins:* Highly radioactive, found only in the Core or Mid-Zone craters. Required for late-game power and high-tier ammo.
    *   *Bio-Mass (Flora/Fungi):* Harvested for hydroponics seeds, basic healing items, and bio-fuel.
*   **Salvage & Scrap (Static/Semi-Renewable):**
    *   *Rusted Husks:* Abandoned vehicles and old mechs. Harvested with a Salvage Torch for mechanical components (gears, servos, wires).
    *   *Data Terminals:* Hacked to yield "Encrypted Drives" (sold for high Credits) or temporary minimap intel.
*   **Loot Containers (Instanced/Guarded):**
    *   *Supply Crates:* Basic scattered loot (ammo, scrap, low-tier meds).
    *   *Automata Receptacles:* Found inside robot structures. Contain specialized tech components (Logic Cores, Optics, Plasma Cells) required for Drones and Turrets.
    *   *Military Lockboxes:* Highly contested containers found at major POIs or airdrops. Contain fully crafted high-tier weapons or rare attachments.

### 2.3 Points of Interest (POIs) & Dynamic Events
*   **Static POIs:** Generated at map creation.
    *   *Automated Fabrication Plants:* Heavily guarded dungeons. Allow crafting one tier above the player's current skill level.
    *   *Derelict Comms Arrays:* Hackable towers that reveal high-tier nodes or player locations.
*   **Dynamic Events:** Spawn organically to drive conflict.
    *   *Meteor Showers:* Bombard zones, leaving temporary exotic mineral nodes.
    *   *Crashed Freighters:* Global broadcast event. Highly radioactive initially, guarded by surviving mechs, containing massive loot payloads.
    *   *Solar Flares:* Temporarily disable minimaps and specific electronic weapons in certain biomes.

### 2.4 Asteroid Bases (Zones 1-100)

* Each faction is assigned a private grid coordinate space representing their asteroid.
* The server dynamically loads/unloads these zones into memory based on faction member presence.

## 3. Core Mechanics & Controls

* **Movement:** WASD for omnidirectional movement.
* **Aiming:** Mouse cursor determines facing/aiming (Twin-stick shooter style). Left Click fires, Right Click interacts/alt-fire.
* **Death Penalty:** Dying in the Overworld results in a total inventory/gear drop at the death location.
* **Respawn:** Players respawn in their Asteroid's Medbay. A "Scrapper Dispenser" (on a cooldown) provides a basic, non-tradable loadout (plasma pistol, mining laser) to prevent soft-locking.

## 4. Progression (Action-Based Mastery)

* No randomized blueprints. Players level up skill branches (Fabrication, Extraction, Ballistics, Cybernetics) by performing related actions.
* Leveling up unlocks the ability to craft higher-tier items and structures within that branch.

## 5. Economy & Anti-Exploit

* **Currency:** "Credits" stored in Faction Vaults.
* **NPC Traders (Scrapper Colonies):**
  * Simulated localized supply and demand. Traders have daily Credit limits and will crash prices if flooded with a single resource type.
* **Player Trading:** No formal UI for currency exchange between players. Players must drop items on the ground to barter, preventing automated market manipulation.

## 6. Base Building & Automation

* **Grid System:** Structures (Walls, Doors, Workbenches) snap to a 2D tile grid.
* **Resource Management:**
  * *Power Grid:* Generators (Solar, Nuclear) must be wired to defenses and workbenches.
  * *Life Support:* If power is cut, bases lose Oxygen. Players inside without spacesuits suffer asphyxiation damage.
  * *Hydroponics:* Used to grow biological materials for crafting.
* **Automation (Stretch Goal):** Craftable drones (Logistics, Agri, Defense) to automate base tasks and sorting.

## 7. The Automata (PvE Threats)

The primary PvE antagonists are a faction of rogue, self-replicating robots. They scale in difficulty toward the map's center.

### 7.1 Enemy Types & Roles

* **Rovers:** Light, fast-moving wheeled or tracked units. They attack in swarms, relying on melee strikes or kamikaze explosions. Primarily found in the Outer Rim.
* **Drones:** Flying utility units. They can bypass ground obstacles and walls. Typically armed with light continuous lasers. They often accompany larger units to provide air support.
* **Walkers (Humanoid/Bipedal):** The backbone of the Automata forces. They utilize cover and pathfinding. Variants include:
  * *Assault Walkers:* Armed with shotguns or rapid-fire lasers; aggressive pushers.
  * *Sniper Walkers:* Armed with railguns; engage from long distances and attempt to kite players.
  * *Heavy Flamethrower Walkers:* Heavily armored; devastating at close range and capable of setting areas (and players) on fire, causing damage-over-time.
* **Heavy Siege Units (Tanks/Mechs):** Slow, massive targets found in the Mid-Zone and Core. Require coordinated faction efforts or heavy explosives to take down. They fire slow-moving, high-damage artillery shells.
* **The Core Wardens (Bosses):** Massive, unique bosses located at the center of the Overworld. They possess multiple attack phases, area-of-effect (AoE) abilities, and spawn adds. Defeating a Warden guarantees high-tier components needed for end-game automation or breaching.

### 7.2 AI & Behavior States

* **Idle/Patrol:** Moving along a defined path or guarding a specific POI.
* **Investigate:** Triggered by player noise (gunfire, mining) or taking damage from an unseen source.
* **Combat:** Active engagement, utilizing cover (Walkers) or swarm tactics (Rovers).
* **Retreat/Repair:** Some advanced units will attempt to flee to a nearby Automata structure to regenerate health if severely damaged.

## 7. The Raid Mechanic (Online-Only)

* Asteroids are completely inaccessible when a faction is offline.
* When online, attackers can craft an expensive "Breach Generator" in the overworld to scan for active Warp Signatures.
* Initiating a breach triggers a 5-10 minute warning UI for the defenders (*"WARP BREACH DETECTED"*), giving them time to recall from the overworld and prepare base defenses before the attackers teleport in.

## 8. Art Style & User Interface

* **Visuals:** Crisp, 2D Vector Art sprites.
* **HUD Layout:** Balanced and clean. Includes Health, Ammo, Hotbar, Minimap, and a Chat window. Event logs and status readouts are integrated into the Chat window rather than cluttering the screen.

## 9. Minimum Viable Product (MVP) Milestone

The first development deliverable must prove the core technical loop:

1. Full client-server WebSocket communication and state synchronization.
2. Basic WASD movement and collision detection in a simple, flat Overworld.
3. The ability to seamlessly teleport between the Overworld and a private Asteroid base instance.
