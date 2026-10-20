# aip-rs

Rust implementations of Google API Improvement Proposals (AIPs).

| Crate | AIP | Content |
| --- | --- | --- |
| [`aip-160`](aip-160) | [AIP-160: Filtering](https://google.aip.dev/160) | A parser for list filters, and an optional conversion to SeaORM 2.0 conditions. |
| [`aip-193`](aip-193) | [AIP-193: Errors](https://google.aip.dev/193) | Error types for JSON APIs, and optional protobuf `google.rpc` types for gRPC and Connect. |
| [`aip-193-derive`](aip-193-derive) | AIP-193 | A derive macro that implements `IntoStatus` for error enums. |
| `aip` | | Re-exports `aip-160` as `aip::filtering` and `aip-193` as `aip::errors`. |

## Features of `aip`

| Feature | Enables |
| --- | --- |
| `derive` | `aip-193/derive` |
| `axum` | `aip-193/axum` and `aip-193/derive` |
| `proto` | `aip-193/proto` |
| `connectrpc` | `aip-193/connectrpc` |
| `sea-orm` | `aip-160/sea-orm` |

## Development

```sh
cargo test --workspace --all-features
```

`aip-193/src/proto/generated/` is generated code. To regenerate it, see [`aip-193/README.md`](aip-193/README.md).

## License

Licensed under the MIT license or the Apache License 2.0, at your option.
