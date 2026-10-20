//! A parser for [AIP-160](https://google.aip.dev/160) list filters, with an
//! optional conversion to SeaORM conditions.
//!
//! The parser follows the
//! [AIP-160 EBNF](https://google.aip.dev/assets/misc/ebnf-filtering.txt):
//!
//! - `OR` binds tighter than whitespace sequences, which bind tighter than
//!   `AND`. `a AND b OR c` means `a AND (b OR c)`.
//! - `AND`, `OR` and `NOT` are case-sensitive keywords.
//! - `NOT a` and `-a` negate a term.
//! - Comparators: `=`, `!=`, `<`, `<=`, `>`, `>=` and `:` (has).
//! - A comparable without a comparator, such as `prod`, is a global
//!   restriction.
//! - Members traverse fields with `.`, as in `labels.env`.
//! - Function calls such as `regex(name, "^prod")` are parsed; services
//!   decide which functions they support.
//! - Values are unquoted text (`ACTIVE`, `42`, `2.5`, `true`, `20s`) or quoted
//!   strings (`"abc"`, `'abc'`). Quoted strings may start or end with the
//!   `*` wildcard.
//!
//! The parser keeps literals as written. [`Value`] has helpers to read them
//! as booleans, numbers, durations or wildcard matches.
//!
//! # Example
//!
//! ```
//! use aip_160::{parse_filter, Comparator, Expression};
//!
//! let filter = parse_filter("state = SUSPENDED AND score >= 2.5").unwrap();
//! let Some(Expression::And(children)) = &filter.expression else { panic!() };
//! let Expression::Restriction(state) = &children[0] else { panic!() };
//! assert_eq!(state.comparable.field_path().as_deref(), Some("state"));
//! assert_eq!(state.comparator, Comparator::Equal);
//! assert_eq!(state.arg.as_value().map(|v| v.as_str()), Some("SUSPENDED"));
//! ```
//!
//! # Features
//!
//! - `sea-orm`: converts a [`Filter`] to a `sea_orm::Condition` with
//!   [`ToSeaOrmCondition`]. See the [`sea_orm`] module for
//!   the supported subset.

pub mod ast;
pub mod error;
pub mod parser;

#[cfg(feature = "sea-orm")]
pub mod sea_orm;

pub use ast::{
    Arg, Comparable, Comparator, Expression, Filter, Function, Member, Restriction, StringMatch,
    Value,
};
pub use error::{FilterError, Result};
pub use parser::{DEFAULT_MAX_DEPTH, parse_filter, parse_filter_with_max_depth};

#[cfg(feature = "sea-orm")]
pub use sea_orm::{ToSeaOrmCondition, column_from_str};
