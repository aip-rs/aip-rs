//! Abstract syntax tree for [AIP-160](https://google.aip.dev/160) filters.
//!
//! The types follow the
//! [AIP-160 EBNF](https://google.aip.dev/assets/misc/ebnf-filtering.txt).
//! The parser keeps literal values as the text that the caller wrote.
//! Services decide how to interpret a value, for example as a number, a
//! boolean, an enum value or a duration, with the helpers on [`Value`].

use std::fmt;
use std::time::Duration;

/// A parsed filter.
///
/// An empty filter is valid in AIP-160 and matches every resource. In that
/// case [`Filter::expression`] is `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    /// The root expression, or `None` for an empty filter.
    pub expression: Option<Expression>,
}

/// A filter expression.
///
/// AIP-160 gives `OR` a higher precedence than `AND`, so `a AND b OR c`
/// is `And([a, Or([b, c])])`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A conjunction joined by the `AND` keyword: `a AND b AND c`.
    And(Vec<Expression>),
    /// Factors separated only by whitespace: `New York Giants`.
    ///
    /// Services with exact-match semantics treat a sequence as `AND`.
    /// Services with fuzzy-match semantics may rank results instead.
    Sequence(Vec<Expression>),
    /// A disjunction: `a OR b OR c`.
    Or(Vec<Expression>),
    /// A negation written as `NOT a` or `-a`.
    Not(Box<Expression>),
    /// A comparison such as `a = b` or `labels:env`.
    Restriction(Restriction),
    /// A comparable without a comparator, for example `prod`.
    ///
    /// AIP-160 calls this a global restriction. Services usually treat it
    /// as a free-text search.
    Global(Comparable),
}

/// A comparison between a comparable and an argument.
#[derive(Debug, Clone, PartialEq)]
pub struct Restriction {
    /// The left side, usually a field path.
    pub comparable: Comparable,
    /// The comparison operator.
    pub comparator: Comparator,
    /// The right side.
    pub arg: Arg,
}

/// The comparison operators of AIP-160.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Comparator {
    /// `=`
    Equal,
    /// `!=`
    NotEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `:`, the "has" operator.
    Has,
}

/// The left side of a restriction, or a function argument.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparable {
    /// A value or a field path such as `a.b.c`.
    Member(Member),
    /// A function call such as `regex(m.key, "^prod")`.
    Function(Function),
}

/// A value followed by zero or more `.`-separated fields.
///
/// `expr.type_map.1.type` has the value `expr` and the fields `type_map`,
/// `1` and `type`. A bare literal such as `42` is a member without fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    /// The first segment.
    pub value: Value,
    /// The segments after each `.`.
    pub fields: Vec<Value>,
}

/// A function call.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// The name segments: `math.mem` is `["math", "mem"]`.
    pub name: Vec<String>,
    /// The arguments in order.
    pub args: Vec<Arg>,
}

/// The right side of a restriction, or a function argument.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// A value, field path or function call.
    Comparable(Comparable),
    /// A parenthesized expression.
    Composite(Box<Expression>),
}

/// A literal segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    /// Unquoted text such as `ACTIVE`, `42`, `2.5`, `true` or `20s`.
    Text(String),
    /// A quoted string with its escape sequences resolved.
    String(String),
}

