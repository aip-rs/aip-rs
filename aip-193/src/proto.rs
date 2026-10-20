//! Protobuf `google.rpc` types for AIP-193, generated with [buffa].
//!
//! Enabled by the `proto` feature. The module holds the messages of
//! `google/rpc/code.proto`, `google/rpc/status.proto` and
//! `google/rpc/error_details.proto`, generated from the googleapis sources in
//! `aip-193/proto/`. Use these types when an error travels as protobuf, for
//! example as a gRPC or Connect error detail, or as a `google.rpc.Status`
//! field inside a response message.
//!
//! # Use the types in your own generated code
//!
//! A crate that generates buffa code for protos that import
//! `google/rpc/status.proto` can map the package to this module instead of
//! generating a second copy:
//!
//! ```ignore
//! // build.rs
//! let mut buffa = buffa_codegen::CodeGenConfig::default();
//! buffa.extern_paths.push((
//!     ".google.rpc".to_owned(),
//!     "::aip_193::proto::google::rpc".to_owned(),
//! ));
//! connectrpc_build::Config::new()
//!     .buffa_config(buffa)
//!     // ...
//!     .compile()?;
//! ```
//!
//! The generated code depends on the `buffa` 0.9 runtime.
//!
//! # Build an error
//!
//! ```
//! use aip_193::proto::{Code, ErrorInfo, FieldViolation, BadRequest, Status};
//!
//! let status = Status::new(Code::INVALID_ARGUMENT, "The request contains an invalid field.")
//!     .with_detail(&ErrorInfo::new("INVALID_PAGE_SIZE", "example.com"))
//!     .with_detail(&BadRequest::from(FieldViolation::new(
//!         "page_size",
//!         "must not be negative",
//!     )));
//! assert_eq!(status.code(), Code::INVALID_ARGUMENT);
//! assert_eq!(status.error_info().unwrap().reason, "INVALID_PAGE_SIZE");
//! ```
//!
//! [buffa]: https://docs.rs/buffa

#[cfg(feature = "connectrpc")]
mod connect;
#[allow(
    rustdoc::broken_intra_doc_links,
    rustdoc::bare_urls,
    reason = "The generated docs copy the googleapis comments, which link to \
              `crate::google::rpc` and contain bare URLs."
)]
#[rustfmt::skip]
mod generated;

pub use generated::google;
pub use google::rpc::{
    BadRequest, Code, DebugInfo, ErrorInfo, Help, LocalizedMessage, PreconditionFailure,
    QuotaFailure, RequestInfo, ResourceInfo, RetryInfo, Status, bad_request::FieldViolation,
    register_types,
};

use buffa::{Enumeration, Message, MessageName};
use buffa_types::google::protobuf::Any;

impl Status {
    /// Creates a status with a code, a developer-facing message and no details.
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code: code.to_i32(),
            message: message.into(),
            ..Default::default()
        }
    }

    /// Appends one detail message, packed as `google.protobuf.Any`.
    ///
    /// AIP-193 requires exactly one [`ErrorInfo`] in every error. Add it
    /// first, so that clients that read only the first detail still find it.
    #[must_use]
    pub fn with_detail<M: Message + MessageName>(mut self, detail: &M) -> Self {
        self.details.push(Any::pack(detail, M::TYPE_URL));
        self
    }

    /// Returns the code as a [`Code`].
    ///
    /// A number that `google.rpc.Code` does not define returns
    /// [`Code::UNKNOWN`].
    pub fn code(&self) -> Code {
        Code::from_i32(self.code).unwrap_or(Code::UNKNOWN)
    }

    /// Decodes the first detail of type `M`.
    ///
    /// Returns `None` when no detail has the type URL of `M`, or when that
    /// detail does not decode.
    pub fn detail<M: Message + MessageName>(&self) -> Option<M> {
        self.details
            .iter()
            .find(|any| any.is_type(M::TYPE_URL))
            .and_then(|any| any.unpack_unchecked().ok())
    }

    /// Decodes the first [`ErrorInfo`] detail.
    pub fn error_info(&self) -> Option<ErrorInfo> {
        self.detail()
    }
}

