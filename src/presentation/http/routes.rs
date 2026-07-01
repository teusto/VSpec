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
    response::{IntoResponse, Response},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tower_http::trace::TraceLayer;

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
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: u16,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = Json(ErrorEnvelope {
            error: ErrorBody {
                code: self.status.as_u16(),
                message: self.message,
            },
        });

        (self.status, body).into_response()
    }
}

/// `GET /health` health-check endpoint.
async fn health() -> &'static str {
    "ok"
}

/// `GET /projects` placeholder endpoint that lists projects.
async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<ProjectResponse>>, AppError> {
    let conn = state.conn.clone();

    let projects = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to acquire database lock"))?;
        let repo = ProjectRepository::new(&conn);

        repo.list()
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to list projects"))
    })
    .await
    .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "project listing task failed"))??;

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
) -> Result<(StatusCode, Json<ProjectResponse>), AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::new(StatusCode::BAD_REQUEST, "name must not be empty"));
    }

    let conn = state.conn.clone();

    let created = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to acquire database lock"))?;
        let repo = ProjectRepository::new(&conn);

        repo.create(&NewProject {
            name: payload.name,
            description: payload.description,
        })
        .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to create project"))
    })
    .await
    .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "project creation task failed"))??;

    Ok((StatusCode::CREATED, Json(project_to_response(created))))
}

/// `GET /projects/{id}` placeholder endpoint that returns a single project.
async fn get_project(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<ProjectResponse>, AppError> {
    let conn = state.conn.clone();

    let project = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to acquire database lock"))?;
        let repo = ProjectRepository::new(&conn);

        repo.find_by_id(id)
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch project"))
    })
    .await
    .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "project fetch task failed"))??
    .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "project not found"))?;

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
) -> Result<Json<SpecResponse>, AppError> {
    let parsed_tag = SpecTag::from_str(&tag)
        .map_err(|_| AppError::new(StatusCode::BAD_REQUEST, "invalid spec tag"))?;
    let conn = state.conn.clone();

    let spec = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to acquire database lock"))?;
        let project_repo = ProjectRepository::new(&conn);

        let project_exists = project_repo
            .find_by_id(project_id)
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch project"))?
            .is_some();

        if !project_exists {
            return Err(AppError::new(StatusCode::NOT_FOUND, "project not found"));
        }

        let repo = crate::infrastructure::persistence::spec::SpecRepository::new(&conn);

        repo.find_by_project_and_tag(project_id, parsed_tag)
            .map_err(map_spec_repo_error)
    })
    .await
    .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "spec lookup task failed"))??
    .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "spec not found"))?;

    Ok(Json(spec_to_response(spec)))
}

/// `GET /projects/{id}/specs` returns all specs for a project.
async fn list_project_specs(
    Path(project_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SpecResponse>>, AppError> {
    let conn = state.conn.clone();

    let specs = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to acquire database lock"))?;
        let repo = crate::infrastructure::persistence::spec::SpecRepository::new(&conn);

        repo.list_by_project(project_id).map_err(map_spec_repo_error)
    })
    .await
    .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "spec listing task failed"))??;

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
) -> Result<(StatusCode, Json<SpecResponse>), AppError> {
    let parsed_tag = SpecTag::from_str(&tag)
        .map_err(|_| AppError::new(StatusCode::BAD_REQUEST, "invalid spec tag"))?;

    if payload.content.summary.trim().is_empty() {
        return Err(AppError::new(StatusCode::BAD_REQUEST, "content.summary must not be empty"));
    }

    let conn = state.conn.clone();

    let created = tokio::task::spawn_blocking(move || {
        let conn = conn
            .lock()
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to acquire database lock"))?;
        let project_repo = ProjectRepository::new(&conn);

        let project_exists = project_repo
            .find_by_id(project_id)
            .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "failed to fetch project"))?
            .is_some();

        if !project_exists {
            return Err(AppError::new(StatusCode::NOT_FOUND, "project not found"));
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
    .map_err(|_| AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "spec creation task failed"))??;

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

fn map_spec_repo_error(error: RepoError) -> AppError {
    match error {
        RepoError::UniqueConstraint => {
            AppError::new(StatusCode::CONFLICT, "a spec with this tag already exists for the project")
        }
        RepoError::Sqlite(rusqlite::Error::SqliteFailure(sqlite_error, _))
            if sqlite_error.extended_code == 787 =>
        {
            AppError::new(StatusCode::NOT_FOUND, "project not found")
        }
        RepoError::Sqlite(_) => AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "database error"),
        RepoError::InvalidTag(_) => AppError::new(StatusCode::BAD_REQUEST, "invalid spec tag"),
        RepoError::JsonEncode(_) | RepoError::JsonDecode(_) => {
            AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "spec content serialization error")
        }
    }
}
