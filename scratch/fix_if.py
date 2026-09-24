import re

with open('src/lexer.rs', 'r') as f:
    c = f.read()

c = c.replace('If,\n    Else,', 'If,\n    Then,\n    Else,')
c = c.replace('"if" => TokenKind::If,\n                "else" => TokenKind::Else,', '"if" => TokenKind::If,\n                "then" => TokenKind::Then,\n                "else" => TokenKind::Else,')

with open('src/lexer.rs', 'w') as f:
    f.write(c)

with open('src/parser.rs', 'r') as f:
    c = f.read()

match_if = """            TokenKind::If => {
                let start_span = tok.span;
                self.advance();
                let cond = self.parse_expression()?;
                self.expect(TokenKind::LBrace, "'{'")?;
                let conseq = self.parse_expression()?;
                self.expect(TokenKind::RBrace, "'}'")?;
                self.expect(TokenKind::Else, "'else'")?;
                self.expect(TokenKind::LBrace, "'{'")?;
                let alt = self.parse_expression()?;
                self.expect(TokenKind::RBrace, "'}'")?;"""

repl_if = """            TokenKind::If => {
                let start_span = tok.span;
                self.advance();
                let cond = self.parse_expression()?;
                self.expect(TokenKind::Then, "'then'")?;
                let conseq = self.parse_expression()?;
                self.expect(TokenKind::Else, "'else'")?;
                let alt = self.parse_expression()?;"""

c = c.replace(match_if, repl_if)

with open('src/parser.rs', 'w') as f:
    f.write(c)

with open('tests/sigma_booleans.rs', 'r') as f:
    c = f.read()

c = c.replace('if True { False } else { True }', 'if True then False else True')
with open('tests/sigma_booleans.rs', 'w') as f:
    f.write(c)

with open('examples/poc_sigma_bool.rs', 'r') as f:
    c = f.read()

c = c.replace('if b { False } else { True }', 'if b then False else True')
c = c.replace('if b { UnitType } else { Bool }', 'if b then UnitType else Bool')
with open('examples/poc_sigma_bool.rs', 'w') as f:
    f.write(c)

