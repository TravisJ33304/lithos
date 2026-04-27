# Lithos

> Top-down 2D multiplayer survival-crafting game — Rust meets Rimworld, in the browser.

Factions build persistent bases on private asteroids, venturing into a shared, procedurally generated overworld to gather resources, fight PvE/PvP threats, and extract loot back home.

## Architecture

| Component | Stack | Purpose |
|---|---|---|
| **Dedicated Game Server** | Rust, tokio, bevy_ecs, WebSocket | Authoritative game state, fixed-tick ECS loop |
| **Central API** | Rust, axum, Supabase (Postgres) | Auth, factions, server browser, leaderboards |
| **Game Client** | TypeScript, Phaser 3, Vite | WebGL rendering, client-side prediction |
| **Protocol** | MessagePack over WebSocket | Compact binary serialization between client & server |

See [docs/architecture.md](docs/architecture.md) for detailed diagrams.

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 22+
- [Lefthook](https://github.com/evilmartians/lefthook) (optional, for git hooks)

### Server

```bash
cargo build --workspace
cargo run --bin lithos-server
```

### Client

```bash
cd client
npm install
npm run dev
```

### Docker (full stack)

```bash
cd deploy
docker compose up --build
```

## Quality Tooling

| Tool | Scope | Purpose |
|---|---|---|
| `rustfmt` | Rust | Formatting |
| `clippy` | Rust | Linting |
| `cargo-deny` | Rust | License + vulnerability scanning |
| `cargo-audit` | Rust | CVE advisory scanning |
| `cargo-nextest` | Rust | Test runner |
| Biome | TypeScript | Linting + formatting |
| Vitest | TypeScript | Unit tests |

All checks run automatically on push/PR via GitHub Actions. Pre-commit hooks
are managed by Lefthook — run `lefthook install` to enable locally.

## Project Structure

```
crates/
  lithos-protocol/   # Shared types and MessagePack codec
  lithos-world/      # ECS components, systems, world generation
  lithos-server/     # Dedicated Game Server binary
  lithos-api/        # Central Orchestration API binary
client/              # Phaser 3 TypeScript game client
deploy/              # Dockerfiles and docker-compose
docs/                # Architecture, contributing, ADRs
```

## License

[MIT](LICENSE)
