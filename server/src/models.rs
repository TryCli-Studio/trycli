use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProjectSummary {
    pub slug: String,
    pub image_tag: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i64,
    pub login: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
pub struct PublishRequest {
    pub container_id: String,
    pub slug: String,
    pub markdown: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // 1. Test User Serialization/Deserialization
    #[test]
    fn test_user_serde() {
        let user_json = json!({
            "id": 123456,
            "login": "octocat",
            "avatar_url": "https://github.com/images/error/octocat_happy.gif"
        });

        // Test Deserialization (JSON -> Struct)
        let user: User = serde_json::from_value(user_json.clone()).expect("Failed to deserialize User");
        assert_eq!(user.id, 123456);
        assert_eq!(user.login, "octocat");
        assert_eq!(user.avatar_url, "https://github.com/images/error/octocat_happy.gif");

        // Test Serialization (Struct -> JSON)
        let serialized = serde_json::to_value(&user).expect("Failed to serialize User");
        assert_eq!(serialized, user_json);
    }

    // 2. Test ProjectSummary Serde
    #[test]
    fn test_project_summary_serde() {
        let summary = ProjectSummary {
            slug: "my-awesome-tool".to_string(),
            image_tag: "trycli/my-awesome-tool:latest".to_string(),
        };

        let json_output = serde_json::to_value(&summary).expect("Failed to serialize ProjectSummary");
        
        assert_eq!(json_output["slug"], "my-awesome-tool");
        assert_eq!(json_output["image_tag"], "trycli/my-awesome-tool:latest");
    }

    // 3. Test PublishRequest Deserialization
    #[test]
    fn test_publish_request_deserialization() {
        let payload = json!({
            "container_id": "a1b2c3d4e5",
            "slug": "rust-cli-demo",
            "markdown": "# Hello World\n\nRun `cargo run` to start."
        });

        let req: PublishRequest = serde_json::from_value(payload).expect("Failed to deserialize PublishRequest");

        assert_eq!(req.container_id, "a1b2c3d4e5");
        assert_eq!(req.slug, "rust-cli-demo");
        assert!(req.markdown.contains("# Hello World"));
    }

    // 4. Test Missing Field Validation
    #[test]
    fn test_publish_request_missing_fields() {
        let bad_payload = json!({
            "container_id": "123",
            // "slug" is missing
            "markdown": "test"
        });

        let result: Result<PublishRequest, _> = serde_json::from_value(bad_payload);
        assert!(result.is_err(), "Should have failed due to missing 'slug' field");
    }
}