//! Lexer for the concrete syntax of AXIOM-0.
//!
//! Scans UTF-8 source into atomic tokens with byte spans. Token text borrows source
//! slices; collecting tokens and reporting errors may allocate.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    Module,
    Import,
    Struct,
    Type,
    Fn,
    Let,
    Loop,
    If,
    Else,
    Sigma,
    Continue,
    Return,
    Session,
    Select,
    Proof,
    AutoSmt,
    Theorem,

    // Identifiers and Literals
    Ident,
    NatLiteral(u64),

    // Quantitative Annotations
    QuantZero,  // :^0
    QuantOne,   // :^1
    QuantOmega, // :^w, :^ω or :^omega

    // Punctuation
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Arrow,     // ->
    Colon,     // :
    ColonEq,   // :=
    Semicolon, // ;
    Comma,     // ,
    Dot,       // .
    At,        // @
    Eq,        // =
    EqEq,      // ==
    NotEq,     // !=
    Percent,   // %
    Plus,      // +
    Asterisk,  // *
    Bang,      // !
    Question,  // ?
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub text: &'a str,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexerError {
    pub span: Span,
    pub message: String,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "lexing error at [{}, {}): {}",
            self.span.start, self.span.end, self.message
        )
    }
}

impl std::error::Error for LexerError {}

