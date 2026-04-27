# ADR 0001: Technology Stack

**Status:** Accepted
**Date:** 2026-04-26

## Context

Lithos is a top-down 2D multiplayer survival-crafting game played in the
browser. We need to choose a technology stack for:

1. The game server (authoritative game state, physics, networking)
2. The game client (rendering, input, UI)
3. The wire protocol between them
4. Authentication and player data persistence

## Decision

### Server: Rust + bevy_ecs + tokio

- **Rust** for memory safety, performance, and strong type system.
- **bevy_ecs** (standalone, without the Bevy engine) for data-oriented game
  state management with built-in parallelism and change detection.
- **tokio** for async networking (WebSocket via `tokio-tungstenite`).
- **SQLite** (via `rusqlite`) for per-shard asteroid base persistence.
- Network IO and game logic bridged via `mpsc` channels.

### Client: Phaser 3 + TypeScript + Vite

- **Phaser 3** as a batteries-included 2D game framework (physics, scenes,
  input, camera). All Phaser usage is behind an abstraction layer
  (`src/engine/`) to allow future engine swaps.
- **TypeScript** for type safety.
- **Vite** for fast builds and HMR during development.

### Wire Protocol: MessagePack over WebSocket

- **MessagePack** for compact binary serialization natively supported by both
  Rust (`rmp-serde`) and TypeScript (`@msgpack/msgpack`).

### Auth & Database: Supabase

- **Supabase Auth** for player authentication (JWT-based).
- **Supabase hosted PostgreSQL** for the Central API's persistent data
  (accounts, factions, leaderboards).

### Deployment: Docker

- Multi-stage Dockerfiles for server, API, and client.
- `docker-compose` for local development.
- Container images published to GHCR on tagged releases.

## Consequences

- Developers need Rust and Node.js toolchains installed locally.
- The bevy_ecs standalone approach requires manual game loop management.
- MessagePack types must be kept in sync between Rust and TypeScript manually
  (future: automated codegen).
- Supabase dependency means a network dependency for auth during development
  (mitigated by stub/mock auth in dev mode).
