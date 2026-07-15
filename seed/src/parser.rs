//! Hand-written recursive descent, like every language that cares about
//! error messages and speed. Crude in the seed: parse failures just panic.

use crate::ast::Expression;
use crate::lexer::{self, Token, TokenKind};

pub fn parse(source: &str) -> Expression {
    let tokens = lexer::lex(source);
    let mut parser = Parser {
        position: 0,
        tokens,
    };
    let expression = parser.expression();
    parser.expect_end();
    expression
}

struct Parser<'source> {
    position: usize,
    tokens: Vec<Token<'source>>,
}

impl Parser<'_> {
    fn expression(&mut self) -> Expression {
        self.addition()
    }

    fn addition(&mut self) -> Expression {
        let mut left = self.primary();
        while self.peek_kind() == Some(TokenKind::Plus) {
            self.position += 1;
            let right = self.primary();
            left = Expression::Add {
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        left
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.tokens.get(self.position).map(|token| token.kind)
    }

    fn primary(&mut self) -> Expression {
        let token = self.advance();
        match token.kind {
            TokenKind::Integer => {
                let value = token.text.parse().expect("integer literal out of range");
                Expression::Integer(value)
            }
            TokenKind::String => {
                let content = &token.text[1..token.text.len() - 1];
                Expression::String(content.to_string())
            }
            TokenKind::LeftParen => {
                let inner = self.expression();
                if self.peek_kind() != Some(TokenKind::RightParen) {
                    panic!(
                        "expected closing paren, got {:?}",
                        self.tokens.get(self.position)
                    );
                }
                self.position += 1;
                inner
            }
            _ => panic!("unexpected token {token:?}"),
        }
    }

    fn advance(&mut self) -> Token<'_> {
        let token = *self
            .tokens
            .get(self.position)
            .expect("unexpected end of input");
        self.position += 1;
        token
    }

    fn expect_end(&self) {
        if self.position < self.tokens.len() {
            panic!(
                "unexpected trailing tokens: {:?}",
                &self.tokens[self.position..]
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_integer_literal() {
        assert_eq!(parse("42"), Expression::Integer(42));
    }

    #[test]
    fn parses_addition() {
        assert_eq!(
            parse("1 + 2"),
            Expression::Add {
                left: Box::new(Expression::Integer(1)),
                right: Box::new(Expression::Integer(2)),
            }
        );
    }

    #[test]
    fn addition_is_left_associative() {
        assert_eq!(
            parse("1 + 2 + 3"),
            Expression::Add {
                left: Box::new(Expression::Add {
                    left: Box::new(Expression::Integer(1)),
                    right: Box::new(Expression::Integer(2)),
                }),
                right: Box::new(Expression::Integer(3)),
            }
        );
    }

    #[test]
    fn parses_parenthesized_expressions() {
        assert_eq!(parse("(42)"), Expression::Integer(42));
        assert_eq!(
            parse("1 + (2 + 3)"),
            Expression::Add {
                left: Box::new(Expression::Integer(1)),
                right: Box::new(Expression::Add {
                    left: Box::new(Expression::Integer(2)),
                    right: Box::new(Expression::Integer(3)),
                }),
            }
        );
    }

    #[test]
    #[should_panic(expected = "expected closing paren")]
    fn panics_on_an_unclosed_paren() {
        parse("(1 + 2");
    }

    #[test]
    fn parses_a_string_literal() {
        assert_eq!(
            parse(r#""hello portland""#),
            Expression::String("hello portland".to_string())
        );
    }
}
