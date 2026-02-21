use crate::mapping::lexer::Span;

/// A field path like `.name`, `.users.[0].name`, or `.a.b.c`.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: Vec<PathSegment>,
    pub span: Span,
}

/// A single segment in a field path.
#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    /// A named field: `.name`
    Field(String),
    /// An array index: `.[0]`
    Index(i64),
    /// A wildcard: `.[*]`
    Wildcard,
}

/// The target type for cast operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastType {
    Int,
    Float,
    String,
    Bool,
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Gt,
    GtEq,
    Lt,
    LtEq,
    And,
    Or,
}

/// A unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// An expression in the mapping language.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A literal value: `42`, `"hello"`, `true`, `null`.
    Literal(crate::value::Value),
    /// A path reference: `.name`, `.users.[0].age`.
    Path(Path),
    /// A function call: `lower(.name)`, `replace(.s, "a", "b")`.
    FunctionCall {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// A binary operation: `.a + .b`, `.x == 42`.
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    /// A unary operation: `not .active`, `-.value`.
    UnaryOp { op: UnaryOp, expr: Box<Expr> },
}

/// A statement in the mapping language.
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// `rename .old -> .new`
    Rename { from: Path, to: Path, span: Span },
    /// `select .a, .b, .c`
    Select { paths: Vec<Path>, span: Span },
    /// `drop .x, .y`
    Drop { paths: Vec<Path>, span: Span },
    /// `set .x = <expr>`
    Set { path: Path, expr: Expr, span: Span },
    /// `default .x = <expr>`
    Default { path: Path, expr: Expr, span: Span },
    /// `cast .x as int`
    Cast {
        path: Path,
        target_type: CastType,
        span: Span,
    },
    /// `flatten .address` or `flatten .address -> prefix "addr"`
    Flatten {
        path: Path,
        prefix: Option<String>,
        span: Span,
    },
    /// `nest .a_x, .a_y -> .a`
    Nest {
        paths: Vec<Path>,
        target: Path,
        span: Span,
    },
    /// `where <condition>` — filter array elements by condition
    Where { condition: Expr, span: Span },
}

/// A parsed mapping program: a list of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}
