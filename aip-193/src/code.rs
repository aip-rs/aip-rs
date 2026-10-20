use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

/// The canonical error codes for gRPC APIs.
/// 
/// <https://github.com/grpc/grpc/blob/master/doc/statuscodes.md>
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    // serde
    Serialize,
    Deserialize,
    // strum
    Display,
    EnumString,
    AsRefStr,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "utoipa", schema(as = ErrorCode))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[repr(i32)]
pub enum Code {
    /// Not an error; returned on success.
    /// HTTP Mapping: 200 OK
    Ok = 0,
    /// The operation was cancelled, typically by the caller.
    /// HTTP Mapping: 499 Client Closed Request
    Cancelled = 1,
    /// Unknown error.
    /// HTTP Mapping: 500 Internal Server Error
    Unknown = 2,
    /// The client specified an invalid argument.
    /// HTTP Mapping: 400 Bad Request
    InvalidArgument = 3,
    /// The deadline expired before the operation could complete.
    /// HTTP Mapping: 504 Gateway Timeout
    DeadlineExceeded = 4,
    /// Some requested entity was not found.
    /// HTTP Mapping: 404 Not Found
    NotFound = 5,
    /// The entity already exists.
    /// HTTP Mapping: 409 Conflict
    AlreadyExists = 6,
    /// The caller does not have permission.
    /// HTTP Mapping: 403 Forbidden
    PermissionDenied = 7,
    /// Some resource has been exhausted.
    /// HTTP Mapping: 429 Too Many Requests
    ResourceExhausted = 8,
    /// The system is not in a required state.
    /// HTTP Mapping: 400 Bad Request
    FailedPrecondition = 9,
    /// The operation was aborted.
    /// HTTP Mapping: 409 Conflict
    Aborted = 10,
    /// The operation was attempted past the valid range.
    /// HTTP Mapping: 400 Bad Request
    OutOfRange = 11,
    /// The operation is not implemented.
    /// HTTP Mapping: 501 Not Implemented
    Unimplemented = 12,
    /// Internal errors.
    /// HTTP Mapping: 500 Internal Server Error
    Internal = 13,
    /// The service is currently unavailable.
    /// HTTP Mapping: 503 Service Unavailable
    Unavailable = 14,
    /// Unrecoverable data loss or corruption.
    /// HTTP Mapping: 500 Internal Server Error
    DataLoss = 15,
    /// The request does not have valid authentication credentials.
    /// HTTP Mapping: 401 Unauthorized
    Unauthenticated = 16,
}

