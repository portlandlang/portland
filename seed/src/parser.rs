//! Hand-written recursive descent, like every language that cares about
//! error messages and speed. Crude in the seed: parse failures just panic.

use crate::ast::{BinaryOperator, Expression, Program, Statement, UnaryOperator};
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

/// Decode a raw string token (quotes included) into its value.
fn unescape(text: &str) -> String {
    let content = &text[1..text.len() - 1];
    let mut result = String::with_capacity(content.len());
    let mut characters = content.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        match characters.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('"') => result.push('"'),
            Some('\\') => result.push('\\'),
            other => panic!("unknown escape sequence \\{other:?}"),
        }
    }
    result
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
        if self.peek_is_keyword("def") {
            return self.method_definition();
        }
        if self.peek_is_keyword("while") {
            return self.while_statement();
        }
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

    fn method_definition(&mut self) -> Statement {
        self.position += 1; // the `def`
        let token = self.advance();
        if token.kind != TokenKind::Identifier {
            panic!("expected method name after def, got {token:?}");
        }
        let name = token.text.to_string();
        let parameters = if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.position += 1;
            self.parameters()
        } else {
            Vec::new()
        };
        self.expect_statement_boundary();
        self.skip_newlines();
        let body = self.body_until(&["end"], &format!("def {name}"));
        self.position += 1; // the `end`
        Statement::MethodDefinition {
            body,
            name,
            parameters,
        }
    }

    fn while_statement(&mut self) -> Statement {
        self.position += 1; // the `while`
        let condition = self.expression();
        self.expect_statement_boundary();
        self.skip_newlines();
        let body = self.body_until(&["end"], "while");
        self.position += 1; // the `end`
        Statement::While { body, condition }
    }

    /// Parse statements up to (not consuming) one of the terminator keywords.
    fn body_until(&mut self, terminators: &[&str], context: &str) -> Vec<Statement> {
        let mut body = Vec::new();
        loop {
            if terminators.iter().any(|word| self.peek_is_keyword(word)) {
                return body;
            }
            if self.position >= self.tokens.len() {
                panic!("expected end to close {context}");
            }
            body.push(self.statement());
            self.expect_statement_boundary();
            self.skip_newlines();
        }
    }

    fn if_expression(&mut self) -> Expression {
        let condition = Box::new(self.expression());
        self.expect_statement_boundary();
        self.skip_newlines();
        let then_body = self.body_until(&["else", "elsif", "end"], "if");
        let else_body;
        if self.peek_is_keyword("elsif") {
            // Sugar for `else` holding a nested `if`; the chain shares one `end`,
            // which the recursive call consumes.
            self.position += 1; // the `elsif`
            else_body = vec![Statement::Expression(self.if_expression())];
        } else {
            else_body = if self.peek_is_keyword("else") {
                self.position += 1; // the `else`
                self.expect_statement_boundary();
                self.skip_newlines();
                self.body_until(&["end"], "else")
            } else {
                Vec::new()
            };
            self.position += 1; // the `end`
        }
        Expression::If {
            condition,
            else_body,
            then_body,
        }
    }

    /// Parse a comma-separated parameter name list, consuming the closing paren.
    fn parameters(&mut self) -> Vec<String> {
        let mut parameters = Vec::new();
        if self.peek_kind() == Some(TokenKind::Identifier) {
            parameters.push(self.advance().text.to_string());
            while self.peek_kind() == Some(TokenKind::Comma) {
                self.position += 1;
                let token = self.advance();
                if token.kind != TokenKind::Identifier {
                    panic!("expected parameter name, got {token:?}");
                }
                parameters.push(token.text.to_string());
            }
        }
        if self.peek_kind() != Some(TokenKind::RightParen) {
            panic!(
                "expected closing paren after parameters, got {:?}",
                self.tokens.get(self.position)
            );
        }
        self.position += 1;
        parameters
    }

    fn peek_is_keyword(&self, word: &str) -> bool {
        self.tokens
            .get(self.position)
            .is_some_and(|token| token.kind == TokenKind::Keyword && token.text == word)
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
        self.comparison()
    }

    fn comparison(&mut self) -> Expression {
        let mut left = self.addition();
        while let Some(operator) = match self.peek_kind() {
            Some(TokenKind::EqualEqual) => Some(BinaryOperator::Equals),
            Some(TokenKind::Greater) => Some(BinaryOperator::Greater),
            Some(TokenKind::GreaterEqual) => Some(BinaryOperator::GreaterOrEqual),
            Some(TokenKind::Less) => Some(BinaryOperator::Less),
            Some(TokenKind::LessEqual) => Some(BinaryOperator::LessOrEqual),
            Some(TokenKind::NotEqual) => Some(BinaryOperator::NotEquals),
            _ => None,
        } {
            self.position += 1;
            let right = self.addition();
            left = Expression::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }
        left
    }

    fn addition(&mut self) -> Expression {
        let mut left = self.multiplication();
        while let Some(operator) = match self.peek_kind() {
            Some(TokenKind::Minus) => Some(BinaryOperator::Subtract),
            Some(TokenKind::Plus) => Some(BinaryOperator::Add),
            _ => None,
        } {
            self.position += 1;
            let right = self.multiplication();
            left = Expression::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }
        left
    }

    fn multiplication(&mut self) -> Expression {
        let mut left = self.unary();
        while let Some(operator) = match self.peek_kind() {
            Some(TokenKind::Percent) => Some(BinaryOperator::Modulo),
            Some(TokenKind::Slash) => Some(BinaryOperator::Divide),
            Some(TokenKind::Star) => Some(BinaryOperator::Multiply),
            _ => None,
        } {
            self.position += 1;
            let right = self.unary();
            left = Expression::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
        }
        left
    }

    fn unary(&mut self) -> Expression {
        if self.peek_kind() == Some(TokenKind::Minus) {
            self.position += 1;
            // Ruby-style: -5 is a negative literal, so -5.abs is 5, not -(5.abs).
            if self.peek_kind() == Some(TokenKind::Integer) {
                let token = self.advance();
                let value: i64 = token.text.parse().expect("integer literal out of range");
                return self.postfix_from(Expression::Integer(-value));
            }
            return Expression::Unary {
                operand: Box::new(self.unary()),
                operator: UnaryOperator::Negate,
            };
        }
        self.postfix()
    }

    /// Chained `.method` calls, binding tighter than any operator.
    fn postfix(&mut self) -> Expression {
        let expression = self.primary();
        self.postfix_from(expression)
    }

    fn postfix_from(&mut self, mut expression: Expression) -> Expression {
        loop {
            match self.peek_kind() {
                Some(TokenKind::Dot) => {
                    self.position += 1; // the `.`
                    let token = self.advance();
                    if token.kind != TokenKind::Identifier {
                        panic!("expected method name after dot, got {token:?}");
                    }
                    let arguments = if self.peek_kind() == Some(TokenKind::LeftParen) {
                        self.position += 1; // the `(`
                        self.arguments()
                    } else {
                        Vec::new()
                    };
                    expression = Expression::MethodCall {
                        arguments,
                        name: token.text.to_string(),
                        receiver: Box::new(expression),
                    };
                }
                Some(TokenKind::LeftBracket) => {
                    self.position += 1; // the `[`
                    let index = Box::new(self.expression());
                    if self.peek_kind() != Some(TokenKind::RightBracket) {
                        panic!(
                            "expected closing bracket, got {:?}",
                            self.tokens.get(self.position)
                        );
                    }
                    self.position += 1; // the `]`
                    expression = Expression::Index {
                        index,
                        receiver: Box::new(expression),
                    };
                }
                _ => return expression,
            }
        }
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
            TokenKind::String => Expression::String(unescape(token.text)),
            TokenKind::LeftBracket => {
                let mut elements = Vec::new();
                if self.peek_kind() != Some(TokenKind::RightBracket) {
                    elements.push(self.expression());
                    while self.peek_kind() == Some(TokenKind::Comma) {
                        self.position += 1;
                        elements.push(self.expression());
                    }
                }
                if self.peek_kind() != Some(TokenKind::RightBracket) {
                    panic!(
                        "expected closing bracket, got {:?}",
                        self.tokens.get(self.position)
                    );
                }
                self.position += 1;
                Expression::ArrayLiteral(elements)
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
            TokenKind::Keyword => match token.text {
                "false" => Expression::Boolean(false),
                "if" => self.if_expression(),
                "true" => Expression::Boolean(true),
                _ => panic!("unexpected keyword {:?}", token.text),
            },
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
                    value: Expression::Binary {
                        operator: BinaryOperator::Add,
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
                arguments: vec![Expression::Binary {
                    operator: BinaryOperator::Add,
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
    fn parses_a_method_definition() {
        let source = "def greet(name)\n  \"hello \" + name\nend\n";
        assert_eq!(
            parse(source).statements,
            vec![Statement::MethodDefinition {
                body: vec![Statement::Expression(Expression::Binary {
                    operator: BinaryOperator::Add,
                    left: Box::new(Expression::String("hello ".to_string())),
                    right: Box::new(Expression::Variable("name".to_string())),
                })],
                name: "greet".to_string(),
                parameters: vec!["name".to_string()],
            }]
        );
    }

    #[test]
    fn parses_a_method_definition_without_parameters() {
        assert_eq!(
            parse("def pdx\n  \"portland\"\nend\n").statements,
            vec![Statement::MethodDefinition {
                body: vec![Statement::Expression(Expression::String(
                    "portland".to_string()
                ))],
                name: "pdx".to_string(),
                parameters: vec![],
            }]
        );
    }

    #[test]
    #[should_panic(expected = "expected end")]
    fn panics_on_an_unclosed_method_definition() {
        parse("def greet(name)\n  name\n");
    }

    #[test]
    fn parses_an_integer_literal() {
        assert_eq!(expression("42"), Expression::Integer(42));
    }

    #[test]
    fn parses_addition() {
        assert_eq!(
            expression("1 + 2"),
            Expression::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(Expression::Integer(1)),
                right: Box::new(Expression::Integer(2)),
            }
        );
    }

    #[test]
    fn addition_is_left_associative() {
        assert_eq!(
            expression("1 + 2 + 3"),
            Expression::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(Expression::Binary {
                    operator: BinaryOperator::Add,
                    left: Box::new(Expression::Integer(1)),
                    right: Box::new(Expression::Integer(2)),
                }),
                right: Box::new(Expression::Integer(3)),
            }
        );
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        assert_eq!(
            expression("1 + 2 * 3"),
            Expression::Binary {
                left: Box::new(Expression::Integer(1)),
                operator: BinaryOperator::Add,
                right: Box::new(Expression::Binary {
                    left: Box::new(Expression::Integer(2)),
                    operator: BinaryOperator::Multiply,
                    right: Box::new(Expression::Integer(3)),
                }),
            }
        );
    }

    #[test]
    fn parses_parenthesized_expressions() {
        assert_eq!(expression("(42)"), Expression::Integer(42));
        assert_eq!(
            expression("1 + (2 + 3)"),
            Expression::Binary {
                operator: BinaryOperator::Add,
                left: Box::new(Expression::Integer(1)),
                right: Box::new(Expression::Binary {
                    operator: BinaryOperator::Add,
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
            Expression::Binary {
                operator: BinaryOperator::Add,
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
