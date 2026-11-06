use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    errors::AppError,
    models::{
        CreateTaskRequest, HealthResponse, PaginatedTasks, PaginationMetadata, PaginationQuery,
        Task, UpdateTaskRequest,
    },
    state::AppState,
};

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "nexora-api".to_string(),
    })
}

pub async fn readiness_check(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, AppError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
        .map(|_| {
            Json(HealthResponse {
                status: "ready".to_string(),
                service: "nexora-api".to_string(),
            })
        })
        .map_err(|error| {
            tracing::error!("Database readiness check failed: {}", error);
            AppError::service_unavailable("Database unavailable")
        })
}

pub async fn create_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    payload: Result<Json<CreateTaskRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<(StatusCode, Json<Task>), AppError> {
    let payload = payload?;
    payload.validate()?;

    let task = state.tasks.create_task(user.id, &payload).await?;
    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn list_tasks(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    query: Result<Query<PaginationQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<PaginatedTasks>, AppError> {
    let Query(query) = query?;
    let pagination = query.validate()?;
    let (tasks, total) = tokio::try_join!(
        state.tasks.list_tasks(user.id, &pagination),
        state.tasks.count_tasks(user.id, pagination.completed)
    )?;
    let total_pages = (total / pagination.limit) as u64 + u64::from(total % pagination.limit != 0);

    Ok(Json(PaginatedTasks {
        data: tasks,
        pagination: PaginationMetadata {
            page: pagination.page,
            limit: pagination.limit as u32,
            total,
            total_pages,
        },
    }))
}

pub async fn route_not_found() -> impl IntoResponse {
    AppError::not_found("ROUTE_NOT_FOUND", "Route not found")
}

pub async fn method_not_allowed() -> impl IntoResponse {
    AppError::method_not_allowed()
}

pub async fn get_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<Task>, AppError> {
    let task_id = Uuid::parse_str(&id)
        .map_err(|_| AppError::bad_request("INVALID_UUID", "Invalid UUID format"))?;

    let task = state.tasks.get_task_by_id(user.id, task_id).await?;
    let task = task.ok_or_else(|| AppError::not_found("TASK_NOT_FOUND", "Task not found"))?;

    Ok(Json(task))
}

pub async fn update_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
    payload: Result<Json<UpdateTaskRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Task>, AppError> {
    let task_id = Uuid::parse_str(&id)
        .map_err(|_| AppError::bad_request("INVALID_UUID", "Invalid UUID format"))?;

    let payload = payload?;
    payload.validate()?;

    let task = state.tasks.update_task(user.id, task_id, payload.0).await?;
    let task = task.ok_or_else(|| AppError::not_found("TASK_NOT_FOUND", "Task not found"))?;

    Ok(Json(task))
}

pub async fn delete_task(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let task_id = Uuid::parse_str(&id)
        .map_err(|_| AppError::bad_request("INVALID_UUID", "Invalid UUID format"))?;

    let deleted = state.tasks.delete_task(user.id, task_id).await?;
    if !deleted {
        return Err(AppError::not_found("TASK_NOT_FOUND", "Task not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}
