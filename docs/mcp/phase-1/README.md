# Phase 1 — MCP Foundations for `specs-v`

This document is the **Phase 1 learning + implementation guide** to make this project MCP-ready.

Goal of Phase 1: understand the MCP basics deeply enough to explain how an agent discovers and calls capabilities exposed by your app.

---

## 1) MCP roles: Host, Client, Server

### Concept
- **Host**: the application where an agent runs (IDE, chat app, etc.).
- **MCP Client**: protocol layer inside the host that connects to MCP servers.
- **MCP Server**: your process that exposes tools/resources/prompts.

### How this maps to this project
- `specs-v` currently behaves as an HTTP API server (Axum).
- In MCP terms, this repo will also run an MCP server alongside (or integrated with) the existing API service.
- Existing route handlers are your best starting point for MCP tool behavior.

Project anchors:
- Router and endpoints: `src/presentation/http/routes.rs`
- Runtime bootstrap: `src/main.rs`

### Phase 1 checklist
- [ ] I can explain host/client/server without notes.
- [ ] I can identify where `specs-v` fits today and what needs to be added for MCP server responsibilities.

---

## 2) MCP primitives: Tools, Resources, Prompts

### Concept
- **Tools**: executable actions with typed inputs/outputs.
- **Resources**: read-only context a model can fetch.
- **Prompts**: reusable prompt templates that standardize agent behavior.

### How this maps to this project
Initial mapping for this API:
- Tools (first candidates):
  - `health_check`
  - `list_projects`
  - `get_project`
  - `create_project`
  - `list_project_specs`
  - `get_project_spec_by_tag`
  - `create_project_spec`
- Resources (later):
  - static docs like API conventions, tag definitions, schema explanation.
- Prompts (later):
  - helper prompts like “create project + baseline spec” workflows.

Project anchors:
- Existing operations already implemented in handlers: `src/presentation/http/routes.rs`
- JSON payload types and response shapes: `src/presentation/http/routes.rs`

### Phase 1 checklist
- [ ] I can classify every current endpoint as tool/resource/prompt candidate.
- [ ] I can justify why write operations belong in tools.

---

## 3) MCP protocol lifecycle

### Concept
At high level, MCP interactions follow:
1. **Initialize** (handshake + capabilities)
2. **List capabilities** (e.g., tools)
3. **Invoke capability** (`tools/call`, etc.)
4. **Return structured result or error**

### How this maps to this project
- Your API already has consistent structured errors; this is excellent preparation for tool-call error contracts.
- Your tracing setup is also useful to observe tool-call flow once MCP transport is added.

Project anchors:
- API error envelope pattern: `src/presentation/http/routes.rs`
- Tracing init: `src/main.rs`
- Request tracing middleware: `src/presentation/http/routes.rs`

### Phase 1 checklist
- [ ] I can describe the initialization and capability negotiation flow.
- [ ] I can describe where tool errors should be normalized in this codebase.

---

## 4) Transports: stdio first, then network

### Concept
Common MCP transport options:
- **stdio**: easiest local integration, good for learning.
- **network transport** (SSE/streaming HTTP variants): useful for remote deployment.

### How this maps to this project
- Start with **stdio MCP server** for rapid local testing with an MCP-capable host.
- Keep existing HTTP API as-is; MCP can call the same application logic.

Project anchors:
- Existing service startup location where MCP bootstrap can be added: `src/main.rs`

### Phase 1 checklist
- [ ] I can explain why stdio is preferred for first implementation.
- [ ] I can list pros/cons of stdio vs network transport for this repo.

---

## 5) Schemas and contracts

### Concept
MCP tools are reliable when inputs/outputs are explicitly typed with JSON schema.

### How this maps to this project
- You already added dependencies helpful for this direction:
  - `rmcp` (MCP server SDK)
  - `schemars` (schema derivation)
- Existing serde data structures can become tool contracts.

Project anchors:
- Dependencies: `Cargo.toml`
- Request/response structs: `src/presentation/http/routes.rs`

### Phase 1 checklist
- [ ] I can explain why schema-driven tools improve agent reliability.
- [ ] I can identify first structs to expose as MCP tool input/output.

---

## 6) Phase 1 completion criteria

You are done with Phase 1 when:

- [ ] You can explain MCP architecture (roles + primitives + lifecycle) in your own words.
- [ ] You can map each current API operation to an MCP tool candidate.
- [ ] You can explain how current error/tracing work will transfer to MCP debugging.
- [ ] You can explain why `rmcp` + `schemars` were added and what each will do next.

---

## 7) What comes next (Phase 2 preview)

In Phase 2, you will define and freeze the first MCP tool surface for this project:
- exact tool names
- input/output schema
- error contract
- first end-to-end tool call from an agent host
