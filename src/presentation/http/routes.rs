use crate::{
    app::AppState,
    infrastructure::persistence::{
        project::{NewProject, Project, ProjectRepository},
        spec::{NewSpec, RepoError, Spec, SpecContentTemplate, SpecTag},
    },
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
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
async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<ProjectResponse>>, StatusCode> {
    let conn = state.conn.clone();

    let projects = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let repo = ProjectRepository::new(&conn);

        repo.list().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok(Json(
        projects
            .into_iter()
            .map(project_to_response)
            .collect::<Vec<_>>(),
    ))
}

/// `POST /projects` placeholder endpoint that creates a project.
async fn create_project(
    State(state): State<AppState>,
    Json(payload): Json<CreateProjectPayload>,
) -> Result<(StatusCode, Json<ProjectResponse>), StatusCode> {
    if payload.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let conn = state.conn.clone();

    let created = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let repo = ProjectRepository::new(&conn);

        repo.create(&NewProject {
            name: payload.name,
            description: payload.description,
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok((StatusCode::CREATED, Json(project_to_response(created))))
}

/// `GET /projects/{id}` placeholder endpoint that returns a single project.
async fn get_project(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<ProjectResponse>, StatusCode> {
    let conn = state.conn.clone();

    let project = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let repo = ProjectRepository::new(&conn);

        repo.find_by_id(id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(project_to_response(project)))
}

/// Request body for creating a project.
#[derive(Debug, Deserialize)]
struct CreateProjectPayload {
    name: String,
    description: Option<String>,
}

/// Response body returned by project endpoints.
#[derive(Debug, Serialize)]
struct ProjectResponse {
    id: i64,
    name: String,
    description: Option<String>,
    created_at: String,
    updated_at: String,
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
    State(state): State<AppState>,
) -> Result<Json<SpecResponse>, StatusCode> {
    let parsed_tag = SpecTag::from_str(&tag).map_err(|_| StatusCode::BAD_REQUEST)?;
    let conn = state.conn.clone();

    let spec = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let project_repo = ProjectRepository::new(&conn);

        let project_exists = project_repo
            .find_by_id(project_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .is_some();

        if !project_exists {
            return Err(StatusCode::NOT_FOUND);
        }

        let repo = crate::infrastructure::persistence::spec::SpecRepository::new(&conn);

        repo.find_by_project_and_tag(project_id, parsed_tag)
            .map_err(map_spec_repo_error)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(spec_to_response(spec)))
}

/// `GET /projects/{id}/specs` returns all specs for a project.
async fn list_project_specs(
    Path(project_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SpecResponse>>, StatusCode> {
    let conn = state.conn.clone();

    let specs = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let repo = crate::infrastructure::persistence::spec::SpecRepository::new(&conn);

        repo.list_by_project(project_id).map_err(map_spec_repo_error)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok(Json(
        specs
            .into_iter()
            .map(spec_to_response)
            .collect::<Vec<_>>(),
    ))
}

/// `POST /projects/{id}/specs/{tag}` creates a spec for the given tag.
async fn create_project_spec(
    Path((project_id, tag)): Path<(i64, String)>,
    State(state): State<AppState>,
    Json(payload): Json<CreateSpecPayload>,
) -> Result<(StatusCode, Json<SpecResponse>), StatusCode> {
    let parsed_tag = SpecTag::from_str(&tag).map_err(|_| StatusCode::BAD_REQUEST)?;

    if payload.content.summary.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let conn = state.conn.clone();

    let created = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let project_repo = ProjectRepository::new(&conn);

        let project_exists = project_repo
            .find_by_id(project_id)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .is_some();

        if !project_exists {
            return Err(StatusCode::NOT_FOUND);
        }

        let repo = crate::infrastructure::persistence::spec::SpecRepository::new(&conn);

        repo.create(&NewSpec {
            project_id,
            tag: parsed_tag,
            content: payload.content,
        })
        .map_err(map_spec_repo_error)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    Ok((
        StatusCode::CREATED,
        Json(spec_to_response(created)),
    ))
}

fn project_to_response(project: Project) -> ProjectResponse {
    ProjectResponse {
        id: project.id,
        name: project.name,
        description: project.description,
        created_at: project.created_at,
        updated_at: project.updated_at,
    }
}

fn spec_to_response(spec: Spec) -> SpecResponse {
    SpecResponse {
        project_id: spec.project_id,
        tag: spec.tag.as_str().to_string(),
        content: spec.content,
    }
}

fn map_spec_repo_error(error: RepoError) -> StatusCode {
    match error {
        RepoError::UniqueConstraint => StatusCode::CONFLICT,
        RepoError::Sqlite(rusqlite::Error::SqliteFailure(sqlite_error, _))
            if sqlite_error.extended_code == 787 =>
        {
            StatusCode::NOT_FOUND
        }
        RepoError::Sqlite(_) | RepoError::InvalidTag(_) | RepoError::JsonEncode(_) | RepoError::JsonDecode(_) => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
