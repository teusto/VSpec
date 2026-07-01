# Phase 4 — Error Handling + Observability for Agent Use

This phase standardizes MCP error semantics, adds per-tool tracing context safe for agent environments, and introduces correlation ID propagation for both HTTP and MCP paths.

---

## 1) What was implemented

### A. Standardized MCP error classes
In `src/presentation/mcp/server.rs`, MCP tool failures are normalized into four semantic categories:

- `validation` → `invalid_params` + `http_code: 400`
- `not_found` → `resource_not_found` + `http_code: 404`
- `conflict` → `invalid_request` + `http_code: 409`
- `internal` → `internal_error` + `http_code: 500`

Each MCP error now includes metadata in `error.data`:

```json
{
  "error_type": "validation|not_found|conflict|internal",
  "http_code": 400,
  "correlation_id": "..."
}
```

This aligns MCP tool errors with the API semantics already used in HTTP.

---

### B. Per-tool observability context
A `run_tool(...)` wrapper was added in `src/presentation/mcp/server.rs` to instrument each tool invocation with:

- `tool_name`
- `correlation_id`
- `args_hash`
- `duration_ms`
- success/failure result

This wrapper logs start/finish/failure events with consistent fields.

---

### C. Sensitive payload protection in logs
To avoid leaking raw tool inputs, logs do **not** include full args.
Instead, they include only a deterministic `args_hash`.

A shared helper was added in `src/observability.rs`:
- `new_correlation_id()`
- `args_hash<T: Serialize>(&T)`

---

### D. Correlation IDs across HTTP and MCP
#### HTTP path
Added request ID middleware in `src/presentation/http/routes.rs`:
- `SetRequestIdLayer::x_request_id(MakeRequestUuid)`
- `PropagateRequestIdLayer::x_request_id()`

This ensures `x-request-id` is set and returned in responses.

#### MCP path
Each tool call now generates a `correlation_id` and includes it in:
- logs
- MCP error metadata (`error.data.correlation_id`)

So both transports have traceable request identifiers.

---

## 2) Files changed

- `Cargo.toml`
  - enabled `tower-http` feature: `request-id`
- `src/main.rs`
  - added `mod observability;`
- `src/observability.rs` (new)
  - shared correlation/hash utilities
- `src/presentation/http/routes.rs`
  - request-id middleware layers
- `src/presentation/mcp/server.rs`
  - standardized error mapping + tool tracing wrapper + safe logging

---

## 3) Validation

`cargo check` passes after Phase 4 integration.

Remaining warnings are pre-existing dead-code warnings in `RepoError` payload fields.

---

## 4) How to run

### HTTP only
```bash
cargo run
```

### HTTP + MCP stdio
```bash
ENABLE_MCP_STDIO=1 cargo run
```

---

## 5) Notes

- HTTP correlation ID currently travels via `x-request-id` headers and tracing middleware.
- MCP correlation ID is generated per tool invocation and embedded in logs + MCP error metadata.
- This gives end-to-end debugging handles across both protocols without logging sensitive payload content.
