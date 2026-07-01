# Phase 2 — MCP Tool Surface for `specs-v`

This document defines the first MCP tool surface for `specs-v`, including tool-vs-REST decisions, naming conventions, schemas, error shape, and idempotency behavior.

---

## 1) Tool vs REST-only decisions

### Policy
- Keep the current REST API unchanged and available.
- Expose core project/spec workflows as MCP tools.
- In this phase, no current business endpoint is REST-only.

### Route-to-tool matrix

| REST Route | MCP Tool | Decision | Notes |
|---|---|---|---|
| `GET /health` | `health_check` | MCP tool + REST | Agent-friendly readiness check |
| `GET /projects` | `list_projects` | MCP tool + REST | Core read workflow |
| `GET /projects/{id}` | `get_project` | MCP tool + REST | Core read workflow |
| `POST /projects` | `create_project` | MCP tool + REST | Core write workflow |
| `GET /projects/{id}/specs` | `list_project_specs` | MCP tool + REST | Core read workflow |
| `GET /projects/{id}/specs/{tag}` | `get_project_spec_by_tag` | MCP tool + REST | Core read workflow |
| `POST /projects/{id}/specs/{tag}` | `create_project_spec` | MCP tool + REST | Core write workflow |

---

## 2) Naming conventions (v1)

### Tool names
- Use `snake_case`.
- Use stable semantic names aligned with user intent:
  - reads: `health_check`, `list_*`, `get_*`
  - writes: `create_*`
- Do not encode transport details in tool names.

### Argument names
- Use stable, explicit names:
  - `project_id: integer`
  - `tag: string` (`architecture | business | qa | security`)
  - `content: object` (spec template)
  - `name: string`
  - `description: string | null`
- Do not rename arguments across tools for the same concept.

---

## 3) Shared schemas

These schemas are derived from current HTTP DTOs and persistence models.

### `SpecTag` (string enum)
```json
{
  "type": "string",
  "enum": ["architecture", "business", "qa", "security"]
}
```

### `SpecContentTemplate`
```json
{
  "type": "object",
  "required": ["summary", "goals", "requirements", "acceptance_criteria"],
  "properties": {
    "summary": { "type": "string" },
    "goals": { "type": "array", "items": { "type": "string" } },
    "requirements": { "type": "array", "items": { "type": "string" } },
    "acceptance_criteria": { "type": "array", "items": { "type": "string" } },
    "notes": { "type": ["string", "null"] }
  }
}
```

### `ProjectResponse`
```json
{
  "type": "object",
  "required": ["id", "name", "created_at", "updated_at"],
  "properties": {
    "id": { "type": "integer" },
    "name": { "type": "string" },
    "description": { "type": ["string", "null"] },
    "created_at": { "type": "string" },
    "updated_at": { "type": "string" }
  }
}
```

### `SpecResponse`
```json
{
  "type": "object",
  "required": ["project_id", "tag", "content"],
  "properties": {
    "project_id": { "type": "integer" },
    "tag": { "$ref": "#/definitions/SpecTag" },
    "content": { "$ref": "#/definitions/SpecContentTemplate" }
  }
}
```

---

## 4) Shared error shape (mapped from API envelope)

Current API envelope:
```json
{
  "error": {
    "code": 400,
    "message": "..."
  }
}
```

MCP tool errors must preserve equivalent semantics:
- `code` = HTTP-like status category used by current API behavior.
- `message` = stable, readable reason string.

### Error classes
- `400` validation/input error (e.g., invalid tag, empty name, empty `content.summary`)
- `404` missing resource (project/spec not found)
- `409` conflict (duplicate spec for `(project_id, tag)`)
- `500` internal error (db lock/task failure/db serialization)

---

## 5) Tool contract table (minimal useful slice)

## `health_check`
- **Input schema**
```json
{ "type": "object", "properties": {}, "additionalProperties": false }
```
- **Output schema**
```json
{ "type": "object", "required": ["status"], "properties": { "status": { "type": "string", "const": "ok" } } }
```
- **Error shape**: normally none; if transport/runtime fails use shared error shape.
- **Idempotency / side effects**: idempotent, no side effects.

