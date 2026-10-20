//! A recursive-descent parser for the
//! [AIP-160 grammar](https://google.aip.dev/assets/misc/ebnf-filtering.txt).
//!
//! The parser works on characters, not on a separate token stream, because
//! two rules depend on context: `-` negates a term at the start of a term
//! but belongs to the text of an argument (`a > -30`), and the keywords
//! `AND`, `OR` and `NOT` are only keywords when whitespace surrounds them.

use crate::ast::{
    Arg, Comparable, Comparator, Expression, Filter, Function, Member, Restriction, Value,
};
use crate::error::{FilterError, Result};

/// The default limit for nested parentheses and function calls.
pub const DEFAULT_MAX_DEPTH: usize = 32;

/// Parses a filter with the default nesting limit.
///
/// # Errors
///
/// Returns [`FilterError::Syntax`] when the input does not follow the
/// AIP-160 grammar, and [`FilterError::TooDeep`] when it nests more than
/// [`DEFAULT_MAX_DEPTH`] levels.
///
/// # Examples
///
/// ```
/// use aip_160::{parse_filter, Expression};
///
/// // OR binds tighter than AND.
/// let filter = parse_filter("a = 1 AND b = 2 OR c = 3").unwrap();
/// let Some(Expression::And(children)) = &filter.expression else { panic!() };
/// assert!(matches!(children[1], Expression::Or(_)));
/// assert_eq!(filter.to_string(), "a = 1 AND b = 2 OR c = 3");
/// ```
pub fn parse_filter(input: &str) -> Result<Filter> {
    parse_filter_with_max_depth(input, DEFAULT_MAX_DEPTH)
}

/// Parses a filter and rejects nesting deeper than `max_depth`.
///
/// Each parenthesized expression and each function call adds one level.
///
/// # Errors
///
/// See [`parse_filter`].
pub fn parse_filter_with_max_depth(input: &str, max_depth: usize) -> Result<Filter> {
    let mut parser = Parser {
        input,
        pos: 0,
        depth: 0,
        max_depth,
    };
    parser.skip_ws();
    if parser.at_end() {
        return Ok(Filter { expression: None });
    }
    let expression = parser.expression()?;
    parser.skip_ws();
    if !parser.at_end() {
        return Err(parser.error("unexpected input"));
    }
    Ok(Filter {
        expression: Some(expression),
    })
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    depth: usize,
    max_depth: usize,
}

