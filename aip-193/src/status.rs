use serde::{Deserialize, Serialize};

use crate::{Code, ErrorInfo};

/// The `Status` type defines a logical error model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(as = ErrorStatus))]
pub struct Status {
    // The HTTP status code that corresponds to `google.rpc.Status.code`.
    pub code: i32,

    /// A developer-facing error message in English.
    pub message: String,

    /// The status code (google.rpc.Code enum value).
    pub status: Code,

    /// A list of messages that carry the error details.
    pub details: StatusDetails,
}

/// A generic container for arbitrary serialized messages.
///
/// This is used to store error details in the `details` field of the `Status` type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(as = ErrorStatusDetails))]
pub struct StatusDetails {
    pub error_info: ErrorInfo,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Status Tests ============

    #[test]
    #[cfg(feature = "http")]
    fn test_status_default() {
        let status = Status {
            code: 200,
            message: String::new(),
            status: Code::Ok,
            details: StatusDetails::default(),
        };
        assert_eq!(status.code, 200);
        assert_eq!(status.status, Code::Ok);
        assert!(status.message.is_empty());
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_with_code_and_message() {
        let status = Status {
            code: 404,
            message: "User not found".to_string(),
            status: Code::NotFound,
            details: StatusDetails::default(),
        };

        assert_eq!(status.code, 404);
        assert_eq!(status.status, Code::NotFound);
        assert_eq!(status.message, "User not found");
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_with_error_info() {
        let error_info =
            ErrorInfo::new("USER_NOT_FOUND", "myapp.example.com").with_metadata("user_id", "123");

        let status = Status {
            code: 404,
            message: "User not found".to_string(),
            status: Code::NotFound,
            details: StatusDetails { error_info },
        };

        assert_eq!(status.details.error_info.reason, "USER_NOT_FOUND");
        assert_eq!(status.details.error_info.domain, "myapp.example.com");
        assert_eq!(
            status.details.error_info.metadata.get("user_id"),
            Some(&"123".to_string())
        );
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_clone() {
        let status = Status {
            code: 400,
            message: "Invalid argument".to_string(),
            status: Code::InvalidArgument,
            details: StatusDetails {
                error_info: ErrorInfo::new("INVALID_EMAIL", "auth.example.com"),
            },
        };

        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_equality() {
        let status1 = Status {
            code: 404,
            message: "Not found".to_string(),
            status: Code::NotFound,
            details: StatusDetails::default(),
        };

        let status2 = Status {
            code: 404,
            message: "Not found".to_string(),
            status: Code::NotFound,
            details: StatusDetails::default(),
        };

        let status3 = Status {
            code: 400,
            message: "Not found".to_string(),
            status: Code::InvalidArgument,
            details: StatusDetails::default(),
        };

        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }

    // ============ Status Serde Tests ============

    #[test]
    #[cfg(feature = "http")]
    fn test_status_serialize_minimal() {
        let status = Status {
            code: 200,
            message: String::new(),
            status: Code::Ok,
            details: StatusDetails::default(),
        };

        let json = serde_json::to_string(&status).unwrap();
        // message is required but empty, details.error_info is required but empty
        assert!(json.contains(r#""code":200"#));
        assert!(json.contains(r#""status":"OK""#));
        assert!(json.contains(r#""message":"""#));
        assert!(json.contains(r#""details""#));
        assert!(json.contains(r#""error_info""#));
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_serialize_with_message() {
        let status = Status {
            code: 404,
            message: "Resource not found".to_string(),
            status: Code::NotFound,
            details: StatusDetails::default(),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(r#""code":404"#));
        assert!(json.contains(r#""status":"NOT_FOUND""#));
        assert!(json.contains(r#""message":"Resource not found""#));
        // details is now required
        assert!(json.contains(r#""details""#));
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_serialize_full() {
        let status = Status {
            code: 403,
            message: "Permission denied".to_string(),
            status: Code::PermissionDenied,
            details: StatusDetails {
                error_info: ErrorInfo::new("ACCESS_DENIED", "iam.example.com")
                    .with_metadata("resource", "projects/123"),
            },
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(r#""code":403"#));
        assert!(json.contains(r#""status":"PERMISSION_DENIED""#));
        assert!(json.contains(r#""message":"Permission denied""#));
        assert!(json.contains(r#""details""#));
        assert!(json.contains(r#""error_info""#));
        assert!(json.contains(r#""reason":"ACCESS_DENIED""#));
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_deserialize_minimal() {
        let json = r#"{"code":200,"message":"","status":"OK","details":{"error_info":{"reason":"","domain":""}}}"#;
        let status: Status = serde_json::from_str(json).unwrap();

        assert_eq!(status.code, 200);
        assert_eq!(status.status, Code::Ok);
        assert!(status.message.is_empty());
        assert!(status.details.error_info.reason.is_empty());
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_deserialize_full() {
        let json = r#"{
            "code": 404,
            "status": "NOT_FOUND",
            "message": "User not found",
            "details": {
                "error_info": {
                    "reason": "USER_NOT_FOUND",
                    "domain": "users.example.com",
                    "metadata": {
                        "user_id": "usr_123"
                    }
                }
            }
        }"#;

        let status: Status = serde_json::from_str(json).unwrap();
        assert_eq!(status.code, 404);
        assert_eq!(status.status, Code::NotFound);
        assert_eq!(status.message, "User not found");

        assert_eq!(status.details.error_info.reason, "USER_NOT_FOUND");
        assert_eq!(status.details.error_info.domain, "users.example.com");
        assert_eq!(
            status.details.error_info.metadata.get("user_id"),
            Some(&"usr_123".to_string())
        );
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_status_roundtrip() {
        let original = Status {
            code: 429,
            message: "Rate limit exceeded".to_string(),
            status: Code::ResourceExhausted,
            details: StatusDetails {
                error_info: ErrorInfo::new("RATE_LIMITED", "api.example.com")
                    .with_metadata("retry_after", "30")
                    .with_metadata("quota_limit", "100"),
            },
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Status = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    // ============ StatusDetails Tests ============

    #[test]
    fn test_status_details_default() {
        let details = StatusDetails::default();
        assert!(details.error_info.reason.is_empty());
        assert!(details.error_info.domain.is_empty());
    }

    #[test]
    fn test_status_details_with_error_info() {
        let details = StatusDetails {
            error_info: ErrorInfo::new("ERROR", "test.com"),
        };

        assert_eq!(details.error_info.reason, "ERROR");
        assert_eq!(details.error_info.domain, "test.com");
    }

    #[test]
    fn test_status_details_serialize_with_error_info() {
        let details = StatusDetails {
            error_info: ErrorInfo::new("TEST_ERROR", "test.com"),
        };

        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains(r#""error_info""#));
        assert!(json.contains(r#""reason":"TEST_ERROR""#));
    }

    #[test]
    fn test_status_details_clone() {
        let details = StatusDetails {
            error_info: ErrorInfo::new("ERROR", "test.com"),
        };

        let cloned = details.clone();
        assert_eq!(details, cloned);
    }

    #[test]
    fn test_status_details_equality() {
        let details1 = StatusDetails {
            error_info: ErrorInfo::new("ERROR", "test.com"),
        };
        let details2 = StatusDetails {
            error_info: ErrorInfo::new("ERROR", "test.com"),
        };
        let details3 = StatusDetails {
            error_info: ErrorInfo::new("OTHER", "test.com"),
        };

        assert_eq!(details1, details2);
        assert_ne!(details1, details3);
    }

    // ============ Status Code Mapping Tests ============

    #[test]
    #[cfg(feature = "http")]
    fn test_status_common_codes() {
        // Test common gRPC status codes
        let codes = [
            (Code::Ok, "OK", 200),
            (Code::Cancelled, "CANCELLED", 499),
            (Code::Unknown, "UNKNOWN", 500),
            (Code::InvalidArgument, "INVALID_ARGUMENT", 400),
            (Code::DeadlineExceeded, "DEADLINE_EXCEEDED", 504),
            (Code::NotFound, "NOT_FOUND", 404),
            (Code::AlreadyExists, "ALREADY_EXISTS", 409),
            (Code::PermissionDenied, "PERMISSION_DENIED", 403),
            (Code::ResourceExhausted, "RESOURCE_EXHAUSTED", 429),
            (Code::FailedPrecondition, "FAILED_PRECONDITION", 400),
            (Code::Aborted, "ABORTED", 409),
            (Code::OutOfRange, "OUT_OF_RANGE", 400),
            (Code::Unimplemented, "UNIMPLEMENTED", 501),
            (Code::Internal, "INTERNAL", 500),
            (Code::Unavailable, "UNAVAILABLE", 503),
            (Code::DataLoss, "DATA_LOSS", 500),
            (Code::Unauthenticated, "UNAUTHENTICATED", 401),
        ];

        for (status_code, name, http_code) in codes {
            let status = Status {
                code: http_code,
                message: format!("Testing {}", name),
                status: status_code,
                details: StatusDetails::default(),
            };
            assert_eq!(status.code, http_code);
            assert_eq!(status.status, status_code);
        }
    }
}
