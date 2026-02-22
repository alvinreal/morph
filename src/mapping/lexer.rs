use crate::error;

/// Position in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Token types for the morph mapping language.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Rename,
    Select,
    Drop,
    Set,
    Default,
    Cast,
    As,
    Where,
    Sort,
    Each,
    When,
    Not,
    And,
    Or,
    Flatten,
    Nest,
    Asc,
    Desc,

    // Operators
    Arrow,   // ->
    Eq,      // =
    EqEq,    // ==
    NotEq,   // !=
    Gt,      // >
    GtEq,    // >=
    Lt,      // <
    LtEq,    // <=
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %

    // Delimiters
    LBrace,   // {
    RBrace,   // }
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    Comma,    // ,
    Dot,      // .

    // Literals
    InterpolatedString(Vec<InterpolatedPart>),
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
    True,
    False,
    Null,

    // Identifiers (function names etc.)
    Ident(String),

    // Newline (significant as statement separator)
    Newline,
}

/// Part of an interpolated string token.
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolatedPart {
    Literal(String),
    Expression(String),
}

/// A token with its position in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Tokenize morph mapping language source into a stream of tokens.
pub fn tokenize(input: &str) -> error::Result<Vec<Token>> {
    let mut lexer = Lexer::new(input);
    lexer.tokenize()
}

struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.input.get(self.pos).copied()?;
        self.pos += 1;
        if ch == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.column)
    }

    fn tokenize(&mut self) -> error::Result<Vec<Token>> {
        let mut tokens: Vec<Token> = Vec::new();

        while self.pos < self.input.len() {
            let ch = self.peek().unwrap();

            match ch {
                // Skip spaces and tabs
                b' ' | b'\t' | b'\r' => {
                    self.advance();
                }

                // Newlines (significant)
                b'\n' => {
                    let span = self.span();
                    self.advance();
                    // Collapse consecutive newlines into one, and skip newlines
                    // after another newline token or at the start
                    if let Some(last) = tokens.last() {
                        if last.kind != TokenKind::Newline {
                            tokens.push(Token::new(TokenKind::Newline, span));
                        }
                    }
                }

                // Comments
                b'#' => {
                    // Skip to end of line
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                }

                // String literals
                b'"' => {
                    tokens.push(self.read_string()?);
                }

                // Operators and delimiters
                b'-' => {
                    let span = self.span();
                    self.advance();
                    if self.peek() == Some(b'>') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::Arrow, span));
                    } else if let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            // Negative number — but only if previous token
                            // is not a literal/ident/path that would make
                            // this a subtraction operator.
                            let is_unary = matches!(
                                tokens.last().map(|t| &t.kind),
                                None | Some(TokenKind::Newline)
                                    | Some(TokenKind::LParen)
                                    | Some(TokenKind::LBracket)
                                    | Some(TokenKind::Comma)
                                    | Some(TokenKind::Eq)
                                    | Some(TokenKind::EqEq)
                                    | Some(TokenKind::NotEq)
                                    | Some(TokenKind::Gt)
                                    | Some(TokenKind::GtEq)
                                    | Some(TokenKind::Lt)
                                    | Some(TokenKind::LtEq)
                                    | Some(TokenKind::Arrow)
                                    | Some(TokenKind::Plus)
                                    | Some(TokenKind::Minus)
                                    | Some(TokenKind::Star)
                                    | Some(TokenKind::Slash)
                                    | Some(TokenKind::Percent)
                            );
                            if is_unary {
                                tokens.push(self.read_number(span, true)?);
                            } else {
                                tokens.push(Token::new(TokenKind::Minus, span));
                            }
                        } else {
                            tokens.push(Token::new(TokenKind::Minus, span));
                        }
                    } else {
                        tokens.push(Token::new(TokenKind::Minus, span));
                    }
                }

                b'=' => {
                    let span = self.span();
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::EqEq, span));
                    } else {
                        tokens.push(Token::new(TokenKind::Eq, span));
                    }
                }

                b'!' => {
                    let span = self.span();
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::NotEq, span));
                    } else {
                        return Err(error::MorphError::mapping_at(
                            "unexpected character '!'",
                            span.line,
                            span.column,
                        ));
                    }
                }

                b'>' => {
                    let span = self.span();
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::GtEq, span));
                    } else {
                        tokens.push(Token::new(TokenKind::Gt, span));
                    }
                }

                b'<' => {
                    let span = self.span();
                    self.advance();
                    if self.peek() == Some(b'=') {
                        self.advance();
                        tokens.push(Token::new(TokenKind::LtEq, span));
                    } else {
                        tokens.push(Token::new(TokenKind::Lt, span));
                    }
                }

                b'+' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::Plus, span));
                }

                b'*' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::Star, span));
                }

                b'/' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::Slash, span));
                }

                b'%' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::Percent, span));
                }

                b'{' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::LBrace, span));
                }

                b'}' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::RBrace, span));
                }

                b'(' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::LParen, span));
                }

                b')' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::RParen, span));
                }

                b'[' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::LBracket, span));
                }

                b']' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::RBracket, span));
                }

                b',' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::Comma, span));
                }

                b'.' => {
                    let span = self.span();
                    self.advance();
                    tokens.push(Token::new(TokenKind::Dot, span));
                }

                b'0'..=b'9' => {
                    let span = self.span();
                    tokens.push(self.read_number(span, false)?);
                }

                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    tokens.push(self.read_ident_or_keyword());
                }

                _ => {
                    let span = self.span();
                    return Err(error::MorphError::mapping_at(
                        format!("unexpected character '{}'", ch as char),
                        span.line,
                        span.column,
                    ));
                }
            }
        }

        Ok(tokens)
    }

    fn read_string(&mut self) -> error::Result<Token> {
        let span = self.span();
        self.advance(); // skip opening "

        let mut parts: Vec<InterpolatedPart> = Vec::new();
        let mut current_literal = String::new();
        let mut has_interpolation = false;

        loop {
            match self.peek() {
                None => {
                    return Err(error::MorphError::mapping_at(
                        "unterminated string literal",
                        span.line,
                        span.column,
                    ));
                }
                Some(b'"') => {
                    self.advance();
                    break;
                }
                Some(b'{') => {
                    // Check if this is an interpolation
                    has_interpolation = true;
                    self.advance(); // consume '{'
                    if !current_literal.is_empty() {
                        parts.push(InterpolatedPart::Literal(std::mem::take(
                            &mut current_literal,
                        )));
                    }
                    let mut expr_str = String::new();
                    let mut depth = 1;
                    loop {
                        match self.advance() {
                            None => {
                                return Err(error::MorphError::mapping_at(
                                    "unterminated interpolation in string",
                                    span.line,
                                    span.column,
                                ));
                            }
                            Some(b'{') => {
                                depth += 1;
                                expr_str.push('{');
                            }
                            Some(b'}') => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                                expr_str.push('}');
                            }
                            Some(c) => {
                                if c < 0x80 {
                                    expr_str.push(c as char);
                                } else {
                                    self.pos -= 1;
                                    self.column -= 1;
                                    let remaining = &self.input[self.pos..];
                                    let remaining_str =
                                        std::str::from_utf8(remaining).map_err(|_| {
                                            error::MorphError::mapping_at(
                                                "invalid UTF-8 in string",
                                                self.line,
                                                self.column,
                                            )
                                        })?;
                                    let ch = remaining_str.chars().next().unwrap();
                                    expr_str.push(ch);
                                    let len = ch.len_utf8();
                                    for _ in 0..len {
                                        self.advance();
                                    }
                                }
                            }
                        }
                    }
                    parts.push(InterpolatedPart::Expression(expr_str));
                }
                Some(b'\\') => {
                    self.advance(); // consume backslash
                    match self.advance() {
                        Some(b'"') => current_literal.push('"'),
                        Some(b'\\') => current_literal.push('\\'),
                        Some(b'n') => current_literal.push('\n'),
                        Some(b't') => current_literal.push('\t'),
                        Some(b'r') => current_literal.push('\r'),
                        Some(b'{') => current_literal.push('{'),
                        Some(b'u') => {
                            // Unicode escape: \uXXXX
                            let mut hex = String::with_capacity(4);
                            for _ in 0..4 {
                                match self.advance() {
                                    Some(c) if c.is_ascii_hexdigit() => hex.push(c as char),
                                    _ => {
                                        return Err(error::MorphError::mapping_at(
                                            "invalid unicode escape sequence",
                                            span.line,
                                            span.column,
                                        ));
                                    }
                                }
                            }
                            let code = u32::from_str_radix(&hex, 16).unwrap();
                            match char::from_u32(code) {
                                Some(c) => current_literal.push(c),
                                None => {
                                    return Err(error::MorphError::mapping_at(
                                        format!("invalid unicode code point: \\u{hex}"),
                                        span.line,
                                        span.column,
                                    ));
                                }
                            }
                        }
                        Some(c) => {
                            return Err(error::MorphError::mapping_at(
                                format!("invalid escape sequence: \\{}", c as char),
                                span.line,
                                span.column,
                            ));
                        }
                        None => {
                            return Err(error::MorphError::mapping_at(
                                "unterminated string literal",
                                span.line,
                                span.column,
                            ));
                        }
                    }
                }
                Some(c) => {
                    self.advance();
                    // Handle multi-byte UTF-8
                    if c < 0x80 {
                        current_literal.push(c as char);
                    } else {
                        // Rewind one byte and read the full UTF-8 character
                        self.pos -= 1;
                        self.column -= 1;
                        let remaining = &self.input[self.pos..];
                        let remaining_str = std::str::from_utf8(remaining).map_err(|_| {
                            error::MorphError::mapping_at(
                                "invalid UTF-8 in string",
                                self.line,
                                self.column,
                            )
                        })?;
                        let ch = remaining_str.chars().next().unwrap();
                        current_literal.push(ch);
                        let len = ch.len_utf8();
                        for _ in 0..len {
                            self.advance();
                        }
                    }
                }
            }
        }

        if has_interpolation {
            if !current_literal.is_empty() {
                parts.push(InterpolatedPart::Literal(current_literal));
            }
            Ok(Token::new(TokenKind::InterpolatedString(parts), span))
        } else {
            Ok(Token::new(TokenKind::StringLit(current_literal), span))
        }
    }

    fn read_number(&mut self, span: Span, negative: bool) -> error::Result<Token> {
        let mut num_str = String::new();
        if negative {
            num_str.push('-');
        }

        let mut has_dot = false;
        let mut has_e = false;

        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => {
                    num_str.push(c as char);
                    self.advance();
                }
                b'.' => {
                    // Check if this is a decimal point or a path separator
                    if has_dot || has_e {
                        // Second dot or dot after e: error
                        return Err(error::MorphError::mapping_at(
                            format!("invalid number: {num_str}."),
                            span.line,
                            span.column,
                        ));
                    }
                    // Look ahead: if next char is a digit, it's a decimal
                    if let Some(next) = self.peek_next() {
                        if next.is_ascii_digit() {
                            has_dot = true;
                            num_str.push('.');
                            self.advance();
                        } else {
                            // It's a path separator — stop reading the number
                            break;
                        }
                    } else {
                        break;
                    }
                }
                b'e' | b'E' => {
                    if has_e {
                        break;
                    }
                    has_e = true;
                    has_dot = true; // treat scientific notation as float
                    num_str.push(c as char);
                    self.advance();
                    // Optional sign after e
                    if let Some(sign) = self.peek() {
                        if sign == b'+' || sign == b'-' {
                            num_str.push(sign as char);
                            self.advance();
                        }
                    }
                }
                b'_' => {
                    // Underscore in numbers (readability), skip it
                    self.advance();
                }
                _ => break,
            }
        }

        if has_dot || has_e {
            let f: f64 = num_str.parse().map_err(|_| {
                error::MorphError::mapping_at(
                    format!("invalid float literal: {num_str}"),
                    span.line,
                    span.column,
                )
            })?;
            Ok(Token::new(TokenKind::FloatLit(f), span))
        } else {
            let i: i64 = num_str.parse().map_err(|_| {
                error::MorphError::mapping_at(
                    format!("invalid integer literal: {num_str}"),
                    span.line,
                    span.column,
                )
            })?;
            Ok(Token::new(TokenKind::IntLit(i), span))
        }
    }

    fn read_ident_or_keyword(&mut self) -> Token {
        let span = self.span();
        let start = self.pos;

        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let word = std::str::from_utf8(&self.input[start..self.pos]).unwrap();

        let kind = match word {
            "rename" => TokenKind::Rename,
            "select" => TokenKind::Select,
            "drop" => TokenKind::Drop,
            "set" => TokenKind::Set,
            "default" => TokenKind::Default,
            "cast" => TokenKind::Cast,
            "as" => TokenKind::As,
            "where" => TokenKind::Where,
            "sort" => TokenKind::Sort,
            "each" => TokenKind::Each,
            "when" => TokenKind::When,
            "not" => TokenKind::Not,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "flatten" => TokenKind::Flatten,
            "nest" => TokenKind::Nest,
            "asc" => TokenKind::Asc,
            "desc" => TokenKind::Desc,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            _ => TokenKind::Ident(word.to_string()),
        };

        Token::new(kind, span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to extract token kinds from a tokenize result
    fn kinds(input: &str) -> Vec<TokenKind> {
        tokenize(input)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    // -----------------------------------------------------------------------
    // Single tokens: each keyword
    // -----------------------------------------------------------------------

    #[test]
    fn keyword_rename() {
        assert_eq!(kinds("rename"), vec![TokenKind::Rename]);
    }

    #[test]
    fn keyword_select() {
        assert_eq!(kinds("select"), vec![TokenKind::Select]);
    }

    #[test]
    fn keyword_drop() {
        assert_eq!(kinds("drop"), vec![TokenKind::Drop]);
    }

    #[test]
    fn keyword_set() {
        assert_eq!(kinds("set"), vec![TokenKind::Set]);
    }

    #[test]
    fn keyword_default() {
        assert_eq!(kinds("default"), vec![TokenKind::Default]);
    }

    #[test]
    fn keyword_cast() {
        assert_eq!(kinds("cast"), vec![TokenKind::Cast]);
    }

    #[test]
    fn keyword_as() {
        assert_eq!(kinds("as"), vec![TokenKind::As]);
    }

    #[test]
    fn keyword_where() {
        assert_eq!(kinds("where"), vec![TokenKind::Where]);
    }

    #[test]
    fn keyword_sort() {
        assert_eq!(kinds("sort"), vec![TokenKind::Sort]);
    }

    #[test]
    fn keyword_each() {
        assert_eq!(kinds("each"), vec![TokenKind::Each]);
    }

    #[test]
    fn keyword_when() {
        assert_eq!(kinds("when"), vec![TokenKind::When]);
    }

    #[test]
    fn keyword_not() {
        assert_eq!(kinds("not"), vec![TokenKind::Not]);
    }

    #[test]
    fn keyword_and() {
        assert_eq!(kinds("and"), vec![TokenKind::And]);
    }

    #[test]
    fn keyword_or() {
        assert_eq!(kinds("or"), vec![TokenKind::Or]);
    }

    #[test]
    fn keyword_flatten() {
        assert_eq!(kinds("flatten"), vec![TokenKind::Flatten]);
    }

    #[test]
    fn keyword_nest() {
        assert_eq!(kinds("nest"), vec![TokenKind::Nest]);
    }

    #[test]
    fn keyword_asc() {
        assert_eq!(kinds("asc"), vec![TokenKind::Asc]);
    }

    #[test]
    fn keyword_desc() {
        assert_eq!(kinds("desc"), vec![TokenKind::Desc]);
    }

    #[test]
    fn all_keywords() {
        let input = "rename select drop set default cast as where sort each when not and or flatten nest asc desc";
        let k = kinds(input);
        assert_eq!(
            k,
            vec![
                TokenKind::Rename,
                TokenKind::Select,
                TokenKind::Drop,
                TokenKind::Set,
                TokenKind::Default,
                TokenKind::Cast,
                TokenKind::As,
                TokenKind::Where,
                TokenKind::Sort,
                TokenKind::Each,
                TokenKind::When,
                TokenKind::Not,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Flatten,
                TokenKind::Nest,
                TokenKind::Asc,
                TokenKind::Desc,
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Single tokens: each operator
    // -----------------------------------------------------------------------

    #[test]
    fn operator_arrow() {
        assert_eq!(kinds("->"), vec![TokenKind::Arrow]);
    }

    #[test]
    fn operator_eq() {
        assert_eq!(kinds("="), vec![TokenKind::Eq]);
    }

    #[test]
    fn operator_eqeq() {
        assert_eq!(kinds("=="), vec![TokenKind::EqEq]);
    }

    #[test]
    fn operator_noteq() {
        assert_eq!(kinds("!="), vec![TokenKind::NotEq]);
    }

    #[test]
    fn operator_gt() {
        assert_eq!(kinds(">"), vec![TokenKind::Gt]);
    }

    #[test]
    fn operator_gteq() {
        assert_eq!(kinds(">="), vec![TokenKind::GtEq]);
    }

    #[test]
    fn operator_lt() {
        assert_eq!(kinds("<"), vec![TokenKind::Lt]);
    }

    #[test]
    fn operator_lteq() {
        assert_eq!(kinds("<="), vec![TokenKind::LtEq]);
    }

    #[test]
    fn operator_plus() {
        assert_eq!(kinds("+"), vec![TokenKind::Plus]);
    }

    #[test]
    fn operator_minus() {
        // Bare minus (not followed by digit) is minus operator
        assert_eq!(kinds("- "), vec![TokenKind::Minus]);
    }

    #[test]
    fn operator_star() {
        assert_eq!(kinds("*"), vec![TokenKind::Star]);
    }

    #[test]
    fn operator_slash() {
        assert_eq!(kinds("/"), vec![TokenKind::Slash]);
    }

    #[test]
    fn operator_percent() {
        assert_eq!(kinds("%"), vec![TokenKind::Percent]);
    }

    // -----------------------------------------------------------------------
    // Single tokens: each delimiter
    // -----------------------------------------------------------------------

    #[test]
    fn delimiter_lbrace() {
        assert_eq!(kinds("{"), vec![TokenKind::LBrace]);
    }

    #[test]
    fn delimiter_rbrace() {
        assert_eq!(kinds("}"), vec![TokenKind::RBrace]);
    }

    #[test]
    fn delimiter_lparen() {
        assert_eq!(kinds("("), vec![TokenKind::LParen]);
    }

    #[test]
    fn delimiter_rparen() {
        assert_eq!(kinds(")"), vec![TokenKind::RParen]);
    }

    #[test]
    fn delimiter_lbracket() {
        assert_eq!(kinds("["), vec![TokenKind::LBracket]);
    }

    #[test]
    fn delimiter_rbracket() {
        assert_eq!(kinds("]"), vec![TokenKind::RBracket]);
    }

    #[test]
    fn delimiter_comma() {
        assert_eq!(kinds(","), vec![TokenKind::Comma]);
    }

    #[test]
    fn delimiter_dot() {
        assert_eq!(kinds("."), vec![TokenKind::Dot]);
    }

    // -----------------------------------------------------------------------
    // String literals
    // -----------------------------------------------------------------------

    #[test]
    fn string_simple() {
        assert_eq!(
            kinds("\"hello\""),
            vec![TokenKind::StringLit("hello".into())]
        );
    }

    #[test]
    fn string_empty() {
        assert_eq!(kinds("\"\""), vec![TokenKind::StringLit("".into())]);
    }

    #[test]
    fn string_with_spaces() {
        assert_eq!(
            kinds("\"hello world\""),
            vec![TokenKind::StringLit("hello world".into())]
        );
    }

    #[test]
    fn string_escape_quote() {
        assert_eq!(
            kinds(r#""say \"hi\"""#),
            vec![TokenKind::StringLit("say \"hi\"".into())]
        );
    }

    #[test]
    fn string_escape_backslash() {
        assert_eq!(
            kinds(r#""path\\to""#),
            vec![TokenKind::StringLit("path\\to".into())]
        );
    }

    #[test]
    fn string_escape_newline() {
        assert_eq!(
            kinds(r#""line\nbreak""#),
            vec![TokenKind::StringLit("line\nbreak".into())]
        );
    }

    #[test]
    fn string_escape_tab() {
        assert_eq!(
            kinds(r#""col\tcol""#),
            vec![TokenKind::StringLit("col\tcol".into())]
        );
    }

    #[test]
    fn string_escape_cr() {
        assert_eq!(
            kinds(r#""a\rb""#),
            vec![TokenKind::StringLit("a\rb".into())]
        );
    }

    #[test]
    fn string_unicode_escape() {
        assert_eq!(kinds(r#""\u0041""#), vec![TokenKind::StringLit("A".into())]);
    }

    #[test]
    fn string_unicode_escape_emoji() {
        // 🦀 = U+1F980 — but \u only supports 4 hex chars (BMP).
        // Test a BMP character: ñ = U+00F1
        assert_eq!(kinds(r#""\u00F1""#), vec![TokenKind::StringLit("ñ".into())]);
    }

    #[test]
    fn string_with_unicode_literal() {
        // Direct UTF-8 in source
        assert_eq!(
            kinds("\"🦀hello\""),
            vec![TokenKind::StringLit("🦀hello".into())]
        );
    }

    // -----------------------------------------------------------------------
    // Number literals
    // -----------------------------------------------------------------------

    #[test]
    fn number_integer() {
        assert_eq!(kinds("42"), vec![TokenKind::IntLit(42)]);
    }

    #[test]
    fn number_zero() {
        assert_eq!(kinds("0"), vec![TokenKind::IntLit(0)]);
    }

    #[test]
    fn number_negative() {
        assert_eq!(kinds("-7"), vec![TokenKind::IntLit(-7)]);
    }

    #[test]
    fn number_float() {
        assert_eq!(kinds("3.25"), vec![TokenKind::FloatLit(3.25)]);
    }

    #[test]
    fn number_negative_float() {
        assert_eq!(kinds("-1.5"), vec![TokenKind::FloatLit(-1.5)]);
    }

    #[test]
    fn number_scientific() {
        assert_eq!(kinds("1e10"), vec![TokenKind::FloatLit(1e10)]);
    }

    #[test]
    fn number_scientific_upper() {
        assert_eq!(kinds("1E10"), vec![TokenKind::FloatLit(1e10)]);
    }

    #[test]
    fn number_scientific_negative_exp() {
        assert_eq!(kinds("5e-3"), vec![TokenKind::FloatLit(5e-3)]);
    }

    #[test]
    fn number_scientific_positive_exp() {
        assert_eq!(kinds("5e+3"), vec![TokenKind::FloatLit(5e3)]);
    }

    #[test]
    fn number_large_integer() {
        assert_eq!(kinds("1000000"), vec![TokenKind::IntLit(1_000_000)]);
    }

    // -----------------------------------------------------------------------
    // Boolean and null literals
    // -----------------------------------------------------------------------

    #[test]
    fn literal_true() {
        assert_eq!(kinds("true"), vec![TokenKind::True]);
    }

    #[test]
    fn literal_false() {
        assert_eq!(kinds("false"), vec![TokenKind::False]);
    }

    #[test]
    fn literal_null() {
        assert_eq!(kinds("null"), vec![TokenKind::Null]);
    }

    // -----------------------------------------------------------------------
    // Identifiers
    // -----------------------------------------------------------------------

    #[test]
    fn identifier_simple() {
        assert_eq!(kinds("my_func"), vec![TokenKind::Ident("my_func".into())]);
    }

    #[test]
    fn identifier_with_numbers() {
        assert_eq!(kinds("func123"), vec![TokenKind::Ident("func123".into())]);
    }

    #[test]
    fn identifier_starts_with_underscore() {
        assert_eq!(kinds("_private"), vec![TokenKind::Ident("_private".into())]);
    }

    #[test]
    fn identifier_not_keyword_prefix() {
        // "setter" starts with "set" but is an identifier
        assert_eq!(kinds("setter"), vec![TokenKind::Ident("setter".into())]);
    }

    // -----------------------------------------------------------------------
    // Paths: .a, .a.b.c, .[0], .[*], .["key"]
    // -----------------------------------------------------------------------

    #[test]
    fn path_single_field() {
        assert_eq!(
            kinds(".name"),
            vec![TokenKind::Dot, TokenKind::Ident("name".into())]
        );
    }

    #[test]
    fn path_nested() {
        assert_eq!(
            kinds(".a.b.c"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("a".into()),
                TokenKind::Dot,
                TokenKind::Ident("b".into()),
                TokenKind::Dot,
                TokenKind::Ident("c".into()),
            ]
        );
    }

    #[test]
    fn path_array_index() {
        assert_eq!(
            kinds(".[0]"),
            vec![
                TokenKind::Dot,
                TokenKind::LBracket,
                TokenKind::IntLit(0),
                TokenKind::RBracket,
            ]
        );
    }

    #[test]
    fn path_array_wildcard() {
        assert_eq!(
            kinds(".[*]"),
            vec![
                TokenKind::Dot,
                TokenKind::LBracket,
                TokenKind::Star,
                TokenKind::RBracket,
            ]
        );
    }

    #[test]
    fn path_quoted_key() {
        assert_eq!(
            kinds(".[\"key\"]"),
            vec![
                TokenKind::Dot,
                TokenKind::LBracket,
                TokenKind::StringLit("key".into()),
                TokenKind::RBracket,
            ]
        );
    }

    #[test]
    fn path_mixed() {
        // .users.[0].name
        assert_eq!(
            kinds(".users.[0].name"),
            vec![
                TokenKind::Dot,
                TokenKind::Ident("users".into()),
                TokenKind::Dot,
                TokenKind::LBracket,
                TokenKind::IntLit(0),
                TokenKind::RBracket,
                TokenKind::Dot,
                TokenKind::Ident("name".into()),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Comments
    // -----------------------------------------------------------------------

    #[test]
    fn comment_ignored() {
        assert_eq!(kinds("# this is a comment"), vec![]);
    }

    #[test]
    fn comment_with_code_before() {
        assert_eq!(kinds("rename # comment"), vec![TokenKind::Rename]);
    }

    #[test]
    fn comment_preserves_newline() {
        assert_eq!(
            kinds("rename\n# comment\nselect"),
            vec![TokenKind::Rename, TokenKind::Newline, TokenKind::Select,]
        );
    }

    #[test]
    fn comment_at_end_of_line() {
        let k = kinds("set .x = 42 # set x to 42\nselect .x");
        assert!(k.contains(&TokenKind::Set));
        assert!(k.contains(&TokenKind::Select));
    }

    // -----------------------------------------------------------------------
    // Multi-line: newlines as statement separators
    // -----------------------------------------------------------------------

    #[test]
    fn newline_between_statements() {
        assert_eq!(
            kinds("rename\nselect"),
            vec![TokenKind::Rename, TokenKind::Newline, TokenKind::Select,]
        );
    }

    #[test]
    fn multiple_newlines_collapsed() {
        assert_eq!(
            kinds("rename\n\n\nselect"),
            vec![TokenKind::Rename, TokenKind::Newline, TokenKind::Select,]
        );
    }

    #[test]
    fn no_trailing_newline_token() {
        // Newline at end of input should not produce a trailing Newline token
        // (since there's no next statement)
        assert_eq!(
            kinds("rename\n"),
            vec![TokenKind::Rename, TokenKind::Newline,]
        );
    }

    #[test]
    fn no_leading_newline_token() {
        // Newlines at start of input should be skipped
        assert_eq!(kinds("\n\nrename"), vec![TokenKind::Rename]);
    }

    // -----------------------------------------------------------------------
    // Whitespace: spaces and tabs ignored between tokens
    // -----------------------------------------------------------------------

    #[test]
    fn whitespace_spaces() {
        assert_eq!(
            kinds("   rename   select   "),
            vec![TokenKind::Rename, TokenKind::Select]
        );
    }

    #[test]
    fn whitespace_tabs() {
        assert_eq!(
            kinds("\trename\t\tselect"),
            vec![TokenKind::Rename, TokenKind::Select]
        );
    }

    #[test]
    fn whitespace_mixed() {
        assert_eq!(
            kinds("  \t rename \t select  \t "),
            vec![TokenKind::Rename, TokenKind::Select]
        );
    }

    // -----------------------------------------------------------------------
    // Full statement: rename .old -> .new
    // -----------------------------------------------------------------------

    #[test]
    fn full_statement_rename() {
        assert_eq!(
            kinds("rename .old -> .new"),
            vec![
                TokenKind::Rename,
                TokenKind::Dot,
                TokenKind::Ident("old".into()),
                TokenKind::Arrow,
                TokenKind::Dot,
                TokenKind::Ident("new".into()),
            ]
        );
    }

    #[test]
    fn full_statement_set() {
        assert_eq!(
            kinds("set .active = true"),
            vec![
                TokenKind::Set,
                TokenKind::Dot,
                TokenKind::Ident("active".into()),
                TokenKind::Eq,
                TokenKind::True,
            ]
        );
    }

    #[test]
    fn full_statement_select() {
        assert_eq!(
            kinds("select .name, .age"),
            vec![
                TokenKind::Select,
                TokenKind::Dot,
                TokenKind::Ident("name".into()),
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Ident("age".into()),
            ]
        );
    }

    #[test]
    fn full_statement_cast() {
        assert_eq!(
            kinds("cast .age as \"int\""),
            vec![
                TokenKind::Cast,
                TokenKind::Dot,
                TokenKind::Ident("age".into()),
                TokenKind::As,
                TokenKind::StringLit("int".into()),
            ]
        );
    }

    #[test]
    fn full_multi_statement() {
        let input = "rename .old -> .new\nset .x = 42\nselect .new, .x";
        let k = kinds(input);
        assert_eq!(
            k,
            vec![
                // rename .old -> .new
                TokenKind::Rename,
                TokenKind::Dot,
                TokenKind::Ident("old".into()),
                TokenKind::Arrow,
                TokenKind::Dot,
                TokenKind::Ident("new".into()),
                TokenKind::Newline,
                // set .x = 42
                TokenKind::Set,
                TokenKind::Dot,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::IntLit(42),
                TokenKind::Newline,
                // select .new, .x
                TokenKind::Select,
                TokenKind::Dot,
                TokenKind::Ident("new".into()),
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Ident("x".into()),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Error: unterminated string
    // -----------------------------------------------------------------------

    #[test]
    fn error_unterminated_string() {
        let err = tokenize("\"hello").unwrap_err();
        match err {
            error::MorphError::Mapping {
                message,
                line,
                column,
            } => {
                assert!(message.contains("unterminated"), "msg: {message}");
                assert_eq!(line, Some(1));
                assert_eq!(column, Some(1));
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    #[test]
    fn error_unterminated_string_on_line_2() {
        let err = tokenize("rename\n\"hello").unwrap_err();
        match err {
            error::MorphError::Mapping { line, column, .. } => {
                assert_eq!(line, Some(2));
                assert_eq!(column, Some(1));
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: invalid character
    // -----------------------------------------------------------------------

    #[test]
    fn error_invalid_character() {
        let err = tokenize("@").unwrap_err();
        match err {
            error::MorphError::Mapping {
                message,
                line,
                column,
            } => {
                assert!(message.contains("unexpected character"), "msg: {message}");
                assert_eq!(line, Some(1));
                assert_eq!(column, Some(1));
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    #[test]
    fn error_invalid_character_position() {
        let err = tokenize("rename @").unwrap_err();
        match err {
            error::MorphError::Mapping { line, column, .. } => {
                assert_eq!(line, Some(1));
                assert_eq!(column, Some(8));
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Error: invalid number
    // -----------------------------------------------------------------------

    #[test]
    fn error_invalid_number_double_dot() {
        let err = tokenize("42.43.44").unwrap_err();
        match err {
            error::MorphError::Mapping {
                message,
                line,
                column,
            } => {
                assert!(message.contains("invalid number"), "msg: {message}");
                assert_eq!(line, Some(1));
                assert_eq!(column, Some(1));
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Empty input
    // -----------------------------------------------------------------------

    #[test]
    fn empty_input() {
        assert_eq!(kinds(""), Vec::<TokenKind>::new());
    }

    #[test]
    fn only_whitespace() {
        assert_eq!(kinds("   \t  "), Vec::<TokenKind>::new());
    }

    #[test]
    fn only_comments() {
        assert_eq!(kinds("# just a comment"), Vec::<TokenKind>::new());
    }

    // -----------------------------------------------------------------------
    // Span correctness
    // -----------------------------------------------------------------------

    #[test]
    fn span_first_token() {
        let tokens = tokenize("rename").unwrap();
        assert_eq!(tokens[0].span, Span::new(1, 1));
    }

    #[test]
    fn span_second_token() {
        let tokens = tokenize("rename select").unwrap();
        assert_eq!(tokens[0].span, Span::new(1, 1));
        assert_eq!(tokens[1].span, Span::new(1, 8));
    }

    #[test]
    fn span_multiline() {
        let tokens = tokenize("rename\nselect").unwrap();
        // rename at (1,1), newline at (1,7), select at (2,1)
        assert_eq!(tokens[0].span, Span::new(1, 1));
        assert_eq!(tokens[2].span, Span::new(2, 1));
    }

    #[test]
    fn span_after_string() {
        let tokens = tokenize("\"abc\" rename").unwrap();
        assert_eq!(tokens[0].span, Span::new(1, 1));
        assert_eq!(tokens[1].span, Span::new(1, 7));
    }

    // -----------------------------------------------------------------------
    // Subtraction vs negative number
    // -----------------------------------------------------------------------

    #[test]
    fn minus_after_number_is_operator() {
        let k = kinds("42 - 7");
        assert_eq!(
            k,
            vec![
                TokenKind::IntLit(42),
                TokenKind::Minus,
                TokenKind::IntLit(7)
            ]
        );
    }

    #[test]
    fn minus_after_ident_is_operator() {
        let k = kinds("x -7");
        assert_eq!(
            k,
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::Minus,
                TokenKind::IntLit(7),
            ]
        );
    }

    #[test]
    fn negative_number_at_start() {
        assert_eq!(kinds("-42"), vec![TokenKind::IntLit(-42)]);
    }

    #[test]
    fn negative_number_after_eq() {
        let k = kinds("= -5");
        assert_eq!(k, vec![TokenKind::Eq, TokenKind::IntLit(-5)]);
    }

    #[test]
    fn negative_number_in_parens() {
        let k = kinds("(-5)");
        assert_eq!(
            k,
            vec![TokenKind::LParen, TokenKind::IntLit(-5), TokenKind::RParen,]
        );
    }

    // -----------------------------------------------------------------------
    // Error: invalid escape in string
    // -----------------------------------------------------------------------

    #[test]
    fn error_invalid_escape() {
        let err = tokenize(r#""\q""#).unwrap_err();
        match err {
            error::MorphError::Mapping { message, .. } => {
                assert!(message.contains("invalid escape"), "msg: {message}");
            }
            other => panic!("expected Mapping error, got: {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Complex expression
    // -----------------------------------------------------------------------

    #[test]
    fn complex_expression() {
        let input = "where .age >= 18 and .name != \"admin\"";
        let k = kinds(input);
        assert_eq!(
            k,
            vec![
                TokenKind::Where,
                TokenKind::Dot,
                TokenKind::Ident("age".into()),
                TokenKind::GtEq,
                TokenKind::IntLit(18),
                TokenKind::And,
                TokenKind::Dot,
                TokenKind::Ident("name".into()),
                TokenKind::NotEq,
                TokenKind::StringLit("admin".into()),
            ]
        );
    }

    #[test]
    fn arithmetic_expression() {
        let input = "set .total = .price * .qty + .tax";
        let k = kinds(input);
        assert_eq!(
            k,
            vec![
                TokenKind::Set,
                TokenKind::Dot,
                TokenKind::Ident("total".into()),
                TokenKind::Eq,
                TokenKind::Dot,
                TokenKind::Ident("price".into()),
                TokenKind::Star,
                TokenKind::Dot,
                TokenKind::Ident("qty".into()),
                TokenKind::Plus,
                TokenKind::Dot,
                TokenKind::Ident("tax".into()),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Sort expression
    // -----------------------------------------------------------------------

    #[test]
    fn sort_with_direction() {
        let k = kinds("sort .name asc, .age desc");
        assert_eq!(
            k,
            vec![
                TokenKind::Sort,
                TokenKind::Dot,
                TokenKind::Ident("name".into()),
                TokenKind::Asc,
                TokenKind::Comma,
                TokenKind::Dot,
                TokenKind::Ident("age".into()),
                TokenKind::Desc,
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Each/when block with braces
    // -----------------------------------------------------------------------

    #[test]
    fn each_block() {
        let input = "each .items {\n  set .processed = true\n}";
        let k = kinds(input);
        assert_eq!(
            k,
            vec![
                TokenKind::Each,
                TokenKind::Dot,
                TokenKind::Ident("items".into()),
                TokenKind::LBrace,
                TokenKind::Newline,
                TokenKind::Set,
                TokenKind::Dot,
                TokenKind::Ident("processed".into()),
                TokenKind::Eq,
                TokenKind::True,
                TokenKind::Newline,
                TokenKind::RBrace,
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Function call syntax
    // -----------------------------------------------------------------------

    #[test]
    fn function_call() {
        let k = kinds("set .name = lower(.name)");
        assert_eq!(
            k,
            vec![
                TokenKind::Set,
                TokenKind::Dot,
                TokenKind::Ident("name".into()),
                TokenKind::Eq,
                TokenKind::Ident("lower".into()),
                TokenKind::LParen,
                TokenKind::Dot,
                TokenKind::Ident("name".into()),
                TokenKind::RParen,
            ]
        );
    }

    #[test]
    fn function_call_multi_arg() {
        let k = kinds("replace(.name, \"old\", \"new\")");
        assert_eq!(
            k,
            vec![
                TokenKind::Ident("replace".into()),
                TokenKind::LParen,
                TokenKind::Dot,
                TokenKind::Ident("name".into()),
                TokenKind::Comma,
                TokenKind::StringLit("old".into()),
                TokenKind::Comma,
                TokenKind::StringLit("new".into()),
                TokenKind::RParen,
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Number followed by dot (path vs decimal)
    // -----------------------------------------------------------------------

    #[test]
    fn integer_then_dot_field() {
        // "42.field" should be Int(42), Dot, Ident("field")
        // because after "42" the "." is not followed by a digit
        let k = kinds("42.field");
        assert_eq!(
            k,
            vec![
                TokenKind::IntLit(42),
                TokenKind::Dot,
                TokenKind::Ident("field".into()),
            ]
        );
    }
}