impl ErrorInfo {
    /// Creates an `ErrorInfo` with a reason and a domain and no metadata.
    ///
    /// `reason` is an `UPPER_SNAKE_CASE` identifier of at most 63 characters.
    /// See [`crate::ErrorInfo::is_valid_reason`].
    pub fn new(reason: impl Into<String>, domain: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            domain: domain.into(),
            ..Default::default()
        }
    }

    /// Adds one metadata entry. A later entry with the same key replaces the
    /// earlier one.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl FieldViolation {
    /// Creates a violation for one request field.
    ///
    /// `field` is a path in the request message, for example
    /// `writes[2].etag`.
    pub fn new(field: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            description: description.into(),
            ..Default::default()
        }
    }

    /// Sets the `UPPER_SNAKE_CASE` reason of this violation.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }
}

impl BadRequest {
    /// Creates a `BadRequest` from a list of field violations.
    pub fn new(field_violations: impl IntoIterator<Item = FieldViolation>) -> Self {
        Self {
            field_violations: field_violations.into_iter().collect(),
            ..Default::default()
        }
    }
}

impl From<FieldViolation> for BadRequest {
    fn from(violation: FieldViolation) -> Self {
        Self::new([violation])
    }
}

impl ResourceInfo {
    /// Creates a `ResourceInfo` that names one resource.
    ///
    /// `resource_type` is the AIP-123 type, for example
    /// `library.googleapis.com/Book`. `resource_name` is the AIP-122 name.
    pub fn new(resource_type: impl Into<String>, resource_name: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_name: resource_name.into(),
            ..Default::default()
        }
    }

    /// Sets the owner of the resource.
    #[must_use]
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = owner.into();
        self
    }

    /// Sets the description of the error that relates to the resource.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

impl PreconditionFailure {
    /// Creates a `PreconditionFailure` from a list of violations.
    pub fn new(
        violations: impl IntoIterator<Item = google::rpc::precondition_failure::Violation>,
    ) -> Self {
        Self {
            violations: violations.into_iter().collect(),
            ..Default::default()
        }
    }
}

impl google::rpc::precondition_failure::Violation {
    /// Creates a violation with a type, a subject and a description.
    pub fn new(
        r#type: impl Into<String>,
        subject: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            r#type: r#type.into(),
            subject: subject.into(),
            description: description.into(),
            ..Default::default()
        }
    }
}

impl LocalizedMessage {
    /// Creates a message in one BCP 47 locale, for example `en-US`.
    pub fn new(locale: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            locale: locale.into(),
            message: message.into(),
            ..Default::default()
        }
    }
}

impl From<crate::Code> for Code {
    fn from(code: crate::Code) -> Self {
        match code {
            crate::Code::Ok => Self::OK,
            crate::Code::Cancelled => Self::CANCELLED,
            crate::Code::Unknown => Self::UNKNOWN,
            crate::Code::InvalidArgument => Self::INVALID_ARGUMENT,
            crate::Code::DeadlineExceeded => Self::DEADLINE_EXCEEDED,
            crate::Code::NotFound => Self::NOT_FOUND,
            crate::Code::AlreadyExists => Self::ALREADY_EXISTS,
            crate::Code::PermissionDenied => Self::PERMISSION_DENIED,
            crate::Code::ResourceExhausted => Self::RESOURCE_EXHAUSTED,
            crate::Code::FailedPrecondition => Self::FAILED_PRECONDITION,
            crate::Code::Aborted => Self::ABORTED,
            crate::Code::OutOfRange => Self::OUT_OF_RANGE,
            crate::Code::Unimplemented => Self::UNIMPLEMENTED,
            crate::Code::Internal => Self::INTERNAL,
            crate::Code::Unavailable => Self::UNAVAILABLE,
            crate::Code::DataLoss => Self::DATA_LOSS,
            crate::Code::Unauthenticated => Self::UNAUTHENTICATED,
        }
    }
}

impl From<Code> for crate::Code {
    fn from(code: Code) -> Self {
        match code {
            Code::OK => Self::Ok,
            Code::CANCELLED => Self::Cancelled,
            Code::UNKNOWN => Self::Unknown,
            Code::INVALID_ARGUMENT => Self::InvalidArgument,
            Code::DEADLINE_EXCEEDED => Self::DeadlineExceeded,
            Code::NOT_FOUND => Self::NotFound,
            Code::ALREADY_EXISTS => Self::AlreadyExists,
            Code::PERMISSION_DENIED => Self::PermissionDenied,
            Code::RESOURCE_EXHAUSTED => Self::ResourceExhausted,
            Code::FAILED_PRECONDITION => Self::FailedPrecondition,
            Code::ABORTED => Self::Aborted,
            Code::OUT_OF_RANGE => Self::OutOfRange,
            Code::UNIMPLEMENTED => Self::Unimplemented,
            Code::INTERNAL => Self::Internal,
            Code::UNAVAILABLE => Self::Unavailable,
            Code::DATA_LOSS => Self::DataLoss,
            Code::UNAUTHENTICATED => Self::Unauthenticated,
        }
    }
}

