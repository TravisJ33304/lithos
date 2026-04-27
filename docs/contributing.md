# Contributing to Lithos

## Development Setup

### Prerequisites

- **Rust** (stable toolchain) — install via [rustup](https://rustup.rs/)
- **Node.js** 22+ and npm
- **Lefthook** — `cargo install lefthook` or via your package manager
- **cargo-nextest** — `cargo install cargo-nextest`
- **cargo-deny** — `cargo install cargo-deny`

### First-Time Setup

```bash
# Clone the repo
git clone https://github.com/TravisJ33304/lithos.git
cd lithos

# Install git hooks
lefthook install

# Build the Rust workspace
cargo build --workspace

# Set up the client
cd client
npm install
```

## Branching Model

We use **trunk-based development** on `main`:

- All changes go through **pull requests** to `main`.
- Feature branches should be short-lived (< 1 week).
- Branch naming: `feat/description`, `fix/description`, `docs/description`.

## Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add player respawn system
fix: correct collision detection at zone boundaries
docs: update networking protocol spec
chore: bump bevy_ecs to 0.15
refactor: extract zone manager into separate module
test: add round-trip tests for ZoneTransfer messages
ci: add cargo-deny check to CI pipeline
```

## Pull Request Checklist

Before submitting a PR, ensure:

- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo nextest run --workspace` passes
- [ ] `cd client && npx biome check .` passes
- [ ] `cd client && npx tsc --noEmit` passes
- [ ] New public Rust items have `///` doc comments
- [ ] New protocol messages are mirrored in `client/src/types/protocol.ts`

## Code Style

### Rust

- Follow `rustfmt` defaults (configured in `rustfmt.toml`).
- Use `thiserror` for library error types, `anyhow` for application error handling.
- All public items must have doc comments (`///`).
- Prefer `tracing` macros over `println!` for logging.

### TypeScript

- Biome handles formatting and linting — run `npx biome check .` to verify.
- Use explicit types for function parameters and return values.
- Keep Phaser API usage behind the engine abstraction layer (`src/engine/`).

## Architecture Decision Records

Major architectural decisions are documented in `docs/adr/`. When proposing a
significant change, create a new ADR following the numbered naming convention:

```
docs/adr/NNNN-short-title.md
```

See [0001-technology-stack.md](adr/0001-technology-stack.md) for an example.
