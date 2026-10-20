# aip-193

Error types for [Google AIP-193: Errors](https://google.aip.dev/193).

## Features

| Feature | Default | Content |
| --- | --- | --- |
| `http` | yes | Conversions between `Code` and `http::StatusCode`. |
| `axum` | no | `IntoResponse` for `Status`. |
| `derive` | no | `#[derive(IntoStatus)]` for error enums. |
| `utoipa` | no | OpenAPI schemas for the JSON types. |
| `proto` | no | Protobuf `google.rpc` types in `aip_193::proto`, generated with [buffa](https://docs.rs/buffa) 0.9. |
| `connectrpc` | no | Conversions between `aip_193::proto::Status` and `connectrpc::ConnectError`. Enables `proto`. |

## JSON errors

`Status`, `ErrorInfo` and `Code` are serde types for HTTP JSON APIs.

```rust
use aip_193::{Code, ErrorInfo};

let info = ErrorInfo::new("BOOK_NOT_FOUND", "library.example.com");
assert!(info.is_valid_reason());
assert_eq!(Code::NotFound as i32, 5);
```

## Protobuf errors

The `proto` feature adds the messages of `google/rpc/code.proto`, `google/rpc/status.proto` and `google/rpc/error_details.proto`: `Status`, `Code`, `ErrorInfo`, `BadRequest`, `ResourceInfo`, `PreconditionFailure` and the other error details. Use them when an error travels as protobuf.

```rust,ignore
use aip_193::proto::{BadRequest, Code, ErrorInfo, FieldViolation, Status};

let status = Status::new(Code::INVALID_ARGUMENT, "The request contains an invalid field.")
    .with_detail(&ErrorInfo::new("INVALID_PAGE_SIZE", "library.example.com"))
    .with_detail(&BadRequest::from(
        FieldViolation::new("page_size", "must not be negative").with_reason("INVALID_PAGE_SIZE"),
    ));

// With the `connectrpc` feature:
let error = connectrpc::ConnectError::from(status);
```

AIP-193 requires exactly one `ErrorInfo` in every error. Add it as the first detail.

### Use the types in generated code

If your protos import `google/rpc/status.proto` or `google/rpc/error_details.proto`, map the `google.rpc` package to this crate. Your build then does not generate a second copy:

```rust,ignore
// build.rs
let mut buffa = buffa_codegen::CodeGenConfig::default();
buffa.extern_paths.push((
    ".google.rpc".to_owned(),
    "::aip_193::proto::google::rpc".to_owned(),
));
connectrpc_build::Config::new()
    .buffa_config(buffa)
    .files(&["my/service.proto"])
    .compile()?;
```

To render `google.protobuf.Any` details in Connect JSON, register the types once:

```rust,ignore
let mut registry = buffa::type_registry::TypeRegistry::new();
buffa_types::register_wkt_types(&mut registry);
aip_193::proto::register_types(&mut registry);
```

### Regenerate the protobuf code

The protos in `proto/` come unchanged from [googleapis](https://github.com/googleapis/googleapis) commit `ba591f1d7fdedc610639425448756cd8a64adc0f`. Their license is in `proto/LICENSE` (Apache License 2.0).

To regenerate `src/proto/generated/`, install `protoc` and run this command at the repository root:

```sh
cargo run --manifest-path aip-193/codegen/Cargo.toml
```

Keep the `connectrpc-build` version in `codegen/Cargo.toml` in step with the `buffa` version in `Cargo.toml`.

## License

Licensed under the MIT license or the Apache License 2.0, at your option. The files in `proto/` are licensed under the Apache License 2.0.
