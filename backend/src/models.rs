use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Debug, Default, PartialEq, Eq)]
pub enum PatchField<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

fn deserialize_patch_field<'de, D, T>(deserializer: D) -> Result<PatchField<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|value| match value {
        Some(value) => PatchField::Value(value),
        None => PatchField::Null,
    })
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PaginationMetadata {
    pub page: u32,
    pub limit: u32,
    pub total: i64,
    pub total_pages: u64,
}

#[derive(Debug, Serialize)]
pub struct PaginatedTasks {
    pub data: Vec<Task>,
    pub pagination: PaginationMetadata,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pagination {
    pub page: u32,
    pub limit: i64,
    pub offset: i64,
    pub completed: Option<bool>,
}

impl PaginationQuery {
    pub fn validate(self) -> Result<Pagination, AppError> {
        let page = self.page.unwrap_or(1);
        let limit = self.limit.unwrap_or(10);

        if page == 0 {
            return Err(AppError::bad_request(
                "INVALID_PAGE",
                "Page must be greater than zero",
            ));
        }

        if !(1..=100).contains(&limit) {
            return Err(AppError::bad_request(
                "INVALID_LIMIT",
                "Limit must be between 1 and 100",
            ));
        }

        let limit = i64::from(limit);
        let offset = i64::from(page - 1) * limit;
        Ok(Pagination {
            page,
            limit,
            offset,
            completed: self.completed,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
}

impl CreateTaskRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_title(self.title.trim())
    }

    pub fn normalized_title(&self) -> Result<String, AppError> {
        let cleaned = self.title.trim();
        validate_title(cleaned)?;
        Ok(cleaned.to_string())
    }
}

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::validation(
            "EMPTY_TITLE",
            "Title must not be empty",
        ));
    }

    if title.chars().count() > 200 {
        return Err(AppError::validation(
            "TITLE_TOO_LONG",
            "Title must be 200 characters or fewer",
        ));
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_patch_field")]
    pub description: PatchField<String>,
    pub completed: Option<bool>,
}

impl UpdateTaskRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        if let Some(title) = &self.title {
            validate_title(title.trim())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateTaskRequest, PatchField, UpdateTaskRequest};
    use serde_json::json;

    #[test]
    fn create_task_rejects_empty_and_whitespace_only_titles() {
        let empty = CreateTaskRequest {
            title: String::new(),
            description: None,
        };
        let request = CreateTaskRequest {
            title: "   ".to_string(),
            description: Some("Learn Docker".to_string()),
        };

        assert!(empty.validate().is_err());
        assert!(request.validate().is_err());
    }

    #[test]
    fn create_task_accepts_200_ascii_characters_and_rejects_201() {
        assert!(create_request("x".repeat(200)).validate().is_ok());
        assert!(create_request("x".repeat(201)).validate().is_err());
    }

    #[test]
    fn create_task_accepts_200_bengali_characters_and_rejects_201() {
        assert!(create_request("ক".repeat(200)).validate().is_ok());
        assert!(create_request("ক".repeat(201)).validate().is_err());
    }

    #[test]
    fn create_task_accepts_200_characters_with_surrounding_whitespace_after_trimming() {
        assert!(
            create_request(format!("  {}  ", "x".repeat(200)))
                .validate()
                .is_ok()
        );
        assert!(
            create_request(format!("  {}  ", "ক".repeat(200)))
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn update_task_applies_same_unicode_character_limit() {
        assert!(update_request(Some("x".repeat(200))).validate().is_ok());
        assert!(update_request(Some("x".repeat(201))).validate().is_err());
        assert!(update_request(Some("ক".repeat(200))).validate().is_ok());
        assert!(update_request(Some("ক".repeat(201))).validate().is_err());
    }

    #[test]
    fn update_task_rejects_empty_and_whitespace_only_titles() {
        let request = UpdateTaskRequest {
            title: Some(String::new()),
            description: PatchField::Missing,
            completed: None,
        };
        let whitespace_request = update_request(Some("   ".to_string()));

        assert!(request.validate().is_err());
        assert!(whitespace_request.validate().is_err());
    }

    #[test]
    fn update_description_deserializes_missing_as_unchanged() {
        let request: UpdateTaskRequest = serde_json::from_value(json!({})).unwrap();

        assert_eq!(request.description, PatchField::Missing);
        assert_eq!(request.title, None);
        assert_eq!(request.completed, None);
    }

    #[test]
    fn update_description_deserializes_null_as_clear() {
        let request: UpdateTaskRequest =
            serde_json::from_value(json!({"description": null})).unwrap();

        assert_eq!(request.description, PatchField::Null);
    }

    #[test]
    fn update_description_deserializes_string_as_value() {
        let request: UpdateTaskRequest =
            serde_json::from_value(json!({"description": "text"})).unwrap();

        assert_eq!(request.description, PatchField::Value("text".to_string()));
    }

    #[test]
    fn update_task_deserializes_completed_false_and_keeps_omitted_fields_missing() {
        let request: UpdateTaskRequest =
            serde_json::from_value(json!({"completed": false})).unwrap();

        assert_eq!(request.completed, Some(false));
        assert_eq!(request.title, None);
        assert_eq!(request.description, PatchField::Missing);
    }

    #[test]
    fn pagination_uses_defaults_and_rejects_invalid_page_or_limit() {
        let pagination = super::PaginationQuery {
            page: None,
            limit: None,
            completed: None,
        }
        .validate()
        .unwrap();
        assert_eq!(
            pagination,
            super::Pagination {
                page: 1,
                limit: 10,
                offset: 0,
                completed: None,
            }
        );

        assert!(
            super::PaginationQuery {
                page: Some(0),
                limit: None,
                completed: None,
            }
            .validate()
            .is_err()
        );
        assert!(
            super::PaginationQuery {
                page: None,
                limit: Some(101),
                completed: None,
            }
            .validate()
            .is_err()
        );
    }

    fn create_request(title: String) -> CreateTaskRequest {
        CreateTaskRequest {
            title,
            description: None,
        }
    }

    fn update_request(title: Option<String>) -> UpdateTaskRequest {
        UpdateTaskRequest {
            title,
            description: PatchField::Missing,
            completed: None,
        }
    }
}
