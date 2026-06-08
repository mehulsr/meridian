//! Hand-written recursive descent SQL-ish parser.
//!
//! Port of `meridian.query.parser`. Lexer extracts tokens with regex;
//! parser does recursive descent. AND-flattening simplified to collect
//! parts into a vec (matches observable output per PLAYBOOK §3.3).

use std::collections::VecDeque;

use regex::Regex;

use crate::core::enums::{AggFunc, CompareOp};
use crate::core::errors::{MeridianError, Result};
use crate::core::types::{DataType, Value};
use crate::query::ast::{
    AggExpr, AndExpr, BinaryExpr, ColumnRef, Expr, JoinQuery, JoinSpec, LiteralExpr, OrderByItem,
    Query, SelectItem, SelectQuery,
};

/// Lazy-initialized TOKEN_RE regex.
fn token_regex() -> &'static Regex {
    Box::leak(Box::new(
        Regex::new(
            r#"(?i)\s*(?P<TOKEN>SELECT|FROM|WHERE|GROUP|BY|HAVING|ORDER|LIMIT|JOIN|ON|INNER|ASC|DESC|AS|AND|COUNT|SUM|AVG|MIN|MAX|=|!=|<>|<=|>=|<|>|\(|\)|,|\*|"[^"]*"|'[^']*'|-?\d+(?:\.\d+)?|[a-zA-Z_][a-zA-Z0-9_]*)"#,
        )
        .unwrap(),
    ))
}

/// Port of the `Lexer` class.
struct Lexer {
    tokens: VecDeque<String>,
}

impl Lexer {
    fn new(text: &str) -> Lexer {
        let regex = token_regex();
        let tokens = regex
            .captures_iter(text)
            .filter_map(|cap| cap.name("TOKEN").map(|m| m.as_str().to_string()))
            .collect();
        Lexer { tokens }
    }

    fn peek(&self) -> Option<String> {
        self.tokens.front().cloned()
    }

    fn consume(&mut self, expected: Option<&str>) -> Result<String> {
        let tok = self.tokens.pop_front().ok_or_else(|| {
            MeridianError::Parse("unexpected end of input".to_string())
        })?;
        if let Some(exp) = expected {
            if tok.to_uppercase() != exp.to_uppercase() {
                return Err(MeridianError::Parse(format!("expected {exp}, got {tok}")));
            }
        }
        Ok(tok)
    }

    fn match_any(&mut self, options: &[&str]) -> Option<String> {
        let tok = self.peek()?;
        for opt in options {
            if tok.to_uppercase() == opt.to_uppercase() {
                return self.tokens.pop_front();
            }
        }
        None
    }
}

/// Port of the `Parser` class.
pub struct Parser {
    lex: Lexer,
}

impl Parser {
    fn new(text: &str) -> Parser {
        Parser { lex: Lexer::new(text) }
    }

    fn parse(&mut self) -> Result<Query> {
        let query = self.parse_select()?;
        if self.maybe_join() {
            let join_spec = self.parse_join()?;
            let right = self.parse_select()?;
            return Ok(Query::Join(JoinQuery { left: query, join: join_spec, right }));
        }
        Ok(Query::Select(query))
    }

    fn maybe_join(&self) -> bool {
        if let Some(tok) = self.lex.peek() {
            let upper = tok.to_uppercase();
            upper == "JOIN" || upper == "INNER"
        } else {
            false
        }
    }

    fn parse_join(&mut self) -> Result<JoinSpec> {
        self.lex.match_any(&["INNER"]);
        self.lex.consume(Some("JOIN"))?;
        let table = self.lex.consume(None)?;
        self.lex.consume(Some("ON"))?;
        let left = self.lex.consume(None)?;
        self.lex.consume(Some("="))?;
        let right = self.lex.consume(None)?;
        Ok(JoinSpec::new(table, left, right))
    }

