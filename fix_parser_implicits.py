import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

# Fix LParen
match_lparen = """            TokenKind::LParen => {
                self.advance();
                let name_span = self.peek().unwrap().span;
                let name = match self.peek().unwrap().kind {
                    TokenKind::Ident(n) => n,
                    _ => return Err(ParseError::UnexpectedToken {
                        expected: "identifier",
                        found: Some(self.peek().unwrap().kind),
                        span: Some(convert_span(name_span)),
                    }),
                };
                self.advance();
                self.expect(TokenKind::Colon, "':'")?;
                let domain = self.parse_expression()?;
                self.expect(TokenKind::RParen, "')'")?;
                
                names.push(name.to_string());
                let (codomain, _) = self.parse_under_binder(names)?;
                names.pop();
                
                Ok((self.ast.push(Expr::Pi { plicity: crate::ast::Plicity::Explicit, quantity: Quantity::Omega, domain, codomain })?, names))
            }"""

repl_lparen = """            TokenKind::LParen | TokenKind::LBrace => {
                let plicity = if tok.kind == TokenKind::LBrace { crate::ast::Plicity::Implicit } else { crate::ast::Plicity::Explicit };
                let end_tok = if tok.kind == TokenKind::LBrace { TokenKind::RBrace } else { TokenKind::RParen };
                self.advance();
                let name_span = self.peek().unwrap().span;
                let name = match self.peek().unwrap().kind {
                    TokenKind::Ident(n) => n,
                    _ => return Err(ParseError::UnexpectedToken {
                        expected: "identifier",
                        found: Some(self.peek().unwrap().kind),
                        span: Some(convert_span(name_span)),
                    }),
                };
                self.advance();
                self.expect(TokenKind::Colon, "':'")?;
                let domain = self.parse_expression()?;
                self.expect(end_tok, "closing brace/paren")?;
                
                names.push(name.to_string());
                let (codomain, _) = self.parse_under_binder(names)?;
                names.pop();
                
                Ok((self.ast.push(Expr::Pi { plicity, quantity: Quantity::Omega, domain, codomain })?, names))
            }"""

c = c.replace(match_lparen, repl_lparen)

# Fix Ident
match_ident = """            TokenKind::Ident(name) => {
                self.advance();
                names.push(name.to_string());
                let (body, _) = self.parse_under_binder(names)?;
                names.pop();
                Ok((self.ast.push(Expr::Lambda { plicity: crate::ast::Plicity::Explicit, quantity: Quantity::Omega, body })?, names))
            }"""

repl_ident = """            TokenKind::Ident(name) => {
                self.advance();
                names.push(name.to_string());
                let (body, _) = self.parse_under_binder(names)?;
                names.pop();
                Ok((self.ast.push(Expr::Lambda { plicity: crate::ast::Plicity::Explicit, quantity: Quantity::Omega, body })?, names))
            }
            TokenKind::LBrace => {
                self.advance();
                let name = match self.peek().unwrap().kind {
                    TokenKind::Ident(n) => n,
                    _ => return Err(ParseError::UnexpectedToken {
                        expected: "identifier",
                        found: Some(self.peek().unwrap().kind),
                        span: None,
                    }),
                };
                self.advance();
                self.expect(TokenKind::RBrace, "'}'")?;
                names.push(name.to_string());
                let (body, _) = self.parse_under_binder(names)?;
                names.pop();
                Ok((self.ast.push(Expr::Lambda { plicity: crate::ast::Plicity::Implicit, quantity: Quantity::Omega, body })?, names))
            }"""
c = c.replace(match_ident, repl_ident)

# Fix App
match_app = """                TokenKind::Ident(_) | TokenKind::LParen | TokenKind::Question | TokenKind::Type | TokenKind::Zero | TokenKind::One | TokenKind::Omega | TokenKind::Star | TokenKind::True | TokenKind::False => {
                    let argument = self.parse_atom()?;
                    expr = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit,
                        function: expr,
                        argument,
                    })?;
                }"""

repl_app = """                TokenKind::Ident(_) | TokenKind::LParen | TokenKind::Question | TokenKind::Type | TokenKind::Zero | TokenKind::One | TokenKind::Omega | TokenKind::Star | TokenKind::True | TokenKind::False => {
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
                    expr = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Implicit, function: expr, argument })?;
                }"""
c = c.replace(match_app, repl_app)

with open('src/parser.rs', 'w') as f:
    f.write(c)
