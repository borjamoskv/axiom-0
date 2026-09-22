use crate::ast::{Ast, AstError, Command, Expr, ExprId, Level, Quantity, Span as AstSpan};
use crate::lexer::{Span as LexerSpan, Token, TokenKind};
use std::fmt;

const MAX_PARSE_DEPTH: usize = 128;

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
    UniverseLevelOverflow {
        level: u64,
        span: AstSpan,
    },
    NestingLimitExceeded {
        limit: usize,
        span: Option<AstSpan>,
    },
    AstError(AstError),
}

impl From<AstError> for ParseError {
    fn from(err: AstError) -> Self {
        Self::AstError(err)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of file"),
            Self::UnexpectedToken {
                expected, found, ..
            } => write!(f, "expected {}, but found {:?}", expected, found),
            Self::UnknownVariable { name, .. } => write!(f, "unknown variable '{}'", name),
            Self::UniverseLevelOverflow { level, .. } => {
                write!(f, "universe level {} exceeds {}", level, u32::MAX)
            }
            Self::NestingLimitExceeded { limit, .. } => {
                write!(f, "expression nesting exceeds the limit of {}", limit)
            }
            Self::AstError(err) => write!(f, "ast error: {}", err),
        }
    }
}
impl std::error::Error for ParseError {}

fn convert_span(span: LexerSpan) -> AstSpan {
    AstSpan::new(span.start, span.end).unwrap()
}