    fn parse_select(&mut self) -> Result<SelectQuery> {
        self.lex.consume(Some("SELECT"))?;
        let items = self.parse_select_list()?;
        self.lex.consume(Some("FROM"))?;
        let table = self.lex.consume(None)?;

        let where_clause = if self.lex.match_any(&["WHERE"]).is_some() {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let mut group_by = Vec::new();
        if self.lex.match_any(&["GROUP"]).is_some() {
            self.lex.consume(Some("BY"))?;
            group_by = self.parse_column_list()?;
        }

        let having = if self.lex.match_any(&["HAVING"]).is_some() {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let mut order_by = Vec::new();
        if self.lex.match_any(&["ORDER"]).is_some() {
            self.lex.consume(Some("BY"))?;
            order_by = self.parse_order_list()?;
        }

        let limit = if self.lex.match_any(&["LIMIT"]).is_some() {
            let tok = self.lex.consume(None)?;
            Some(tok.parse::<usize>().map_err(|_| {
                MeridianError::Parse(format!("invalid limit: {tok}"))
            })?)
        } else {
            None
        };

        let mut query = SelectQuery::new(items, table);
        query.where_clause = where_clause;
        query.group_by = group_by;
        query.having = having;
        query.order_by = order_by;
        query.limit = limit;
        Ok(query)
    }

    fn parse_select_list(&mut self) -> Result<Vec<SelectItem>> {
        let mut items = Vec::new();
        loop {
            let item = if self.lex.match_any(&["*"]).is_some() {
                SelectItem::new(Expr::Column(ColumnRef::new("*")))
            } else if let Some(tok) = self.lex.peek() {
                let upper = tok.to_uppercase();
                if matches!(upper.as_str(), "COUNT" | "SUM" | "AVG" | "MIN" | "MAX") {
                    let expr = self.parse_agg()?;
                    let alias = if self.lex.match_any(&["AS"]).is_some() {
                        Some(self.lex.consume(None)?)
                    } else {
                        None
                    };
                    SelectItem {
                        expr: Expr::Agg(expr),
                        alias,
                    }
                } else {
                    let name = self.lex.consume(None)?;
                    let alias = if self.lex.match_any(&["AS"]).is_some() {
                        Some(self.lex.consume(None)?)
                    } else {
                        None
                    };
                    SelectItem { expr: Expr::Column(ColumnRef::new(name)), alias }
                }
            } else {
                return Err(MeridianError::Parse("unexpected end in select list".to_string()));
            };
            items.push(item);
            if self.lex.match_any(&[","]).is_none() {
                break;
            }
        }
        Ok(items)
    }

    fn parse_column_list(&mut self) -> Result<Vec<ColumnRef>> {
        let mut cols = Vec::new();
        loop {
            let name = self.lex.consume(None)?;
            cols.push(ColumnRef::new(name));
            if self.lex.match_any(&[","]).is_none() {
                break;
            }
        }
        Ok(cols)
    }

    fn parse_order_list(&mut self) -> Result<Vec<OrderByItem>> {
        let mut items = Vec::new();
        loop {
            let col = ColumnRef::new(self.lex.consume(None)?);
            let descending = self.lex.match_any(&["DESC"]).is_some();
            if !descending {
                self.lex.match_any(&["ASC"]);
            }
            items.push(OrderByItem { expr: col, descending });
            if self.lex.match_any(&[","]).is_none() {
                break;
            }
        }
        Ok(items)
    }

    fn parse_agg(&mut self) -> Result<AggExpr> {
        let func_tok = self.lex.consume(None)?.to_uppercase();
        let func = match func_tok.as_str() {
            "COUNT" => AggFunc::Count,
            "SUM" => AggFunc::Sum,
            "AVG" => AggFunc::Avg,
            "MIN" => AggFunc::Min,
            "MAX" => AggFunc::Max,
            _ => return Err(MeridianError::Parse(format!("unknown function {func_tok}"))),
        };
        self.lex.consume(Some("("))?;
        let arg = if self.lex.match_any(&["*"]).is_none() {
            Some(ColumnRef::new(self.lex.consume(None)?))
        } else {
            None
        };
        self.lex.consume(Some(")"))?;
        Ok(AggExpr { func, arg })
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        let mut parts = vec![self.parse_comparison()?];
        while self.lex.match_any(&["AND"]).is_some() {
            parts.push(self.parse_comparison()?);
        }
        if parts.len() == 1 {
            Ok(parts.pop().unwrap())
        } else {
            Ok(Expr::And(AndExpr::new(parts)))
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let left = self.parse_value()?;
        let op_tok = self.lex.consume(None)?;
        let op = match op_tok.as_str() {
            "=" => CompareOp::Eq,
            "!" => {
                if self.lex.peek() == Some("=".to_string()) {
                    self.lex.consume(None)?;
                    CompareOp::Ne
                } else {
                    return Err(MeridianError::Parse(format!("unknown operator {op_tok}")));
                }
            }
            "<>" => CompareOp::Ne,
            "<" => CompareOp::Lt,
            "<=" => CompareOp::Le,
            ">" => CompareOp::Gt,
            ">=" => CompareOp::Ge,
            _ => return Err(MeridianError::Parse(format!("unknown operator {op_tok}"))),
        };
        let right = self.parse_value()?;
        Ok(Expr::Binary(BinaryExpr::new(op, left, right)))
    }

    fn parse_value(&mut self) -> Result<Expr> {
        let tok = self
            .lex
            .peek()
            .ok_or_else(|| MeridianError::Parse("expected value".to_string()))?;

        if tok.starts_with('\'') || tok.starts_with('"') {
            self.lex.consume(None)?;
            let raw = &tok[1..tok.len() - 1];
            return Ok(Expr::Literal(LiteralExpr::new(Value::from_str(raw))));
        }

        if tok.replace('.', "").chars().all(|c| c.is_ascii_digit() || c == '-') {
            self.lex.consume(None)?;
            if tok.contains('.') {
                let f = tok.parse::<f64>().map_err(|_| {
                    MeridianError::Parse(format!("invalid float: {tok}"))
                })?;
                return Ok(Expr::Literal(LiteralExpr::new(Value::from_float(f))));
            } else {
                let i = tok.parse::<i64>().map_err(|_| {
                    MeridianError::Parse(format!("invalid int: {tok}"))
                })?;
                return Ok(Expr::Literal(LiteralExpr::new(Value::from_int(i))));
            }
        }

        self.lex.consume(None)?;
        Ok(Expr::Column(ColumnRef::new(tok)))
    }
}

/// Port of Python's `parse_query` function.
pub fn parse_query(text: &str) -> Result<Query> {
    Parser::new(text).parse()
}
