use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Describes the cause of the error with structured details.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(as = ErrorStatusDetailsInfo))]
pub struct ErrorInfo {
    /// The reason of the error (UPPER_SNAKE_CASE).
    /// Example: "API_DISABLED", "STOCKOUT"
    pub reason: String,

    /// The logical grouping to which the "reason" belongs.
    /// Example: "googleapis.com", "pubsub.googleapis.com"
    pub domain: String,

    /// Additional structured details about this error.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl ErrorInfo {
    /// Creates a new ErrorInfo with reason and domain.
    pub fn new(reason: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            domain: domain.into(),
            metadata: HashMap::new(),
        }
    }

    /// Creates a new builder.
    pub fn builder() -> ErrorInfoBuilder {
        ErrorInfoBuilder::default()
    }

    /// Adds a metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Checks if the reason follows UPPER_SNAKE_CASE pattern.
    pub fn is_valid_reason(&self) -> bool {
        if self.reason.len() > 63 || self.reason.is_empty() {
            return false;
        }
        // Pattern: [A-Z][A-Z0-9_]+[A-Z0-9]
        let chars: Vec<char> = self.reason.chars().collect();
        if chars.len() < 3 {
            return false;
        }
        chars[0].is_ascii_uppercase()
            && chars[1..chars.len() - 1]
                .iter()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
            && (chars.last().unwrap().is_ascii_uppercase()
                || chars.last().unwrap().is_ascii_digit())
    }
}

#[derive(Debug, Default)]
pub struct ErrorInfoBuilder {
    reason: String,
    domain: String,
    metadata: HashMap<String, String>,
}