impl From<crate::ErrorInfo> for ErrorInfo {
    fn from(info: crate::ErrorInfo) -> Self {
        Self {
            reason: info.reason,
            domain: info.domain,
            metadata: info.metadata.into_iter().collect(),
            ..Default::default()
        }
    }
}

impl From<ErrorInfo> for crate::ErrorInfo {
    fn from(info: ErrorInfo) -> Self {
        Self {
            reason: info.reason,
            domain: info.domain,
            metadata: info.metadata.into_iter().collect(),
        }
    }
}

/// Converts the JSON status into a protobuf status.
///
/// The protobuf code comes from [`crate::Status::status`]. The HTTP status
/// code in [`crate::Status::code`] has no protobuf field and is dropped. The
/// `ErrorInfo` becomes the only detail.
impl From<crate::Status> for Status {
    fn from(status: crate::Status) -> Self {
        Self::new(status.status.into(), status.message)
            .with_detail(&ErrorInfo::from(status.details.error_info))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_packs_details_in_order_and_reads_them_back() {
        let status = Status::new(Code::NOT_FOUND, "missing")
            .with_detail(&ErrorInfo::new("BOOK_NOT_FOUND", "example.com").with_metadata("k", "v"))
            .with_detail(&ResourceInfo::new("example.com/Book", "shelves/1/books/2"));

        assert_eq!(status.code, 5);
        assert_eq!(status.code(), Code::NOT_FOUND);
        let urls: Vec<&str> = status.details.iter().map(|d| d.type_url.as_str()).collect();
        assert_eq!(
            urls,
            [
                "type.googleapis.com/google.rpc.ErrorInfo",
                "type.googleapis.com/google.rpc.ResourceInfo",
            ]
        );
        let info = status.error_info().expect("ErrorInfo");
        assert_eq!(info.reason, "BOOK_NOT_FOUND");
        assert_eq!(info.metadata.get("k").map(String::as_str), Some("v"));
        let resource: ResourceInfo = status.detail().expect("ResourceInfo");
        assert_eq!(resource.resource_name, "shelves/1/books/2");
        assert!(status.detail::<BadRequest>().is_none());
    }

    #[test]
    fn undefined_code_number_reads_as_unknown() {
        let status = Status {
            code: 99,
            ..Default::default()
        };
        assert_eq!(status.code(), Code::UNKNOWN);
    }

    #[test]
    fn every_code_converts_both_ways() {
        for number in 0..=16 {
            let proto = Code::from_i32(number).expect("defined code");
            let json = crate::Code::from(proto);
            assert_eq!(json as i32, number);
            assert_eq!(Code::from(json), proto);
        }
    }

    #[test]
    fn json_status_converts_to_protobuf_status() {
        let json = crate::Status {
            code: 403,
            message: "denied".to_owned(),
            status: crate::Code::PermissionDenied,
            details: crate::StatusDetails {
                error_info: crate::ErrorInfo::new("ACCESS_DENIED", "example.com"),
            },
        };
        let status = Status::from(json);
        assert_eq!(status.code(), Code::PERMISSION_DENIED);
        assert_eq!(status.message, "denied");
        assert_eq!(status.details.len(), 1);
        assert_eq!(
            status.error_info().expect("ErrorInfo").reason,
            "ACCESS_DENIED"
        );
    }

    #[test]
    fn field_violation_builds_a_bad_request() {
        let bad_request = BadRequest::from(
            FieldViolation::new("page_size", "too small").with_reason("TOO_SMALL"),
        );
        let [violation] = bad_request.field_violations.as_slice() else {
            panic!("one violation");
        };
        assert_eq!(violation.field, "page_size");
        assert_eq!(violation.reason, "TOO_SMALL");
    }
}
