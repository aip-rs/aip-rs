//! Converts filters to SeaORM conditions.
//!
//! The conversion supports restrictions on top-level columns, joined with
//! `AND`, `OR`, `NOT` and sequences. Sequences use exact-match semantics,
//! so they behave like `AND`. Each literal is interpreted with the type of
//! its column:
//!
//! | Column type | Accepted literal |
//! | --- | --- |
//! | boolean | `true`, `false` |
//! | signed or unsigned integer | `42`, `-7` |
//! | float, double, decimal, money | `2.5`, `1e3` |
//! | enum | `ACTIVE` or `"ACTIVE"`, cast to the database enum type |
//! | uuid, timestamp, date, time | a quoted string, cast to the column type |
//! | other types | text or a quoted string |
//!
//! `field = null` and `field != null` become `IS NULL` and `IS NOT NULL`.
//! On string columns, `field = "abc*"`, `"*abc"` and `"*abc*"` become
//! `LIKE` patterns, and `field:"abc"` matches a substring.

use std::fmt::Display;
use std::str::FromStr;

use sea_orm::sea_query::{Alias, ColumnType, Expr, ExprTrait, LikeExpr};
use sea_orm::{ColumnTrait, Condition, EntityTrait, IntoSimpleExpr};

use crate::ast::{Comparator, Expression, Filter, Restriction, StringMatch, Value};
use crate::error::{FilterError, Result};

/// Resolves a field name to a column expression.
///
/// The name may match the column directly, or match after a conversion
/// from `snake_case` to `PascalCase`.
///
/// # Errors
///
/// Returns [`FilterError::InvalidField`] when no column matches.
pub fn column_from_str<C>(field: &str) -> Result<Expr>
where
    C: FromStr + IntoSimpleExpr,
    C::Err: Display,
{
    get_column::<C>(field).map(IntoSimpleExpr::into_simple_expr)
}

/// Converts a parsed filter to a SeaORM [`Condition`] for entity `E`.
///
/// # Example
///
/// ```ignore
/// use aip_160::{parse_filter, ToSeaOrmCondition};
///
/// let filter = parse_filter(r#"name = "Alice" AND age > 18"#)?;
/// let condition = filter.to_condition::<user::Entity>()?;
/// user::Entity::find().filter(condition).all(db).await?;
/// ```
pub trait ToSeaOrmCondition {
    /// Builds the condition. An empty filter yields a condition that
    /// matches every row.
    ///
    /// # Errors
    ///
    /// Returns [`FilterError::InvalidField`] for an unknown column,
    /// [`FilterError::InvalidValue`] for a literal that does not fit its
    /// column, and [`FilterError::Unsupported`] for global restrictions,
    /// field traversal, functions and composite arguments.
    fn to_condition<E>(&self) -> Result<Condition>
    where
        E: EntityTrait,
        E::Column: FromStr,
        <E::Column as FromStr>::Err: Display;
}

impl ToSeaOrmCondition for Filter {
    fn to_condition<E>(&self) -> Result<Condition>
    where
        E: EntityTrait,
        E::Column: FromStr,
        <E::Column as FromStr>::Err: Display,
    {
        match &self.expression {
            Some(expression) => expression.to_condition::<E>(),
            None => Ok(Condition::all()),
        }
    }
}

impl ToSeaOrmCondition for Expression {
    fn to_condition<E>(&self) -> Result<Condition>
    where
        E: EntityTrait,
        E::Column: FromStr,
        <E::Column as FromStr>::Err: Display,
    {
        match self {
            Expression::And(children) | Expression::Sequence(children) => children
                .iter()
                .try_fold(Condition::all(), |condition, child| {
                    Ok(condition.add(child.to_condition::<E>()?))
                }),
            Expression::Or(children) => children
                .iter()
                .try_fold(Condition::any(), |condition, child| {
                    Ok(condition.add(child.to_condition::<E>()?))
                }),
            Expression::Not(inner) => Ok(inner.to_condition::<E>()?.not()),
            Expression::Restriction(restriction) => restriction_condition::<E>(restriction),
            Expression::Global(comparable) => Err(FilterError::Unsupported(format!(
                "global restriction `{comparable}` needs a free-text search"
            ))),
        }
    }
}

fn get_column<C>(field: &str) -> Result<C>
where
    C: FromStr,
    C::Err: Display,
{
    if let Ok(column) = field.parse::<C>() {
        return Ok(column);
    }
    let pascal_case: String = field
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect();
    pascal_case
        .parse::<C>()
        .map_err(|error| FilterError::InvalidField(format!("{field}: {error}")))
}

fn restriction_condition<E>(restriction: &Restriction) -> Result<Condition>
where
    E: EntityTrait,
    E::Column: FromStr,
    <E::Column as FromStr>::Err: Display,
{
    let field = match restriction.comparable.as_member() {
        Some(member) if member.is_simple() && !member.value.is_quoted() => member.value.as_str(),
        _ => {
            return Err(FilterError::Unsupported(format!(
                "`{}` is not a top-level field",
                restriction.comparable
            )));
        }
    };
    let Some(value) = restriction.arg.as_value() else {
        return Err(FilterError::Unsupported(format!(
            "the argument of `{field}` must be a literal, not `{}`",
            restriction.arg
        )));
    };
    let column = get_column::<E::Column>(field)?;
    let column_type = column.def().get_column_type().clone();
    let target = column.into_expr();
    let comparator = restriction.comparator;

    if value.is_null() {
        return match comparator {
            Comparator::Equal => Ok(Condition::all().add(target.is_null())),
            Comparator::NotEqual => Ok(Condition::all().add(target.is_not_null())),
            _ => Err(invalid(field, "null only supports = and !=")),
        };
    }

    if is_string_column(&column_type)
        && let Some(pattern) = like_pattern(comparator, value)
    {
        let like = LikeExpr::new(pattern).escape('\\');
        let condition = match comparator {
            Comparator::NotEqual => target.not_like(like),
            _ => target.like(like),
        };
        return Ok(Condition::all().add(condition));
    }

    let literal = literal(field, value, &column_type)?;
    let condition = match comparator {
        Comparator::Equal => target.eq(literal),
        Comparator::NotEqual => target.ne(literal),
        Comparator::GreaterThan => target.gt(literal),
        Comparator::GreaterThanOrEqual => target.gte(literal),
        Comparator::LessThan => target.lt(literal),
        Comparator::LessThanOrEqual => target.lte(literal),
        // String columns returned a LIKE condition above.
        Comparator::Has => {
            return Err(FilterError::Unsupported(format!(
                "`{field}:` needs a string column"
            )));
        }
    };
    Ok(Condition::all().add(condition))
}