impl ErrorInfoBuilder {
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }

    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> ErrorInfo {
        ErrorInfo {
            reason: self.reason,
            domain: self.domain,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ ErrorInfo Tests ============

    #[test]
    fn test_error_info_new() {
        let info = ErrorInfo::new("API_DISABLED", "googleapis.com");
        assert_eq!(info.reason, "API_DISABLED");
        assert_eq!(info.domain, "googleapis.com");
        assert!(info.metadata.is_empty());
    }

    #[test]
    fn test_error_info_new_with_into() {
        let reason = String::from("STOCKOUT");
        let domain = String::from("inventory.example.com");
        let info = ErrorInfo::new(reason, domain);
        assert_eq!(info.reason, "STOCKOUT");
        assert_eq!(info.domain, "inventory.example.com");
    }

    #[test]
    fn test_error_info_with_metadata() {
        let info = ErrorInfo::new("RATE_LIMITED", "api.example.com")
            .with_metadata("retry_after", "30")
            .with_metadata("limit", "100");

        assert_eq!(info.metadata.get("retry_after"), Some(&"30".to_string()));
        assert_eq!(info.metadata.get("limit"), Some(&"100".to_string()));
    }

    #[test]
    fn test_error_info_with_metadata_overwrite() {
        let info = ErrorInfo::new("ERROR", "test.com")
            .with_metadata("key", "value1")
            .with_metadata("key", "value2");

        assert_eq!(info.metadata.get("key"), Some(&"value2".to_string()));
        assert_eq!(info.metadata.len(), 1);
    }

    #[test]
    fn test_error_info_default() {
        let info = ErrorInfo::default();
        assert!(info.reason.is_empty());
        assert!(info.domain.is_empty());
        assert!(info.metadata.is_empty());
    }

    #[test]
    fn test_error_info_clone() {
        let info = ErrorInfo::new("ERROR", "test.com").with_metadata("key", "value");
        let cloned = info.clone();

        assert_eq!(info, cloned);
        assert_eq!(cloned.reason, "ERROR");
        assert_eq!(cloned.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_error_info_equality() {
        let info1 = ErrorInfo::new("ERROR", "test.com");
        let info2 = ErrorInfo::new("ERROR", "test.com");
        let info3 = ErrorInfo::new("OTHER", "test.com");

        assert_eq!(info1, info2);
        assert_ne!(info1, info3);
    }

    // ============ ErrorInfo Validation Tests ============

    #[test]
    fn test_is_valid_reason_valid() {
        // Valid UPPER_SNAKE_CASE reasons
        assert!(ErrorInfo::new("API_DISABLED", "").is_valid_reason());
        assert!(ErrorInfo::new("NOT_FOUND", "").is_valid_reason());
        assert!(ErrorInfo::new("RATE_LIMITED_429", "").is_valid_reason());
        assert!(ErrorInfo::new("ERROR123", "").is_valid_reason());
        assert!(ErrorInfo::new("A1B", "").is_valid_reason());
    }

    #[test]
    fn test_is_valid_reason_invalid() {
        // Too short
        assert!(!ErrorInfo::new("AB", "").is_valid_reason());
        assert!(!ErrorInfo::new("A", "").is_valid_reason());

        // Empty
        assert!(!ErrorInfo::new("", "").is_valid_reason());

        // Lowercase
        assert!(!ErrorInfo::new("api_disabled", "").is_valid_reason());
        assert!(!ErrorInfo::new("Api_Disabled", "").is_valid_reason());

        // Invalid characters
        assert!(!ErrorInfo::new("API-DISABLED", "").is_valid_reason());
        assert!(!ErrorInfo::new("API DISABLED", "").is_valid_reason());

        // Starts with number
        assert!(!ErrorInfo::new("1API", "").is_valid_reason());

        // Ends with underscore
        assert!(!ErrorInfo::new("API_", "").is_valid_reason());
    }

    #[test]
    fn test_is_valid_reason_too_long() {
        let long_reason = "A".repeat(64);
        assert!(!ErrorInfo::new(long_reason, "").is_valid_reason());

        let max_reason = "A".repeat(63);
        assert!(ErrorInfo::new(max_reason, "").is_valid_reason());
    }

    // ============ ErrorInfo Serde Tests ============

    #[test]
    fn test_error_info_serialize() {
        let info =
            ErrorInfo::new("API_DISABLED", "googleapis.com").with_metadata("service", "pubsub");

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains(r#""reason":"API_DISABLED""#));
        assert!(json.contains(r#""domain":"googleapis.com""#));
        assert!(json.contains(r#""metadata""#));
    }

    #[test]
    fn test_error_info_deserialize() {
        let json = r#"{
            "reason": "NOT_FOUND",
            "domain": "example.com",
            "metadata": {"resource_id": "123"}
        }"#;

        let info: ErrorInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.reason, "NOT_FOUND");
        assert_eq!(info.domain, "example.com");
        assert_eq!(info.metadata.get("resource_id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_error_info_always_serializes_reason_and_domain() {
        let info = ErrorInfo::default();
        let json = serde_json::to_string(&info).unwrap();
        // `reason` and `domain` are required; empty metadata is skipped.
        assert_eq!(json, r#"{"reason":"","domain":""}"#);
    }

    #[test]
    fn test_error_info_skip_empty_metadata() {
        let info = ErrorInfo::new("ERROR", "test.com");
        let json = serde_json::to_string(&info).unwrap();

        // metadata should be skipped when empty
        assert!(!json.contains("metadata"));
    }

    #[test]
    fn test_error_info_roundtrip() {
        let original = ErrorInfo::new("RATE_LIMITED", "api.example.com")
            .with_metadata("retry_after", "30")
            .with_metadata("quota_limit", "1000");

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ErrorInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    // ============ ErrorInfoBuilder Tests ============

    #[test]
    fn test_builder_basic() {
        let info = ErrorInfo::builder()
            .reason("API_DISABLED")
            .domain("googleapis.com")
            .build();

        assert_eq!(info.reason, "API_DISABLED");
        assert_eq!(info.domain, "googleapis.com");
        assert!(info.metadata.is_empty());
    }

    #[test]
    fn test_builder_with_metadata() {
        let info = ErrorInfo::builder()
            .reason("RATE_LIMITED")
            .domain("api.example.com")
            .metadata("retry_after", "60")
            .metadata("quota_remaining", "0")
            .build();

        assert_eq!(info.reason, "RATE_LIMITED");
        assert_eq!(info.domain, "api.example.com");
        assert_eq!(info.metadata.get("retry_after"), Some(&"60".to_string()));
        assert_eq!(info.metadata.get("quota_remaining"), Some(&"0".to_string()));
    }

    #[test]
    fn test_builder_default() {
        let info = ErrorInfo::builder().build();

        assert!(info.reason.is_empty());
        assert!(info.domain.is_empty());
        assert!(info.metadata.is_empty());
    }

    #[test]
    fn test_builder_with_string_types() {
        let reason = String::from("ERROR");
        let domain = String::from("test.com");
        let key = String::from("key");
        let value = String::from("value");

        let info = ErrorInfo::builder()
            .reason(reason)
            .domain(domain)
            .metadata(key, value)
            .build();

        assert_eq!(info.reason, "ERROR");
        assert_eq!(info.domain, "test.com");
        assert_eq!(info.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_builder_method_chaining() {
        // Ensure all methods return Self for chaining
        let info = ErrorInfo::builder()
            .reason("A")
            .reason("B") // Overwrite
            .domain("x")
            .domain("y") // Overwrite
            .metadata("k1", "v1")
            .metadata("k2", "v2")
            .build();

        assert_eq!(info.reason, "B");
        assert_eq!(info.domain, "y");
        assert_eq!(info.metadata.len(), 2);
    }

    #[test]
    fn test_builder_debug() {
        let builder = ErrorInfo::builder().reason("TEST").domain("test.com");

        // Should implement Debug
        let debug_str = format!("{:?}", builder);
        assert!(debug_str.contains("ErrorInfoBuilder"));
    }
}
