//! Integration tests for aip-193-derive crate
//!
//! These tests verify that the `#[derive(IntoStatus)]` macro generates
//! correct implementations of the `IntoStatus` trait and optionally
//! the `IntoResponse` trait for axum integration.

#![allow(dead_code)]
#![allow(unused_variables)]

use aip_193::{Code, IntoStatus, Status};
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

// ============================================================================
// Basic Derive Tests
// ============================================================================

/// Simple error enum with unit variants
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = "test.example.com", use_display = false)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum SimpleError {
    #[status(code = NotFound)]
    NotFound,

    #[status(code = InvalidArgument)]
    InvalidInput,

    #[status(code = Internal)]
    InternalError,
}

#[test]
fn test_simple_error_code() {
    assert_eq!(SimpleError::NotFound.code(), Code::NotFound);
    assert_eq!(SimpleError::InvalidInput.code(), Code::InvalidArgument);
    assert_eq!(SimpleError::InternalError.code(), Code::Internal);
}

#[test]
fn test_simple_error_reason() {
    assert_eq!(SimpleError::NotFound.reason(), "NOT_FOUND");
    assert_eq!(SimpleError::InvalidInput.reason(), "INVALID_INPUT");
    assert_eq!(SimpleError::InternalError.reason(), "INTERNAL_ERROR");
}

#[test]
fn test_simple_error_domain() {
    assert_eq!(SimpleError::NotFound.domain(), "test.example.com");
    assert_eq!(SimpleError::InvalidInput.domain(), "test.example.com");
    assert_eq!(SimpleError::InternalError.domain(), "test.example.com");
}

#[test]
fn test_simple_error_default_message() {
    // Default message is the variant name
    assert_eq!(SimpleError::NotFound.message(), "NotFound");
    assert_eq!(SimpleError::InvalidInput.message(), "InvalidInput");
    assert_eq!(SimpleError::InternalError.message(), "InternalError");
}

#[test]
fn test_simple_error_metadata() {
    // Unit variants have empty metadata
    assert!(SimpleError::NotFound.metadata().is_empty());
    assert!(SimpleError::InvalidInput.metadata().is_empty());
}

// ============================================================================
// Custom Message Tests
// ============================================================================

/// Error enum with custom messages
#[derive(Debug, Clone, IntoStatus, AsRefStr)]
#[status(domain = "api.myservice.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum ErrorWithMessage {
    #[status(code = NotFound, message = "The requested resource was not found")]
    ResourceNotFound,

    #[status(code = InvalidArgument, message = "Invalid request parameters")]
    BadRequest,

    #[status(code = PermissionDenied, message = "You don't have permission to access this resource")]
    Forbidden,
}

#[test]
fn test_custom_message() {
    assert_eq!(
        ErrorWithMessage::ResourceNotFound.message(),
        "The requested resource was not found"
    );
    assert_eq!(
        ErrorWithMessage::BadRequest.message(),
        "Invalid request parameters"
    );
    assert_eq!(
        ErrorWithMessage::Forbidden.message(),
        "You don't have permission to access this resource"
    );
}

// ============================================================================
// Struct Variant with Metadata Tests
// ============================================================================

/// Error enum with struct variants and metadata
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = "users.myapp.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum UserError {
    #[status(code = NotFound, message = "User not found")]
    UserNotFound {
        #[status(metadata)]
        user_id: String,
    },

    #[status(code = InvalidArgument, message = "Invalid email format")]
    InvalidEmail {
        #[status(metadata)]
        email: String,
        #[status(metadata)]
        reason: String,
    },

    #[status(code = AlreadyExists, message = "User already exists")]
    UserAlreadyExists {
        #[status(metadata)]
        email: String,
    },

    #[status(code = PermissionDenied)]
    AccessDenied {
        #[status(metadata)]
        user_id: String,
        #[status(metadata)]
        resource: String,
        #[status(metadata)]
        action: String,
    },
}

#[test]
fn test_struct_variant_code() {
    let err = UserError::UserNotFound {
        user_id: "123".to_string(),
    };
    assert_eq!(err.code(), Code::NotFound);

    let err = UserError::InvalidEmail {
        email: "bad".to_string(),
        reason: "missing @".to_string(),
    };
    assert_eq!(err.code(), Code::InvalidArgument);
}