## `list_projects`
- **Input schema**
```json
{ "type": "object", "properties": {}, "additionalProperties": false }
```
- **Output schema**
```json
{ "type": "array", "items": { "$ref": "#/definitions/ProjectResponse" } }
```
- **Error shape**: `500` possible for DB/task failures.
- **Idempotency / side effects**: idempotent, no side effects.

## `get_project`
- **Input schema**
```json
{
  "type": "object",
  "required": ["project_id"],
  "properties": { "project_id": { "type": "integer", "minimum": 1 } },
  "additionalProperties": false
}
```
- **Output schema**
```json
{ "$ref": "#/definitions/ProjectResponse" }
```
- **Error shape**: `404` if project missing, `500` on internal failures.
- **Idempotency / side effects**: idempotent, no side effects.

## `create_project`
- **Input schema**
```json
{
  "type": "object",
  "required": ["name"],
  "properties": {
    "name": { "type": "string", "minLength": 1 },
    "description": { "type": ["string", "null"] }
  },
  "additionalProperties": false
}
```
- **Output schema**
```json
{ "$ref": "#/definitions/ProjectResponse" }
```
- **Error shape**: `400` when `name` is empty after trim, `500` on internal failures.
- **Idempotency / side effects**: non-idempotent, creates DB row.

## `list_project_specs`
- **Input schema**
```json
{
  "type": "object",
  "required": ["project_id"],
  "properties": { "project_id": { "type": "integer", "minimum": 1 } },
  "additionalProperties": false
}
```
- **Output schema**
```json
{ "type": "array", "items": { "$ref": "#/definitions/SpecResponse" } }
```
- **Error shape**: `500` on internal/repository failures.
- **Idempotency / side effects**: idempotent, no side effects.

## `get_project_spec_by_tag`
- **Input schema**
```json
{
  "type": "object",
  "required": ["project_id", "tag"],
  "properties": {
    "project_id": { "type": "integer", "minimum": 1 },
    "tag": { "$ref": "#/definitions/SpecTag" }
  },
  "additionalProperties": false
}
```
- **Output schema**
```json
{ "$ref": "#/definitions/SpecResponse" }
```
- **Error shape**:
  - `400` invalid tag
  - `404` project/spec not found
  - `500` internal/repository failures
- **Idempotency / side effects**: idempotent, no side effects.

## `create_project_spec`
- **Input schema**
```json
{
  "type": "object",
  "required": ["project_id", "tag", "content"],
  "properties": {
    "project_id": { "type": "integer", "minimum": 1 },
    "tag": { "$ref": "#/definitions/SpecTag" },
    "content": { "$ref": "#/definitions/SpecContentTemplate" }
  },
  "additionalProperties": false
}
```
- **Output schema**
```json
{ "$ref": "#/definitions/SpecResponse" }
```
- **Error shape**:
  - `400` invalid tag / empty `content.summary`
  - `404` project not found
  - `409` duplicate `(project_id, tag)`
  - `500` internal/repository failures
- **Idempotency / side effects**: non-idempotent create; duplicate key returns conflict (`409`).

---

## 6) Canonical argument/response examples

### Example A: read tool — `get_project_spec_by_tag`

Request:
```json
{
  "project_id": 12,
  "tag": "architecture"
}
```

Success response:
```json
{
  "project_id": 12,
  "tag": "architecture",
  "content": {
    "summary": "Service boundaries and data flow",
    "goals": ["Define ownership"],
    "requirements": ["Document API boundaries"],
    "acceptance_criteria": ["Boundary diagram approved"],
    "notes": null
  }
}
```

Error response:
```json
{
  "error": {
    "code": 404,
    "message": "spec not found"
  }
}
```

### Example B: write tool — `create_project_spec`

Request:
```json
{
  "project_id": 12,
  "tag": "security",
  "content": {
    "summary": "Threat model baseline",
    "goals": ["Identify top risks"],
    "requirements": ["List mitigations"],
    "acceptance_criteria": ["Top 5 threats documented"],
    "notes": "Initial draft"
  }
}
```

Conflict error response:
```json
{
  "error": {
    "code": 409,
    "message": "a spec with this tag already exists for the project"
  }
}
```

---

## 7) Source anchors (current implementation)
- Routes and endpoint behavior: `src/presentation/http/routes.rs`
- Error envelope and mappings: `src/presentation/http/routes.rs`
- Spec content model: `src/infrastructure/persistence/spec.rs`