/// Characters that end unquoted text.
fn is_text_char(c: char) -> bool {
    !c.is_whitespace()
        && !matches!(
            c,
            '.' | '(' | ')' | ',' | '"' | '\'' | '=' | '<' | '>' | '!' | ':'
        )
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn at_end(&self) -> bool {
        self.pos == self.input.len()
    }

    fn bump(&mut self, c: char) {
        self.pos += c.len_utf8();
    }

    /// Skips whitespace and returns `true` if it skipped any.
    fn skip_ws(&mut self) -> bool {
        let start = self.pos;
        while let Some(c) = self.peek().filter(|c| c.is_whitespace()) {
            self.bump(c);
        }
        self.pos > start
    }

    fn error(&self, message: impl Into<String>) -> FilterError {
        FilterError::Syntax {
            offset: self.pos,
            message: message.into(),
        }
    }

    /// Returns `true` when `keyword` starts here and whitespace follows it.
    fn at_keyword(&self, keyword: &str) -> bool {
        self.rest()
            .strip_prefix(keyword)
            .and_then(|after| after.chars().next())
            .is_some_and(char::is_whitespace)
    }

    /// Consumes `WS keyword WS` and returns `true`, or consumes nothing.
    fn eat_infix_keyword(&mut self, keyword: &str) -> bool {
        let start = self.pos;
        if self.skip_ws() && self.at_keyword(keyword) {
            self.pos += keyword.len();
            self.skip_ws();
            return true;
        }
        self.pos = start;
        false
    }

    fn enter(&mut self) -> Result<()> {
        if self.depth >= self.max_depth {
            return Err(FilterError::TooDeep {
                limit: self.max_depth,
            });
        }
        self.depth += 1;
        Ok(())
    }

    fn expect(&mut self, expected: char) -> Result<()> {
        if self.peek() == Some(expected) {
            self.bump(expected);
            Ok(())
        } else {
            Err(self.error(format!("expected '{expected}'")))
        }
    }

    /// `expression: sequence {WS AND WS sequence}`
    fn expression(&mut self) -> Result<Expression> {
        let mut children = vec![self.sequence()?];
        while self.eat_infix_keyword("AND") {
            children.push(self.sequence()?);
        }
        Ok(collapse(children, Expression::And))
    }

    /// `sequence: factor {WS factor}`
    fn sequence(&mut self) -> Result<Expression> {
        let mut children = vec![self.factor()?];
        loop {
            let start = self.pos;
            if !self.skip_ws()
                || self.at_end()
                || self.peek() == Some(')')
                || self.at_keyword("AND")
            {
                self.pos = start;
                break;
            }
            children.push(self.factor()?);
        }
        Ok(collapse(children, Expression::Sequence))
    }

    /// `factor: term {WS OR WS term}`
    fn factor(&mut self) -> Result<Expression> {
        let mut children = vec![self.term()?];
        while self.eat_infix_keyword("OR") {
            children.push(self.term()?);
        }
        Ok(collapse(children, Expression::Or))
    }

    /// `term: [(NOT WS | MINUS)] simple`
    fn term(&mut self) -> Result<Expression> {
        if self.at_keyword("NOT") {
            self.pos += "NOT".len();
            self.skip_ws();
            return Ok(Expression::Not(Box::new(self.simple()?)));
        }
        if self.peek() == Some('-') {
            self.bump('-');
            return Ok(Expression::Not(Box::new(self.simple()?)));
        }
        self.simple()
    }

    /// `simple: restriction | composite`
    fn simple(&mut self) -> Result<Expression> {
        if self.peek() == Some('(') {
            return self.composite();
        }
        let comparable = self.comparable()?;
        let start = self.pos;
        self.skip_ws();
        let Some(comparator) = self.comparator() else {
            self.pos = start;
            return Ok(Expression::Global(comparable));
        };
        self.skip_ws();
        let arg = self.arg()?;
        Ok(Expression::Restriction(Restriction {
            comparable,
            comparator,
            arg,
        }))
    }

    /// `composite: LPAREN expression RPAREN`
    fn composite(&mut self) -> Result<Expression> {
        self.enter()?;
        self.bump('(');
        self.skip_ws();
        let expression = self.expression()?;
        self.skip_ws();
        self.expect(')')?;
        self.depth -= 1;
        Ok(expression)
    }

    fn comparator(&mut self) -> Option<Comparator> {
        const COMPARATORS: [(&str, Comparator); 7] = [
            ("<=", Comparator::LessThanOrEqual),
            ("<", Comparator::LessThan),
            (">=", Comparator::GreaterThanOrEqual),
            (">", Comparator::GreaterThan),
            ("!=", Comparator::NotEqual),
            ("=", Comparator::Equal),
            (":", Comparator::Has),
        ];
        let (symbol, comparator) = COMPARATORS
            .into_iter()
            .find(|(symbol, _)| self.rest().starts_with(symbol))?;
        self.pos += symbol.len();
        Some(comparator)
    }

    /// `arg: comparable | composite`
    fn arg(&mut self) -> Result<Arg> {
        if self.peek() == Some('(') {
            return Ok(Arg::Composite(Box::new(self.composite()?)));
        }
        Ok(Arg::Comparable(self.comparable()?))
    }

    /// `comparable: member | function`, where
    /// `member: value {DOT field}` and
    /// `function: name {DOT name} LPAREN [argList] RPAREN`.
    fn comparable(&mut self) -> Result<Comparable> {
        let start = self.pos;
        let value = self.value()?;
        let mut fields = Vec::new();
        while self.peek() == Some('.') {
            self.bump('.');
            fields.push(self.value()?);
        }
        if self.peek() == Some('(') {
            return self.function(start, value, fields);
        }
        if let Value::Text(text) = &value
            && matches!(text.as_str(), "AND" | "OR" | "NOT")
        {
            return Err(FilterError::Syntax {
                offset: start,
                message: format!("unexpected keyword {text}"),
            });
        }
        Ok(Comparable::Member(merge_number(Member { value, fields })))
    }

    fn function(&mut self, start: usize, value: Value, fields: Vec<Value>) -> Result<Comparable> {
        let mut name = Vec::with_capacity(fields.len() + 1);
        for segment in std::iter::once(value).chain(fields) {
            match segment {
                Value::Text(text) => name.push(text),
                Value::String(_) => {
                    return Err(FilterError::Syntax {
                        offset: start,
                        message: "a function name cannot be quoted".into(),
                    });
                }
            }
        }
        self.enter()?;
        self.bump('(');
        self.skip_ws();
        let mut args = Vec::new();
        if self.peek() != Some(')') {
            loop {
                args.push(self.arg()?);
                self.skip_ws();
                if self.peek() != Some(',') {
                    break;
                }
                self.bump(',');
                self.skip_ws();
            }
        }
        self.expect(')')?;
        self.depth -= 1;
        Ok(Comparable::Function(Function { name, args }))
    }

    /// `value: TEXT | STRING`
    fn value(&mut self) -> Result<Value> {
        match self.peek() {
            Some(quote @ ('"' | '\'')) => self.string(quote),
            Some(c) if is_text_char(c) => {
                let start = self.pos;
                while let Some(c) = self.peek().filter(|&c| is_text_char(c)) {
                    self.bump(c);
                }
                Ok(Value::Text(self.input[start..self.pos].to_owned()))
            }
            Some(_) => Err(self.error("expected a value")),
            None => Err(self.error("unexpected end of filter")),
        }
    }

    fn string(&mut self, quote: char) -> Result<Value> {
        let start = self.pos;
        self.bump(quote);
        let mut text = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(FilterError::Syntax {
                    offset: start,
                    message: "unterminated string".into(),
                });
            };
            self.bump(c);
            match c {
                c if c == quote => return Ok(Value::String(text)),
                '\\' => {
                    let Some(escaped) = self.peek() else {
                        continue;
                    };
                    self.bump(escaped);
                    text.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    });
                }
                c => text.push(c),
            }
        }
    }
}