#[test]
fn test_struct_variant_metadata() {
    let err = UserError::UserNotFound {
        user_id: "usr_123".to_string(),
    };
    let metadata = err.metadata();
    assert_eq!(metadata.get("user_id"), Some(&"usr_123".to_string()));
    assert_eq!(metadata.len(), 1);
}

#[test]
fn test_struct_variant_multiple_metadata() {
    let err = UserError::InvalidEmail {
        email: "invalid-email".to_string(),
        reason: "missing domain".to_string(),
    };
    let metadata = err.metadata();
    assert_eq!(metadata.get("email"), Some(&"invalid-email".to_string()));
    assert_eq!(metadata.get("reason"), Some(&"missing domain".to_string()));
    assert_eq!(metadata.len(), 2);
}

#[test]
fn test_struct_variant_many_metadata_fields() {
    let err = UserError::AccessDenied {
        user_id: "user_456".to_string(),
        resource: "documents/secret".to_string(),
        action: "read".to_string(),
    };
    let metadata = err.metadata();
    assert_eq!(metadata.get("user_id"), Some(&"user_456".to_string()));
    assert_eq!(
        metadata.get("resource"),
        Some(&"documents/secret".to_string())
    );
    assert_eq!(metadata.get("action"), Some(&"read".to_string()));
    assert_eq!(metadata.len(), 3);
}

// ============================================================================
// Custom Metadata Key Tests
// ============================================================================

/// Error enum with custom metadata key names
#[derive(Debug, Clone, IntoStatus, AsRefStr)]
#[status(domain = "api.example.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum ErrorWithCustomKeys {
    #[status(code = ResourceExhausted, message = "Rate limit exceeded")]
    RateLimited {
        #[status(metadata, metadata_key = "retry_after_seconds")]
        retry_after: u32,

        #[status(metadata, metadata_key = "quota_limit")]
        limit: u32,

        #[status(metadata, metadata_key = "quota_used")]
        used: u32,
    },
}

#[test]
fn test_custom_metadata_keys() {
    let err = ErrorWithCustomKeys::RateLimited {
        retry_after: 60,
        limit: 1000,
        used: 1000,
    };
    let metadata = err.metadata();

    // Keys should use custom names
    assert_eq!(metadata.get("retry_after_seconds"), Some(&"60".to_string()));
    assert_eq!(metadata.get("quota_limit"), Some(&"1000".to_string()));
    assert_eq!(metadata.get("quota_used"), Some(&"1000".to_string()));

    // Original field names should not be present
    assert!(metadata.get("retry_after").is_none());
    assert!(metadata.get("limit").is_none());
    assert!(metadata.get("used").is_none());
}

// ============================================================================
// Mixed Variant Styles Tests
// ============================================================================

/// Error enum mixing unit and struct variants
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = "mixed.example.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum MixedError {
    #[status(code = Unknown)]
    Unknown,

    #[status(code = NotFound)]
    ItemNotFound {
        #[status(metadata)]
        item_id: String,
    },

    #[status(code = Internal, message = "Database connection failed")]
    DatabaseError,

    #[status(code = Unavailable)]
    ServiceDown {
        #[status(metadata)]
        service_name: String,
        #[status(metadata, metadata_key = "retry_in_ms")]
        retry_ms: u64,
    },
}

#[test]
fn test_mixed_variants() {
    // Unit variant
    let err = MixedError::Unknown;
    assert_eq!(err.code(), Code::Unknown);
    assert!(err.metadata().is_empty());

    // Struct variant with metadata
    let err = MixedError::ItemNotFound {
        item_id: "item_789".to_string(),
    };
    assert_eq!(err.code(), Code::NotFound);
    assert_eq!(err.metadata().get("item_id"), Some(&"item_789".to_string()));

    // Unit variant with custom message
    let err = MixedError::DatabaseError;
    assert_eq!(err.code(), Code::Internal);
    assert_eq!(err.message(), "Database connection failed");

    // Struct variant with custom key
    let err = MixedError::ServiceDown {
        service_name: "payment-service".to_string(),
        retry_ms: 5000,
    };
    assert_eq!(err.code(), Code::Unavailable);
    let metadata = err.metadata();
    assert_eq!(
        metadata.get("service_name"),
        Some(&"payment-service".to_string())
    );
    assert_eq!(metadata.get("retry_in_ms"), Some(&"5000".to_string()));
}

// ============================================================================
// Conversion to Status Tests
// ============================================================================