fn is_string_column(column_type: &ColumnType) -> bool {
    matches!(
        column_type,
        ColumnType::Char(_) | ColumnType::String(_) | ColumnType::Text
    )
}

/// Returns a `LIKE` pattern for `:` and for `=` or `!=` with a wildcard.
fn like_pattern(comparator: Comparator, value: &Value) -> Option<String> {
    let escape = |text: &str| {
        text.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    };
    if comparator == Comparator::Has {
        return Some(format!("%{}%", escape(value.as_str())));
    }
    if !matches!(comparator, Comparator::Equal | Comparator::NotEqual) {
        return None;
    }
    match value.string_match()? {
        StringMatch::Exact(_) => None,
        StringMatch::Prefix(core) => Some(format!("{}%", escape(core))),
        StringMatch::Suffix(core) => Some(format!("%{}", escape(core))),
        StringMatch::Contains(core) => Some(format!("%{}%", escape(core))),
    }
}

fn invalid(field: &str, message: &str) -> FilterError {
    FilterError::InvalidValue {
        field: field.to_owned(),
        message: message.to_owned(),
    }
}

/// Interprets `value` with the type of its column.
fn literal(field: &str, value: &Value, column_type: &ColumnType) -> Result<Expr> {
    let cast = |type_name: &str| {
        if value.is_quoted() {
            Ok(Expr::val(value.as_str()).cast_as(Alias::new(type_name)))
        } else {
            Err(invalid(field, "expected a quoted string"))
        }
    };
    match column_type {
        ColumnType::Boolean => value
            .as_bool()
            .map(Expr::val)
            .ok_or_else(|| invalid(field, "expected true or false")),
        ColumnType::TinyInteger
        | ColumnType::SmallInteger
        | ColumnType::Integer
        | ColumnType::BigInteger => value
            .as_i64()
            .map(Expr::val)
            .ok_or_else(|| invalid(field, "expected an integer")),
        ColumnType::TinyUnsigned
        | ColumnType::SmallUnsigned
        | ColumnType::Unsigned
        | ColumnType::BigUnsigned => value
            .as_u64()
            .map(Expr::val)
            .ok_or_else(|| invalid(field, "expected an unsigned integer")),
        ColumnType::Float | ColumnType::Double | ColumnType::Decimal(_) | ColumnType::Money(_) => {
            value
                .as_f64()
                .map(Expr::val)
                .ok_or_else(|| invalid(field, "expected a number"))
        }
        ColumnType::Uuid => cast("uuid"),
        ColumnType::TimestampWithTimeZone => cast("timestamptz"),
        ColumnType::Timestamp | ColumnType::DateTime => cast("timestamp"),
        ColumnType::Date => cast("date"),
        ColumnType::Time => cast("time"),
        ColumnType::Enum { name, .. } => {
            Ok(Expr::val(value.as_str()).cast_as(Alias::new(name.to_string())))
        }
        _ => Ok(Expr::val(value.as_str())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum MockColumn {
        Name,
        UserName,
        CreatedAt,
    }

    impl FromStr for MockColumn {
        type Err = String;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            match s {
                "Name" => Ok(MockColumn::Name),
                "UserName" => Ok(MockColumn::UserName),
                "CreatedAt" => Ok(MockColumn::CreatedAt),
                _ => Err(format!("unknown column {s}")),
            }
        }
    }

    #[test]
    fn resolves_snake_case_and_direct_names() {
        assert_eq!(
            get_column::<MockColumn>("user_name"),
            Ok(MockColumn::UserName)
        );
        assert_eq!(
            get_column::<MockColumn>("created_at"),
            Ok(MockColumn::CreatedAt)
        );
        assert_eq!(get_column::<MockColumn>("Name"), Ok(MockColumn::Name));
        assert!(matches!(
            get_column::<MockColumn>("missing"),
            Err(FilterError::InvalidField(_))
        ));
    }

    #[test]
    fn builds_escaped_like_patterns() {
        let s = |text: &str| Value::String(text.into());
        assert_eq!(
            like_pattern(Comparator::Has, &s("50%_off")),
            Some(r"%50\%\_off%".into())
        );
        assert_eq!(
            like_pattern(Comparator::Equal, &s("ab*")),
            Some("ab%".into())
        );
        assert_eq!(
            like_pattern(Comparator::NotEqual, &s("*ab")),
            Some("%ab".into())
        );
        assert_eq!(
            like_pattern(Comparator::Equal, &s("*ab*")),
            Some("%ab%".into())
        );
        assert_eq!(like_pattern(Comparator::Equal, &s("ab")), None);
        assert_eq!(
            like_pattern(Comparator::Equal, &Value::Text("ab*".into())),
            None
        );
        assert_eq!(like_pattern(Comparator::GreaterThan, &s("ab*")), None);
    }
}
