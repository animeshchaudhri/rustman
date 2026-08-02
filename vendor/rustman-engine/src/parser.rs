//! Recursive-descent parser: tokens -> AST.

use crate::ast::{BinOp, Expr, Stmt, UnaryOp};
use crate::lexer::Token;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
}

pub fn parse(tokens: &[(Token, usize)]) -> Result<Vec<Stmt>, ParseError> {
    let mut parser = Parser { tokens, pos: 0 };
    let mut statements = Vec::new();
    while !parser.is_at_end() {
        statements.push(parser.statement()?);
    }
    Ok(statements)
}

struct Parser<'a> {
    tokens: &'a [(Token, usize)],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].0
    }

    fn line(&self) -> usize {
        self.tokens[self.pos].1
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].0.clone();
        if !self.is_at_end() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, expected: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(expected)
    }

    fn eat(&mut self, expected: &Token, what: &str) -> Result<Token, ParseError> {
        if self.check(expected) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("expected {what}, found {:?}", self.peek()),
                line: self.line(),
            })
        }
    }

    /// Consumes a single optional statement terminator (`;` or nothing —
    /// newlines aren't tracked as tokens, so statements are simply
    /// terminated by the next statement/brace starting).
    fn eat_optional_semi(&mut self) {
        if matches!(self.peek(), Token::Semicolon) {
            self.advance();
        }
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        match self.peek() {
            Token::Let => {
                self.advance();
                let name = match self.advance() {
                    Token::Ident(name) => name,
                    other => {
                        return Err(ParseError {
                            message: format!("expected identifier after 'let', found {other:?}"),
                            line: self.line(),
                        });
                    }
                };
                self.eat(&Token::Eq, "'='")?;
                let value = self.expr()?;
                self.eat_optional_semi();
                Ok(Stmt::Let { name, value })
            }
            Token::If => {
                self.advance();
                let cond = self.expr()?;
                self.eat(&Token::LBrace, "'{'")?;
                let then_branch = self.block()?;
                let else_branch = if matches!(self.peek(), Token::Else) {
                    self.advance();
                    self.eat(&Token::LBrace, "'{' after 'else'")?;
                    self.block()?
                } else {
                    Vec::new()
                };
                Ok(Stmt::If { cond, then_branch, else_branch })
            }
            _ => {
                let expr = self.expr()?;
                self.eat_optional_semi();
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = Vec::new();
        while !matches!(self.peek(), Token::RBrace) && !self.is_at_end() {
            statements.push(self.statement()?);
        }
        self.eat(&Token::RBrace, "'}'")?;
        Ok(statements)
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.and_expr()?;
        while matches!(self.peek(), Token::OrOr) {
            self.advance();
            let right = self.and_expr()?;
            left = Expr::Binary(Box::new(left), BinOp::Or, Box::new(right));
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.equality()?;
        while matches!(self.peek(), Token::AndAnd) {
            self.advance();
            let right = self.equality()?;
            left = Expr::Binary(Box::new(left), BinOp::And, Box::new(right));
        }
        Ok(left)
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.comparison()?;
        loop {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::NotEq => BinOp::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.comparison()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.additive()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.additive()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.unary()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.peek(), Token::Bang) {
            self.advance();
            let operand = self.unary()?;
            return Ok(Expr::Unary(UnaryOp::Not, Box::new(operand)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    let name = match self.advance() {
                        Token::Ident(name) => name,
                        other => {
                            return Err(ParseError {
                                message: format!("expected field name after '.', found {other:?}"),
                                line: self.line(),
                            });
                        }
                    };
                    expr = Expr::Field(Box::new(expr), name);
                }
                Token::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Token::RParen) {
                        args.push(self.expr()?);
                        while matches!(self.peek(), Token::Comma) {
                            self.advance();
                            args.push(self.expr()?);
                        }
                    }
                    self.eat(&Token::RParen, "')'")?;
                    expr = Expr::Call(Box::new(expr), args);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        match self.advance() {
            Token::Number(n) => Ok(Expr::Number(n)),
            Token::String(s) => Ok(Expr::String(s)),
            Token::True => Ok(Expr::Bool(true)),
            Token::False => Ok(Expr::Bool(false)),
            Token::Null => Ok(Expr::Null),
            Token::Ident(name) => Ok(Expr::Ident(name)),
            Token::LParen => {
                let expr = self.expr()?;
                self.eat(&Token::RParen, "')'")?;
                Ok(expr)
            }
            other => Err(ParseError {
                message: format!("unexpected token {other:?} in expression"),
                line: self.line(),
            }),
        }
    }
}
