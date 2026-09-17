use micro_axiom_0::lexer::{Lexer, Span, TokenKind};

#[test]
fn quantities_preserve_their_spelling_and_leave_delimiters_for_the_next_token() {
    let quantities = [
        (":^0", TokenKind::QuantZero),
        (":^1", TokenKind::QuantOne),
        (":^w", TokenKind::QuantOmega),
        (":^ω", TokenKind::QuantOmega),
        (":^omega", TokenKind::QuantOmega),
    ];
    for (spelling, kind) in quantities {
        for (delimiter, next_kind) in [
            (";", TokenKind::Semicolon),
            (")", TokenKind::RParen),
            ("=", TokenKind::Eq),
            ("->", TokenKind::Arrow),
        ] {
            let source = format!("{spelling}{delimiter}");
            let tokens = Lexer::new(&source).tokenize_all().unwrap();
            assert_eq!(tokens.len(), 2, "{source:?}");
            assert_eq!(tokens[0].kind, kind, "{source:?}");
            assert_eq!(tokens[0].text, spelling);
            assert_eq!(
                tokens[0].span,
                Span {
                    start: 0,
                    end: spelling.len()
                }
            );
            assert_eq!(tokens[1].kind, next_kind);
            assert_eq!(tokens[1].text, delimiter);
            assert_eq!(tokens[1].span.start, spelling.len());
        }
        let token = Lexer::new(spelling).next_token().unwrap().unwrap();
        assert_eq!(token.kind, kind);
        assert_eq!(token.text, spelling);
    }
}

#[test]
fn malformed_quantities_fail_before_returning_a_valid_prefix() {
    for malformed in [":^", ":^2", ":^01", ":^wat", ":^="] {
        let error = Lexer::new(malformed)
            .next_token()
            .expect_err("a malformed annotation must not yield a partial token");
        let consumed = malformed.strip_suffix('=').unwrap_or(malformed);
        assert_eq!(
            error.span,
            Span {
                start: 0,
                end: consumed.len()
            },
            "{malformed:?}"
        );
        assert_eq!(
            malformed.get(error.span.start..error.span.end),
            Some(consumed)
        );
        assert!(error.message.contains("quantity"), "{error}");
    }
}

#[test]
fn quantity_suffixes_are_rejected_as_one_annotation_including_unicode() {
    for annotation in [":^0x", ":^1_", ":^w2", ":^omega_", ":^ωx", ":^1é", ":^ω９"] {
        let source = format!("λ {annotation};");
        let mut lexer = Lexer::new(&source);
        assert_eq!(lexer.next_token().unwrap().unwrap().text, "λ");
        let error = lexer
            .next_token()
            .expect_err("suffix must invalidate the quantity");
        let start = "λ ".len();
        assert_eq!(
            error.span,
            Span {
                start,
                end: start + annotation.len()
            },
            "{source:?}"
        );
        assert_eq!(
            source.get(error.span.start..error.span.end),
            Some(annotation)
        );
        assert_eq!(
            lexer.next_token().unwrap().unwrap().kind,
            TokenKind::Semicolon
        );
        assert!(lexer.next_token().unwrap().is_none());
    }
}

#[test]
fn token_spans_are_borrowed_utf8_byte_slices_after_comments_and_whitespace() {
    let source = "-- café ω\r\n\u{2003}变量_2 :^ω (é, 1_024) -- fin 🦀";
    let expected = [
        ("变量_2", TokenKind::Ident),
        (":^ω", TokenKind::QuantOmega),
        ("(", TokenKind::LParen),
        ("é", TokenKind::Ident),
        (",", TokenKind::Comma),
        ("1_024", TokenKind::NatLiteral(1024)),
        (")", TokenKind::RParen),
    ];
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_all().unwrap();
    assert_eq!(tokens.len(), expected.len());
    let mut previous_end = 0;
    for (token, (text, kind)) in tokens.iter().zip(expected) {
        assert_eq!(token.kind, kind);
        assert_eq!(token.text, text);
        assert!(token.span.start >= previous_end);
        let slice = source
            .get(token.span.start..token.span.end)
            .expect("UTF-8 boundaries");
        assert_eq!(slice, text);
        assert_eq!(
            slice.as_ptr(),
            token.text.as_ptr(),
            "tokens must borrow the source"
        );
        assert_eq!(token.span.end - token.span.start, text.len());
        previous_end = token.span.end;
    }
    assert!(lexer.next_token().unwrap().is_none());
    assert!(lexer.next_token().unwrap().is_none());
}

#[test]
fn unexpected_multibyte_character_has_a_complete_error_span_and_advances() {
    let source = "é 🦀 :^1";
    let mut lexer = Lexer::new(source);
    assert_eq!(lexer.next_token().unwrap().unwrap().text, "é");
    let error = lexer.next_token().unwrap_err();
    assert_eq!(error.span, Span { start: 3, end: 7 });
    assert_eq!(source.get(error.span.start..error.span.end), Some("🦀"));
    assert_eq!(
        lexer.next_token().unwrap().unwrap().kind,
        TokenKind::QuantOne
    );
    assert!(lexer.next_token().unwrap().is_none());
}

#[test]
fn naturals_accept_single_separators_between_digits_and_the_u64_boundary() {
    for (source, expected) in [
        ("0", 0),
        ("0_0", 0),
        ("1_000_009", 1_000_009),
        ("18446744073709551615", u64::MAX),
        ("18_446_744_073_709_551_615", u64::MAX),
    ] {
        let tokens = Lexer::new(source).tokenize_all().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0].kind,
            TokenKind::NatLiteral(expected),
            "{source:?}"
        );
        assert_eq!(tokens[0].text, source);
        assert_eq!(
            tokens[0].span,
            Span {
                start: 0,
                end: source.len()
            }
        );
    }
    for source in ["_1", "_1_0"] {
        let tokens = Lexer::new(source).tokenize_all().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Ident);
        assert_eq!(tokens[0].text, source);
    }
}

#[test]
fn malformed_natural_separators_report_the_whole_literal() {
    for literal in ["1_", "1__0", "1___0", "1_0_", "0__"] {
        let source = format!("{literal};");
        let mut lexer = Lexer::new(&source);
        let error = lexer
            .next_token()
            .expect_err("malformed separator must be rejected");
        assert_eq!(
            error.span,
            Span {
                start: 0,
                end: literal.len()
            },
            "{source:?}"
        );
        assert_eq!(source.get(error.span.start..error.span.end), Some(literal));
        assert!(error.message.contains("underscore"), "{error}");
        assert_eq!(
            lexer.next_token().unwrap().unwrap().kind,
            TokenKind::Semicolon
        );
    }
}

#[test]
fn overflowing_naturals_return_an_error_without_wrapping_or_losing_the_span() {
    for literal in [
        "18446744073709551616",
        "18_446_744_073_709_551_616",
        "999999999999999999999999999999999999999999999999999999999999999999",
    ] {
        let source = format!("ω {literal},");
        let mut lexer = Lexer::new(&source);
        assert_eq!(lexer.next_token().unwrap().unwrap().text, "ω");
        let error = lexer
            .next_token()
            .expect_err("values above u64::MAX must fail");
        let start = "ω ".len();
        assert_eq!(
            error.span,
            Span {
                start,
                end: start + literal.len()
            }
        );
        assert_eq!(source.get(error.span.start..error.span.end), Some(literal));
        assert!(error.message.contains("u64::MAX"), "{error}");
        assert_eq!(lexer.next_token().unwrap().unwrap().kind, TokenKind::Comma);
        assert!(lexer.next_token().unwrap().is_none());
    }
}
