import re

# 1. Lexer
with open('src/lexer.rs', 'r') as f:
    c = f.read()
c = c.replace('"sigma" => TokenKind::Sigma,', '"sigma" => TokenKind::Sigma,\n                "Nat" => TokenKind::NatType,\n                "Z" => TokenKind::Zero,\n                "S" => TokenKind::Succ,\n                "ind" => TokenKind::Ind,')
c = c.replace('Sigma,\n    Continue,', 'Sigma,\n    NatType,\n    Zero,\n    Succ,\n    Ind,\n    Continue,')
with open('src/lexer.rs', 'w') as f:
    f.write(c)

# 2. AST
with open('src/ast.rs', 'r') as f:
    c = f.read()

c = c.replace('    False,\n    Hole,', '    False,\n    Hole,\n    NatType,\n    Zero,\n    Succ(ExprId),\n    Ind {\n        mot: ExprId,\n        z: ExprId,\n        s: ExprId,\n        target: ExprId,\n    },')

match_walk = """            Expr::Var(_) | Expr::Universe(_) | Expr::UnitType | Expr::Unit | Expr::Bool | Expr::True | Expr::False | Expr::Hole | Expr::Meta(_) => {}"""
repl_walk = """            Expr::Succ(n) => self.expr(n)?,\n            Expr::Ind { mot, z, s, target } => {\n                self.expr(mot)?;\n                self.expr(z)?;\n                self.expr(s)?;\n                self.expr(target)?;\n            }\n            Expr::Var(_) | Expr::Universe(_) | Expr::UnitType | Expr::Unit | Expr::Bool | Expr::True | Expr::False | Expr::Hole | Expr::Meta(_) | Expr::NatType | Expr::Zero => {}"""
c = c.replace(match_walk, repl_walk)

match_fmt = """            Expr::Meta(id) => write!(f, "?{}", id.0),"""
repl_fmt = """            Expr::Meta(id) => write!(f, "?{}", id.0),\n            Expr::NatType => write!(f, "Nat"),\n            Expr::Zero => write!(f, "Z"),\n            Expr::Succ(n) => {\n                write!(f, "S(")?;\n                self.print_expr(*n, f)?;\n                write!(f, ")")\n            }\n            Expr::Ind { mot, z, s, target } => {\n                write!(f, "ind(")?;\n                self.print_expr(*mot, f)?;\n                write!(f, ", ")?;\n                self.print_expr(*z, f)?;\n                write!(f, ", ")?;\n                self.print_expr(*s, f)?;\n                write!(f, ", ")?;\n                self.print_expr(*target, f)?;\n                write!(f, ")")\n            }"""
c = c.replace(match_fmt, repl_fmt)

with open('src/ast.rs', 'w') as f:
    f.write(c)

# 3. Parser
with open('src/parser.rs', 'r') as f:
    c = f.read()

match_parse = """            TokenKind::Sigma => {"""
repl_parse = """            TokenKind::NatType => {\n                let span = tok.span;\n                self.advance();\n                Ok(self.ast.push_spanned_exact(Expr::NatType, convert_span(span))?)\n            }\n            TokenKind::Zero => {\n                let span = tok.span;\n                self.advance();\n                Ok(self.ast.push_spanned_exact(Expr::Zero, convert_span(span))?)\n            }\n            TokenKind::Succ => {\n                let start_span = tok.span;\n                self.advance();\n                self.expect(TokenKind::LParen, "'('")?;\n                let n = self.parse_expression()?;\n                self.expect(TokenKind::RParen, "')'")?;\n                let end_span = self.tokens[self.cursor - 1].span;\n                Ok(self.ast.push_spanned_exact(Expr::Succ(n), AstSpan::new(start_span.start, end_span.end).unwrap())?)\n            }\n            TokenKind::Ind => {\n                let start_span = tok.span;\n                self.advance();\n                self.expect(TokenKind::LParen, "'('")?;\n                let mot = self.parse_expression()?;\n                self.expect(TokenKind::Comma, "','")?;\n                let z = self.parse_expression()?;\n                self.expect(TokenKind::Comma, "','")?;\n                let s = self.parse_expression()?;\n                self.expect(TokenKind::Comma, "','")?;\n                let target = self.parse_expression()?;\n                self.expect(TokenKind::RParen, "')'")?;\n                let end_span = self.tokens[self.cursor - 1].span;\n                Ok(self.ast.push_spanned_exact(Expr::Ind { mot, z, s, target }, AstSpan::new(start_span.start, end_span.end).unwrap())?)\n            }\n            TokenKind::Sigma => {"""
c = c.replace(match_parse, repl_parse)
with open('src/parser.rs', 'w') as f:
    f.write(c)