/// Wraps several children with `wrap`, or returns a single child as is.
fn collapse(mut children: Vec<Expression>, wrap: fn(Vec<Expression>) -> Expression) -> Expression {
    if children.len() == 1 {
        children.remove(0)
    } else {
        wrap(children)
    }
}

/// Joins `2` `.` `5` into the number `2.5`, and `1` `.` `5s` into the
/// duration `1.5s`.
///
/// The EBNF excludes `.` from TEXT, so a decimal number arrives as a member
/// with one field. Services almost always mean a number in that case.
fn merge_number(mut member: Member) -> Member {
    let merged = match (&member.value, member.fields.first()) {
        (Value::Text(integer), Some(Value::Text(fraction))) => {
            let unsigned = integer.strip_prefix('-').unwrap_or(integer);
            let is_integer = !unsigned.is_empty() && unsigned.bytes().all(|b| b.is_ascii_digit());
            let is_fraction = fraction.starts_with(|c: char| c.is_ascii_digit());
            (is_integer && is_fraction).then(|| format!("{integer}.{fraction}"))
        }
        _ => None,
    };
    if let Some(merged) = merged {
        member.fields.remove(0);
        member.value = Value::Text(merged);
    }
    member
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Value {
        Value::Text(s.into())
    }

    fn member(s: &str) -> Comparable {
        Comparable::Member(Member {
            value: text(s),
            fields: vec![],
        })
    }

    fn restriction(field: &str, comparator: Comparator, arg: Value) -> Expression {
        Expression::Restriction(Restriction {
            comparable: member(field),
            comparator,
            arg: Arg::Comparable(Comparable::Member(Member {
                value: arg,
                fields: vec![],
            })),
        })
    }

    fn parse(input: &str) -> Expression {
        parse_filter(input).unwrap().expression.unwrap()
    }

    #[test]
    fn empty_filter_matches_everything() {
        assert!(parse_filter("").unwrap().is_empty());
        assert!(parse_filter("  \t\n").unwrap().is_empty());
    }

    #[test]
    fn or_binds_tighter_than_and() {
        let a = restriction("a", Comparator::Equal, text("1"));
        let b = restriction("b", Comparator::Equal, text("2"));
        let c = restriction("c", Comparator::Equal, text("3"));
        assert_eq!(
            parse("a = 1 AND b = 2 OR c = 3"),
            Expression::And(vec![a.clone(), Expression::Or(vec![b.clone(), c.clone()])])
        );
        assert_eq!(
            parse("a = 1 OR b = 2 AND c = 3"),
            Expression::And(vec![Expression::Or(vec![a, b]), c])
        );
    }

    #[test]
    fn sequence_binds_between_and_and_or() {
        assert_eq!(
            parse("New York Giants OR Yankees"),
            Expression::Sequence(vec![
                Expression::Global(member("New")),
                Expression::Global(member("York")),
                Expression::Or(vec![
                    Expression::Global(member("Giants")),
                    Expression::Global(member("Yankees")),
                ]),
            ])
        );
        assert_eq!(
            parse("a b AND c"),
            Expression::And(vec![
                Expression::Sequence(vec![
                    Expression::Global(member("a")),
                    Expression::Global(member("b")),
                ]),
                Expression::Global(member("c")),
            ])
        );
    }

    #[test]
    fn unquoted_enum_values_and_literals() {
        assert_eq!(
            parse("state = SUSPENDED"),
            restriction("state", Comparator::Equal, text("SUSPENDED"))
        );
        assert_eq!(
            parse("score >= 2.5"),
            restriction("score", Comparator::GreaterThanOrEqual, text("2.5"))
        );
        assert_eq!(
            parse("delta > -30"),
            restriction("delta", Comparator::GreaterThan, text("-30"))
        );
        assert_eq!(
            parse("ttl < 1.5s"),
            restriction("ttl", Comparator::LessThan, text("1.5s"))
        );
    }

    #[test]
    fn restrictions_ignore_whitespace_around_comparators() {
        assert_eq!(parse("a=b"), parse("a = b"));
        assert_eq!(parse("a!=b"), parse("a  !=  b"));
        assert_eq!(parse("labels:env"), parse("labels : env"));
    }

    #[test]
    fn negation_forms() {
        let inner = restriction("file", Comparator::Has, Value::String(".java".into()));
        assert_eq!(
            parse(r#"-file:".java""#),
            Expression::Not(Box::new(inner.clone()))
        );
        assert_eq!(
            parse(r#"NOT file:".java""#),
            Expression::Not(Box::new(inner))
        );
        assert_eq!(
            parse("NOT (a OR b)"),
            Expression::Not(Box::new(Expression::Or(vec![
                Expression::Global(member("a")),
                Expression::Global(member("b")),
            ])))
        );
    }

    #[test]
    fn keywords_are_case_sensitive() {
        assert_eq!(
            parse("a and b"),
            Expression::Sequence(vec![
                Expression::Global(member("a")),
                Expression::Global(member("and")),
                Expression::Global(member("b")),
            ])
        );
    }

    #[test]
    fn member_traversal_and_functions() {
        let Expression::Restriction(r) = parse("expr.type_map.1.type = x") else {
            panic!("expected a restriction");
        };
        assert_eq!(
            r.comparable.field_path().as_deref(),
            Some("expr.type_map.1.type")
        );

        let Expression::Global(Comparable::Function(f)) = parse("regex(m.key, '^.*prod.*$')")
        else {
            panic!("expected a function");
        };
        assert_eq!(f.name, ["regex"]);
        assert_eq!(f.args.len(), 2);
        assert_eq!(
            f.args[1].as_value(),
            Some(&Value::String("^.*prod.*$".into()))
        );

        let Expression::Global(Comparable::Function(f)) = parse("math.mem('30mb')") else {
            panic!("expected a function");
        };
        assert_eq!(f.name, ["math", "mem"]);
    }

    #[test]
    fn strings_resolve_escapes() {
        assert_eq!(
            parse(r#"msg = "say \"hi\"\n""#),
            restriction(
                "msg",
                Comparator::Equal,
                Value::String("say \"hi\"\n".into())
            )
        );
        assert_eq!(
            parse(r"msg = 'it\'s'"),
            restriction("msg", Comparator::Equal, Value::String("it's".into()))
        );
    }

    #[test]
    fn display_round_trips() {
        for input in [
            "a = 1 AND b = 2 OR c = 3",
            "(a = 1 AND b = 2) OR c = 3",
            "a b AND c",
            "a (b AND c)",
            "NOT (a OR b)",
            "NOT a = \"x\"",
            "regex(m.key, \"^prod\") AND x.y:z",
            "a = (b OR c)",
        ] {
            let filter = parse_filter(input).unwrap();
            let printed = filter.to_string();
            assert_eq!(
                parse_filter(&printed).unwrap(),
                filter,
                "{input} -> {printed}"
            );
        }
    }

    #[test]
    fn rejects_invalid_input() {
        for input in [
            "a =", "a AND", "AND a", "(a = 1", "a = 1)", "\"open", "a = = b", "f(a,", "a OR",
        ] {
            assert!(
                matches!(parse_filter(input), Err(FilterError::Syntax { .. })),
                "{input} should fail, got {:?}",
                parse_filter(input)
            );
        }
    }

    #[test]
    fn limits_nesting_depth() {
        let deep = format!("{}a{}", "(".repeat(40), ")".repeat(40));
        assert_eq!(
            parse_filter(&deep),
            Err(FilterError::TooDeep {
                limit: DEFAULT_MAX_DEPTH
            })
        );
        assert!(parse_filter_with_max_depth(&deep, 40).is_ok());
    }
}