pub struct Lexer<'a> {
    source: &'a str,
    chars: std::str::CharIndices<'a>,
    peeked: Option<(usize, char)>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut chars = source.char_indices();
        let peeked = chars.next();
        Self {
            source,
            chars,
            peeked,
        }
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let current = self.peeked;
        self.peeked = self.chars.next();
        current
    }

    fn peek(&self) -> Option<(usize, char)> {
        self.peeked
    }

    fn current_pos(&self) -> usize {
        self.peeked.map(|(pos, _)| pos).unwrap_or(self.source.len())
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some((_, ch)) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '-' {
                let current_pos = self.current_pos();
                let rest = &self.source[current_pos..];
                if rest.starts_with("--") {
                    // Line comment: skip until newline
                    while let Some((_, c)) = self.peek() {
                        self.advance();
                        if c == '\n' {
                            break;
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Result<Option<Token<'a>>, LexerError> {
        self.skip_whitespace_and_comments();
        let Some((start, ch)) = self.advance() else {
            return Ok(None);
        };

        // A quantity must occupy its entire identifier-like suffix. In particular,
        // :^01 and :^wat are errors, rather than a quantity followed by a token.
        if ch == ':' {
            if let Some((_, '^')) = self.peek() {
                self.advance(); // consume '^'
                let quantity_start = self.current_pos();
                while let Some((_, c)) = self.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let end = self.current_pos();
                let kind = match &self.source[quantity_start..end] {
                    "0" => TokenKind::QuantZero,
                    "1" => TokenKind::QuantOne,
                    "w" | "ω" | "omega" => TokenKind::QuantOmega,
                    _ => {
                        return Err(LexerError {
                            span: Span { start, end },
                            message: "invalid quantity annotation: expected :^0, :^1, :^w, :^ω or :^omega"
                                .to_owned(),
                        });
                    }
                };
                return Ok(Some(Token {
                    kind,
                    text: &self.source[start..end],
                    span: Span { start, end },
                }));
            }
            if let Some((_, '=')) = self.peek() {
                self.advance();
                let end = self.current_pos();
                return Ok(Some(Token {
                    kind: TokenKind::ColonEq,
                    text: &self.source[start..end],
                    span: Span { start, end },
                }));
            }
            return Ok(Some(Token {
                kind: TokenKind::Colon,
                text: &self.source[start..start + 1],
                span: Span {
                    start,
                    end: start + 1,
                },
            }));
        }

        // Two-character punctuation
        if ch == '-' && self.peek().map(|(_, c)| c) == Some('>') {
            self.advance();
            let end = self.current_pos();
            return Ok(Some(Token {
                kind: TokenKind::Arrow,
                text: &self.source[start..end],
                span: Span { start, end },
            }));
        }
        if ch == '=' {
            if self.peek().map(|(_, c)| c) == Some('=') {
                self.advance();
                let end = self.current_pos();
                return Ok(Some(Token {
                    kind: TokenKind::EqEq,
                    text: &self.source[start..end],
                    span: Span { start, end },
                }));
            }
            return Ok(Some(Token {
                kind: TokenKind::Eq,
                text: &self.source[start..start + 1],
                span: Span {
                    start,
                    end: start + 1,
                },
            }));
        }
        if ch == '!' && self.peek().map(|(_, c)| c) == Some('=') {
            self.advance();
            let end = self.current_pos();
            return Ok(Some(Token {
                kind: TokenKind::NotEq,
                text: &self.source[start..end],
                span: Span { start, end },
            }));
        }

        // Single-character punctuation
        let single_kind = match ch {
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            '{' => Some(TokenKind::LBrace),
            '}' => Some(TokenKind::RBrace),
            '[' => Some(TokenKind::LBracket),
            ']' => Some(TokenKind::RBracket),
            ';' => Some(TokenKind::Semicolon),
            ',' => Some(TokenKind::Comma),
            '.' => Some(TokenKind::Dot),
            '@' => Some(TokenKind::At),
            '%' => Some(TokenKind::Percent),
            '+' => Some(TokenKind::Plus),
            '*' => Some(TokenKind::Asterisk),
            '!' => Some(TokenKind::Bang),
            '?' => Some(TokenKind::Question),
            _ => None,
        };

        if let Some(kind) = single_kind {
            return Ok(Some(Token {
                kind,
                text: &self.source[start..start + ch.len_utf8()],
                span: Span {
                    start,
                    end: start + ch.len_utf8(),
                },
            }));
        }

        // Natural literals permit a single underscore between digits.
        if ch.is_ascii_digit() {
            while let Some((_, c)) = self.peek() {
                if c.is_ascii_digit() || c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            let end = self.current_pos();
            let raw_text = &self.source[start..end];
            if raw_text.ends_with('_') || raw_text.as_bytes().windows(2).any(|pair| pair == b"__") {
                return Err(LexerError {
                    span: Span { start, end },
                    message:
                        "invalid integer literal: underscores must occur singly between digits"
                            .to_owned(),
                });
            }
            let val = raw_text
                .bytes()
                .filter(|&byte| byte != b'_')
                .try_fold(0u64, |value, digit| {
                    value.checked_mul(10)?.checked_add(u64::from(digit - b'0'))
                })
                .ok_or_else(|| LexerError {
                    span: Span { start, end },
                    message: "invalid integer literal: value exceeds u64::MAX".to_owned(),
                })?;
            return Ok(Some(Token {
                kind: TokenKind::NatLiteral(val),
                text: raw_text,
                span: Span { start, end },
            }));
        }

        // Identifiers and Keywords
        if ch.is_alphabetic() || ch == '_' {
            while let Some((_, c)) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            let end = self.current_pos();
            let text = &self.source[start..end];
            let kind = match text {
                "module" => TokenKind::Module,
                "import" => TokenKind::Import,
                "struct" => TokenKind::Struct,
                "type" => TokenKind::Type,
                "fn" => TokenKind::Fn,
                "let" => TokenKind::Let,
                "loop" => TokenKind::Loop,
                "if" => TokenKind::If,
                "else" => TokenKind::Else,
                "sigma" => TokenKind::Sigma,
                "continue" => TokenKind::Continue,
                "return" => TokenKind::Return,
                "session" => TokenKind::Session,
                "select" => TokenKind::Select,
                "proof" => TokenKind::Proof,
                "auto_smt" => TokenKind::AutoSmt,
                "theorem" => TokenKind::Theorem,
                _ => TokenKind::Ident,
            };
            return Ok(Some(Token {
                kind,
                text,
                span: Span { start, end },
            }));
        }

        Err(LexerError {
            span: Span {
                start,
                end: start + ch.len_utf8(),
            },
            message: format!("unexpected character: '{}'", ch),
        })
    }

    pub fn tokenize_all(&mut self) -> Result<Vec<Token<'a>>, LexerError> {
        let mut tokens = Vec::new();
        while let Some(tok) = self.next_token()? {
            tokens.push(tok);
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexes_keywords_and_symbols() {
        let src = "module Babylon.Core fn write_manifest (m :^1 Manifest) -> :^1 Manifest;";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().expect("lexing succeeded");
        assert_eq!(tokens[0].kind, TokenKind::Module);
        assert_eq!(tokens[1].text, "Babylon");
        assert_eq!(tokens[2].kind, TokenKind::Dot);
        assert_eq!(tokens[3].text, "Core");
        assert_eq!(tokens[4].kind, TokenKind::Fn);
        assert_eq!(tokens[5].text, "write_manifest");
        assert_eq!(tokens[6].kind, TokenKind::LParen);
        assert_eq!(tokens[7].text, "m");
        assert_eq!(tokens[8].kind, TokenKind::QuantOne);
        assert_eq!(tokens[9].text, "Manifest");
        assert_eq!(tokens[10].kind, TokenKind::RParen);
        assert_eq!(tokens[11].kind, TokenKind::Arrow);
        assert_eq!(tokens[12].kind, TokenKind::QuantOne);
        assert_eq!(tokens[13].text, "Manifest");
        assert_eq!(tokens[14].kind, TokenKind::Semicolon);
    }

    #[test]
    fn test_lexes_comments_and_quantities() {
        let src = "-- Line comment
fn f(x :^0 A, y :^w B) -> C";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().expect("lexing succeeded");
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[4].kind, TokenKind::QuantZero);
        assert_eq!(tokens[8].kind, TokenKind::QuantOmega);
    }
}
