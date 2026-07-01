# Phase 3 — MCP Server Implementation (`rmcp` + stdio)

This phase implements a working MCP server in the project, exposes the first full tool set, and keeps the existing HTTP API running unchanged.

---

## 1) What was implemented

### MCP server module
- Added MCP presentation module:
  - `src/presentation/mcp/mod.rs`
  - `src/presentation/mcp/server.rs`
- Exposed it in `src/presentation/mod.rs`.

### Runtime wiring
- `main.rs` now supports optional MCP stdio startup alongside HTTP:
  - controlled by `ENABLE_MCP_STDIO=1`
  - HTTP server remains on `0.0.0.0:3000`

### Dependencies
- `rmcp` now includes stdio transport support:
  - `features = ["server", "transport-io"]`

---

## 2) Implemented MCP tools (Phase 2 contract coverage)

All 7 tools are implemented in `src/presentation/mcp/server.rs`:

1. `health_check`
2. `list_projects`
3. `get_project`
4. `create_project`
5. `list_project_specs`
6. `get_project_spec_by_tag`
7. `create_project_spec`

The implementation reuses the same repositories and validation patterns used by HTTP routes.

---

## 3) Important bug fix from runtime panic

### Symptom
`rmcp` panicked with:

> MCP specification requires tool outputSchema to have root type 'object', but found 'array'

### Root cause
MCP tool structured outputs must have an object root schema. Returning `Json<Vec<...>>` violates this requirement.

### Fix
Array outputs were wrapped in object-root response types:
- `list_projects` now returns `{ "projects": [...] }`
- `list_project_specs` now returns `{ "specs": [...] }`

This aligns with MCP output schema requirements and prevents startup panic.

---

## 4) Error behavior mapping

The MCP server maps domain/repository errors into MCP `ErrorData` categories:
- Validation errors → `invalid_params`
- Missing resources → `resource_not_found`
- Internal failures (lock/task/db) → `internal_error`
- Unique spec conflicts include machine hint data (`http_code: 409`) in error `data`

This keeps behavior close to the existing HTTP JSON envelope semantics.

---

## 5) Shared model/schema updates

To support schema generation for tool inputs/outputs:
- `SpecContentTemplate` now derives `JsonSchema` in persistence model.

This allows structured tool payloads with typed schema metadata.

---

## 6) How to run

### HTTP only
```bash
cargo run
```

### HTTP + MCP stdio
```bash
ENABLE_MCP_STDIO=1 cargo run
```

Use an MCP-capable host/client configured for stdio transport to connect to this process.

---

## 7) Validation done

- `cargo check` passes after MCP integration.
- Existing warnings remain in `RepoError` payload fields (unrelated to MCP runtime behavior).

---

## 8) Notes for Phase 4

Next recommended work:
- add MCP integration tests (tool discovery + call success/error paths)
- standardize conflict/error metadata contract for clients
- add optional per-tool tracing fields (tool name, duration, outcome)