impl From<Code> for i32 {
    fn from(code: Code) -> i32 {
        code as i32
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_to_i32() {
        assert_eq!(Code::Ok as i32, 0);
        assert_eq!(Code::Cancelled as i32, 1);
        assert_eq!(Code::Unknown as i32, 2);
        assert_eq!(Code::InvalidArgument as i32, 3);
        assert_eq!(Code::DeadlineExceeded as i32, 4);
        assert_eq!(Code::NotFound as i32, 5);
        assert_eq!(Code::AlreadyExists as i32, 6);
        assert_eq!(Code::PermissionDenied as i32, 7);
        assert_eq!(Code::ResourceExhausted as i32, 8);
        assert_eq!(Code::FailedPrecondition as i32, 9);
        assert_eq!(Code::Aborted as i32, 10);
        assert_eq!(Code::OutOfRange as i32, 11);
        assert_eq!(Code::Unimplemented as i32, 12);
        assert_eq!(Code::Internal as i32, 13);
        assert_eq!(Code::Unavailable as i32, 14);
        assert_eq!(Code::DataLoss as i32, 15);
        assert_eq!(Code::Unauthenticated as i32, 16);
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_code_to_http_status() {
        use http::StatusCode;
        assert_eq!(StatusCode::from(Code::Ok), StatusCode::OK);
        assert_eq!(StatusCode::from(Code::InvalidArgument), StatusCode::BAD_REQUEST);
        assert_eq!(StatusCode::from(Code::Unauthenticated), StatusCode::UNAUTHORIZED);
        assert_eq!(StatusCode::from(Code::PermissionDenied), StatusCode::FORBIDDEN);
        assert_eq!(StatusCode::from(Code::NotFound), StatusCode::NOT_FOUND);
        assert_eq!(StatusCode::from(Code::AlreadyExists), StatusCode::CONFLICT);
        assert_eq!(StatusCode::from(Code::ResourceExhausted), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(StatusCode::from(Code::Unimplemented), StatusCode::NOT_IMPLEMENTED);
        assert_eq!(StatusCode::from(Code::Internal), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(StatusCode::from(Code::Unavailable), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(StatusCode::from(Code::DeadlineExceeded), StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(StatusCode::from(Code::Unknown), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(StatusCode::from(Code::DataLoss), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(StatusCode::from(Code::FailedPrecondition), StatusCode::BAD_REQUEST);
        assert_eq!(StatusCode::from(Code::Aborted), StatusCode::CONFLICT);
        assert_eq!(StatusCode::from(Code::OutOfRange), StatusCode::BAD_REQUEST);
    }

    #[test]
    #[cfg(feature = "http")]
    fn test_http_status_to_code() {
        use http::StatusCode;
        // 2xx -> Ok
        assert_eq!(Code::from(StatusCode::OK), Code::Ok);
        assert_eq!(Code::from(StatusCode::CREATED), Code::Ok);
        assert_eq!(Code::from(StatusCode::NO_CONTENT), Code::Ok);

        // Specific 4xx mappings
        assert_eq!(Code::from(StatusCode::BAD_REQUEST), Code::InvalidArgument);
        assert_eq!(Code::from(StatusCode::UNAUTHORIZED), Code::Unauthenticated);
        assert_eq!(Code::from(StatusCode::FORBIDDEN), Code::PermissionDenied);
        assert_eq!(Code::from(StatusCode::NOT_FOUND), Code::NotFound);
        assert_eq!(Code::from(StatusCode::CONFLICT), Code::AlreadyExists);
        assert_eq!(Code::from(StatusCode::TOO_MANY_REQUESTS), Code::ResourceExhausted);

        // Specific 5xx mappings
        assert_eq!(Code::from(StatusCode::NOT_IMPLEMENTED), Code::Unimplemented);
        assert_eq!(Code::from(StatusCode::SERVICE_UNAVAILABLE), Code::Unavailable);
        assert_eq!(Code::from(StatusCode::GATEWAY_TIMEOUT), Code::DeadlineExceeded);

        // Other 4xx -> FailedPrecondition
        assert_eq!(Code::from(StatusCode::GONE), Code::FailedPrecondition);
        assert_eq!(Code::from(StatusCode::PRECONDITION_FAILED), Code::FailedPrecondition);

        // Other 5xx -> Internal
        assert_eq!(Code::from(StatusCode::INTERNAL_SERVER_ERROR), Code::Internal);
        assert_eq!(Code::from(StatusCode::BAD_GATEWAY), Code::Internal);

        // Unknown status codes
        assert_eq!(Code::from(StatusCode::CONTINUE), Code::Unknown);
    }

    #[test]
    fn test_code_display() {
        assert_eq!(Code::Ok.to_string(), "OK");
        assert_eq!(Code::NotFound.to_string(), "NOT_FOUND");
        assert_eq!(Code::InvalidArgument.to_string(), "INVALID_ARGUMENT");
        assert_eq!(Code::PermissionDenied.to_string(), "PERMISSION_DENIED");
        assert_eq!(Code::ResourceExhausted.to_string(), "RESOURCE_EXHAUSTED");
    }

    #[test]
    fn test_code_from_str() {
        use std::str::FromStr;

        assert_eq!(Code::from_str("OK").unwrap(), Code::Ok);
        assert_eq!(Code::from_str("NOT_FOUND").unwrap(), Code::NotFound);
        assert_eq!(Code::from_str("INVALID_ARGUMENT").unwrap(), Code::InvalidArgument);
        assert_eq!(Code::from_str("PERMISSION_DENIED").unwrap(), Code::PermissionDenied);
        assert!(Code::from_str("INVALID").is_err());
    }

    #[test]
    fn test_code_as_ref() {
        assert_eq!(Code::Ok.as_ref(), "OK");
        assert_eq!(Code::NotFound.as_ref(), "NOT_FOUND");
        assert_eq!(Code::Internal.as_ref(), "INTERNAL");
    }

    #[test]
    fn test_code_serde() {
        // Serialize
        let code = Code::NotFound;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, r#""NOT_FOUND""#);

        // Deserialize
        let deserialized: Code = serde_json::from_str(r#""NOT_FOUND""#).unwrap();
        assert_eq!(deserialized, Code::NotFound);

        // Round-trip all codes
        for code in [
            Code::Ok, Code::Cancelled, Code::Unknown, Code::InvalidArgument,
            Code::DeadlineExceeded, Code::NotFound, Code::AlreadyExists,
            Code::PermissionDenied, Code::ResourceExhausted, Code::FailedPrecondition,
            Code::Aborted, Code::OutOfRange, Code::Unimplemented, Code::Internal,
            Code::Unavailable, Code::DataLoss, Code::Unauthenticated,
        ] {
            let json = serde_json::to_string(&code).unwrap();
            let back: Code = serde_json::from_str(&json).unwrap();
            assert_eq!(back, code);
        }
    }

    #[test]
    fn test_code_equality_and_hash() {
        use std::collections::HashSet;

        assert_eq!(Code::Ok, Code::Ok);
        assert_ne!(Code::Ok, Code::NotFound);

        let mut set = HashSet::new();
        set.insert(Code::Ok);
        set.insert(Code::NotFound);
        set.insert(Code::Ok); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_code_clone_and_copy() {
        let code = Code::NotFound;
        let cloned = code.clone();
        let copied = code;

        assert_eq!(code, cloned);
        assert_eq!(code, copied);
    }
}