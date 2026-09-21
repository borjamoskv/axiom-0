import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

match_fn = """                let is_pi = if let Some(tok) = self.peek() {
                    tok.kind == TokenKind::LParen
                } else {
                    false
                };

                if is_pi {
                    self.advance(); // consume LParen
                    let param_tok = self.expect(TokenKind::Ident, "identifier")?;
                    let param_name = param_tok.text;
                    self.expect(TokenKind::Colon, "':'")?;
                    let domain = self.parse_expression()?;
                    self.expect(TokenKind::RParen, "')'")?;

                    self.expect(TokenKind::Arrow, "'->'")?;
                    let codomain = self.parse_under_binder(param_name)?;

                    let end_span = self
                        .tokens
                        .get(self.cursor.saturating_sub(1))
                        .map(|t| t.span)
                        .unwrap_or(start_span);
                    let span = AstSpan::new(start_span.start, end_span.end).unwrap();
                    Ok(self.ast.push_spanned_exact(
                        Expr::Pi { plicity: crate::ast::Plicity::Explicit,
                            quantity,
                            domain,
                            codomain,
                        },
                        span,
                    )?)
                } else {
                    let param_tok = self.expect(TokenKind::Ident, "identifier")?;
                    let param_name = param_tok.text;

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
                        .push_spanned_exact(Expr::Lambda { plicity: crate::ast::Plicity::Explicit, quantity, body }, span)?)
                }"""

repl_fn = """                let (is_pi, is_implicit) = if let Some(tok) = self.peek() {
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
                }"""

c = c.replace(match_fn, repl_fn)

# Also fix the application parsing for LBrace
match_app_loop = """        while let Some(tok) = self.peek() {
            match tok.kind {
                TokenKind::Ident | TokenKind::LParen | TokenKind::Fn | TokenKind::Type | TokenKind::Sigma | TokenKind::If | TokenKind::Question => {"""

repl_app_loop = """        while let Some(tok) = self.peek() {
            match tok.kind {
                TokenKind::Ident | TokenKind::LParen | TokenKind::Fn | TokenKind::Type | TokenKind::Sigma | TokenKind::If | TokenKind::Question => {
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
                }"""
c = c.replace(match_app_loop, repl_app_loop)

# The inner matching in while loop needs to drop the explicit assignment to avoid duplication
match_app_loop_inner = """                TokenKind::Ident | TokenKind::LParen | TokenKind::Fn | TokenKind::Type | TokenKind::Sigma | TokenKind::If | TokenKind::Question => {
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
                    let argument = self.parse_atom()?;
                    expr = self.ast.push(Expr::App { plicity: crate::ast::Plicity::Explicit,
                        function: expr,
                        argument,
                    })?;
                }"""

repl_app_loop_inner = """                TokenKind::Ident | TokenKind::LParen | TokenKind::Fn | TokenKind::Type | TokenKind::Sigma | TokenKind::If | TokenKind::Question => {
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
                }"""
c = c.replace(match_app_loop_inner, repl_app_loop_inner)

with open('src/parser.rs', 'w') as f:
    f.write(c)

