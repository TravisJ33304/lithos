# Lithos Implementation Progress (2026-05-01)

This document summarizes the implementation pass that addressed the GDD alignment plan items excluding vector sprite production.

## Delivered In This Pass

- Protocol contracts expanded with structured inventory snapshots, crafting catalog payloads, interactable updates, power state snapshots, raid target discovery, and richer client action messages.
- DOM UI shell added for menu, HUD, inventory, crafting, progression, chat, onboarding, and death-state overlays.
- Right-click interaction path added and wired through server-side interaction handling.
- Titanium resource nodes now spawn and can be mined.
- Salvage/hacking/fabrication-plant interactions now produce gameplay outcomes and progression hooks.
- Dynamic events now produce gameplay-side physical effects (resource or loot spawns in addition to announcements).
- Progression/economy expanded with Ballistics/Cybernetics recipes, drop-to-world item flow, medkit use behavior, equip hooks, and trader daily limits.
- Base systems extended with hydroponics/drone bay components and power-state reporting payloads for UI.
- NPC retreat/repair behavior completed with health-threshold transitions and recovery at retreat targets.
- Raid flow improved with raid target discovery, breach generator consumption requirement, and defender-online validation.

## Validation Run

- `cargo test -p lithos-protocol` passes.
- `cargo check` passes for full Rust workspace.
- `npm test` passes in `client`.
- `npm run build` passes in `client`.
