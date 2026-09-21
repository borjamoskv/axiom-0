import re

with open('src/parser.rs', 'r') as f:
    c = f.read()

match_app = """        let mut expr = self.parse_atom()?;
        while let Some(tok) = self.peek() {
            match tok.kind {
                TokenKind::Ident | TokenKind::LParen | TokenKind::Fn | TokenKind::Type | TokenKind::Sigma | TokenKind::If | TokenKind::Question => {
                    let argument = self.parse_atom()?;
                    expr = self.ast.push(Expr::App {
                                plicity: crate::ast::Plicity::Explicit,
                                function: expr,
                                argument,
                            })?;
                }
                TokenKind::Dot => {"""

repl_app = """        let mut expr = self.parse_atom()?;
        while let Some(tok) = self.peek() {
            match tok.kind {
                TokenKind::Ident | TokenKind::LParen | TokenKind::Fn | TokenKind::Type | TokenKind::Sigma | TokenKind::If | TokenKind::Question => {
                    let argument = self.parse_atom()?;
                    expr = self.ast.push(Expr::App {
                                plicity: crate::ast::Plicity::Explicit,
                                function: expr,
                                argument,
                            })?;
                }
                TokenKind::LBrace => {
                    self.advance();
                    let argument = self.parse_expression()?;
                    self.expect(TokenKind::RBrace, "'}'")?;
                    expr = self.ast.push(Expr::App {
                                plicity: crate::ast::Plicity::Implicit,
                                function: expr,
                                argument,
                            })?;
                }
                TokenKind::Dot => {"""

c = c.replace(match_app, repl_app)

with open('src/parser.rs', 'w') as f:
    f.write(c)
