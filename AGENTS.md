# AGENTS.md

Guidance for AI agents working in this repository.

## Project Overview

**lazysql** — terminal UI (TUI) for exploring PostgreSQL databases.
Allows connecting to a configured database, browsing schemas, tables, and field metadata.

- **Language**: Rust (edition 2024)
- **MSRV**: stable, edition 2024
- **Status**: early development, breaking changes expected

## Stack

| Crate            | Purpose                                        |
| ---------------- | ---------------------------------------------- |
| `tokio`          | async runtime (`features = ["full"]`)          |
| `tokio-postgres` | PostgreSQL driver                              |
| `ratatui`        | TUI rendering                                  |
| `crossterm`      | terminal backend for ratatui                   |
| `async-trait`    | async methods in traits                        |
| `serde`          | serialization/deserialization                  |
| `toml`           | TOML config file parsing/writing               |
| `serde_json`     | JSON support for postgres driver               |
| `chrono`         | datetime support for postgres driver           |
| `futures`        | async stream utilities                         |
| `sqlparser`      | SQL syntax highlighting in the editor          |

## Architecture

```
src/
├── main.rs               # entrypoint — must be #[tokio::main] async
├── config.rs             # PostgresConfig, Connect enum, DbKind
├── config/
│   └── storage.rs        # TOML config persistence (~/.config/lazysql/config.toml)
├── db.rs
├── db/
│   ├── postgres.rs       # re-exports
│   ├── postgres/
│   │   ├── init.rs       # PostgresRepo::new, connection setup
│   │   └── tables.rs     # impl Database for PostgresRepo
│   ├── repo.rs           # re-exports
│   └── repo/
│       ├── db_repo.rs    # DbClient enum, DbError
│       └── tables_repo.rs # Database trait + all data types
├── state.rs
├── state/
│   ├── app.rs            # AppState — central state, all async DB methods
│   ├── mode.rs           # AppMode (Normal/Insert/Search/Command/Result)
│   ├── navigation.rs     # Router + Screen enum (TUI navigation stack)
│   ├── records/
│   │   └── mod.rs        # RecordsState, RecordsSource, pagination helpers
│   ├── connection/
│   │   ├── mod.rs        # ConnectState, ConnectionStatus, ConnectionMeta, ActivePane
│   │   └── form.rs       # FormState and connection form validation
│   ├── search.rs         # SearchState
│   └── sql_input.rs      # SqlInputState, SqlResult
├── ui.rs
└── ui/
    ├── layout.rs         # layout helpers
    ├── theme.rs          # color palette / styles
    ├── screens.rs        # re-exports all screen modules
    ├── screens/
    │   ├── connect.rs    # connection list screen (with error popup)
    │   ├── add_connection.rs # add-connection form screen
    │   ├── database.rs   # schema/table browser (split pane)
    │   ├── inspect.rs    # table details screen (fields, indexes, FK refs)
    │   └── records.rs    # paginated records table screen
    ├── widgets.rs        # re-exports all widget modules
    └── widgets/
        ├── help.rs       # help overlay
        ├── hintbar.rs    # bottom hint bar
        ├── search.rs     # search input widget
        ├── sql.rs        # SQL result popup
        ├── sql_editor.rs # SQL editor with sqlparser highlighting
        └── statusbar.rs  # status bar
```

### Key types

- `Connect` / `PostgresConfig` — connection config. `Connect` is the enum that gates DB kind.
- `DbClient` — enum wrapping the active connection (`DbClient::Postgres(PostgresRepo)`).
- `DbError` — unified error type. All DB functions return `Result<_, DbError>`.
- `Database` trait — async interface; implement per DB backend. Methods: `get_schemas`, `get_tables`, `get_table_details`, `fetch_rows`, `execute_sql`, `execute_sql_with_options`.
- `AppState` — central state struct; owns connections, current DB client, all sub-states, and provides async methods for DB operations.
- `AppMode` — vim-style modal state: `Normal | Insert | Search | Command | Result`.
- `Router` — stack-based screen navigator. `push`/`pop`/`current`.
- `Screen` — `Connect | AddConnection | Database | Inspect | Records`.
- `TableRef` — schema-qualified table reference returned by schema discovery.
- `TableDetails` — rich table metadata for Inspect screen: fields, indexes, FK refs, row count, size.
- `FetchRowsResult` — paginated result: columns, rows (`Vec<Option<String>>`), total_count.
- `ConnectionStatus` — `Unknown | Online | Offline` — reachability state per saved connection.
- `ConnectionMeta` — display-only, driver-agnostic view of a connection config.
- `ActivePane` — `Schemas | Tables` — focus state for the Database split view.
- `ConfigStorage` — reads/writes `~/.config/lazysql/config.toml` (TOML, implemented).

## Development Setup

Test database runs via Podman Compose. Credentials are in `docker-compose.yml`.

```bash
# Spin up test DB, run all integration tests, tear down
just test

# Keep DB running for manual inspection
just up

# Connect to test DB with pgcli (requires DB running)
just connect

# Release build
just build

# Run the app
just dev
```

`just test` is the only supported way to run integration tests. It uses
`podman-compose`, exports the test database password, runs `cargo test`, and
tears the database down.

## Building & Testing

```bash
cargo build
just test
cargo fmt
cargo clippy -- -D warnings
```

Integration tests in `db/postgres/tables.rs` and `db/postgres/init.rs` require the
test database environment prepared by `just test`. Do not run DB integration tests
with plain `cargo test`; it will miss the test database setup and credentials.

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
/// Returns all user-visible table references for the current connection.
pub async fn get_schemas(&self) -> Result<Vec<TableRef>, DbError> { ... }
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

### TDD (Test-Driven Development)

Write tests BEFORE implementation:

1. Write failing test defining expected behavior
2. Implement minimal code to pass
3. Refactor if needed
4. Repeat

Test location: `#[cfg(test)] mod test` at bottom of each file.
Integration tests requiring DB: run exclusively via `just test`.

## Adding a New DB Backend

1. Add a variant to `Connect` and `DbKind` in `config.rs`.
2. Create `src/db/<backend>/init.rs` with a `<Backend>Repo` struct and `new()`.
3. Implement the `Database` trait in `src/db/<backend>/tables.rs`.
4. Add a variant to `DbClient` in `db_repo.rs` and handle it in `DbClient::new`.

## Agent Capabilities

Agents can:

- Implement and extend `Database` trait for new backends
- Add new `Screen` variants and navigation logic in `navigation.rs`
- Add UI screens in `ui/screens/` and widgets in `ui/widgets/`
- Extend `AppState` with new async DB methods
- Write tests inside `#[cfg(test)] mod test` at the bottom of each file
- Run `cargo fmt`, `cargo clippy`, `just test`

## Restricted Actions

- **No hardcoded secrets** — credentials belong in `docker-compose.yml` (dev) or env vars (prod). Never in `.rs` source files outside of test fixtures that are explicitly local-only.
- **No `unwrap()` in non-test code** — propagate errors with `?` or return `DbError`.
- **No `unsafe`** — not needed anywhere in this codebase.
- **Don't skip CI checks** — all code must pass `cargo build`, `just test`, `cargo clippy`.

## Security Notes

- `NoTls` is intentional for local dev. Production will require TLS — don't remove the `NoTls` path but add a TLS variant when networking leaves localhost.
- Connection strings built in `PostgresConfig::connection_string()` must never be logged.

---

**Last Updated**: 2026-05-14
