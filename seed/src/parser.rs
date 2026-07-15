//! Hand-written recursive descent, like every language that cares about
//! error messages and speed. Crude in the seed: parse failures just panic.

use crate::ast::{Expression, Program, Statement};
use crate::lexer::{self, Token, TokenKind};

pub fn parse(source: &str) -> Program {
    let tokens = lexer::lex(source);
    let mut parser = Parser {
        position: 0,
        tokens,
    };
    let program = parser.program();
    parser.expect_end();
    program
}

struct Parser<'source> {
    position: usize,
    tokens: Vec<Token<'source>>,
}

impl<'source> Parser<'source> {
    fn program(&mut self) -> Program {
        let mut statements = Vec::new();
        self.skip_newlines();
        while self.position < self.tokens.len() {
            statements.push(self.statement());
            self.expect_statement_boundary();
            self.skip_newlines();
        }
        Program { statements }
    }

    fn statement(&mut self) -> Statement {
        if self.peek_kind() == Some(TokenKind::Identifier)
            && self.peek_kind_at(1) == Some(TokenKind::Equal)
        {
            let name = self.advance().text.to_string();
            self.position += 1; // the `=`
            let value = self.expression();
            return Statement::Assignment { name, value };
        }
        Statement::Expression(self.expression())
    }

    fn skip_newlines(&mut self) {
        while self.peek_kind() == Some(TokenKind::Newline) {
            self.position += 1;
        }
    }

    fn expect_statement_boundary(&self) {
        match self.peek_kind() {
            None | Some(TokenKind::Newline) => {}
            _ => panic!(
                "expected a newline after statement, got {:?}",
                self.tokens.get(self.position)
            ),
        }
    }

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
        self.peek_kind_at(0)
    }

    fn peek_kind_at(&self, offset: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.position + offset)
            .map(|token| token.kind)
    }

    fn primary(&mut self) -> Expression {
        let token = self.advance();
        match token.kind {
            TokenKind::Integer => {
                let value = token.text.parse().expect("integer literal out of range");
                Expression::Integer(value)
            }
            TokenKind::Identifier => {
                if self.peek_kind() == Some(TokenKind::LeftParen) {
                    self.position += 1; // the `(`
                    let arguments = self.arguments();
                    Expression::Call {
                        arguments,
                        name: token.text.to_string(),
                    }
                } else {
                    Expression::Variable(token.text.to_string())
                }
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

    /// Parse a comma-separated argument list, consuming the closing paren.
    fn arguments(&mut self) -> Vec<Expression> {
        let mut arguments = Vec::new();
        if self.peek_kind() != Some(TokenKind::RightParen) {
            arguments.push(self.expression());
            while self.peek_kind() == Some(TokenKind::Comma) {
                self.position += 1;
                arguments.push(self.expression());
            }
        }
        if self.peek_kind() != Some(TokenKind::RightParen) {
            panic!(
                "expected closing paren after arguments, got {:?}",
                self.tokens.get(self.position)
            );
        }
        self.position += 1;
        arguments
    }

    fn advance(&mut self) -> Token<'source> {
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

    /// Parse a source expected to be a single expression statement.
    fn expression(source: &str) -> Expression {
        let mut statements = parse(source).statements;
        assert_eq!(statements.len(), 1, "expected exactly one statement");
        match statements.remove(0) {
            Statement::Expression(expression) => expression,
            other => panic!("expected expression statement, got {other:?}"),
        }
    }

    #[test]
    fn parses_an_empty_program() {
        assert_eq!(parse("").statements, vec![]);
        assert_eq!(parse("\n\n").statements, vec![]);
    }

    #[test]
    fn parses_newline_separated_statements() {
        assert_eq!(
            parse("1\n2\n").statements,
            vec![
                Statement::Expression(Expression::Integer(1)),
                Statement::Expression(Expression::Integer(2)),
            ]
        );
    }

    #[test]
    #[should_panic(expected = "expected a newline after statement")]
    fn panics_on_two_expressions_without_a_newline() {
        parse("1 2");
    }

    #[test]
    fn parses_an_assignment() {
        assert_eq!(
            parse(r#"greeting = "hi""#).statements,
            vec![Statement::Assignment {
                name: "greeting".to_string(),
                value: Expression::String("hi".to_string()),
            }]
        );
    }

    #[test]
    fn parses_assignment_then_use() {
        assert_eq!(
            parse("total = 1 + 2\ntotal\n").statements,
            vec![
                Statement::Assignment {
                    name: "total".to_string(),
                    value: Expression::Add {
                        left: Box::new(Expression::Integer(1)),
                        right: Box::new(Expression::Integer(2)),
                    },
                },
                Statement::Expression(Expression::Variable("total".to_string())),
            ]
        );
    }

    #[test]
    fn parses_a_method_call_with_arguments() {
        assert_eq!(
            expression(r#"greet("world", 2)"#),
            Expression::Call {
                arguments: vec![
                    Expression::String("world".to_string()),
                    Expression::Integer(2),
                ],
                name: "greet".to_string(),
            }
        );
    }

    #[test]
    fn parses_a_method_call_with_no_arguments() {
        assert_eq!(
            expression("greet()"),
            Expression::Call {
                arguments: vec![],
                name: "greet".to_string(),
            }
        );
    }

    #[test]
    fn parses_nested_method_calls() {
        assert_eq!(
            expression("outer(inner(1) + 2)"),
            Expression::Call {
                arguments: vec![Expression::Add {
                    left: Box::new(Expression::Call {
                        arguments: vec![Expression::Integer(1)],
                        name: "inner".to_string(),
                    }),
                    right: Box::new(Expression::Integer(2)),
                }],
                name: "outer".to_string(),
            }
        );
    }

    #[test]
    fn parses_an_integer_literal() {
        assert_eq!(expression("42"), Expression::Integer(42));
    }

    #[test]
    fn parses_addition() {
        assert_eq!(
            expression("1 + 2"),
            Expression::Add {
                left: Box::new(Expression::Integer(1)),
                right: Box::new(Expression::Integer(2)),
            }
        );
    }

    #[test]
    fn addition_is_left_associative() {
        assert_eq!(
            expression("1 + 2 + 3"),
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
        assert_eq!(expression("(42)"), Expression::Integer(42));
        assert_eq!(
            expression("1 + (2 + 3)"),
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
    fn parses_a_variable_reference() {
        assert_eq!(
            expression("greeting"),
            Expression::Variable("greeting".to_string())
        );
        assert_eq!(
            expression(r#""hello " + name"#),
            Expression::Add {
                left: Box::new(Expression::String("hello ".to_string())),
                right: Box::new(Expression::Variable("name".to_string())),
            }
        );
    }

    #[test]
    fn parses_a_string_literal() {
        assert_eq!(
            expression(r#""hello portland""#),
            Expression::String("hello portland".to_string())
        );
    }
}