/// How a quoted string matches, based on a leading or trailing `*`.
///
/// AIP-160 allows a `*` wildcard at the start or the end of a quoted
/// string in a restriction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringMatch<'a> {
    /// No wildcard: `"abc"`.
    Exact(&'a str),
    /// A trailing wildcard: `"abc*"`.
    Prefix(&'a str),
    /// A leading wildcard: `"*abc"`.
    Suffix(&'a str),
    /// Wildcards on both sides: `"*abc*"`.
    Contains(&'a str),
}

impl Filter {
    /// Returns `true` when the filter is empty and matches every resource.
    pub fn is_empty(&self) -> bool {
        self.expression.is_none()
    }
}

impl Comparable {
    /// Returns the member when this comparable is not a function call.
    pub fn as_member(&self) -> Option<&Member> {
        match self {
            Comparable::Member(member) => Some(member),
            Comparable::Function(_) => None,
        }
    }

    /// Returns the field path when this comparable is a member whose
    /// segments are all unquoted, for example `labels.env`.
    pub fn field_path(&self) -> Option<String> {
        self.as_member().and_then(Member::field_path)
    }
}

impl Member {
    /// Returns `true` when the member has no `.` fields.
    pub fn is_simple(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns the `.`-joined path when every segment is unquoted text.
    pub fn field_path(&self) -> Option<String> {
        let mut path = String::new();
        for (index, segment) in std::iter::once(&self.value).chain(&self.fields).enumerate() {
            let Value::Text(text) = segment else {
                return None;
            };
            if index > 0 {
                path.push('.');
            }
            path.push_str(text);
        }
        Some(path)
    }
}

impl Arg {
    /// Returns the literal value when the argument is a member without
    /// fields, such as `42`, `ACTIVE` or `"abc"`.
    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Arg::Comparable(Comparable::Member(member)) if member.is_simple() => {
                Some(&member.value)
            }
            _ => None,
        }
    }
}

impl Value {
    /// Returns the text of the value, quoted or not.
    pub fn as_str(&self) -> &str {
        match self {
            Value::Text(text) | Value::String(text) => text,
        }
    }

    /// Returns `true` for a quoted string.
    pub fn is_quoted(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Interprets unquoted `true` or `false` as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Text(text) if text == "true" => Some(true),
            Value::Text(text) if text == "false" => Some(false),
            _ => None,
        }
    }

    /// Interprets unquoted text as a signed integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Text(text) => text.parse().ok(),
            Value::String(_) => None,
        }
    }

    /// Interprets unquoted text as an unsigned integer.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Text(text) => text.parse().ok(),
            Value::String(_) => None,
        }
    }

    /// Interprets unquoted text as a floating-point number.
    ///
    /// Only finite numbers in decimal or exponent notation are accepted, so
    /// `inf` and `NaN` return `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Text(text) if is_decimal_number(text) => {
                text.parse().ok().filter(|n: &f64| n.is_finite())
            }
            _ => None,
        }
    }

    /// Returns `true` for unquoted `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Text(text) if text == "null")
    }

    /// Interprets unquoted text such as `20s` or `1.5s` as a duration.
    ///
    /// AIP-160 writes durations as a number of seconds followed by `s`.
    /// Negative durations return `None`.
    pub fn as_duration(&self) -> Option<Duration> {
        let Value::Text(text) = self else {
            return None;
        };
        let seconds = text.strip_suffix('s')?;
        if !is_decimal_number(seconds) || seconds.starts_with('-') {
            return None;
        }
        Duration::try_from_secs_f64(seconds.parse().ok()?).ok()
    }

    /// Returns how a quoted string matches, based on its `*` wildcards.
    ///
    /// Returns `None` for unquoted text. A string that is only `*` or `**`
    /// matches everything and returns `Contains("")`.
    pub fn string_match(&self) -> Option<StringMatch<'_>> {
        let Value::String(text) = self else {
            return None;
        };
        let leading = text.starts_with('*');
        let trailing = text.len() > usize::from(leading) && text.ends_with('*');
        let start = usize::from(leading);
        let end = text.len() - usize::from(trailing);
        let core = &text[start..end.max(start)];
        Some(match (leading, trailing) {
            (false, false) => StringMatch::Exact(core),
            (false, true) => StringMatch::Prefix(core),
            (true, false) if core.is_empty() => StringMatch::Contains(core),
            (true, false) => StringMatch::Suffix(core),
            (true, true) => StringMatch::Contains(core),
        })
    }
}

