import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

# Update parser for pi binders
match_pi_binder = """            TokenKind::LParen => {
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
                
                Ok((self.ast.push(Expr::Pi { quantity: Quantity::Omega, domain, codomain })?, names))
            }"""

repl_pi_binder = """            TokenKind::LParen | TokenKind::LBrace => {
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
c = c.replace(match_pi_binder, repl_pi_binder)

# Update parser for lambda binders
match_lam_binder = """            TokenKind::Ident(name) => {
                self.advance();
                names.push(name.to_string());
                let (body, _) = self.parse_under_binder(names)?;
                names.pop();
                Ok((self.ast.push(Expr::Lambda { quantity: Quantity::Omega, body })?, names))
            }"""

repl_lam_binder = """            TokenKind::Ident(name) => {
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
c = c.replace(match_lam_binder, repl_lam_binder)

# Update parser for Application (implicit argument parsing)
match_app_loop = """        while let Some(tok) = self.peek() {
            match tok.kind {
                TokenKind::Ident(_) | TokenKind::LParen | TokenKind::Question | TokenKind::Type | TokenKind::Zero | TokenKind::One | TokenKind::Omega | TokenKind::Star | TokenKind::True | TokenKind::False => {
                    let arg = self.parse_atom()?;
                    expr = self.ast.push(Expr::App { function: expr, argument: arg })?;
                }"""

repl_app_loop = """        while let Some(tok) = self.peek() {
            match tok.kind {
                TokenKind::Ident(_) | TokenKind::LParen | TokenKind::Question | TokenKind::Type | TokenKind::Zero | TokenKind::One | TokenKind::Omega | TokenKind::Star | TokenKind::True | TokenKind::False => {
                    let arg = self.parse_atom()?;
                    expr = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit, function: expr, argument: arg })?;
                }
                TokenKind::LBrace => {
                    self.advance();
                    let arg = self.parse_expression()?;
                    self.expect(TokenKind::RBrace, "'}'")?;
                    expr = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Implicit, function: expr, argument: arg })?;
                }"""
c = c.replace(match_app_loop, repl_app_loop)

# Update parser for arrow `A -> B`
match_arrow = """                    expr = self.ast.push(Expr::Pi { quantity: Quantity::Omega, domain: expr, codomain })?;"""
repl_arrow = """                    expr = self.ast.push(Expr::Pi { plicity: crate::ast::Plicity::Explicit, quantity: Quantity::Omega, domain: expr, codomain })?;"""
c = c.replace(match_arrow, repl_arrow)

with open('src/parser.rs', 'w') as f:
    f.write(c)
