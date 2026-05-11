# AGENTS.md

Guidance for AI agents working in this repository.

## Project Overview

**lazysql** — terminal UI (TUI) for exploring PostgreSQL databases.
Allows connecting to a configured database, browsing schemas, tables, and field metadata.

- **Language**: Rust (edition 2024)
- **MSRV**: stable, edition 2024
- **Status**: early development, breaking changes expected

## Stack

| Crate | Purpose |
|---|---|
| `tokio` | async runtime (`features = ["full"]`) |
| `tokio-postgres` | PostgreSQL driver |
| `ratatui` | TUI rendering |
| `crossterm` | terminal backend for ratatui |
| `async-trait` | async methods in traits |

## Architecture

```
src/
├── main.rs               # entrypoint — must be #[tokio::main] async
├── config.rs             # PostgresConfig, Connect enum, DbKind
├── config/
│   └── storage.rs        # (planned) read/write connection configs from disk
├── db.rs
├── db/
│   ├── postgres.rs       # re-exports
│   ├── postgres/
│   │   ├── init.rs       # PostgresRepo::new, connection setup
│   │   └── tables.rs     # impl Database for PostgresRepo
│   ├── repo.rs           # re-exports
│   └── repo/
│       ├── db_repo.rs    # DbClient enum, DbError
│       └── tables_repo.rs # Database trait, Table/Schema/TableField/ConstraintType
├── state.rs
└── state/
    ├── app_state.rs      # AppState — connection list, current DbClient
    ├── router.rs         # Router + Screen enum (TUI navigation stack)
    └── connect.rs        # (planned) connection screen logic
```

### Key types

- `Connect` / `PostgresConfig` — connection config. `Connect` is the enum that gates DB kind.
- `DbClient` — enum wrapping the active connection (`DbClient::Postgres(PostgresRepo)`).
- `DbError` — unified error type. All DB functions return `Result<_, DbError>`.
- `Database` trait — async interface; implement per DB backend.
- `AppState` — holds the list of `Connect` configs and the optional active `DbClient`.
- `Router` — stack-based screen navigator. `push`/`pop`/`current`.

## Development Setup

Test database runs via Podman Compose. Credentials are in `docker-compose.yml`.

```bash
# Spin up test DB, run all tests, tear down
just test

# Keep DB running for manual inspection
just run_postgres

# Release build
just build
```

`just test` uses `podman-compose` — requires Podman installed.

## Building & Testing

```bash
cargo build
cargo test
cargo fmt
cargo clippy -- -D warnings
```

Integration tests in `db/postgres/tables.rs` and `db/postgres/init.rs` require the
test database to be running. Run `just test` or `just run_postgres` first.

Always run `cargo fmt` and `cargo clippy` after any `.rs` change. Fix all warnings before committing.

## Coding Conventions

### Early returns

Prefer early returns over nested `match`/`if let`. Avoids rightward drift.

```rust
// bad
fn foo(x: Option<i32>) -> i32 {
    if let Some(v) = x {
        if v > 0 {
            return v * 2;
        }
    }
    0
}

// good
fn foo(x: Option<i32>) -> i32 {
    let Some(v) = x else { return 0 };
    if v <= 0 { return 0; }
    v * 2
}
```

### Error handling

- Use `?` everywhere the parent function returns `Result`. No manual `match Ok/Err` just to re-wrap.
- All DB functions return `Result<T, DbError>`. Map driver errors with `.map_err(DbError::Postgres)`.

```rust
// bad
let rows = match client.query(...).await {
    Ok(r) => r,
    Err(e) => return Err(DbError::Postgres(e)),
};

// good
let rows = client.query(...).await.map_err(DbError::Postgres)?;
```

### Doc comments

Every public function, struct, enum, and trait gets a `///` doc comment.
One concise sentence is enough. Add a second sentence only if the behaviour is non-obvious.

```rust
/// Returns all schemas visible to the current connection, excluding system catalogs.
pub async fn get_schemas(&self) -> Result<Vec<Schema>, DbError> { ... }
```

Private helpers don't need doc comments unless the logic is tricky.

### Inline comments

Use `//` only for non-obvious WHY — a constraint, workaround, or invariant a reader would miss.
Don't narrate what the code already says.

### Async

All DB access is async. Use `#[async_trait]` when implementing async trait methods.
`main` must be `#[tokio::main] async fn main()`.

### Naming

- Types / enum variants: `PascalCase`
- Functions, fields, variables, modules: `snake_case`
- Constants / statics: `UPPER_SNAKE_CASE`
- Avoid shadowing variable names inside the same scope.

### Standard traits

Prefer implementing `Display`, `From`, `TryFrom` over ad-hoc conversion methods.
`PostgresConfig::from()` should be `impl Display for PostgresConfig` or
`fn connection_string(&self) -> String` — the name `from` conflicts with the `From` trait.

### Port type

Use `u16` for port numbers (`PostgresConfig::port`), not `u32`.

## Adding a New DB Backend

1. Add a variant to `Connect` and `DbKind` in `config.rs`.
2. Create `src/db/<backend>/init.rs` with a `<Backend>Repo` struct and `new()`.
3. Implement the `Database` trait in `src/db/<backend>/tables.rs`.
4. Add a variant to `DbClient` in `db_repo.rs` and handle it in `DbClient::new`.

## Agent Capabilities

Agents can:

- Implement and extend `Database` trait for new backends
- Add new `Screen` variants and navigation logic in `router.rs`
- Implement `config/storage.rs` (TOML-based config persistence)
- Write tests inside `#[cfg(test)] mod test` at the bottom of each file
- Run `cargo fmt`, `cargo clippy`, `just test`

## Restricted Actions

- **No hardcoded secrets** — credentials belong in `docker-compose.yml` (dev) or env vars (prod). Never in `.rs` source files outside of test fixtures that are explicitly local-only.
- **No `unwrap()` in non-test code** — propagate errors with `?` or return `DbError`.
- **No `unsafe`** — not needed anywhere in this codebase.
- **Don't skip CI checks** — all code must pass `cargo build`, `cargo test`, `cargo clippy`.

## Security Notes

- `NoTls` is intentional for local dev. Production will require TLS — don't remove the `NoTls` path but add a TLS variant when networking leaves localhost.
- Connection strings built in `PostgresConfig::connection_string()` must never be logged.

---

**Last Updated**: 2026-05-11
