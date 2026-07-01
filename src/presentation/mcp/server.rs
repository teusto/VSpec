use crate::{
    app::AppState,
    infrastructure::persistence::{
        project::{NewProject, Project, ProjectRepository},
        spec::{NewSpec, RepoError, Spec, SpecContentTemplate, SpecRepository, SpecTag},
    },
    observability,
};
use rmcp::{
    ErrorData as McpError, Json, ServerHandler, ServiceExt,
    handler::server::{
        tool::ToolRouter,
        wrapper::Parameters,
    },
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::str::FromStr;
use std::time::Instant;

#[derive(Clone)]
pub struct McpServer {
    state: AppState,
    tool_router: ToolRouter<Self>,
}

impl McpServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    pub async fn run_stdio(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let service = self.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    }
}

#[derive(Debug, Serialize, JsonSchema)]
struct HealthCheckOutput {
    status: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct EmptyInput {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ProjectByIdInput {
    project_id: i64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CreateProjectInput {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ProjectSpecByTagInput {
    project_id: i64,
    tag: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CreateProjectSpecInput {
    project_id: i64,
    tag: String,
    content: SpecContentTemplate,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ProjectOutput {
    id: i64,
    name: String,
    description: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ProjectsListOutput {
    projects: Vec<ProjectOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ProjectToolOutput {
    project: ProjectOutput,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SpecOutput {
    project_id: i64,
    tag: String,
    content: SpecContentTemplate,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SpecsListOutput {
    specs: Vec<SpecOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SpecToolOutput {
    spec: SpecOutput,
}

#[tool_router]
impl McpServer {
    #[tool(name = "health_check", description = "Check if API is running")]
    async fn health_check(&self) -> Result<Json<HealthCheckOutput>, McpError> {
        let correlation_id = observability::new_correlation_id();
        let args_hash = observability::args_hash(&EmptyInput {});

        run_tool("health_check", correlation_id, args_hash, async move {
            Ok(Json(HealthCheckOutput {
                status: "ok".to_string(),
            }))
        })
        .await
    }

    #[tool(name = "list_projects", description = "List all projects")]
    async fn list_projects(
        &self,
        Parameters(_): Parameters<EmptyInput>,
    ) -> Result<Json<ProjectsListOutput>, McpError> {
        let correlation_id = observability::new_correlation_id();
        let args_hash = observability::args_hash(&EmptyInput {});
        let conn = self.state.conn.clone();
        let error_correlation_id = correlation_id.clone();
        let spawn_correlation_id = error_correlation_id.clone();

        run_tool("list_projects", correlation_id, args_hash, async move {
            let projects = tokio::task::spawn_blocking(move || {
                let conn = conn.lock().map_err(|_| {
                    internal_error("failed to acquire database lock", &spawn_correlation_id)
                })?;
                let repo = ProjectRepository::new(&conn);

                repo.list()
                    .map_err(|_| internal_error("failed to list projects", &spawn_correlation_id))
            })
            .await
            .map_err(|_| internal_error("project listing task failed", &error_correlation_id))??;

            Ok(Json(ProjectsListOutput {
                projects: projects
                    .into_iter()
                    .map(project_to_output)
                    .collect::<Vec<_>>(),
            }))
        })
        .await
    }

    #[tool(name = "get_project", description = "Get one project by id")]
    async fn get_project(
        &self,
        Parameters(input): Parameters<ProjectByIdInput>,
    ) -> Result<Json<ProjectToolOutput>, McpError> {
        let correlation_id = observability::new_correlation_id();
        let args_hash = observability::args_hash(&input);
        let conn = self.state.conn.clone();
        let error_correlation_id = correlation_id.clone();
        let spawn_correlation_id = error_correlation_id.clone();

        run_tool("get_project", correlation_id, args_hash, async move {
            let project = tokio::task::spawn_blocking(move || {
                let conn = conn.lock().map_err(|_| {
                    internal_error("failed to acquire database lock", &spawn_correlation_id)
                })?;
                let repo = ProjectRepository::new(&conn);

                repo.find_by_id(input.project_id)
                    .map_err(|_| internal_error("failed to fetch project", &spawn_correlation_id))
            })
            .await
            .map_err(|_| internal_error("project fetch task failed", &error_correlation_id))??
            .ok_or_else(|| not_found_error("project not found", &error_correlation_id))?;

            Ok(Json(ProjectToolOutput {
                project: project_to_output(project),
            }))
        })
        .await
    }

    #[tool(name = "create_project", description = "Create a new project")]
    async fn create_project(
        &self,
        Parameters(input): Parameters<CreateProjectInput>,
    ) -> Result<Json<ProjectToolOutput>, McpError> {
        let correlation_id = observability::new_correlation_id();
        let args_hash = observability::args_hash(&input);

        if input.name.trim().is_empty() {
            return Err(validation_error("name must not be empty", &correlation_id));
        }

        let conn = self.state.conn.clone();
        let error_correlation_id = correlation_id.clone();
        let spawn_correlation_id = error_correlation_id.clone();

        run_tool("create_project", correlation_id, args_hash, async move {
            let project = tokio::task::spawn_blocking(move || {
                let conn = conn.lock().map_err(|_| {
                    internal_error("failed to acquire database lock", &spawn_correlation_id)
                })?;
                let repo = ProjectRepository::new(&conn);

                repo.create(&NewProject {
                    name: input.name,
                    description: input.description,
                })
                .map_err(|_| internal_error("failed to create project", &spawn_correlation_id))
            })
            .await
            .map_err(|_| internal_error("project creation task failed", &error_correlation_id))??;

            Ok(Json(ProjectToolOutput {
                project: project_to_output(project),
            }))
        })
        .await
    }

    #[tool(name = "list_project_specs", description = "List all specs for a project")]
    async fn list_project_specs(
        &self,
        Parameters(input): Parameters<ProjectByIdInput>,
    ) -> Result<Json<SpecsListOutput>, McpError> {
        let correlation_id = observability::new_correlation_id();
        let args_hash = observability::args_hash(&input);
        let conn = self.state.conn.clone();
        let error_correlation_id = correlation_id.clone();
        let spawn_correlation_id = error_correlation_id.clone();

        run_tool("list_project_specs", correlation_id, args_hash, async move {
            let specs = tokio::task::spawn_blocking(move || {
                let conn = conn.lock().map_err(|_| {
                    internal_error("failed to acquire database lock", &spawn_correlation_id)
                })?;
                let repo = SpecRepository::new(&conn);

                repo.list_by_project(input.project_id)
                    .map_err(|error| map_spec_repo_error(error, &spawn_correlation_id))
            })
            .await
            .map_err(|_| internal_error("spec listing task failed", &error_correlation_id))??;

            Ok(Json(SpecsListOutput {
                specs: specs.into_iter().map(spec_to_output).collect::<Vec<_>>(),
            }))
        })
        .await
    }

    #[tool(
        name = "get_project_spec_by_tag",
        description = "Get one project spec by project_id and tag"
    )]
    async fn get_project_spec_by_tag(
        &self,
        Parameters(input): Parameters<ProjectSpecByTagInput>,
    ) -> Result<Json<SpecToolOutput>, McpError> {
        let correlation_id = observability::new_correlation_id();
        let args_hash = observability::args_hash(&input);

        let tag = SpecTag::from_str(&input.tag)
            .map_err(|_| validation_error("invalid spec tag", &correlation_id))?;
        let conn = self.state.conn.clone();
        let error_correlation_id = correlation_id.clone();
        let spawn_correlation_id = error_correlation_id.clone();

        run_tool("get_project_spec_by_tag", correlation_id, args_hash, async move {
            let spec = tokio::task::spawn_blocking(move || {
                let conn = conn.lock().map_err(|_| {
                    internal_error("failed to acquire database lock", &spawn_correlation_id)
                })?;
                let project_repo = ProjectRepository::new(&conn);

                let project_exists = project_repo
                    .find_by_id(input.project_id)
                    .map_err(|_| internal_error("failed to fetch project", &spawn_correlation_id))?
                    .is_some();

                if !project_exists {
                    return Err(not_found_error("project not found", &spawn_correlation_id));
                }

                let spec_repo = SpecRepository::new(&conn);
                spec_repo
                    .find_by_project_and_tag(input.project_id, tag)
                    .map_err(|error| map_spec_repo_error(error, &spawn_correlation_id))
            })
            .await
            .map_err(|_| internal_error("spec lookup task failed", &error_correlation_id))??
            .ok_or_else(|| not_found_error("spec not found", &error_correlation_id))?;

            Ok(Json(SpecToolOutput {
                spec: spec_to_output(spec),
            }))
        })
        .await
    }

    #[tool(
        name = "create_project_spec",
        description = "Create a new project spec for a tag"
    )]
    async fn create_project_spec(
        &self,
        Parameters(input): Parameters<CreateProjectSpecInput>,
    ) -> Result<Json<SpecToolOutput>, McpError> {
        let correlation_id = observability::new_correlation_id();
        let args_hash = observability::args_hash(&input);

        let tag = SpecTag::from_str(&input.tag)
            .map_err(|_| validation_error("invalid spec tag", &correlation_id))?;

        if input.content.summary.trim().is_empty() {
            return Err(validation_error(
                "content.summary must not be empty",
                &correlation_id,
            ));
        }

        let conn = self.state.conn.clone();
        let error_correlation_id = correlation_id.clone();
        let spawn_correlation_id = error_correlation_id.clone();

        run_tool("create_project_spec", correlation_id, args_hash, async move {
            let spec = tokio::task::spawn_blocking(move || {
                let conn = conn.lock().map_err(|_| {
                    internal_error("failed to acquire database lock", &spawn_correlation_id)
                })?;
                let project_repo = ProjectRepository::new(&conn);

                let project_exists = project_repo
                    .find_by_id(input.project_id)
                    .map_err(|_| internal_error("failed to fetch project", &spawn_correlation_id))?
                    .is_some();

                if !project_exists {
                    return Err(not_found_error("project not found", &spawn_correlation_id));
                }

                let spec_repo = SpecRepository::new(&conn);
                spec_repo
                    .create(&NewSpec {
                        project_id: input.project_id,
                        tag,
                        content: input.content,
                    })
                    .map_err(|error| map_spec_repo_error(error, &spawn_correlation_id))
            })
            .await
            .map_err(|_| internal_error("spec creation task failed", &error_correlation_id))??;

            Ok(Json(SpecToolOutput {
                spec: spec_to_output(spec),
            }))
        })
        .await
    }
}

fn project_to_output(project: Project) -> ProjectOutput {
    ProjectOutput {
        id: project.id,
        name: project.name,
        description: project.description,
        created_at: project.created_at,
        updated_at: project.updated_at,
    }
}

fn spec_to_output(spec: Spec) -> SpecOutput {
    SpecOutput {
        project_id: spec.project_id,
        tag: spec.tag.as_str().to_string(),
        content: spec.content,
    }
}

fn map_spec_repo_error(error: RepoError, correlation_id: &str) -> McpError {
    match error {
        RepoError::UniqueConstraint => {
            conflict_error("a spec with this tag already exists for the project", correlation_id)
        }
        RepoError::Sqlite(rusqlite::Error::SqliteFailure(sqlite_error, _))
            if sqlite_error.extended_code == 787 =>
        {
            not_found_error("project not found", correlation_id)
        }
        RepoError::Sqlite(_) => internal_error("database error", correlation_id),
        RepoError::InvalidTag(_) => validation_error("invalid spec tag", correlation_id),
        RepoError::JsonEncode(_) | RepoError::JsonDecode(_) => {
            internal_error("spec content serialization error", correlation_id)
        }
    }
}

fn validation_error(message: &str, correlation_id: &str) -> McpError {
    attach_error_metadata(
        McpError::invalid_params(message.to_string(), None),
        "validation",
        400,
        correlation_id,
    )
}

fn not_found_error(message: &str, correlation_id: &str) -> McpError {
    attach_error_metadata(
        McpError::resource_not_found(message.to_string(), None),
        "not_found",
        404,
        correlation_id,
    )
}

fn conflict_error(message: &str, correlation_id: &str) -> McpError {
    attach_error_metadata(
        McpError::invalid_request(message.to_string(), None),
        "conflict",
        409,
        correlation_id,
    )
}

fn internal_error(message: &str, correlation_id: &str) -> McpError {
    attach_error_metadata(
        McpError::internal_error(message.to_string(), None),
        "internal",
        500,
        correlation_id,
    )
}

fn attach_error_metadata(
    mut error: McpError,
    error_type: &str,
    http_code: u16,
    correlation_id: &str,
) -> McpError {
    let extra = json!({
        "error_type": error_type,
        "http_code": http_code,
        "correlation_id": correlation_id,
    });

    match error.data.take() {
        Some(serde_json::Value::Object(mut existing)) => {
            if let serde_json::Value::Object(extra_obj) = extra {
                for (k, v) in extra_obj {
                    existing.insert(k, v);
                }
            }
            error.data = Some(serde_json::Value::Object(existing));
        }
        Some(other) => {
            error.data = Some(json!({
                "details": other,
                "error_type": error_type,
                "http_code": http_code,
                "correlation_id": correlation_id,
            }));
        }
        None => {
            error.data = Some(extra);
        }
    }

    error
}

async fn run_tool<T, F>(
    tool_name: &'static str,
    correlation_id: String,
    args_hash: String,
    future: F,
) -> Result<T, McpError>
where
    F: std::future::Future<Output = Result<T, McpError>>,
{
    let started = Instant::now();

    tracing::info!(
        tool_name,
        correlation_id,
        args_hash,
        "MCP tool invocation started"
    );

    match future.await {
        Ok(result) => {
            tracing::info!(
                tool_name,
                correlation_id,
                args_hash,
                duration_ms = started.elapsed().as_millis() as u64,
                "MCP tool invocation finished"
            );
            Ok(result)
        }
        Err(error) => {
            tracing::warn!(
                tool_name,
                correlation_id,
                args_hash,
                duration_ms = started.elapsed().as_millis() as u64,
                error_code = error.code.0,
                error_message = %error.message,
                "MCP tool invocation failed"
            );
            Err(error)
        }
    }
}

#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MCP tools for specs-v project and spec management APIs".to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
