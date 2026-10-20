//! Conversions between [`Status`] and [`connectrpc::ConnectError`].
//!
//! Enabled by the `connectrpc` feature.
//!
//! A Connect error carries each detail as the bare message name, for example
//! `google.rpc.ErrorInfo`, and the wire bytes in unpadded standard base64. A
//! `google.protobuf.Any` carries the name with the `type.googleapis.com/`
//! prefix. The conversions add or remove that prefix and change nothing else.

use super::{Code, Status};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use buffa::Enumeration;
use buffa_types::google::protobuf::Any;
use connectrpc::{ConnectError, ErrorCode, ErrorDetail};

const TYPE_URL_PREFIX: &str = "type.googleapis.com/";

impl From<ErrorCode> for Code {
    fn from(code: ErrorCode) -> Self {
        i32::try_from(code.grpc_code())
            .ok()
            .and_then(Code::from_i32)
            .unwrap_or(Code::UNKNOWN)
    }
}

/// Converts a status into a Connect error.
///
/// Connect has no error code for `OK`. A status with `OK`, or with a number
/// that `google.rpc.Code` does not define, becomes `unknown`. The details
/// keep their order.
impl From<Status> for ConnectError {
    fn from(status: Status) -> Self {
        let code = u32::try_from(status.code)
            .ok()
            .and_then(ErrorCode::from_grpc_code)
            .unwrap_or(ErrorCode::Unknown);
        let mut error = ConnectError::new(code, status.message);
        for any in status.details {
            let type_name = any
                .type_url
                .strip_prefix(TYPE_URL_PREFIX)
                .unwrap_or(&any.type_url)
                .to_owned();
            error = error.with_detail(ErrorDetail {
                type_url: type_name,
                value: Some(STANDARD_NO_PAD.encode(&any.value)),
                debug: None,
            });
        }
        error
    }
}

/// Converts a Connect error into a status, for example on the client side.
///
/// A detail whose value is missing or is not valid base64 is dropped. Both
/// padded and unpadded base64 are accepted.
impl From<&ConnectError> for Status {
    fn from(error: &ConnectError) -> Self {
        let mut status = Status::new(
            Code::from(error.code),
            error.message.clone().unwrap_or_default(),
        );
        for detail in &error.details {
            let Some(value) = detail.value.as_deref() else {
                continue;
            };
            let Ok(bytes) = STANDARD_NO_PAD
                .decode(value.trim_end_matches('='))
                .or_else(|_| STANDARD.decode(value))
            else {
                continue;
            };
            let type_url = if detail.type_url.contains('/') {
                detail.type_url.clone()
            } else {
                format!("{TYPE_URL_PREFIX}{}", detail.type_url)
            };
            status.details.push(Any {
                type_url,
                value: bytes.into(),
                ..Default::default()
            });
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{ErrorInfo, ResourceInfo};

    #[test]
    fn status_becomes_connect_error_with_bare_type_names() {
        let status = Status::new(Code::NOT_FOUND, "missing")
            .with_detail(&ErrorInfo::new("BOOK_NOT_FOUND", "example.com"))
            .with_detail(&ResourceInfo::new("example.com/Book", "books/1"));

        let error = ConnectError::from(status.clone());
        assert_eq!(error.code, ErrorCode::NotFound);
        assert_eq!(error.message.as_deref(), Some("missing"));
        let names: Vec<&str> = error.details.iter().map(|d| d.type_url.as_str()).collect();
        assert_eq!(names, ["google.rpc.ErrorInfo", "google.rpc.ResourceInfo"]);
        let expected = ErrorDetail::from_message(
            "google.rpc.ErrorInfo",
            &ErrorInfo::new("BOOK_NOT_FOUND", "example.com"),
        );
        assert_eq!(error.details[0].value, expected.value);

        assert_eq!(Status::from(&error), status);
    }

    #[test]
    fn ok_and_undefined_codes_become_unknown() {
        for code in [0, 99, -1] {
            let status = Status {
                code,
                ..Default::default()
            };
            assert_eq!(ConnectError::from(status).code, ErrorCode::Unknown);
        }
    }

    #[test]
    fn every_connect_code_maps_to_the_same_number() {
        for number in 1..=16 {
            let code = ErrorCode::from_grpc_code(number).expect("defined code");
            assert_eq!(Code::from(code).to_i32(), i32::try_from(number).unwrap());
        }
    }

    #[test]
    fn padded_and_invalid_detail_values() {
        let info = ErrorInfo::new("R", "d");
        let mut padded = ErrorDetail::from_message("google.rpc.ErrorInfo", &info);
        let value = padded.value.take().unwrap();
        padded.value = Some(format!("{value}{}", "=".repeat((4 - value.len() % 4) % 4)));
        let invalid = ErrorDetail {
            type_url: "google.rpc.ErrorInfo".to_owned(),
            value: Some("not base64!".to_owned()),
            debug: None,
        };
        let error = ConnectError::new(ErrorCode::Internal, "x")
            .with_detail(padded)
            .with_detail(invalid);

        let status = Status::from(&error);
        assert_eq!(status.details.len(), 1);
        assert_eq!(status.error_info(), Some(info));
    }
}