/// Returns `true` for `[-]digits[.digits][(e|E)[+|-]digits]`.
fn is_decimal_number(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = match unsigned.find(['e', 'E']) {
        Some(index) => (&unsigned[..index], Some(&unsigned[index + 1..])),
        None => (unsigned, None),
    };
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (mantissa, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let exponent_ok = exponent.is_none_or(|e| digits(e.strip_prefix(['+', '-']).unwrap_or(e)));
    digits(integer) && fraction.is_none_or(digits) && exponent_ok
}

impl fmt::Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Comparator::Equal => "=",
            Comparator::NotEqual => "!=",
            Comparator::GreaterThan => ">",
            Comparator::GreaterThanOrEqual => ">=",
            Comparator::LessThan => "<",
            Comparator::LessThanOrEqual => "<=",
            Comparator::Has => ":",
        })
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Text(text) => f.write_str(text),
            Value::String(text) => {
                f.write_str("\"")?;
                for c in text.chars() {
                    match c {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        '\n' => f.write_str("\\n")?,
                        '\r' => f.write_str("\\r")?,
                        '\t' => f.write_str("\\t")?,
                        c => write!(f, "{c}")?,
                    }
                }
                f.write_str("\"")
            }
        }
    }
}

impl fmt::Display for Member {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)?;
        for field in &self.fields {
            write!(f, ".{field}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name.join("."))?;
        for (index, arg) in self.args.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{arg}")?;
        }
        f.write_str(")")
    }
}

impl fmt::Display for Comparable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Comparable::Member(member) => write!(f, "{member}"),
            Comparable::Function(function) => write!(f, "{function}"),
        }
    }
}

impl fmt::Display for Arg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Arg::Comparable(comparable) => write!(f, "{comparable}"),
            Arg::Composite(expression) => write!(f, "({expression})"),
        }
    }
}

impl fmt::Display for Restriction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.comparable, self.comparator, self.arg)
    }
}

/// Precedence levels used to decide where `Display` needs parentheses.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    And,
    Sequence,
    Or,
    Term,
}

impl Expression {
    fn level(&self) -> Level {
        match self {
            Expression::And(_) => Level::And,
            Expression::Sequence(_) => Level::Sequence,
            Expression::Or(_) => Level::Or,
            Expression::Not(_) | Expression::Restriction(_) | Expression::Global(_) => Level::Term,
        }
    }

    /// Writes `self` as a child that must bind tighter than `parent`.
    fn fmt_child(&self, f: &mut fmt::Formatter<'_>, parent: Level) -> fmt::Result {
        if self.level() > parent {
            write!(f, "{self}")
        } else {
            write!(f, "({self})")
        }
    }

    fn fmt_joined(
        f: &mut fmt::Formatter<'_>,
        children: &[Expression],
        separator: &str,
        level: Level,
    ) -> fmt::Result {
        for (index, child) in children.iter().enumerate() {
            if index > 0 {
                f.write_str(separator)?;
            }
            child.fmt_child(f, level)?;
        }
        Ok(())
    }
}

impl fmt::Display for Expression {
    /// Writes the expression in AIP-160 syntax. Parsing the output yields an
    /// equal expression.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::And(children) => Self::fmt_joined(f, children, " AND ", Level::And),
            Expression::Sequence(children) => Self::fmt_joined(f, children, " ", Level::Sequence),
            Expression::Or(children) => Self::fmt_joined(f, children, " OR ", Level::Or),
            Expression::Not(inner) => match inner.as_ref() {
                Expression::Restriction(_) | Expression::Global(_) => write!(f, "NOT {inner}"),
                _ => write!(f, "NOT ({inner})"),
            },
            Expression::Restriction(restriction) => write!(f, "{restriction}"),
            Expression::Global(comparable) => write!(f, "{comparable}"),
        }
    }
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.expression {
            Some(expression) => write!(f, "{expression}"),
            None => Ok(()),
        }
    }
}
