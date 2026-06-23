use crate::{
    app::AppState,
    infrastructure::persistence::spec::{SpecContentTemplate, SpecTag},
};
use axum::{Json, Router, extract::Path, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Builds the HTTP router with health, project, and project-spec endpoints.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/{id}", get(get_project))
        .route("/projects/{id}/specs", get(list_project_specs))
        .route(
            "/projects/{id}/specs/{tag}",
            get(get_project_spec_by_tag).post(create_project_spec),
        )
        .with_state(state)
}

/// `GET /health` health-check endpoint.
async fn health() -> &'static str {
    "ok"
}

/// `GET /projects` placeholder endpoint that lists projects.
async fn list_projects() -> &'static str {
    "list projects"
}

/// `POST /projects` placeholder endpoint that creates a project.
async fn create_project() -> &'static str {
    "project created"
}

/// `GET /projects/{id}` placeholder endpoint that returns a single project.
async fn get_project(Path(id): Path<String>) -> String {
    format!("project id: {id}")
}

/// Request body for creating a spec under a project/tag pair.
#[derive(Debug, Deserialize)]
struct CreateSpecPayload {
    content: SpecContentTemplate,
}

/// Response body returned by spec endpoints.
#[derive(Debug, Serialize)]
struct SpecResponse {
    project_id: i64,
    tag: String,
    content: SpecContentTemplate,
}

/// `GET /projects/{id}/specs/{tag}` returns one spec for a project and tag.
async fn get_project_spec_by_tag(
    Path((project_id, tag)): Path<(i64, String)>,
) -> Result<Json<SpecResponse>, StatusCode> {
    let parsed_tag = SpecTag::from_str(&tag).map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(SpecResponse {
        project_id,
        tag: parsed_tag.as_str().to_string(),
        content: default_template(),
    }))
}

/// `GET /projects/{id}/specs` returns all specs for a project.
async fn list_project_specs(Path(project_id): Path<i64>) -> Json<Vec<SpecResponse>> {
    Json(vec![SpecResponse {
        project_id,
        tag: SpecTag::Architecture.as_str().to_string(),
        content: default_template(),
    }])
}

/// `POST /projects/{id}/specs/{tag}` creates a spec for the given tag.
async fn create_project_spec(
    Path((project_id, tag)): Path<(i64, String)>,
    Json(payload): Json<CreateSpecPayload>,
) -> Result<(StatusCode, Json<SpecResponse>), StatusCode> {
    let parsed_tag = SpecTag::from_str(&tag).map_err(|_| StatusCode::BAD_REQUEST)?;

    if payload.content.summary.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok((
        StatusCode::CREATED,
        Json(SpecResponse {
            project_id,
            tag: parsed_tag.as_str().to_string(),
            content: payload.content,
        }),
    ))
}

/// Generates a placeholder spec template used by stub handlers.
fn default_template() -> SpecContentTemplate {
    SpecContentTemplate {
        summary: "placeholder summary".to_string(),
        goals: vec!["placeholder goal".to_string()],
        requirements: vec!["placeholder requirement".to_string()],
        acceptance_criteria: vec!["placeholder acceptance criteria".to_string()],
        notes: Some("placeholder notes".to_string()),
    }
}
