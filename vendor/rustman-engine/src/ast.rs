//! AST node definitions for the script language.

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, value: Expr },
    If { cond: Expr, then_branch: Vec<Stmt>, else_branch: Vec<Stmt> },
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Ident(String),
    Field(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Binary(Box<Expr>, BinOp, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}