pub struct Parser<'a> {
    tokens: &'a [Token<'a>],
    cursor: usize,
    ast: &'a mut Ast,
    env: Vec<String>,
    expression_depth: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token<'a>], ast: &'a mut Ast, global_env: &[String]) -> Self {
        Self {
            tokens,
            cursor: 0,
            ast,
            env: global_env.to_vec(),
            expression_depth: 0,
        }
    }

    fn peek(&self) -> Option<&'a Token<'a>> {
        self.tokens.get(self.cursor)
    }

    fn advance(&mut self) -> Option<&'a Token<'a>> {
        let tok = self.peek()?;
        self.cursor += 1;
        Some(tok)
    }

    fn expect(
        &mut self,
        kind: TokenKind,
        expected: &'static str,
    ) -> Result<&'a Token<'a>, ParseError> {
        let span = self.peek().map(|t| convert_span(t.span));
        let found = self.peek().map(|t| t.kind);
        if found == Some(kind) {
            Ok(self.advance().unwrap())
        } else {
            Err(ParseError::UnexpectedToken {
                expected,
                found,
                span,
            })
        }
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

                self.finish_command()?;
                return Ok(Command::Let { name, ty, term });
            }
        }

        let expr = self.parse_expression()?;
        self.finish_command()?;
        Ok(Command::Eval(expr))
    }

    fn finish_command(&mut self) -> Result<(), ParseError> {
        if let Some(semi) = self.peek() {
            if semi.kind == TokenKind::Semicolon {
                self.advance();
            }
        }
        if let Some(tok) = self.peek() {
            return Err(ParseError::UnexpectedToken {
                expected: "end of command",
                found: Some(tok.kind),
                span: Some(convert_span(tok.span)),
            });
        }
        Ok(())
    }

    pub fn parse_expression(&mut self) -> Result<ExprId, ParseError> {
        if self.expression_depth >= MAX_PARSE_DEPTH {
            return Err(ParseError::NestingLimitExceeded {
                limit: MAX_PARSE_DEPTH,
                span: self.peek().map(|tok| convert_span(tok.span)),
            });
        }
        self.expression_depth += 1;
        let result = self.parse_expression_inner();
        self.expression_depth -= 1;
        result
    }

    fn parse_under_binder(&mut self, name: &str) -> Result<ExprId, ParseError> {
        let outer_len = self.env.len();
        self.env.push(name.to_owned());
        let result = self.parse_expression();
        self.env.truncate(outer_len);
        result
    }

    fn parse_expression_inner(&mut self) -> Result<ExprId, ParseError> {
        let mut expr = self.parse_atom()?;
        while let Some(tok) = self.peek() {
            match tok.kind {
                TokenKind::Ident | TokenKind::LParen | TokenKind::Fn | TokenKind::Type | TokenKind::Sigma | TokenKind::If | TokenKind::Question | TokenKind::NatType | TokenKind::Zero | TokenKind::Succ | TokenKind::Ind | TokenKind::IdType | TokenKind::Refl | TokenKind::J => {
                    let argument = self.parse_atom()?;
                    expr = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit,
                        function: expr,
                        argument,
                    })?;
                }
                TokenKind::LBrace => {
                    self.advance();
                    let argument = self.parse_expression()?;
                    self.expect(TokenKind::RBrace, "'}'")?;
                    expr = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Implicit,
                        function: expr,
                        argument,
                    })?;
                }
                TokenKind::Dot => {
                    self.advance(); // consume Dot
                    if let Some(field) = self.peek() {
                        if let TokenKind::NatLiteral(1) = field.kind {
                            self.advance();
                            expr = self.ast.push(Expr::Fst(expr))?;
                        } else if let TokenKind::NatLiteral(2) = field.kind {
                            self.advance();
                            expr = self.ast.push(Expr::Snd(expr))?;
                        } else {
                            return Err(ParseError::UnexpectedToken {
                                expected: "1 or 2",
                                found: Some(field.kind),
                                span: Some(convert_span(field.span)),
                            });
                        }
                    } else {
                        return Err(ParseError::UnexpectedEof);
                    }
                }
                TokenKind::Arrow => {
                    self.advance();
                    // Even a non-dependent function introduces a level. An empty
                    // name is inaccessible to source identifiers.
                    let codomain = self.parse_under_binder("")?;
                    expr = self.ast.push(Expr::Pi { plicity: crate::ast::Plicity::Explicit,
                        quantity: Quantity::Omega,
                        domain: expr,
                        codomain,
                    })?;
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
                        level =
                            u32::try_from(n).map_err(|_| ParseError::UniverseLevelOverflow {
                                level: n,
                                span: convert_span(next_tok.span),
                            })?;
                        self.advance();
                    }
                }
                let end_span = self
                    .tokens
                    .get(self.cursor.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span);
                let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                Ok(self.ast.push_spanned_exact(Expr::Universe(level), span)?)
            }
            TokenKind::Question => {
                let span = tok.span;
                self.advance();
                Ok(self.ast.push_spanned_exact(Expr::Hole, convert_span(span))?)
            }
            TokenKind::Fn => {
                let start_span = tok.span;
                self.advance();
                let mut quantity = Quantity::Omega;
                if let Some(qtok) = self.peek() {
                    match qtok.kind {
                        TokenKind::QuantZero => {
                            quantity = Quantity::Zero;
                            self.advance();
                        }
                        TokenKind::QuantOne => {
                            quantity = Quantity::One;
                            self.advance();
                        }
                        TokenKind::QuantOmega => {
                            quantity = Quantity::Omega;
                            self.advance();
                        }
                        _ => {}
                    }
                }

                let (is_pi, is_implicit) = if let Some(tok) = self.peek() {
                    let mut next_cursor = self.cursor + 1;
                    let is_brace = tok.kind == TokenKind::LBrace;
                    let mut is_colon = false;
                    while next_cursor < self.tokens.len() {
                        let k = self.tokens[next_cursor].kind;
                        if k == TokenKind::Colon { is_colon = true; break; }
                        if k == TokenKind::RBrace || k == TokenKind::RParen || k == TokenKind::Arrow { break; }
                        next_cursor += 1;
                    }
                    (is_colon, is_brace)
                } else {
                    (false, false)
                };

                let plicity = if is_implicit { crate::ast::Plicity::Implicit } else { crate::ast::Plicity::Explicit };

                if is_pi {
                    let end_tok = if is_implicit { TokenKind::RBrace } else { TokenKind::RParen };
                    self.advance(); // consume LParen/LBrace
                    let param_tok = self.expect(TokenKind::Ident, "identifier")?;
                    let param_name = param_tok.text;
                    self.expect(TokenKind::Colon, "':'")?;
                    let domain = self.parse_expression()?;
                    self.expect(end_tok, "closing bracket")?;

                    self.expect(TokenKind::Arrow, "'->'")?;
                    let codomain = self.parse_under_binder(param_name)?;

                    let end_span = self
                        .tokens
                        .get(self.cursor.saturating_sub(1))
                        .map(|t| t.span)
                        .unwrap_or(start_span);
                    let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                    Ok(self.ast.push_spanned_exact(
                        Expr::Pi { plicity,
                            quantity,
                            domain,
                            codomain,
                        },
                        span,
                    )?)
                } else {
                    if is_implicit { self.advance(); } // consume LBrace
                    let param_tok = self.expect(TokenKind::Ident, "identifier")?;
                    let param_name = param_tok.text;
                    if is_implicit { self.expect(TokenKind::RBrace, "'}'")?; }

                    self.expect(TokenKind::Arrow, "'->'")?;
                    let body = self.parse_under_binder(param_name)?;
                    let end_span = self
                        .tokens
                        .get(self.cursor.saturating_sub(1))
                        .map(|t| t.span)
                        .unwrap_or(start_span);
                    let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                    Ok(self
                        .ast
                        .push_spanned_exact(Expr::Lambda { plicity, quantity, body }, span)?)
                }
            }
            TokenKind::NatType => {
                let span = tok.span;
                self.advance();
                Ok(self.ast.push_spanned_exact(Expr::NatType, convert_span(span))?)
            }
            TokenKind::Zero => {
                let span = tok.span;
                self.advance();
                Ok(self.ast.push_spanned_exact(Expr::Zero, convert_span(span))?)
            }
            TokenKind::Succ => {
                let start_span = tok.span;
                self.advance();
                self.expect(TokenKind::LParen, "'('")?;
                let n = self.parse_expression()?;
                self.expect(TokenKind::RParen, "')'")?;
                let end_span = self.tokens[self.cursor - 1].span;
                Ok(self.ast.push_spanned_exact(Expr::Succ(n), AstSpan::new(start_span.start, end_span.end).unwrap())?)
            }
            TokenKind::Ind => {
                let start_span = tok.span;
                self.advance();
                self.expect(TokenKind::LParen, "'('")?;
                let mot = self.parse_expression()?;
                self.expect(TokenKind::Comma, "','")?;
                let z = self.parse_expression()?;
                self.expect(TokenKind::Comma, "','")?;
                let s = self.parse_expression()?;
                self.expect(TokenKind::Comma, "','")?;
                let target = self.parse_expression()?;
                self.expect(TokenKind::RParen, "')'")?;
                let end_span = self.tokens[self.cursor - 1].span;
                Ok(self.ast.push_spanned_exact(Expr::Ind { mot, z, s, target }, AstSpan::new(start_span.start, end_span.end).unwrap())?)
            }
            TokenKind::IdType => {
                let start_span = tok.span;
                self.advance();
                self.expect(TokenKind::LParen, "'('")?;
                let ty = self.parse_expression()?;
                self.expect(TokenKind::Comma, "','")?;
                let lhs = self.parse_expression()?;
                self.expect(TokenKind::Comma, "','")?;
                let rhs = self.parse_expression()?;
                self.expect(TokenKind::RParen, "')'")?;
                let end_span = self.tokens[self.cursor - 1].span;
                Ok(self.ast.push_spanned_exact(Expr::IdType { ty, lhs, rhs }, AstSpan::new(start_span.start, end_span.end).unwrap())?)
            }
            TokenKind::Refl => {
                let start_span = tok.span;
                self.advance();
                self.expect(TokenKind::LParen, "'('")?;
                let x = self.parse_expression()?;
                self.expect(TokenKind::RParen, "')'")?;
                let end_span = self.tokens[self.cursor - 1].span;
                Ok(self.ast.push_spanned_exact(Expr::Refl(x), AstSpan::new(start_span.start, end_span.end).unwrap())?)
            }
            TokenKind::J => {
                let start_span = tok.span;
                self.advance();
                self.expect(TokenKind::LParen, "'('")?;
                let mot = self.parse_expression()?;
                self.expect(TokenKind::Comma, "','")?;
                let base = self.parse_expression()?;
                self.expect(TokenKind::Comma, "','")?;
                let target = self.parse_expression()?;
                self.expect(TokenKind::RParen, "')'")?;
                let end_span = self.tokens[self.cursor - 1].span;
                Ok(self.ast.push_spanned_exact(Expr::J { mot, base, target }, AstSpan::new(start_span.start, end_span.end).unwrap())?)
            }
            TokenKind::Sigma => {
                let start_span = tok.span;
                self.advance();
                let mut quantity = Quantity::Omega;
                if let Some(qtok) = self.peek() {
                    match qtok.kind {
                        TokenKind::QuantZero => {
                            quantity = Quantity::Zero;
                            self.advance();
                        }
                        TokenKind::QuantOne => {
                            quantity = Quantity::One;
                            self.advance();
                        }
                        TokenKind::QuantOmega => {
                            quantity = Quantity::Omega;
                            self.advance();
                        }
                        _ => {}
                    }
                }
                self.expect(TokenKind::LParen, "'('")?;
                let param_tok = self.expect(TokenKind::Ident, "identifier")?;
                let param_name = param_tok.text;
                self.expect(TokenKind::Colon, "':'")?;
                let domain = self.parse_expression()?;
                self.expect(TokenKind::RParen, "')'")?;

                self.expect(TokenKind::Asterisk, "'*'")?;
                let codomain = self.parse_under_binder(param_name)?;

                let end_span = self
                    .tokens
                    .get(self.cursor.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span);
                let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                Ok(self.ast.push_spanned_exact(
                    Expr::Sigma {
                        quantity,
                        domain,
                        codomain,
                    },
                    span,
                )?)
            }
            TokenKind::If => {
                let start_span = tok.span;
                self.advance();
                let cond = self.parse_expression()?;
                self.expect(TokenKind::Then, "'then'")?;
                let conseq = self.parse_expression()?;
                self.expect(TokenKind::Else, "'else'")?;
                let alt = self.parse_expression()?;

                let end_span = self
                    .tokens
                    .get(self.cursor.saturating_sub(1))
                    .map(|t| t.span)
                    .unwrap_or(start_span);
                let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                Ok(self.ast.push_spanned_exact(Expr::If { cond, conseq, alt }, span)?)
            }
            TokenKind::Ident => {
                let name = tok.text;
                let span = convert_span(tok.span);
                self.advance();
                let expr = if let Some(level) = self.env.iter().rposition(|x| x == name) {
                    Expr::Var(Level(level))
                } else if name == "UnitType" {
                    Expr::UnitType
                } else if name == "Bool" {
                    Expr::Bool
                } else if name == "True" {
                    Expr::True
                } else if name == "False" {
                    Expr::False
                } else {
                    return Err(ParseError::UnknownVariable {
                        name: name.to_string(),
                        span,
                    });
                };
                Ok(self.ast.push_spanned_exact(expr, span)?)
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
                let mut expr = self.parse_expression()?;
                if let Some(comma) = self.peek() {
                    if comma.kind == TokenKind::Comma {
                        self.advance();
                        let second = self.parse_expression()?;
                        expr = self.ast.push(Expr::Pair { first: expr, second })?;
                    }
                }
                self.expect(TokenKind::RParen, "')'")?;
                Ok(expr)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression",
                found: Some(tok.kind),
                span: Some(convert_span(tok.span)),
            }),
        }
    }
}