#[test]
fn test_into_status_conversion() {
    let err = UserError::UserNotFound {
        user_id: "usr_abc".to_string(),
    };

    let status: Status = err.into();

    assert_eq!(status.code, 404);
    assert_eq!(status.status, Code::NotFound);
    assert_eq!(status.message, "User not found");

    let error_info = &status.details.error_info;
    assert_eq!(error_info.reason, "USER_NOT_FOUND");
    assert_eq!(error_info.domain, "users.myapp.com");
    assert_eq!(
        error_info.metadata.get("user_id"),
        Some(&"usr_abc".to_string())
    );
}

#[test]
fn test_status_from_conversion() {
    let err = SimpleError::InternalError;
    let status = Status::from(err);

    assert_eq!(status.code, 500);
    assert_eq!(status.status, Code::Internal);
    assert_eq!(status.message, "InternalError");

    let error_info = &status.details.error_info;
    assert_eq!(error_info.reason, "INTERNAL_ERROR");
    assert_eq!(error_info.domain, "test.example.com");
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_status_json_serialization() {
    let err = UserError::InvalidEmail {
        email: "not-an-email".to_string(),
        reason: "missing @ symbol".to_string(),
    };

    let status: Status = err.into();
    let json = serde_json::to_string(&status).unwrap();

    assert!(json.contains(r#""code":400"#));
    assert!(json.contains(r#""status":"INVALID_ARGUMENT""#));
    assert!(json.contains(r#""message":"Invalid email format""#));
    assert!(json.contains(r#""reason":"INVALID_EMAIL""#));
    assert!(json.contains(r#""domain":"users.myapp.com""#));
    assert!(json.contains(r#""email":"not-an-email""#));
    assert!(json.contains(r#""reason":"missing @ symbol""#));
}

#[test]
fn test_status_json_roundtrip() {
    let err = ErrorWithCustomKeys::RateLimited {
        retry_after: 30,
        limit: 100,
        used: 100,
    };

    let status: Status = err.into();
    let json = serde_json::to_string(&status).unwrap();
    let parsed: Status = serde_json::from_str(&json).unwrap();

    assert_eq!(status, parsed);
}

// ============================================================================
// All Code Types Tests
// ============================================================================

/// Test all possible Code variants
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = "codes.test.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum AllCodesError {
    #[status(code = Ok)]
    Success,
    #[status(code = Cancelled)]
    Cancelled,
    #[status(code = Unknown)]
    Unknown,
    #[status(code = InvalidArgument)]
    InvalidArgument,
    #[status(code = DeadlineExceeded)]
    DeadlineExceeded,
    #[status(code = NotFound)]
    NotFound,
    #[status(code = AlreadyExists)]
    AlreadyExists,
    #[status(code = PermissionDenied)]
    PermissionDenied,
    #[status(code = ResourceExhausted)]
    ResourceExhausted,
    #[status(code = FailedPrecondition)]
    FailedPrecondition,
    #[status(code = Aborted)]
    Aborted,
    #[status(code = OutOfRange)]
    OutOfRange,
    #[status(code = Unimplemented)]
    Unimplemented,
    #[status(code = Internal)]
    Internal,
    #[status(code = Unavailable)]
    Unavailable,
    #[status(code = DataLoss)]
    DataLoss,
    #[status(code = Unauthenticated)]
    Unauthenticated,
}

#[test]
fn test_all_code_mappings() {
    let test_cases = [
        (AllCodesError::Success, Code::Ok),
        (AllCodesError::Cancelled, Code::Cancelled),
        (AllCodesError::Unknown, Code::Unknown),
        (AllCodesError::InvalidArgument, Code::InvalidArgument),
        (AllCodesError::DeadlineExceeded, Code::DeadlineExceeded),
        (AllCodesError::NotFound, Code::NotFound),
        (AllCodesError::AlreadyExists, Code::AlreadyExists),
        (AllCodesError::PermissionDenied, Code::PermissionDenied),
        (AllCodesError::ResourceExhausted, Code::ResourceExhausted),
        (AllCodesError::FailedPrecondition, Code::FailedPrecondition),
        (AllCodesError::Aborted, Code::Aborted),
        (AllCodesError::OutOfRange, Code::OutOfRange),
        (AllCodesError::Unimplemented, Code::Unimplemented),
        (AllCodesError::Internal, Code::Internal),
        (AllCodesError::Unavailable, Code::Unavailable),
        (AllCodesError::DataLoss, Code::DataLoss),
        (AllCodesError::Unauthenticated, Code::Unauthenticated),
    ];

    for (err, expected_code) in test_cases {
        assert_eq!(err.code(), expected_code, "Failed for {:?}", err);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test with Unicode in messages and metadata
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = "i18n.example.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum I18nError {
    #[status(code = InvalidArgument, message = "入力が無効です")]
    InvalidInput {
        #[status(metadata)]
        field: String,
    },
}

#[test]
fn test_unicode_support() {
    let err = I18nError::InvalidInput {
        field: "名前".to_string(),
    };

    assert_eq!(err.message(), "入力が無効です");
    assert_eq!(err.metadata().get("field"), Some(&"名前".to_string()));

    // Should serialize correctly
    let status: Status = err.into();
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("入力が無効です"));
    assert!(json.contains("名前"));
}

/// Test with fields that have special Display implementations
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = "numeric.example.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum NumericError {
    #[status(code = OutOfRange)]
    ValueOutOfRange {
        #[status(metadata)]
        min: i64,
        #[status(metadata)]
        max: i64,
        #[status(metadata)]
        actual: i64,
    },
}

#[test]
fn test_numeric_metadata() {
    let err = NumericError::ValueOutOfRange {
        min: -100,
        max: 100,
        actual: 150,
    };

    let metadata = err.metadata();
    assert_eq!(metadata.get("min"), Some(&"-100".to_string()));
    assert_eq!(metadata.get("max"), Some(&"100".to_string()));
    assert_eq!(metadata.get("actual"), Some(&"150".to_string()));
}

// ============================================================================
// Real-World Scenarios
// ============================================================================

/// Realistic API error enum
#[derive(Debug, Clone, IntoStatus, AsRefStr, Serialize, Deserialize)]
#[status(domain = "api.mycompany.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiError {
    // Authentication errors
    #[status(code = Unauthenticated, message = "Authentication required")]
    Unauthenticated,

    #[status(code = Unauthenticated, message = "Invalid or expired token")]
    InvalidToken {
        #[status(metadata, metadata_key = "token_type")]
        token_type: String,
    },

    // Authorization errors
    #[status(code = PermissionDenied, message = "Insufficient permissions")]
    InsufficientPermissions {
        #[status(metadata)]
        required_permission: String,
        #[status(metadata)]
        resource: String,
    },

    // Validation errors
    #[status(code = InvalidArgument, message = "Validation failed")]
    ValidationFailed {
        #[status(metadata)]
        field: String,
        #[status(metadata)]
        constraint: String,
    },

    // Resource errors
    #[status(code = NotFound, message = "Resource not found")]
    NotFound {
        #[status(metadata, metadata_key = "resource_type")]
        resource_type: String,
        #[status(metadata, metadata_key = "resource_id")]
        id: String,
    },

    #[status(code = AlreadyExists, message = "Resource already exists")]
    AlreadyExists {
        #[status(metadata, metadata_key = "resource_type")]
        resource_type: String,
        #[status(metadata)]
        identifier: String,
    },

    // Rate limiting
    #[status(code = ResourceExhausted, message = "Rate limit exceeded")]
    RateLimited {
        #[status(metadata, metadata_key = "retry_after_seconds")]
        retry_after: u32,
    },

    // Server errors
    #[status(code = Internal, message = "Internal server error")]
    InternalError,

    #[status(code = Unavailable, message = "Service temporarily unavailable")]
    ServiceUnavailable {
        #[status(metadata, metadata_key = "expected_recovery_time")]
        recovery_time: String,
    },
}

#[test]
fn test_realistic_api_errors() {
    // Authentication error
    let err = ApiError::InvalidToken {
        token_type: "Bearer".to_string(),
    };
    let status: Status = err.into();
    assert_eq!(status.code, 401);
    assert_eq!(status.status, Code::Unauthenticated);
    let info = status.details.error_info;
    assert_eq!(info.metadata.get("token_type"), Some(&"Bearer".to_string()));

    // Validation error
    let err = ApiError::ValidationFailed {
        field: "email".to_string(),
        constraint: "must be a valid email address".to_string(),
    };
    let status: Status = err.into();
    assert_eq!(status.code, 400);
    assert_eq!(status.status, Code::InvalidArgument);
    let info = status.details.error_info;
    assert_eq!(info.metadata.get("field"), Some(&"email".to_string()));
    assert_eq!(
        info.metadata.get("constraint"),
        Some(&"must be a valid email address".to_string())
    );

    // Not found with custom keys
    let err = ApiError::NotFound {
        resource_type: "User".to_string(),
        id: "usr_12345".to_string(),
    };
    let status: Status = err.into();
    assert_eq!(status.code, 404);
    assert_eq!(status.status, Code::NotFound);
    let info = status.details.error_info;
    assert_eq!(
        info.metadata.get("resource_type"),
        Some(&"User".to_string())
    );
    assert_eq!(
        info.metadata.get("resource_id"),
        Some(&"usr_12345".to_string())
    );

    // Rate limiting
    let err = ApiError::RateLimited { retry_after: 30 };
    let status: Status = err.into();
    assert_eq!(status.code, 429);
    assert_eq!(status.status, Code::ResourceExhausted);
    let info = status.details.error_info;
    assert_eq!(
        info.metadata.get("retry_after_seconds"),
        Some(&"30".to_string())
    );
}

#[test]
fn test_api_error_json_response() {
    let err = ApiError::InsufficientPermissions {
        required_permission: "documents.write".to_string(),
        resource: "projects/123/documents/456".to_string(),
    };

    let status: Status = err.into();
    let json = serde_json::to_string_pretty(&status).unwrap();

    // This is what an API would return
    assert!(json.contains(r#""code": 403"#));
    assert!(json.contains(r#""status": "PERMISSION_DENIED""#));
    assert!(json.contains(r#""message": "Insufficient permissions""#));
    assert!(json.contains(r#""reason": "INSUFFICIENT_PERMISSIONS""#));
    assert!(json.contains(r#""domain": "api.mycompany.com""#));
    assert!(json.contains(r#""required_permission": "documents.write""#));
    assert!(json.contains(r#""resource": "projects/123/documents/456""#));
}

// ============================================================================
// IntoResponse Integration Tests
// ============================================================================

mod into_response_tests {
    use super::*;
    use axum::response::IntoResponse;
    use axum::http::StatusCode as HttpStatus;

    /// Simple error enum with IntoResponse enabled
    #[derive(Debug, Clone, IntoStatus, AsRefStr)]
    #[status(domain = "response.example.com", into_response = true)]
    #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
    enum ResponseError {
        #[status(code = NotFound, message = "Resource not found")]
        NotFound,

        #[status(code = InvalidArgument, message = "Invalid input")]
        BadRequest,

        #[status(code = Internal, message = "Internal error")]
        InternalError,
    }

    /// Error enum with metadata and IntoResponse
    #[derive(Debug, Clone, IntoStatus, AsRefStr)]
    #[status(domain = "api.response.com", into_response = true)]
    #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
    enum ApiResponseError {
        #[status(code = NotFound, message = "User not found")]
        UserNotFound {
            #[status(metadata)]
            user_id: String,
        },

        #[status(code = InvalidArgument, message = "Validation failed")]
        ValidationError {
            #[status(metadata)]
            field: String,
            #[status(metadata)]
            reason: String,
        },

        #[status(code = Unauthenticated, message = "Authentication required")]
        Unauthorized,

        #[status(code = PermissionDenied, message = "Access denied")]
        Forbidden {
            #[status(metadata)]
            resource: String,
        },

        #[status(code = ResourceExhausted, message = "Rate limit exceeded")]
        RateLimited {
            #[status(metadata, metadata_key = "retry_after_seconds")]
            retry_after: u32,
        },
    }

    #[test]
    fn test_into_response_trait_exists() {
        // This test verifies that IntoResponse is implemented
        fn assert_into_response<T: IntoResponse>() {}
        assert_into_response::<ResponseError>();
        assert_into_response::<ApiResponseError>();
    }

    #[test]
    fn test_into_response_basic() {
        let err = ResponseError::NotFound;
        let response = err.into_response();

        // Response should be created successfully with correct HTTP status
        assert_eq!(response.status(), HttpStatus::NOT_FOUND);
    }

    #[test]
    fn test_into_response_with_metadata() {
        let err = ApiResponseError::UserNotFound {
            user_id: "user_123".to_string(),
        };
        let response = err.into_response();

        // Response should be created successfully with correct HTTP status
        assert_eq!(response.status(), HttpStatus::NOT_FOUND);
    }

    #[test]
    fn test_into_response_preserves_status_data() {
        let err = ApiResponseError::ValidationError {
            field: "email".to_string(),
            reason: "invalid format".to_string(),
        };

        // Convert to Status first to verify data
        let status: Status = err.clone().into();
        assert_eq!(status.code, 400);
        assert_eq!(status.status, Code::InvalidArgument);
        assert_eq!(status.message, "Validation failed");

        let error_info = &status.details.error_info;
        assert_eq!(error_info.reason, "VALIDATION_ERROR");
        assert_eq!(error_info.domain, "api.response.com");
        assert_eq!(error_info.metadata.get("field"), Some(&"email".to_string()));
        assert_eq!(error_info.metadata.get("reason"), Some(&"invalid format".to_string()));

        // Now verify IntoResponse works with correct HTTP status
        let response = err.into_response();
        assert_eq!(response.status(), HttpStatus::BAD_REQUEST);
    }

    #[test]
    fn test_into_response_all_error_codes() {
        let test_cases = vec![
            (ApiResponseError::UserNotFound {
                user_id: "123".to_string(),
            }, HttpStatus::NOT_FOUND),
            (ApiResponseError::ValidationError {
                field: "name".to_string(),
                reason: "too short".to_string(),
            }, HttpStatus::BAD_REQUEST),
            (ApiResponseError::Unauthorized, HttpStatus::UNAUTHORIZED),
            (ApiResponseError::Forbidden {
                resource: "admin".to_string(),
            }, HttpStatus::FORBIDDEN),
            (ApiResponseError::RateLimited { retry_after: 60 }, HttpStatus::TOO_MANY_REQUESTS),
        ];

        for (err, expected_status) in test_cases {
            let response = err.into_response();
            // All responses should be created successfully with correct HTTP status
            assert_eq!(response.status(), expected_status);
        }
    }

    #[test]
    fn test_into_response_can_be_returned_from_handler() {
        // Simulate a handler that returns Result<T, E> where E: IntoResponse
        fn mock_handler(should_fail: bool) -> Result<String, ApiResponseError> {
            if should_fail {
                Err(ApiResponseError::Unauthorized)
            } else {
                Ok("Success".to_string())
            }
        }

        // Error case
        let result = mock_handler(true);
        assert!(result.is_err());
        if let Err(e) = result {
            let _response = e.into_response();
            // Response created successfully
        }

        // Success case
        let result = mock_handler(false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_into_response_with_custom_metadata_keys() {
        let err = ApiResponseError::RateLimited { retry_after: 120 };

        // Verify the status conversion preserves custom keys
        let status: Status = err.clone().into();
        let error_info = &status.details.error_info;
        assert_eq!(
            error_info.metadata.get("retry_after_seconds"),
            Some(&"120".to_string())
        );

        // Verify IntoResponse works with correct HTTP status
        let response = err.into_response();
        assert_eq!(response.status(), HttpStatus::TOO_MANY_REQUESTS);
    }

    /// Test without IntoResponse to ensure it's not generated by default
    #[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
    #[status(domain = "noresponse.example.com")]
    #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
    enum NoResponseError {
        #[status(code = NotFound)]
        NotFound,
    }

    #[test]
    fn test_into_response_not_generated_by_default() {
        // This should compile, showing IntoStatus is implemented
        let err = NoResponseError::NotFound;
        let _status: Status = err.into();

        // The following would NOT compile (uncomment to verify):
        // let _response = NoResponseError::NotFound.into_response();
        // error[E0599]: no method named `into_response` found
    }

    /// Complex real-world error with IntoResponse
    #[derive(Debug, Clone, IntoStatus, AsRefStr, Serialize, Deserialize)]
    #[status(domain = "production.api.com", into_response = true)]
    #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
    enum ProductionError {
        #[status(code = InvalidArgument, message = "Invalid request body")]
        InvalidRequestBody {
            #[status(metadata)]
            field: String,
            #[status(metadata)]
            expected_type: String,
        },

        #[status(code = NotFound, message = "Endpoint not found")]
        EndpointNotFound {
            #[status(metadata)]
            path: String,
            #[status(metadata)]
            method: String,
        },

        #[status(code = Unauthenticated, message = "Token expired")]
        TokenExpired {
            #[status(metadata, metadata_key = "expired_at")]
            expiration_time: String,
        },

        #[status(code = PermissionDenied, message = "Insufficient role")]
        InsufficientRole {
            #[status(metadata, metadata_key = "required_role")]
            required: String,
            #[status(metadata, metadata_key = "current_role")]
            current: String,
        },

        #[status(code = ResourceExhausted, message = "Quota exceeded")]
        QuotaExceeded {
            #[status(metadata)]
            quota_type: String,
            #[status(metadata)]
            limit: u64,
            #[status(metadata)]
            current: u64,
        },

        #[status(code = Internal, message = "Database connection failed")]
        DatabaseError,

        #[status(code = Unavailable, message = "Service maintenance")]
        Maintenance {
            #[status(metadata, metadata_key = "estimated_completion")]
            completion_time: String,
        },
    }

    #[test]
    fn test_production_error_into_response() {
        let test_cases = vec![
            (ProductionError::InvalidRequestBody {
                field: "age".to_string(),
                expected_type: "integer".to_string(),
            }, HttpStatus::BAD_REQUEST),
            (ProductionError::EndpointNotFound {
                path: "/api/v2/users".to_string(),
                method: "POST".to_string(),
            }, HttpStatus::NOT_FOUND),
            (ProductionError::TokenExpired {
                expiration_time: "2024-01-01T00:00:00Z".to_string(),
            }, HttpStatus::UNAUTHORIZED),
            (ProductionError::InsufficientRole {
                required: "admin".to_string(),
                current: "user".to_string(),
            }, HttpStatus::FORBIDDEN),
            (ProductionError::QuotaExceeded {
                quota_type: "api_calls".to_string(),
                limit: 1000,
                current: 1001,
            }, HttpStatus::TOO_MANY_REQUESTS),
            (ProductionError::DatabaseError, HttpStatus::INTERNAL_SERVER_ERROR),
            (ProductionError::Maintenance {
                completion_time: "2024-01-01T02:00:00Z".to_string(),
            }, HttpStatus::SERVICE_UNAVAILABLE),
        ];

        for (err, expected_status) in test_cases {
            // Convert to Status to verify all data is preserved
            let status: Status = err.clone().into();
            assert!(!status.details.error_info.reason.is_empty());

            // Convert to Response with correct HTTP status
            let response = err.into_response();
            assert_eq!(response.status(), expected_status);
        }
    }

    #[test]
    fn test_production_error_response_chain() {
        // Simulate error propagation in a handler chain
        fn database_operation() -> Result<String, ProductionError> {
            Err(ProductionError::DatabaseError)
        }

        fn api_handler() -> Result<String, ProductionError> {
            database_operation()?;
            Ok("Success".to_string())
        }

        match api_handler() {
            Ok(_) => panic!("Expected error"),
            Err(e) => {
                // Verify error can be converted to response with correct HTTP status
                let response = e.into_response();
                assert_eq!(response.status(), HttpStatus::INTERNAL_SERVER_ERROR);
            }
        }
    }
}

// ============================================================================
// Domain Function Tests
// ============================================================================

/// Helper function for static domain
fn get_static_domain() -> &'static str {
    "function.domain.com"
}

/// Helper function for dynamic domain based on environment
fn get_dynamic_domain() -> &'static str {
    if cfg!(test) {
        "test.domain.com"
    } else {
        "prod.domain.com"
    }
}

/// Error enum with function-based domain
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = get_static_domain)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum FunctionDomainError {
    #[status(code = NotFound, message = "Resource not found")]
    NotFound,

    #[status(code = InvalidArgument, message = "Invalid input")]
    InvalidInput {
        #[status(metadata)]
        field: String,
    },

    #[status(code = Internal)]
    InternalError,
}

#[test]
fn test_function_domain_basic() {
    assert_eq!(FunctionDomainError::NotFound.domain(), "function.domain.com");
    assert_eq!(FunctionDomainError::InternalError.domain(), "function.domain.com");
}

#[test]
fn test_function_domain_with_metadata() {
    let err = FunctionDomainError::InvalidInput {
        field: "email".to_string(),
    };

    assert_eq!(err.domain(), "function.domain.com");
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.metadata().get("field"), Some(&"email".to_string()));
}

#[test]
fn test_function_domain_in_status() {
    let err = FunctionDomainError::NotFound;
    let status: Status = err.into();

    assert_eq!(status.code, 404);
    assert_eq!(status.status, Code::NotFound);
    let error_info = &status.details.error_info;
    assert_eq!(error_info.domain, "function.domain.com");
}

/// Error enum with dynamic function-based domain
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = get_dynamic_domain)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum DynamicDomainError {
    #[status(code = NotFound)]
    NotFound,

    #[status(code = InvalidArgument)]
    InvalidArgument {
        #[status(metadata)]
        reason: String,
    },
}

#[test]
fn test_dynamic_function_domain() {
    // In test configuration, should return test domain
    assert_eq!(DynamicDomainError::NotFound.domain(), "test.domain.com");

    let err = DynamicDomainError::InvalidArgument {
        reason: "bad format".to_string(),
    };
    assert_eq!(err.domain(), "test.domain.com");
}

#[test]
fn test_dynamic_domain_in_status() {
    let err = DynamicDomainError::InvalidArgument {
        reason: "validation failed".to_string(),
    };
    let status: Status = err.into();

    let error_info = &status.details.error_info;
    assert_eq!(error_info.domain, "test.domain.com");
    assert_eq!(error_info.metadata.get("reason"), Some(&"validation failed".to_string()));
}

/// Error enum with function domain and IntoResponse
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = get_static_domain, into_response = true)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum FunctionDomainWithResponse {
    #[status(code = NotFound, message = "Not found")]
    NotFound,

    #[status(code = Unauthenticated, message = "Authentication required")]
    Unauthorized {
        #[status(metadata)]
        token_type: String,
    },
}

#[test]
fn test_function_domain_with_into_response() {
    use axum::response::IntoResponse;
    use axum::http::StatusCode as HttpStatus;

    let err = FunctionDomainWithResponse::NotFound;
    assert_eq!(err.domain(), "function.domain.com");

    let response = err.into_response();
    assert_eq!(response.status(), HttpStatus::NOT_FOUND);
}

#[test]
fn test_function_domain_with_into_response_and_metadata() {
    use axum::response::IntoResponse;
    use axum::http::StatusCode as HttpStatus;

    let err = FunctionDomainWithResponse::Unauthorized {
        token_type: "Bearer".to_string(),
    };

    // Verify domain function is called
    assert_eq!(err.domain(), "function.domain.com");

    // Verify Status conversion preserves domain
    let status: Status = err.clone().into();
    let error_info = &status.details.error_info;
    assert_eq!(error_info.domain, "function.domain.com");
    assert_eq!(error_info.metadata.get("token_type"), Some(&"Bearer".to_string()));

    // Verify IntoResponse works
    let response = err.into_response();
    assert_eq!(response.status(), HttpStatus::UNAUTHORIZED);
}

/// Test with module-scoped function
mod domain_helpers {
    pub fn get_module_domain() -> &'static str {
        "module.scoped.domain.com"
    }
}

#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = domain_helpers::get_module_domain)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum ModuleDomainError {
    #[status(code = Internal)]
    InternalError,
}

#[test]
fn test_module_scoped_domain_function() {
    assert_eq!(ModuleDomainError::InternalError.domain(), "module.scoped.domain.com");

    let status: Status = ModuleDomainError::InternalError.into();
    let error_info = &status.details.error_info;
    assert_eq!(error_info.domain, "module.scoped.domain.com");
}

/// Test comparing string domain vs function domain behavior
#[derive(Debug, Clone, IntoStatus, AsRefStr, strum::Display)]
#[status(domain = "static.string.com")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
enum StringDomainError {
    #[status(code = NotFound)]
    NotFound,
}

#[test]
fn test_string_vs_function_domain_equivalence() {
    // Both should work identically in terms of the IntoStatus trait
    let string_err = StringDomainError::NotFound;
    let function_err = FunctionDomainError::NotFound;

    // Both should return their respective domains
    assert_eq!(string_err.domain(), "static.string.com");
    assert_eq!(function_err.domain(), "function.domain.com");

    // Both should convert to Status correctly
    let string_status: Status = string_err.into();
    let function_status: Status = function_err.into();

    assert_eq!(&string_status.details.error_info.domain, "static.string.com");
    assert_eq!(&function_status.details.error_info.domain, "function.domain.com");
}

#[test]
fn test_function_domain_json_serialization() {
    let err = FunctionDomainError::InvalidInput {
        field: "username".to_string(),
    };

    let status: Status = err.into();
    let json = serde_json::to_string(&status).unwrap();

    assert!(json.contains(r#""domain":"function.domain.com""#));
    assert!(json.contains(r#""field":"username""#));
}
