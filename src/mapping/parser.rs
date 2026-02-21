use crate::error;
use crate::mapping::ast::*;
use crate::mapping::lexer::{Span, Token, TokenKind};
use crate::value::Value;

/// Parse a token stream into a Program (list of statements).
pub fn parse(tokens: Vec<Token>) -> error::Result<Program> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

/// Parse source code string into a Program.
pub fn parse_str(input: &str) -> error::Result<Program> {
    let tokens = crate::mapping::lexer::tokenize(input)?;
    parse(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|t| &t.kind)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn expect_exact(&mut self, expected: &TokenKind) -> error::Result<Token> {
        match self.advance() {
            Some(token) if token.kind == *expected => Ok(token),
            Some(token) => Err(error::MorphError::mapping_at(
                format!("expected {}, found {:?}", kind_name(expected), token.kind),
                token.span.line,
                token.span.column,
            )),
            None => Err(error::MorphError::mapping(format!(
                "unexpected end of input, expected {}",
                kind_name(expected)
            ))),
        }
    }

    fn current_span(&self) -> Span {
        self.peek().map(|t| t.span).unwrap_or(Span::new(1, 1))
    }

    fn skip_newlines(&mut self) {
        while let Some(TokenKind::Newline) = self.peek_kind() {
            self.advance();
        }
    }

    fn parse_program(&mut self) -> error::Result<Program> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while self.peek().is_some() {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
            // Consume newlines between statements
            self.skip_newlines();
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> error::Result<Statement> {
        let token = match self.peek() {
            Some(t) => t.clone(),
            None => {
                return Err(error::MorphError::mapping(
                    "unexpected end of input, expected a statement",
                ));
            }
        };

        match &token.kind {
            TokenKind::Rename => self.parse_rename(),
            TokenKind::Select => self.parse_select(),
            TokenKind::Drop => self.parse_drop(),
            TokenKind::Set => self.parse_set(),
            TokenKind::Default => self.parse_default(),
            TokenKind::Cast => self.parse_cast(),
            TokenKind::Flatten => self.parse_flatten(),
            TokenKind::Nest => self.parse_nest(),
            TokenKind::Where => self.parse_where(),
            _ => {
                let suggestion = suggest_keyword(&token.kind);
                let msg = if let Some(s) = suggestion {
                    format!(
                        "unexpected {:?} at start of statement, did you mean '{s}'?",
                        token.kind
                    )
                } else {
                    format!(
                        "unexpected {:?} at start of statement; expected rename, select, drop, set, default, or cast",
                        token.kind
                    )
                };
                Err(error::MorphError::mapping_at(
                    msg,
                    token.span.line,
                    token.span.column,
                ))
            }
        }
    }

    fn parse_rename(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'rename'
        let from = self.parse_path()?;
        self.expect_exact(&TokenKind::Arrow)?;
        let to = self.parse_path()?;
        Ok(Statement::Rename {
            from,
            to,
            span: start.span,
        })
    }

    fn parse_select(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'select'
        let paths = self.parse_path_list()?;
        Ok(Statement::Select {
            paths,
            span: start.span,
        })
    }

    fn parse_drop(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'drop'
        let paths = self.parse_path_list()?;
        Ok(Statement::Drop {
            paths,
            span: start.span,
        })
    }

    fn parse_set(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'set'
        let path = self.parse_path()?;
        self.expect_exact(&TokenKind::Eq)?;
        let expr = self.parse_expr()?;
        Ok(Statement::Set {
            path,
            expr,
            span: start.span,
        })
    }

    fn parse_default(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'default'
        let path = self.parse_path()?;
        self.expect_exact(&TokenKind::Eq)?;
        let expr = self.parse_expr()?;
        Ok(Statement::Default {
            path,
            expr,
            span: start.span,
        })
    }

    fn parse_cast(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'cast'
        let path = self.parse_path()?;
        self.expect_exact(&TokenKind::As)?;
        let target_type = self.parse_cast_type()?;
        Ok(Statement::Cast {
            path,
            target_type,
            span: start.span,
        })
    }

    fn parse_cast_type(&mut self) -> error::Result<CastType> {
        let token = self.advance().ok_or_else(|| {
            error::MorphError::mapping("unexpected end of input, expected a type name")
        })?;

        match &token.kind {
            TokenKind::Ident(name) | TokenKind::StringLit(name) => match name.as_str() {
                "int" | "integer" => Ok(CastType::Int),
                "float" | "number" => Ok(CastType::Float),
                "string" | "str" => Ok(CastType::String),
                "bool" | "boolean" => Ok(CastType::Bool),
                other => Err(error::MorphError::mapping_at(
                    format!("unknown type '{other}'; expected int, float, string, or bool"),
                    token.span.line,
                    token.span.column,
                )),
            },
            _ => Err(error::MorphError::mapping_at(
                format!(
                    "expected type name (int, float, string, bool), found {:?}",
                    token.kind
                ),
                token.span.line,
                token.span.column,
            )),
        }
    }

    fn parse_flatten(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'flatten'
        let path = self.parse_path()?;

        // Check for optional -> prefix "..."
        let prefix = if matches!(self.peek_kind(), Some(TokenKind::Arrow)) {
            self.advance(); // consume '->'
                            // Expect the identifier "prefix"
            match self.peek_kind() {
                Some(TokenKind::Ident(name)) if name == "prefix" => {
                    self.advance(); // consume 'prefix'
                }
                _ => {
                    let span = self.current_span();
                    return Err(error::MorphError::mapping_at(
                        "expected 'prefix' after '->' in flatten",
                        span.line,
                        span.column,
                    ));
                }
            }
            // Expect a string literal for the prefix value
            match self.advance() {
                Some(Token {
                    kind: TokenKind::StringLit(s),
                    ..
                }) => Some(s),
                _ => {
                    let span = self.current_span();
                    return Err(error::MorphError::mapping_at(
                        "expected string literal for prefix value in flatten",
                        span.line,
                        span.column,
                    ));
                }
            }
        } else {
            None
        };

        Ok(Statement::Flatten {
            path,
            prefix,
            span: start.span,
        })
    }

    fn parse_nest(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'nest'
        let paths = self.parse_path_list()?;
        self.expect_exact(&TokenKind::Arrow)?;
        let target = self.parse_path()?;
        Ok(Statement::Nest {
            paths,
            target,
            span: start.span,
        })
    }

    fn parse_where(&mut self) -> error::Result<Statement> {
        let start = self.advance().unwrap(); // consume 'where'
        let condition = self.parse_expr()?;
        Ok(Statement::Where {
            condition,
            span: start.span,
        })
    }

    fn parse_path_list(&mut self) -> error::Result<Vec<Path>> {
        let mut paths = vec![self.parse_path()?];
        while let Some(TokenKind::Comma) = self.peek_kind() {
            self.advance(); // consume comma
            paths.push(self.parse_path()?);
        }
        Ok(paths)
    }

    fn parse_path(&mut self) -> error::Result<Path> {
        let span = self.current_span();
        self.expect_exact(&TokenKind::Dot)?;

        let mut segments = Vec::new();

        // After initial dot, parse first segment
        self.parse_path_segment(&mut segments)?;

        // Parse additional `.field` or `.[index]` segments
        while let Some(TokenKind::Dot) = self.peek_kind() {
            self.advance(); // consume '.'
            self.parse_path_segment(&mut segments)?;
        }

        Ok(Path { segments, span })
    }

    fn parse_path_segment(&mut self, segments: &mut Vec<PathSegment>) -> error::Result<()> {
        match self.peek_kind() {
            Some(TokenKind::Ident(_)) => {
                if let Some(Token {
                    kind: TokenKind::Ident(name),
                    ..
                }) = self.advance()
                {
                    segments.push(PathSegment::Field(name));
                }
            }
            Some(TokenKind::LBracket) => {
                self.advance(); // consume '['
                match self.peek_kind() {
                    Some(TokenKind::IntLit(_)) => {
                        if let Some(Token {
                            kind: TokenKind::IntLit(idx),
                            ..
                        }) = self.advance()
                        {
                            segments.push(PathSegment::Index(idx));
                        }
                    }
                    Some(TokenKind::Star) => {
                        self.advance();
                        segments.push(PathSegment::Wildcard);
                    }
                    Some(TokenKind::StringLit(_)) => {
                        if let Some(Token {
                            kind: TokenKind::StringLit(key),
                            ..
                        }) = self.advance()
                        {
                            segments.push(PathSegment::Field(key));
                        }
                    }
                    _ => {
                        let span = self.current_span();
                        return Err(error::MorphError::mapping_at(
                            "expected index, '*', or string key in brackets",
                            span.line,
                            span.column,
                        ));
                    }
                }
                self.expect_exact(&TokenKind::RBracket)?;
            }
            // Keywords can also be field names (e.g. .sort, .set, .default)
            Some(kind) if is_keyword(kind) => {
                let token = self.advance().unwrap();
                segments.push(PathSegment::Field(keyword_to_string(&token.kind)));
            }
            _ => {
                let span = self.current_span();
                return Err(error::MorphError::mapping_at(
                    "expected field name or '[' in path",
                    span.line,
                    span.column,
                ));
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Expression parsing with precedence climbing
    // -----------------------------------------------------------------------

    fn parse_expr(&mut self) -> error::Result<Expr> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_and_expr()?;
        while let Some(TokenKind::Or) = self.peek_kind() {
            self.advance();
            let right = self.parse_and_expr()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_comparison()?;
        while let Some(TokenKind::And) = self.peek_kind() {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> error::Result<Expr> {
        let left = self.parse_additive()?;
        let op = match self.peek_kind() {
            Some(TokenKind::EqEq) => Some(BinOp::Eq),
            Some(TokenKind::NotEq) => Some(BinOp::NotEq),
            Some(TokenKind::Gt) => Some(BinOp::Gt),
            Some(TokenKind::GtEq) => Some(BinOp::GtEq),
            Some(TokenKind::Lt) => Some(BinOp::Lt),
            Some(TokenKind::LtEq) => Some(BinOp::LtEq),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let right = self.parse_additive()?;
            Ok(Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_additive(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => BinOp::Add,
                Some(TokenKind::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> error::Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Star) => BinOp::Mul,
                Some(TokenKind::Slash) => BinOp::Div,
                Some(TokenKind::Percent) => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> error::Result<Expr> {
        match self.peek_kind() {
            Some(TokenKind::Not) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            Some(TokenKind::Minus) => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> error::Result<Expr> {
        match self.peek_kind().cloned() {
            // Literals
            Some(TokenKind::IntLit(n)) => {
                self.advance();
                Ok(Expr::Literal(Value::Int(n)))
            }
            Some(TokenKind::FloatLit(f)) => {
                self.advance();
                Ok(Expr::Literal(Value::Float(f)))
            }
            Some(TokenKind::StringLit(ref s)) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Literal(Value::String(s)))
            }
            Some(TokenKind::True) => {
                self.advance();
                Ok(Expr::Literal(Value::Bool(true)))
            }
            Some(TokenKind::False) => {
                self.advance();
                Ok(Expr::Literal(Value::Bool(false)))
            }
            Some(TokenKind::Null) => {
                self.advance();
                Ok(Expr::Literal(Value::Null))
            }

            // Path: .field
            Some(TokenKind::Dot) => {
                let path = self.parse_path()?;
                Ok(Expr::Path(path))
            }

            // Function call or identifier: name(...)
            Some(TokenKind::Ident(_)) => {
                let token = self.advance().unwrap();
                let name = match token.kind {
                    TokenKind::Ident(n) => n,
                    _ => unreachable!(),
                };
                if self.peek_kind() == Some(&TokenKind::LParen) {
                    self.advance(); // consume '('
                    let args = self.parse_arg_list()?;
                    self.expect_exact(&TokenKind::RParen)?;
                    Ok(Expr::FunctionCall {
                        name,
                        args,
                        span: token.span,
                    })
                } else {
                    // Bare identifier — treat as a path with single field
                    Ok(Expr::Path(Path {
                        segments: vec![PathSegment::Field(name)],
                        span: token.span,
                    }))
                }
            }

            // Parenthesized expression
            Some(TokenKind::LParen) => {
                self.advance(); // consume '('
                let expr = self.parse_expr()?;
                self.expect_exact(&TokenKind::RParen)?;
                Ok(expr)
            }

            _ => {
                let span = self.current_span();
                let kind_desc = self
                    .peek()
                    .map(|t| format!("{:?}", t.kind))
                    .unwrap_or_else(|| "end of input".to_string());
                Err(error::MorphError::mapping_at(
                    format!("unexpected {kind_desc} in expression"),
                    span.line,
                    span.column,
                ))
            }
        }
    }

    fn parse_arg_list(&mut self) -> error::Result<Vec<Expr>> {
        let mut args = Vec::new();
        if self.peek_kind() == Some(&TokenKind::RParen) {
            return Ok(args);
        }
        args.push(self.parse_expr()?);
        while let Some(TokenKind::Comma) = self.peek_kind() {
            self.advance();
            args.push(self.parse_expr()?);
        }
        Ok(args)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn kind_name(kind: &TokenKind) -> &'static str {
    match kind {
        TokenKind::Arrow => "'->'",
        TokenKind::Eq => "'='",
        TokenKind::EqEq => "'=='",
        TokenKind::NotEq => "'!='",
        TokenKind::Gt => "'>'",
        TokenKind::GtEq => "'>='",
        TokenKind::Lt => "'<'",
        TokenKind::LtEq => "'<='",
        TokenKind::Plus => "'+'",
        TokenKind::Minus => "'-'",
        TokenKind::Star => "'*'",
        TokenKind::Slash => "'/'",
        TokenKind::Percent => "'%'",
        TokenKind::LBrace => "'{'",
        TokenKind::RBrace => "'}'",
        TokenKind::LParen => "'('",
        TokenKind::RParen => "')'",
        TokenKind::LBracket => "'['",
        TokenKind::RBracket => "']'",
        TokenKind::Comma => "','",
        TokenKind::Dot => "'.'",
        TokenKind::As => "'as'",
        TokenKind::Rename => "'rename'",
        TokenKind::Select => "'select'",
        TokenKind::Drop => "'drop'",
        TokenKind::Set => "'set'",
        TokenKind::Default => "'default'",
        TokenKind::Cast => "'cast'",
        _ => "token",
    }
}

fn is_keyword(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Rename
            | TokenKind::Select
            | TokenKind::Drop
            | TokenKind::Set
            | TokenKind::Default
            | TokenKind::Cast
            | TokenKind::As
            | TokenKind::Where
            | TokenKind::Sort
            | TokenKind::Each
            | TokenKind::When
            | TokenKind::Not
            | TokenKind::And
            | TokenKind::Or
            | TokenKind::Flatten
            | TokenKind::Nest
            | TokenKind::Asc
            | TokenKind::Desc
    )
}

fn keyword_to_string(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Rename => "rename".into(),
        TokenKind::Select => "select".into(),
        TokenKind::Drop => "drop".into(),
        TokenKind::Set => "set".into(),
        TokenKind::Default => "default".into(),
        TokenKind::Cast => "cast".into(),
        TokenKind::As => "as".into(),
        TokenKind::Where => "where".into(),
        TokenKind::Sort => "sort".into(),
        TokenKind::Each => "each".into(),
        TokenKind::When => "when".into(),
        TokenKind::Not => "not".into(),
        TokenKind::And => "and".into(),
        TokenKind::Or => "or".into(),
        TokenKind::Flatten => "flatten".into(),
        TokenKind::Nest => "nest".into(),
        TokenKind::Asc => "asc".into(),
        TokenKind::Desc => "desc".into(),
        _ => String::new(),
    }
}

fn suggest_keyword(kind: &TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::Ident(name) => {
            let suggestions: &[(&str, &str)] = &[
                ("ren", "rename"),
                ("rname", "rename"),
                ("sel", "select"),
                ("slect", "select"),
                ("drp", "drop"),
                ("delet", "drop"),
                ("remove", "drop"),
                ("st", "set"),
                ("def", "default"),
                ("cst", "cast"),
                ("convert", "cast"),
            ];
            for (prefix, suggestion) in suggestions {
                if name == prefix {
                    return Some(suggestion);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to parse a string and return the program
    fn parse_ok(input: &str) -> Program {
        parse_str(input).unwrap_or_else(|e| panic!("parse failed for '{input}': {e}"))
    }

    fn parse_err(input: &str) -> error::MorphError {
        parse_str(input).unwrap_err()
    }

    fn first_stmt(input: &str) -> Statement {
        let prog = parse_ok(input);
        assert!(
            !prog.statements.is_empty(),
            "expected at least one statement"
        );
        prog.statements.into_iter().next().unwrap()
    }

    // -----------------------------------------------------------------------
    // rename
    // -----------------------------------------------------------------------

    #[test]
    fn rename_simple() {
        let stmt = first_stmt("rename .a -> .b");
        match stmt {
            Statement::Rename { from, to, .. } => {
                assert_eq!(from.segments, vec![PathSegment::Field("a".into())]);
                assert_eq!(to.segments, vec![PathSegment::Field("b".into())]);
            }
            other => panic!("expected Rename, got: {other:?}"),
        }
    }

    #[test]
    fn rename_nested_paths() {
        let stmt = first_stmt("rename .user.name -> .person.full_name");
        match stmt {
            Statement::Rename { from, to, .. } => {
                assert_eq!(
                    from.segments,
                    vec![
                        PathSegment::Field("user".into()),
                        PathSegment::Field("name".into()),
                    ]
                );
                assert_eq!(
                    to.segments,
                    vec![
                        PathSegment::Field("person".into()),
                        PathSegment::Field("full_name".into()),
                    ]
                );
            }
            other => panic!("expected Rename, got: {other:?}"),
        }
    }

    #[test]
    fn rename_with_array_index() {
        let stmt = first_stmt("rename .items.[0].name -> .first_item");
        match stmt {
            Statement::Rename { from, to, .. } => {
                assert_eq!(
                    from.segments,
                    vec![
                        PathSegment::Field("items".into()),
                        PathSegment::Index(0),
                        PathSegment::Field("name".into()),
                    ]
                );
                assert_eq!(to.segments, vec![PathSegment::Field("first_item".into())]);
            }
            other => panic!("expected Rename, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // select
    // -----------------------------------------------------------------------

    #[test]
    fn select_single() {
        let stmt = first_stmt("select .name");
        match stmt {
            Statement::Select { paths, .. } => {
                assert_eq!(paths.len(), 1);
                assert_eq!(paths[0].segments, vec![PathSegment::Field("name".into())]);
            }
            other => panic!("expected Select, got: {other:?}"),
        }
    }

    #[test]
    fn select_multiple() {
        let stmt = first_stmt("select .a, .b, .c");
        match stmt {
            Statement::Select { paths, .. } => {
                assert_eq!(paths.len(), 3);
                assert_eq!(paths[0].segments, vec![PathSegment::Field("a".into())]);
                assert_eq!(paths[1].segments, vec![PathSegment::Field("b".into())]);
                assert_eq!(paths[2].segments, vec![PathSegment::Field("c".into())]);
            }
            other => panic!("expected Select, got: {other:?}"),
        }
    }

    #[test]
    fn select_nested_paths() {
        let stmt = first_stmt("select .user.name, .user.age");
        match stmt {
            Statement::Select { paths, .. } => {
                assert_eq!(paths.len(), 2);
                assert_eq!(
                    paths[0].segments,
                    vec![
                        PathSegment::Field("user".into()),
                        PathSegment::Field("name".into()),
                    ]
                );
            }
            other => panic!("expected Select, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // drop
    // -----------------------------------------------------------------------

    #[test]
    fn drop_single() {
        let stmt = first_stmt("drop .x");
        match stmt {
            Statement::Drop { paths, .. } => {
                assert_eq!(paths.len(), 1);
                assert_eq!(paths[0].segments, vec![PathSegment::Field("x".into())]);
            }
            other => panic!("expected Drop, got: {other:?}"),
        }
    }

    #[test]
    fn drop_multiple() {
        let stmt = first_stmt("drop .a, .b");
        match stmt {
            Statement::Drop { paths, .. } => {
                assert_eq!(paths.len(), 2);
            }
            other => panic!("expected Drop, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // set with literal
    // -----------------------------------------------------------------------

    #[test]
    fn set_int_literal() {
        let stmt = first_stmt("set .x = 42");
        match stmt {
            Statement::Set { path, expr, .. } => {
                assert_eq!(path.segments, vec![PathSegment::Field("x".into())]);
                assert_eq!(expr, Expr::Literal(Value::Int(42)));
            }
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_float_literal() {
        let stmt = first_stmt("set .x = 3.25");
        match stmt {
            Statement::Set { expr, .. } => {
                assert_eq!(expr, Expr::Literal(Value::Float(3.25)));
            }
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_string_literal() {
        let stmt = first_stmt("set .x = \"hello\"");
        match stmt {
            Statement::Set { expr, .. } => {
                assert_eq!(expr, Expr::Literal(Value::String("hello".into())));
            }
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_bool_true() {
        let stmt = first_stmt("set .active = true");
        match stmt {
            Statement::Set { expr, .. } => {
                assert_eq!(expr, Expr::Literal(Value::Bool(true)));
            }
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_bool_false() {
        let stmt = first_stmt("set .active = false");
        match stmt {
            Statement::Set { expr, .. } => {
                assert_eq!(expr, Expr::Literal(Value::Bool(false)));
            }
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_null() {
        let stmt = first_stmt("set .x = null");
        match stmt {
            Statement::Set { expr, .. } => {
                assert_eq!(expr, Expr::Literal(Value::Null));
            }
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_negative_int() {
        let stmt = first_stmt("set .x = -7");
        match stmt {
            Statement::Set { expr, .. } => {
                assert_eq!(expr, Expr::Literal(Value::Int(-7)));
            }
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // set with path
    // -----------------------------------------------------------------------

    #[test]
    fn set_path_reference() {
        let stmt = first_stmt("set .x = .y");
        match stmt {
            Statement::Set { path, expr, .. } => {
                assert_eq!(path.segments, vec![PathSegment::Field("x".into())]);
                match expr {
                    Expr::Path(p) => {
                        assert_eq!(p.segments, vec![PathSegment::Field("y".into())]);
                    }
                    other => panic!("expected Path expr, got: {other:?}"),
                }
            }
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_nested_path_reference() {
        let stmt = first_stmt("set .x = .user.name");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::Path(p) => {
                    assert_eq!(
                        p.segments,
                        vec![
                            PathSegment::Field("user".into()),
                            PathSegment::Field("name".into()),
                        ]
                    );
                }
                other => panic!("expected Path expr, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // set with function call
    // -----------------------------------------------------------------------

    #[test]
    fn set_function_call() {
        let stmt = first_stmt("set .x = lower(.y)");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "lower");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::Path(p) => {
                            assert_eq!(p.segments, vec![PathSegment::Field("y".into())]);
                        }
                        other => panic!("expected Path arg, got: {other:?}"),
                    }
                }
                other => panic!("expected FunctionCall, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_function_call_multi_args() {
        let stmt = first_stmt("set .x = replace(.name, \"old\", \"new\")");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "replace");
                    assert_eq!(args.len(), 3);
                    assert!(matches!(&args[0], Expr::Path(_)));
                    assert_eq!(args[1], Expr::Literal(Value::String("old".into())));
                    assert_eq!(args[2], Expr::Literal(Value::String("new".into())));
                }
                other => panic!("expected FunctionCall, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_function_call_no_args() {
        let stmt = first_stmt("set .x = now()");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "now");
                    assert!(args.is_empty());
                }
                other => panic!("expected FunctionCall, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_nested_function_call() {
        let stmt = first_stmt("set .x = upper(trim(.name))");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "upper");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::FunctionCall { name, args, .. } => {
                            assert_eq!(name, "trim");
                            assert_eq!(args.len(), 1);
                        }
                        other => panic!("expected nested FunctionCall, got: {other:?}"),
                    }
                }
                other => panic!("expected FunctionCall, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // set with binary op
    // -----------------------------------------------------------------------

    #[test]
    fn set_addition() {
        let stmt = first_stmt("set .x = .a + .b");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, left, right } => {
                    assert_eq!(op, BinOp::Add);
                    assert!(matches!(*left, Expr::Path(_)));
                    assert!(matches!(*right, Expr::Path(_)));
                }
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_subtraction() {
        let stmt = first_stmt("set .x = .a - .b");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, .. } => {
                    assert_eq!(op, BinOp::Sub);
                }
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_multiplication() {
        let stmt = first_stmt("set .x = .a * .b");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Mul),
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_division() {
        let stmt = first_stmt("set .x = .a / .b");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Div),
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn set_modulo() {
        let stmt = first_stmt("set .x = .a % .b");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Mod),
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Operator precedence
    // -----------------------------------------------------------------------

    #[test]
    fn precedence_mul_before_add() {
        // .a + .b * .c should parse as .a + (.b * .c)
        let stmt = first_stmt("set .x = .a + .b * .c");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, left, right } => {
                    assert_eq!(op, BinOp::Add);
                    assert!(matches!(*left, Expr::Path(_)));
                    match *right {
                        Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Mul),
                        other => panic!("expected BinaryOp for right, got: {other:?}"),
                    }
                }
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn precedence_parens_override() {
        // (.a + .b) * .c should parse as (.a + .b) * .c
        let stmt = first_stmt("set .x = (.a + .b) * .c");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, left, .. } => {
                    assert_eq!(op, BinOp::Mul);
                    match *left {
                        Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Add),
                        other => panic!("expected BinaryOp for left, got: {other:?}"),
                    }
                }
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn precedence_comparison_after_arithmetic() {
        // .a + .b > .c should parse as (.a + .b) > .c
        let stmt = first_stmt("set .x = .a + .b > .c");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, left, .. } => {
                    assert_eq!(op, BinOp::Gt);
                    match *left {
                        Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Add),
                        other => panic!("expected BinaryOp for left, got: {other:?}"),
                    }
                }
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn precedence_and_before_or() {
        // .a or .b and .c should parse as .a or (.b and .c)
        let stmt = first_stmt("set .x = .a or .b and .c");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, right, .. } => {
                    assert_eq!(op, BinOp::Or);
                    match *right {
                        Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::And),
                        other => panic!("expected BinaryOp for right, got: {other:?}"),
                    }
                }
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Unary not
    // -----------------------------------------------------------------------

    #[test]
    fn unary_not() {
        let stmt = first_stmt("set .x = not .active");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::UnaryOp { op, .. } => assert_eq!(op, UnaryOp::Not),
                other => panic!("expected UnaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Comparison operators
    // -----------------------------------------------------------------------

    #[test]
    fn comparison_eq() {
        let stmt = first_stmt("set .x = .a == .b");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Eq),
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn comparison_noteq() {
        let stmt = first_stmt("set .x = .a != .b");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::NotEq),
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn comparison_gt() {
        let stmt = first_stmt("set .x = .a > 10");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Gt),
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    #[test]
    fn comparison_lteq() {
        let stmt = first_stmt("set .x = .a <= 100");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::LtEq),
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // default
    // -----------------------------------------------------------------------

    #[test]
    fn default_string() {
        let stmt = first_stmt("default .name = \"hello\"");
        match stmt {
            Statement::Default { path, expr, .. } => {
                assert_eq!(path.segments, vec![PathSegment::Field("name".into())]);
                assert_eq!(expr, Expr::Literal(Value::String("hello".into())));
            }
            other => panic!("expected Default, got: {other:?}"),
        }
    }

    #[test]
    fn default_int() {
        let stmt = first_stmt("default .count = 0");
        match stmt {
            Statement::Default { expr, .. } => {
                assert_eq!(expr, Expr::Literal(Value::Int(0)));
            }
            other => panic!("expected Default, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // cast
    // -----------------------------------------------------------------------

    #[test]
    fn cast_int() {
        let stmt = first_stmt("cast .age as int");
        match stmt {
            Statement::Cast {
                path, target_type, ..
            } => {
                assert_eq!(path.segments, vec![PathSegment::Field("age".into())]);
                assert_eq!(target_type, CastType::Int);
            }
            other => panic!("expected Cast, got: {other:?}"),
        }
    }

    #[test]
    fn cast_float() {
        let stmt = first_stmt("cast .score as float");
        match stmt {
            Statement::Cast { target_type, .. } => {
                assert_eq!(target_type, CastType::Float);
            }
            other => panic!("expected Cast, got: {other:?}"),
        }
    }

    #[test]
    fn cast_string() {
        let stmt = first_stmt("cast .id as string");
        match stmt {
            Statement::Cast { target_type, .. } => {
                assert_eq!(target_type, CastType::String);
            }
            other => panic!("expected Cast, got: {other:?}"),
        }
    }

    #[test]
    fn cast_bool() {
        let stmt = first_stmt("cast .active as bool");
        match stmt {
            Statement::Cast { target_type, .. } => {
                assert_eq!(target_type, CastType::Bool);
            }
            other => panic!("expected Cast, got: {other:?}"),
        }
    }

    #[test]
    fn cast_string_literal_type() {
        let stmt = first_stmt("cast .x as \"int\"");
        match stmt {
            Statement::Cast { target_type, .. } => {
                assert_eq!(target_type, CastType::Int);
            }
            other => panic!("expected Cast, got: {other:?}"),
        }
    }

    #[test]
    fn cast_type_aliases() {
        // integer → Int, number → Float, str → String, boolean → Bool
        assert!(matches!(
            first_stmt("cast .x as integer"),
            Statement::Cast {
                target_type: CastType::Int,
                ..
            }
        ));
        assert!(matches!(
            first_stmt("cast .x as number"),
            Statement::Cast {
                target_type: CastType::Float,
                ..
            }
        ));
        assert!(matches!(
            first_stmt("cast .x as str"),
            Statement::Cast {
                target_type: CastType::String,
                ..
            }
        ));
        assert!(matches!(
            first_stmt("cast .x as boolean"),
            Statement::Cast {
                target_type: CastType::Bool,
                ..
            }
        ));
    }

    // -----------------------------------------------------------------------
    // multi-statement programs
    // -----------------------------------------------------------------------

    #[test]
    fn multi_statement() {
        let prog = parse_ok("rename .old -> .new\nset .x = 42\nselect .new, .x");
        assert_eq!(prog.statements.len(), 3);
        assert!(matches!(prog.statements[0], Statement::Rename { .. }));
        assert!(matches!(prog.statements[1], Statement::Set { .. }));
        assert!(matches!(prog.statements[2], Statement::Select { .. }));
    }

    #[test]
    fn multi_statement_with_blank_lines() {
        let prog = parse_ok("rename .a -> .b\n\n\nselect .b");
        assert_eq!(prog.statements.len(), 2);
    }

    #[test]
    fn multi_statement_with_comments() {
        let prog = parse_ok(
            "# Rename the field\nrename .old -> .new\n# Now set a default\ndefault .x = 0",
        );
        assert_eq!(prog.statements.len(), 2);
        assert!(matches!(prog.statements[0], Statement::Rename { .. }));
        assert!(matches!(prog.statements[1], Statement::Default { .. }));
    }

    // -----------------------------------------------------------------------
    // Empty program
    // -----------------------------------------------------------------------

    #[test]
    fn empty_program() {
        let prog = parse_ok("");
        assert!(prog.statements.is_empty());
    }

    #[test]
    fn comment_only_program() {
        let prog = parse_ok("# nothing here");
        assert!(prog.statements.is_empty());
    }

    // -----------------------------------------------------------------------
    // Path edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn path_with_wildcard() {
        let stmt = first_stmt("select .items.[*].name");
        match stmt {
            Statement::Select { paths, .. } => {
                assert_eq!(
                    paths[0].segments,
                    vec![
                        PathSegment::Field("items".into()),
                        PathSegment::Wildcard,
                        PathSegment::Field("name".into()),
                    ]
                );
            }
            other => panic!("expected Select, got: {other:?}"),
        }
    }

    #[test]
    fn path_with_string_key() {
        let stmt = first_stmt("select .[\"key with spaces\"]");
        match stmt {
            Statement::Select { paths, .. } => {
                assert_eq!(
                    paths[0].segments,
                    vec![PathSegment::Field("key with spaces".into())]
                );
            }
            other => panic!("expected Select, got: {other:?}"),
        }
    }

    #[test]
    fn path_with_keyword_field() {
        // Fields can be named after keywords
        let stmt = first_stmt("select .sort");
        match stmt {
            Statement::Select { paths, .. } => {
                assert_eq!(paths[0].segments, vec![PathSegment::Field("sort".into())]);
            }
            other => panic!("expected Select, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: missing arrow in rename
    // -----------------------------------------------------------------------

    #[test]
    fn error_missing_arrow_in_rename() {
        let err = parse_err("rename .a .b");
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(
                    message.contains("->") || message.contains("arrow"),
                    "msg: {message}"
                );
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: missing = in set
    // -----------------------------------------------------------------------

    #[test]
    fn error_missing_eq_in_set() {
        let err = parse_err("set .x 42");
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(
                    message.contains("=") || message.contains("'='"),
                    "msg: {message}"
                );
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: unknown keyword with suggestion
    // -----------------------------------------------------------------------

    #[test]
    fn error_unknown_keyword() {
        let err = parse_err("unknown .x");
        match err {
            error::MorphError::Mapping {
                message,
                line,
                column,
            } => {
                assert!(
                    message.contains("unexpected") || message.contains("expected"),
                    "msg: {message}"
                );
                assert_eq!(line, Some(1));
                assert_eq!(column, Some(1));
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    #[test]
    fn error_typo_with_suggestion() {
        let err = parse_err("ren .a -> .b");
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(
                    message.contains("rename"),
                    "should suggest 'rename': {message}"
                );
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    #[test]
    fn error_slect_suggests_select() {
        let err = parse_err("slect .a");
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(
                    message.contains("select"),
                    "should suggest 'select': {message}"
                );
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: missing path
    // -----------------------------------------------------------------------

    #[test]
    fn error_missing_path_in_set() {
        let err = parse_err("set = 42");
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: unknown cast type
    // -----------------------------------------------------------------------

    #[test]
    fn error_unknown_cast_type() {
        let err = parse_err("cast .x as potato");
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(message.contains("unknown type"), "msg: {message}");
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: unexpected end of input
    // -----------------------------------------------------------------------

    #[test]
    fn error_incomplete_rename() {
        let err = parse_err("rename .a ->");
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    #[test]
    fn error_incomplete_set() {
        let err = parse_err("set .x =");
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(!message.is_empty());
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Complex program
    // -----------------------------------------------------------------------

    #[test]
    fn complex_program() {
        let input = "\
# Transform user data
rename .firstName -> .first_name
rename .lastName -> .last_name
set .full_name = .first_name + \" \" + .last_name
default .age = 0
cast .age as int
drop .firstName, .lastName
select .full_name, .age, .email
";
        let prog = parse_ok(input);
        assert_eq!(prog.statements.len(), 7);
        assert!(matches!(prog.statements[0], Statement::Rename { .. }));
        assert!(matches!(prog.statements[1], Statement::Rename { .. }));
        assert!(matches!(prog.statements[2], Statement::Set { .. }));
        assert!(matches!(prog.statements[3], Statement::Default { .. }));
        assert!(matches!(prog.statements[4], Statement::Cast { .. }));
        assert!(matches!(prog.statements[5], Statement::Drop { .. }));
        assert!(matches!(prog.statements[6], Statement::Select { .. }));
    }

    // -----------------------------------------------------------------------
    // String concatenation with + operator
    // -----------------------------------------------------------------------

    #[test]
    fn string_concatenation() {
        let stmt = first_stmt("set .name = .first + \" \" + .last");
        match stmt {
            Statement::Set { expr, .. } => match expr {
                Expr::BinaryOp { op, right, left } => {
                    assert_eq!(op, BinOp::Add);
                    // Left should be (.first + " ")
                    match *left {
                        Expr::BinaryOp { op, .. } => assert_eq!(op, BinOp::Add),
                        other => panic!("expected BinaryOp, got: {other:?}"),
                    }
                    // Right should be .last
                    assert!(matches!(*right, Expr::Path(_)));
                }
                other => panic!("expected BinaryOp, got: {other:?}"),
            },
            other => panic!("expected Set, got: {other:?}"),
        }
    }
}
