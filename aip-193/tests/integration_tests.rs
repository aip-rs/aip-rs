//! Integration tests for aip-193 crate
//!
//! These tests verify the public API and interactions between modules.

#![cfg(feature = "http")]

use aip_193::{Code, ErrorInfo, Status, StatusDetails};
use http::StatusCode;
use std::collections::HashMap;

// ============================================================================
// Full API Integration Tests
// ============================================================================

/// Tests a complete error handling workflow from error creation to JSON response
#[test]
fn test_complete_error_workflow() {
    // 1. Create an error with ErrorInfo
    let error_info = ErrorInfo::builder()
        .reason("USER_NOT_FOUND")
        .domain("users.myapp.com")
        .metadata("user_id", "usr_12345")
        .metadata("lookup_method", "email")
        .build();

    // 2. Wrap it in a Status
    let status = Status {
        code: 404,
        message: "The requested user does not exist".to_string(),
        status: Code::NotFound,
        details: StatusDetails { error_info },
    };

    // 3. Serialize to JSON
    let json = serde_json::to_string_pretty(&status).unwrap();

    // 4. Verify JSON structure
    assert!(json.contains(r#""code": 404"#));
    assert!(json.contains(r#""status": "NOT_FOUND""#));
    assert!(json.contains(r#""message": "The requested user does not exist""#));
    assert!(json.contains(r#""reason": "USER_NOT_FOUND""#));
    assert!(json.contains(r#""domain": "users.myapp.com""#));
    assert!(json.contains(r#""user_id": "usr_12345""#));

    // 5. Deserialize back
    let parsed: Status = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.code, 404);
    assert_eq!(parsed.status, Code::NotFound);
    assert_eq!(parsed.message, "The requested user does not exist");

    assert_eq!(parsed.details.error_info.reason, "USER_NOT_FOUND");
}

/// Tests Code to HTTP StatusCode conversion round-trip
#[test]
fn test_code_http_conversion_roundtrip() {
    let test_cases = [
        (Code::Ok, StatusCode::OK),
        (Code::NotFound, StatusCode::NOT_FOUND),
        (Code::InvalidArgument, StatusCode::BAD_REQUEST),
        (Code::Unauthenticated, StatusCode::UNAUTHORIZED),
        (Code::PermissionDenied, StatusCode::FORBIDDEN),
        (Code::ResourceExhausted, StatusCode::TOO_MANY_REQUESTS),
    ];

    for (grpc_code, expected_http) in test_cases {
        let http_status: StatusCode = grpc_code.into();
        assert_eq!(http_status, expected_http, "Code {:?} should map to HTTP {}", grpc_code, expected_http);
    }
}

/// Tests building complex error scenarios
#[test]
fn test_complex_error_scenario() {
    // Scenario: Rate limiting error with retry information
    let error_info = ErrorInfo::new("RATE_LIMITED", "api.myservice.com")
        .with_metadata("retry_after_seconds", "60")
        .with_metadata("quota_limit", "1000")
        .with_metadata("quota_used", "1000")
        .with_metadata("quota_reset_at", "2024-01-01T00:00:00Z");

    let status = Status {
        code: 429,
        message: "Rate limit exceeded. Please retry after 60 seconds.".to_string(),
        status: Code::ResourceExhausted,
        details: StatusDetails { error_info },
    };

    // Verify HTTP mapping
    let http_code: StatusCode = Code::ResourceExhausted.into();
    assert_eq!(http_code, StatusCode::TOO_MANY_REQUESTS);

    // Verify metadata
    assert_eq!(status.details.error_info.metadata.len(), 4);
    assert_eq!(
        status.details.error_info.metadata.get("retry_after_seconds"),
        Some(&"60".to_string())
    );
}

// ============================================================================
// Serialization Format Tests
// ============================================================================

/// Tests that empty optional fields are omitted in JSON
#[test]
fn test_json_skips_only_empty_metadata() {
    // ErrorInfo with only reason
    let info = ErrorInfo {
        reason: "ERROR".to_string(),
        domain: String::new(),
        metadata: HashMap::new(),
    };

    let json = serde_json::to_string(&info).unwrap();
    // `reason` and `domain` are required, so only `metadata` is skipped.
    assert_eq!(json, r#"{"reason":"ERROR","domain":""}"#);
}

/// Tests JSON deserialization without the optional `metadata`, and that
/// the required `domain` must be present.
#[test]
fn test_json_deserialize_partial() {
    let json = r#"{"reason": "TEST", "domain": ""}"#;
    let info: ErrorInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.reason, "TEST");
    assert!(info.domain.is_empty());
    assert!(info.metadata.is_empty());
    assert!(serde_json::from_str::<ErrorInfo>(r#"{"reason": "TEST"}"#).is_err());
}

/// Tests JSON with nested structures
#[test]
fn test_json_nested_structure() {
    let json = r#"{
        "code": 403,
        "status": "PERMISSION_DENIED",
        "message": "Access denied",
        "details": {
            "error_info": {
                "reason": "PERMISSION_DENIED",
                "domain": "iam.googleapis.com",
                "metadata": {
                    "permission": "storage.objects.get",
                    "resource": "projects/my-project/buckets/my-bucket"
                }
            }
        }
    }"#;

    let status: Status = serde_json::from_str(json).unwrap();
    assert_eq!(status.code, 403);
    assert_eq!(status.status, Code::PermissionDenied);

    assert_eq!(status.details.error_info.reason, "PERMISSION_DENIED");
    assert_eq!(
        status.details.error_info.metadata.get("permission"),
        Some(&"storage.objects.get".to_string())
    );
}

// ============================================================================
// Error Builder Pattern Tests
// ============================================================================

/// Tests ErrorInfo builder with all options
#[test]
fn test_error_info_builder_complete() {
    let info = ErrorInfo::builder()
        .reason("VALIDATION_ERROR")
        .domain("validation.example.com")
        .metadata("field", "email")
        .metadata("constraint", "format")
        .metadata("value", "not-an-email")
        .build();

    assert_eq!(info.reason, "VALIDATION_ERROR");
    assert_eq!(info.domain, "validation.example.com");
    assert_eq!(info.metadata.len(), 3);
}

/// Tests ErrorInfo direct construction vs builder equivalence
#[test]
fn test_builder_vs_direct_construction() {
    let via_builder = ErrorInfo::builder()
        .reason("ERROR")
        .domain("test.com")
        .metadata("key", "value")
        .build();

    let via_new = ErrorInfo::new("ERROR", "test.com")
        .with_metadata("key", "value");

    assert_eq!(via_builder, via_new);
}

// ============================================================================
// Code Enum Tests
// ============================================================================

/// Tests all Code values and their i32 representation
#[test]
fn test_all_code_values() {
    let codes = [
        (Code::Ok, 0),
        (Code::Cancelled, 1),
        (Code::Unknown, 2),
        (Code::InvalidArgument, 3),
        (Code::DeadlineExceeded, 4),
        (Code::NotFound, 5),
        (Code::AlreadyExists, 6),
        (Code::PermissionDenied, 7),
        (Code::ResourceExhausted, 8),
        (Code::FailedPrecondition, 9),
        (Code::Aborted, 10),
        (Code::OutOfRange, 11),
        (Code::Unimplemented, 12),
        (Code::Internal, 13),
        (Code::Unavailable, 14),
        (Code::DataLoss, 15),
        (Code::Unauthenticated, 16),
    ];

    for (code, expected_value) in codes {
        assert_eq!(code as i32, expected_value);
        assert_eq!(i32::from(code), expected_value);
    }
}

/// Tests Code string serialization (strum)
#[test]
fn test_code_string_representation() {
    assert_eq!(Code::Ok.to_string(), "OK");
    assert_eq!(Code::NotFound.to_string(), "NOT_FOUND");
    assert_eq!(Code::InvalidArgument.to_string(), "INVALID_ARGUMENT");
    assert_eq!(Code::PermissionDenied.to_string(), "PERMISSION_DENIED");
    assert_eq!(Code::ResourceExhausted.to_string(), "RESOURCE_EXHAUSTED");
    assert_eq!(Code::FailedPrecondition.to_string(), "FAILED_PRECONDITION");
    assert_eq!(Code::DeadlineExceeded.to_string(), "DEADLINE_EXCEEDED");
}

/// Tests Code parsing from string (strum EnumString)
#[test]
fn test_code_from_string() {
    use std::str::FromStr;

    assert_eq!(Code::from_str("OK").unwrap(), Code::Ok);
    assert_eq!(Code::from_str("NOT_FOUND").unwrap(), Code::NotFound);
    assert_eq!(Code::from_str("PERMISSION_DENIED").unwrap(), Code::PermissionDenied);

    // Invalid string
    assert!(Code::from_str("INVALID_CODE").is_err());
    assert!(Code::from_str("not_found").is_err()); // Case sensitive
}

/// Tests Code AsRef<str> (strum)
#[test]
fn test_code_as_ref() {
    let code = Code::Internal;
    let s: &str = code.as_ref();
    assert_eq!(s, "INTERNAL");
}

// ============================================================================
// Validation Tests
// ============================================================================

/// Tests ErrorInfo reason validation
#[test]
fn test_reason_validation() {
    // Valid reasons
    let valid_reasons = [
        "API_DISABLED",
        "NOT_FOUND",
        "RATE_LIMITED_429",
        "ERROR123",
        "ABC",
    ];

    for reason in valid_reasons {
        let info = ErrorInfo::new(reason, "test.com");
        assert!(info.is_valid_reason(), "Expected '{}' to be valid", reason);
    }

    // Invalid reasons
    let invalid_reasons = [
        "",           // Empty
        "AB",         // Too short
        "api_error",  // Lowercase
        "API-ERROR",  // Contains hyphen
        "API ERROR",  // Contains space
        "123ERROR",   // Starts with number
        "ERROR_",     // Ends with underscore
    ];

    for reason in invalid_reasons {
        let info = ErrorInfo::new(reason, "test.com");
        assert!(!info.is_valid_reason(), "Expected '{}' to be invalid", reason);
    }
}

/// Tests maximum reason length (63 chars per AIP spec)
#[test]
fn test_reason_max_length() {
    // Exactly 63 chars - should be valid
    let max_len = "A".repeat(63);
    let info = ErrorInfo::new(&max_len, "test.com");
    assert!(info.is_valid_reason());

    // 64 chars - should be invalid
    let too_long = "A".repeat(64);
    let info = ErrorInfo::new(&too_long, "test.com");
    assert!(!info.is_valid_reason());
}

// ============================================================================
// HTTP Status Code Mapping Tests
// ============================================================================

/// Tests HTTP to gRPC code mapping for all common HTTP status codes
#[test]
fn test_http_to_grpc_mapping() {
    let mappings = [
        // 2xx -> Ok
        (200, Code::Ok),
        (201, Code::Ok),
        (204, Code::Ok),
        // 4xx
        (400, Code::InvalidArgument),
        (401, Code::Unauthenticated),
        (403, Code::PermissionDenied),
        (404, Code::NotFound),
        (409, Code::AlreadyExists),
        (429, Code::ResourceExhausted),
        (499, Code::Cancelled),
        // 5xx
        (500, Code::Internal),
        (501, Code::Unimplemented),
        (503, Code::Unavailable),
        (504, Code::DeadlineExceeded),
    ];

    for (http_code, expected_grpc) in mappings {
        let http_status = StatusCode::from_u16(http_code).unwrap();
        let grpc_code: Code = http_status.into();
        assert_eq!(grpc_code, expected_grpc,
            "HTTP {} should map to {:?}", http_code, expected_grpc);
    }
}

/// Tests fallback HTTP mappings
#[test]
fn test_http_fallback_mappings() {
    // Other 4xx -> FailedPrecondition
    let http_410 = StatusCode::GONE;
    let code: Code = http_410.into();
    assert_eq!(code, Code::FailedPrecondition);

    // Other 5xx -> Internal
    let http_502 = StatusCode::BAD_GATEWAY;
    let code: Code = http_502.into();
    assert_eq!(code, Code::Internal);

    // 1xx/3xx -> Unknown
    let http_100 = StatusCode::CONTINUE;
    let code: Code = http_100.into();
    assert_eq!(code, Code::Unknown);
}

// ============================================================================
// Real-world Scenario Tests
// ============================================================================

/// Simulates a typical API error response
#[test]
fn test_api_error_response_scenario() {
    // Scenario: User tries to access a resource they don't have permission to
    let error_info = ErrorInfo::builder()
        .reason("PERMISSION_DENIED")
        .domain("iam.mycompany.com")
        .metadata("permission", "documents.read")
        .metadata("resource", "documents/confidential-123")
        .metadata("user", "user@example.com")
        .build();

    let status = Status {
        code: 403,
        message: "You don't have permission to read this document".to_string(),
        status: Code::PermissionDenied,
        details: StatusDetails { error_info },
    };

    // API would return this as JSON
    let response_body = serde_json::to_string(&status).unwrap();

    // Client parses the response
    let parsed: Status = serde_json::from_str(&response_body).unwrap();

    // Client can check the code and show appropriate UI
    assert_eq!(parsed.code, 403);
    assert_eq!(parsed.status, Code::PermissionDenied);

    // Client can log structured error details
    assert_eq!(parsed.details.error_info.reason, "PERMISSION_DENIED");
    assert_eq!(
        parsed.details.error_info.metadata.get("permission"),
        Some(&"documents.read".to_string())
    );
}

/// Simulates validation error with multiple fields
#[test]
fn test_validation_error_scenario() {
    // In a real app, you might have multiple validation errors
    // For AIP-193, we use a single ErrorInfo but can include details in metadata

    let error_info = ErrorInfo::builder()
        .reason("INVALID_ARGUMENT")
        .domain("api.myservice.com")
        .metadata("field", "email")
        .metadata("constraint", "must be a valid email address")
        .metadata("provided_value", "not-an-email")
        .build();

    let status = Status {
        code: 400,
        message: "Validation failed: email must be a valid email address".to_string(),
        status: Code::InvalidArgument,
        details: StatusDetails { error_info },
    };

    let json = serde_json::to_string_pretty(&status).unwrap();

    // Verify the structure
    assert!(json.contains("INVALID_ARGUMENT"));
    assert!(json.contains("email"));
    assert!(json.contains("not-an-email"));
}

// ============================================================================
// Edge Cases and Robustness Tests
// ============================================================================

/// Tests handling of Unicode in all fields
#[test]
fn test_unicode_handling() {
    let error_info = ErrorInfo::new("INVALID_INPUT", "api.例え.日本")
        .with_metadata("field_name", "名前")
        .with_metadata("error", "入力が無効です")
        .with_metadata("emoji", "🚫❌⚠️");

    let status = Status {
        code: 400,
        message: "入力エラー: 名前フィールドが無効です 🚫".to_string(),
        status: Code::InvalidArgument,
        details: StatusDetails { error_info },
    };

    // Should serialize and deserialize correctly
    let json = serde_json::to_string(&status).unwrap();
    let parsed: Status = serde_json::from_str(&json).unwrap();

    assert!(parsed.message.contains("入力エラー"));
    assert!(parsed.message.contains("🚫"));

    assert!(parsed.details.error_info.domain.contains("日本"));
    assert_eq!(
        parsed.details.error_info.metadata.get("emoji"),
        Some(&"🚫❌⚠️".to_string())
    );
}

/// Tests handling of special characters in metadata
#[test]
fn test_special_characters_in_metadata() {
    let error_info = ErrorInfo::new("ERROR", "test.com")
        .with_metadata("json_like", r#"{"key": "value"}"#)
        .with_metadata("newlines", "line1\nline2\nline3")
        .with_metadata("tabs", "col1\tcol2\tcol3")
        .with_metadata("quotes", r#"He said "hello""#)
        .with_metadata("backslash", r"path\to\file");

    let json = serde_json::to_string(&error_info).unwrap();
    let parsed: ErrorInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.metadata.get("json_like"), Some(&r#"{"key": "value"}"#.to_string()));
    assert_eq!(parsed.metadata.get("newlines"), Some(&"line1\nline2\nline3".to_string()));
}

/// Tests empty Status
#[test]
fn test_empty_status() {
    let status = Status {
        code: 200,
        message: String::new(),
        status: Code::Ok,
        details: StatusDetails::default(),
    };

    assert_eq!(status.code, 200);
    assert_eq!(status.status, Code::Ok);
    assert!(status.message.is_empty());
    assert!(status.details.error_info.reason.is_empty());
}

/// Tests Status with empty ErrorInfo
#[test]
fn test_status_with_empty_error_info() {
    let status = Status {
        code: 500,
        message: "Unknown error".to_string(),
        status: Code::Unknown,
        details: StatusDetails {
            error_info: ErrorInfo::default(),
        },
    };

    // ErrorInfo is present but empty - details should still serialize
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("details"));
    assert!(json.contains("error_info"));
}
