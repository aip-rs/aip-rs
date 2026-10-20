# aip-160

A parser for [Google AIP-160](https://google.aip.dev/160) list filters, with an optional conversion to SeaORM conditions.

The parser follows the [AIP-160 EBNF grammar](https://google.aip.dev/assets/misc/ebnf-filtering.txt). It is a hand-written recursive-descent parser with no grammar-generator dependency.

## Installation

```toml
[dependencies]
# Parsing only
aip-160 = "0.2"

# Parsing and conversion to SeaORM 2.0 conditions
aip-160 = { version = "0.2", features = ["sea-orm"] }
```

## Parse a filter

```rust
use aip_160::{parse_filter, Comparator, Expression};

let filter = parse_filter("state = SUSPENDED AND score >= 2.5").unwrap();
let Some(Expression::And(children)) = &filter.expression else { panic!() };
let Expression::Restriction(state) = &children[0] else { panic!() };
assert_eq!(state.comparable.field_path().as_deref(), Some("state"));
assert_eq!(state.comparator, Comparator::Equal);
assert_eq!(state.arg.as_value().map(|v| v.as_str()), Some("SUSPENDED"));
```

`Filter` implements `Display`. The output is the canonical form of the filter, with explicit parentheses.

`parse_filter` limits the nesting depth to `DEFAULT_MAX_DEPTH`. Use `parse_filter_with_max_depth` to set a different limit.

## Supported syntax

| Element | Example | Notes |
| --- | --- | --- |
| Comparators | `a = 1`, `a != 1`, `a < 1`, `a <= 1`, `a > 1`, `a >= 1` | |
| Has | `labels:env`, `tags:"prod"` | |
| `AND` | `a = 1 AND b = 2` | Lowest precedence. Case-sensitive. |
| Sequence | `a = 1 b = 2` | Whitespace-separated terms. Binds tighter than `AND`. |
| `OR` | `a = 1 OR a = 2` | Highest precedence. Case-sensitive. |
| Negation | `NOT a = 1`, `-a = 1` | |
| Grouping | `(a = 1 OR b = 2) AND c = 3` | |
| Global restriction | `prod` | A comparable with no comparator. |
| Traversal | `labels.env = prod` | |
| Function call | `regex(name, "^prod")` | Parsed only. The service decides which functions it supports. |
| Unquoted text | `ACTIVE`, `42`, `2.5`, `true`, `20s` | Enum values, numbers, booleans and durations. |
| Quoted string | `"abc"`, `'abc'` | May start or end with the `*` wildcard. |

Precedence follows AIP-160: `OR` binds tighter than a sequence, and a sequence binds tighter than `AND`. So `a AND b OR c` means `a AND (b OR c)`.

The parser keeps literals as written. `Value` has helpers that read a literal as a boolean, a number, a duration or a wildcard match.

## Convert to a SeaORM condition

Enable the `sea-orm` feature. The conversion supports SeaORM 2.0.

```rust,ignore
use aip_160::{parse_filter, ToSeaOrmCondition};

let filter = parse_filter(r#"name = "Alice" AND age > 18"#)?;
let condition = filter.to_condition::<user::Entity>()?;
user::Entity::find().filter(condition).all(db).await?;
```

The conversion supports restrictions on top-level columns, joined with `AND`, `OR`, `NOT` and sequences. A field name matches a column directly, or after a conversion from `snake_case` to `PascalCase`. Each literal is read with the type of its column:

| Column type | Accepted literal |
| --- | --- |
| boolean | `true`, `false` |
| signed or unsigned integer | `42`, `-7` |
| float, double, decimal, money | `2.5`, `1e3` |
| enum | `ACTIVE` or `"ACTIVE"`, cast to the database enum type |
| uuid, timestamp, date, time | a quoted string, cast to the column type |
| other types | text or a quoted string |

`field = null` and `field != null` become `IS NULL` and `IS NOT NULL`. On string columns, `field = "abc*"`, `"*abc"` and `"*abc*"` become `LIKE` patterns, and `field:"abc"` matches a substring.

Traversal, function calls and global restrictions return `FilterError::Unsupported`. An unknown field returns `FilterError::InvalidField`. A literal that does not match the column type returns `FilterError::InvalidValue`. Map these errors to `INVALID_ARGUMENT`, as AIP-160 requires.

## Changes from 0.1

- `OR` now binds tighter than `AND`, as AIP-160 requires. In 0.1, `a AND b OR c` meant `(a AND b) OR c`.
- Unquoted text values, traversal, function calls, global restrictions, `-` negation, sequences and `*` wildcards are new.
- The AST is new. See the `ast` module.
- The `sea-orm` feature uses SeaORM 2.0 instead of 1.1.
- The `pest` dependency is removed.

## License

MIT or Apache-2.0.

## References

- [Google AIP-160: Filtering](https://google.aip.dev/160)
- [AIP-160 EBNF grammar](https://google.aip.dev/assets/misc/ebnf-filtering.txt)
- [SeaORM documentation](https://www.sea-ql.org/SeaORM/)
