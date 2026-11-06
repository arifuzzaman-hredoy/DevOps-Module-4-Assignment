use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    errors::AppError,
    models::{CreateTaskRequest, Pagination, PatchField, Task, UpdateTaskRequest},
};

#[derive(Clone, Debug)]
pub struct TaskRepository {
    pub pool: PgPool,
}

impl TaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_task(
        &self,
        user_id: Uuid,
        request: &CreateTaskRequest,
    ) -> Result<Task, AppError> {
        let created = sqlx::query_as::<_, Task>(
            r#"
            INSERT INTO tasks (id, user_id, title, description, completed, created_at, updated_at)
            VALUES ($1, $2, $3, $4, FALSE, NOW(), NOW())
            RETURNING id, title, description, completed, created_at, updated_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(request.normalized_title()?)
        .bind(request.description.as_deref())
        .fetch_one(&self.pool)
        .await?;

        Ok(created)
    }

    pub async fn list_tasks(
        &self,
        user_id: Uuid,
        pagination: &Pagination,
    ) -> Result<Vec<Task>, AppError> {
        let tasks = sqlx::query_as::<_, Task>(
            r#"
            SELECT id, title, description, completed, created_at, updated_at
            FROM tasks
            WHERE user_id = $1 AND ($2::boolean IS NULL OR completed = $2)
            ORDER BY created_at DESC, id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(user_id)
        .bind(pagination.completed)
        .bind(pagination.limit)
        .bind(pagination.offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(tasks)
    }

    pub async fn count_tasks(
        &self,
        user_id: Uuid,
        completed: Option<bool>,
    ) -> Result<i64, AppError> {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM tasks WHERE user_id = $1 AND ($2::boolean IS NULL OR completed = $2)",
        )
        .bind(user_id)
        .bind(completed)
        .fetch_one(&self.pool)
        .await?;

        Ok(total)
    }

    pub async fn get_task_by_id(&self, user_id: Uuid, id: Uuid) -> Result<Option<Task>, AppError> {
        let task = sqlx::query_as::<_, Task>(
            r#"
            SELECT id, title, description, completed, created_at, updated_at
            FROM tasks
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(task)
    }

    pub async fn update_task(
        &self,
        user_id: Uuid,
        id: Uuid,
        request: UpdateTaskRequest,
    ) -> Result<Option<Task>, AppError> {
        let normalized_title = request.title.map(|value| value.trim().to_owned());
        let (description_is_set, description_value) = match request.description {
            PatchField::Missing => (false, None),
            PatchField::Null => (true, None),
            PatchField::Value(value) => (true, Some(value.trim().to_owned())),
        };

        let task = sqlx::query_as::<_, Task>(
            r#"
            UPDATE tasks
            SET title = COALESCE($1, title),
                description = CASE WHEN $2 THEN $3 ELSE description END,
                completed = COALESCE($4, completed),
                updated_at = NOW()
            WHERE id = $5 AND user_id = $6
            RETURNING id, title, description, completed, created_at, updated_at
            "#,
        )
        .bind(normalized_title.as_deref())
        .bind(description_is_set)
        .bind(description_value.as_deref())
        .bind(request.completed)
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(task)
    }

    pub async fn delete_task(&self, user_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query(r#"DELETE FROM tasks WHERE id = $1 AND user_id = $2"#)
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod postgres_integration_tests {
    use super::*;
    use crate::models::{PaginationQuery, UpdateTaskRequest};
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use std::str::FromStr;

    #[tokio::test]
    #[ignore = "requires a separate local PostgreSQL database via NEXORA_TEST_DATABASE_URL"]
    async fn local_postgres_crud_and_partial_updates() {
        let database_url = std::env::var("NEXORA_TEST_DATABASE_URL")
            .expect("set NEXORA_TEST_DATABASE_URL to a disposable local PostgreSQL database");
        let connection_options =
            PgConnectOptions::from_str(&database_url).expect("parse local test database URL");
        assert!(
            matches!(
                connection_options.get_host(),
                "localhost" | "127.0.0.1" | "::1"
            ),
            "integration test is restricted to a loopback PostgreSQL host"
        );
        let test_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("connect to the explicitly configured local test database");
        sqlx::query(
            r#"
            CREATE TEMP TABLE tasks (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL,
                title VARCHAR(200) NOT NULL,
                description TEXT,
                completed BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            ) ON COMMIT PRESERVE ROWS
            "#,
        )
        .execute(&test_pool)
        .await
        .expect("create isolated test table");

        let repository = TaskRepository::new(test_pool.clone());
        let user_id = Uuid::new_v4();
        let created = repository
            .create_task(
                user_id,
                &CreateTaskRequest {
                    title: "  Verify CRUD  ".to_string(),
                    description: Some("initial description".to_string()),
                },
            )
            .await
            .expect("create task");
        assert_eq!(created.title, "Verify CRUD");
        assert_eq!(created.description.as_deref(), Some("initial description"));

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let pending = repository
            .update_task(
                user_id,
                created.id,
                UpdateTaskRequest {
                    title: None,
                    description: PatchField::Missing,
                    completed: Some(false),
                },
            )
            .await
            .expect("update completion without changing omitted fields")
            .expect("task exists");
        assert!(!pending.completed);
        assert_eq!(pending.description.as_deref(), Some("initial description"));
        assert!(pending.updated_at > created.updated_at);

        let cleared = repository
            .update_task(
                user_id,
                created.id,
                UpdateTaskRequest {
                    title: None,
                    description: PatchField::Null,
                    completed: None,
                },
            )
            .await
            .expect("clear description")
            .expect("task exists");
        assert_eq!(cleared.description, None);

        let replaced = repository
            .update_task(
                user_id,
                created.id,
                UpdateTaskRequest {
                    title: None,
                    description: PatchField::Value("replacement".to_string()),
                    completed: None,
                },
            )
            .await
            .expect("replace description")
            .expect("task exists");
        assert_eq!(replaced.description.as_deref(), Some("replacement"));

        let pagination = PaginationQuery {
            page: Some(1),
            limit: Some(1),
            completed: Some(false),
        }
        .validate()
        .expect("valid pagination");
        assert_eq!(
            repository
                .list_tasks(user_id, &pagination)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            repository.count_tasks(user_id, Some(false)).await.unwrap(),
            1
        );
        assert_eq!(
            repository.count_tasks(user_id, Some(true)).await.unwrap(),
            0
        );

        assert!(repository.delete_task(user_id, created.id).await.unwrap());
        assert!(
            repository
                .get_task_by_id(user_id, created.id)
                .await
                .unwrap()
                .is_none()
        );

        test_pool.close().await;
    }
}
