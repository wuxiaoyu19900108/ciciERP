# ciciERP Copilot Instructions

## Build & Run

```bash
# Start dev server (port 3000)
cargo run -p cicierp-api

# Production build
cargo build --release -p cicierp-api

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p cicierp-db
cargo test -p cicierp-models

# Run a single test by name
cargo test -p cicierp-api test_name

# Generate API docs from inline comments
./scripts/gen_api_docs.sh
```

Environment variables override `config/default.toml`: `HOST`, `PORT`, `CORS_ORIGINS`, `NODE_ENV`.

## Architecture

Rust workspace with 4 internal crates:

| Crate | Path | Role |
|---|---|---|
| `cicierp-api` | `crates/api` | Axum web server, routes, middleware |
| `cicierp-db` | `crates/db` | SQLite connection pool, query structs |
| `cicierp-models` | `crates/models` | Serde data models, request/response types |
| `cicierp-utils` | `crates/utils` | `AppError`, `AppResult`, `ApiResponse` |

**Request flow:** `main.rs` → `routes/mod.rs` → route handler → `db::queries::*Queries` → SQLite

Database: single SQLite file at `data/cicierp.db` (WAL mode). One migration file at `migrations/001_init.sql`, run automatically on startup via `db.run_migrations()`.

## Key Conventions

### Response types

All handlers return `AppResult<Json<ApiResponse<T>>>`. The `ApiResponse<T>` wrapper always serializes as:
```json
{ "code": 200, "message": "success", "data": {...}, "timestamp": 1234567890 }
```

Use `ApiResponse::success(data)` or `ApiResponse::success_message("msg")`. Errors are returned via `AppError` variants which implement `IntoResponse` automatically.

### Route module pattern

Each domain module in `crates/api/src/routes/` exposes:
- `pub fn router() -> Router<AppState>` — protected routes (JWT required)
- `pub fn public_router() -> Router<AppState>` — unauthenticated routes (only when needed)

Routes are registered in `routes/mod.rs`. Protected routes get `.route_layer(from_fn_with_state(state, auth_middleware))`.

### DB query pattern

Queries use a struct-per-module pattern with a borrowed pool:
```rust
let queries = ProductQueries::new(state.db.pool());
let result = queries.list(page, page_size, ...).await?;
```

Dynamic SQL is built with `sqlx::QueryBuilder` (not raw string concat). All list queries filter `deleted_at IS NULL` (soft deletes).

### Auth

JWT via `Authorization: Bearer <token>` header or `auth_token` cookie (for web pages). The authenticated user is injected as `Extension<AuthUser>` into handlers:
```rust
Extension(user): Extension<AuthUser>
```

Integration/webhook routes use a separate `integration_auth_middleware` with a different API key scheme.

### API documentation comments

Inline doc comments follow this format (parsed by `scripts/gen_api_docs.sh`):
```rust
/// @api GET /api/v1/products
/// @desc 获取产品列表
/// @query page: number
/// @response 200 PagedResponse<ProductListItem>
/// @example curl -X GET "http://localhost:3000/api/v1/products"
```

### Pagination

Use `PagedResponse<T>` from `cicierp-models` for paginated list responses:
```rust
PagedResponse::new(items, page, page_size, total)
```

Query structs implement `.page()` and `.page_size()` with defaults (page=1, page_size=20, max=100).

### Error handling

`AppError` variants map directly to HTTP status codes. Use `?` to propagate `sqlx::Error` (auto-converts via `#[from]`). For validation, call `.validate().map_err(AppError::from)?` on request structs.

## Module Status

Implemented: products (+ costs/prices/content), suppliers, customers (+ addresses), orders, inventory (+ alerts), purchases (+ approve/receive), logistics (+ companies/shipments/tracking), proforma invoices (PI), commercial invoices (CI), exchange rates, users/auth (RBAC), integration API (for cicishop etc.), Web SSR UI (Askama templates).

Pending: AI 管家 (Feishu Bot + Claude API), 数据分析看板 (analytics dashboard).
