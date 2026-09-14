use crate::ast::{Ast, AstError, Expr, ExprId, Command, Level, Quantity, Span as AstSpan};
use crate::lexer::{Token, TokenKind, Span as LexerSpan};
use std::fmt;

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedEof,
    UnexpectedToken {
        expected: &'static str,
        found: Option<TokenKind>,
        span: Option<AstSpan>,
    },
    UnknownVariable {
        name: String,
        span: AstSpan,
    },
    AstError(AstError),
}

impl From<AstError> for ParseError {
    fn from(err: AstError) -> Self { Self::AstError(err) }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of file"),
            Self::UnexpectedToken { expected, found, .. } => write!(f, "expected {}, but found {:?}", expected, found),
            Self::UnknownVariable { name, .. } => write!(f, "unknown variable '{}'", name),
            Self::AstError(err) => write!(f, "ast error: {}", err),
        }
    }
}
impl std::error::Error for ParseError {}

fn convert_span(span: LexerSpan) -> AstSpan { AstSpan::new(span.start, span.end).unwrap() }

pub struct Parser<'a> {
    tokens: &'a [Token<'a>],
    cursor: usize,
    ast: &'a mut Ast,
    env: Vec<String>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token<'a>], ast: &'a mut Ast, global_env: &[String]) -> Self {
        Self { tokens, cursor: 0, ast, env: global_env.to_vec() }
    }

    fn peek(&self) -> Option<&'a Token<'a>> { self.tokens.get(self.cursor) }

    fn advance(&mut self) -> Option<&'a Token<'a>> {
        let tok = self.peek()?;
        self.cursor += 1;
        Some(tok)
    }

    fn expect(&mut self, kind: TokenKind, expected: &'static str) -> Result<&'a Token<'a>, ParseError> {
        let span = self.peek().map(|t| convert_span(t.span));
        let found = self.peek().map(|t| t.kind);
        if found == Some(kind) { Ok(self.advance().unwrap()) } else { Err(ParseError::UnexpectedToken { expected, found, span }) }
    }

    pub fn parse_command(&mut self) -> Result<Command, ParseError> {
        if let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Let {
                self.advance();
                let name_tok = self.expect(TokenKind::Ident, "identifier")?;
                let name = name_tok.text.to_string();
                
                let mut ty = None;
                if let Some(colon) = self.peek() {
                    if colon.kind == TokenKind::Colon {
                        self.advance();
                        ty = Some(self.parse_expression()?);
                    }
                }
                
                self.expect(TokenKind::Eq, "'='")?;
                let term = self.parse_expression()?;
                
                if let Some(semi) = self.peek() {
                    if semi.kind == TokenKind::Semicolon {
                        self.advance();
                    }
                }
                
                return Ok(Command::Let { name, ty, term });
            }
        }
        
        let expr = self.parse_expression()?;
        if let Some(semi) = self.peek() {
            if semi.kind == TokenKind::Semicolon {
                self.advance();
            }
        }
        Ok(Command::Eval(expr))
    }

    pub fn parse_expression(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_atom()?;
        while let Some(tok) = self.peek() {
            match tok.kind {
                TokenKind::Ident | TokenKind::LParen | TokenKind::Fn | TokenKind::Type => {
                    let argument = self.parse_atom()?;
                    expr = self.ast.push(Expr::App { function: expr, argument })?;
                }
                TokenKind::Arrow => {
                    self.advance();
                    let codomain = self.parse_expression()?;
                    expr = self.ast.push(Expr::Pi { quantity: Quantity::Omega, domain: expr, codomain })?;
                }
                _ => break,
            }
        }
        if let Some(tok) = self.peek() {
            if tok.kind == TokenKind::Colon {
                self.advance();
                let ty = self.parse_expression()?;
                expr = self.ast.push(Expr::Ann { term: expr, ty })?;
            }
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> Result<ExprId, ParseError> {
        let tok = self.peek().ok_or(ParseError::UnexpectedEof)?;
        match tok.kind {
            TokenKind::Type => {
                let start_span = tok.span;
                self.advance();
                let mut level = 0;
                if let Some(next_tok) = self.peek() {
                    if let TokenKind::NatLiteral(n) = next_tok.kind {
                        level = n as u32;
                        self.advance();
                    }
                }
                let end_span = self.tokens.get(self.cursor.saturating_sub(1)).map(|t| t.span).unwrap_or(start_span);
                let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                Ok(self.ast.push_spanned_exact(Expr::Universe(level), span)?)
            }
            TokenKind::Fn => {
                let start_span = tok.span;
                self.advance();
                let mut quantity = Quantity::Omega;
                if let Some(qtok) = self.peek() {
                    match qtok.kind {
                        TokenKind::QuantZero => { quantity = Quantity::Zero; self.advance(); }
                        TokenKind::QuantOne => { quantity = Quantity::One; self.advance(); }
                        TokenKind::QuantOmega => { quantity = Quantity::Omega; self.advance(); }
                        _ => {}
                    }
                }
                
                let is_pi = if let Some(tok) = self.peek() { tok.kind == TokenKind::LParen } else { false };
                
                if is_pi {
                    self.advance(); // consume LParen
                    let param_tok = self.expect(TokenKind::Ident, "identifier")?;
                    let param_name = param_tok.text;
                    self.expect(TokenKind::Colon, "':'")?;
                    let domain = self.parse_expression()?;
                    self.expect(TokenKind::RParen, "')'")?;
                    
                    self.expect(TokenKind::Arrow, "'->'")?;
                    self.env.push(param_name.to_string());
                    let codomain = self.parse_expression()?;
                    self.env.pop();
                    
                    let end_span = self.tokens.get(self.cursor.saturating_sub(1)).map(|t| t.span).unwrap_or(start_span);
                    let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                    Ok(self.ast.push_spanned_exact(Expr::Pi { quantity, domain, codomain }, span)?)
                } else {
                    let param_tok = self.expect(TokenKind::Ident, "identifier")?;
                    let param_name = param_tok.text;
                    
                    self.expect(TokenKind::Arrow, "'->'")?;
                    self.env.push(param_name.to_string());
                    let body = self.parse_expression()?;
                    self.env.pop();
                    let end_span = self.tokens.get(self.cursor.saturating_sub(1)).map(|t| t.span).unwrap_or(start_span);
                    let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                    Ok(self.ast.push_spanned_exact(Expr::Lambda { quantity, body }, span)?)
                }
            }
            TokenKind::Ident => {
                let name = tok.text;
                let span = convert_span(tok.span);
                self.advance();
                let level = self.env.iter().position(|x| x == name).map(Level).ok_or_else(|| ParseError::UnknownVariable { name: name.to_string(), span })?;
                Ok(self.ast.push_spanned_exact(Expr::Var(level), span)?)
            }
            TokenKind::LParen => {
                let start_span = tok.span;
                self.advance();
                if let Some(next) = self.peek() {
                    if next.kind == TokenKind::RParen {
                        let end_span = next.span;
                        self.advance();
                        let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                        return Ok(self.ast.push_spanned_exact(Expr::Unit, span)?);
                    }
                }
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RParen, "')'")?;
                Ok(expr)
            }
            _ => Err(ParseError::UnexpectedToken { expected: "expression", found: Some(tok.kind), span: Some(convert_span(tok.span)) }),
        }
    }
}
