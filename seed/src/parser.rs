//! Hand-written recursive descent, like every language that cares about
//! error messages and speed. Crude in the seed: parse failures just panic.

use crate::ast::{
    BinaryOperator, Block, CaseBranch, Expression, LogicalOperator, Parameter, Program, Statement,
    UnaryOperator,
};
use crate::lexer::{self, Token, TokenKind};

pub fn parse(source: &str) -> Program {
    let tokens = lexer::lex(source);
    let mut parser = Parser {
        depth: 0,
        position: 0,
        tokens,
    };
    let program = parser.program();
    parser.expect_end();
    program
}

/// Deep enough for any human-written program, shallow enough to fail cleanly
/// before the Rust stack runs out (overflow hangs rather than crashes on macOS 26).
const MAXIMUM_NESTING: usize = 10_000;

struct Parser<'source> {
    depth: usize,
    position: usize,
    tokens: Vec<Token<'source>>,
}

/// Decode a raw string token (quotes included) into an expression:
/// a plain literal, or — when it contains `#{...}` — a `+` chain with
/// each interpolation wrapped in `.to_s`.
fn string_expression(text: &str) -> Expression {
    let content = &text[1..text.len() - 1];
    if text.starts_with('\'') {
        // Single-quoted: everything is literal except \' and \\.
        let mut result = String::with_capacity(content.len());
        let mut characters = content.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\\' && matches!(characters.peek(), Some('\'') | Some('\\')) {
                result.push(characters.next().unwrap());
            } else {
                result.push(character);
            }
        }
        return Expression::String(result);
    }
    let mut parts: Vec<Expression> = Vec::new();
    let mut literal = String::new();
    let mut characters = content.char_indices().peekable();

    while let Some((index, character)) = characters.next() {
        match character {
            '\\' => match characters.next() {
                Some((_, 'n')) => literal.push('\n'),
                Some((_, 't')) => literal.push('\t'),
                Some((_, '"')) => literal.push('"'),
                Some((_, '\\')) => literal.push('\\'),
                Some((_, '#')) => literal.push('#'),
                other => panic!("unknown escape sequence \\{:?}", other.map(|(_, c)| c)),
            },
            '#' if matches!(characters.peek(), Some(&(_, '{'))) => {
                characters.next(); // the `{`
                let inner_start = index + 2;
                // Matching close brace by depth; nested string literals are
                // skipped so their braces and quotes don't miscount.
                let mut depth = 1;
                let inner_end = loop {
                    match characters.next() {
                        None => panic!("unterminated interpolation in string {text}"),
                        Some((_, '{')) => depth += 1,
                        Some((position, '}')) => {
                            depth -= 1;
                            if depth == 0 {
                                break position;
                            }
                        }
                        Some((_, '"')) => loop {
                            match characters.next() {
                                None => panic!("unterminated interpolation in string {text}"),
                                Some((_, '\\')) => {
                                    characters.next();
                                }
                                Some((_, '"')) => break,
                                Some(_) => {}
                            }
                        },
                        Some(_) => {}
                    }
                };
                if !literal.is_empty() {
                    parts.push(Expression::String(std::mem::take(&mut literal)));
                }
                let inner = expression_from(&content[inner_start..inner_end]);
                parts.push(Expression::MethodCall {
                    arguments: Vec::new(),
                    block: None,
                    keyword_arguments: Vec::new(),
                    name: "to_s".to_string(),
                    receiver: Box::new(inner),
                });
            }
            _ => literal.push(character),
        }
    }

    if parts.is_empty() {
        return Expression::String(literal);
    }
    if !literal.is_empty() {
        parts.push(Expression::String(literal));
    }
    parts
        .into_iter()
        .reduce(|left, right| Expression::Binary {
            left: Box::new(left),
            operator: BinaryOperator::Add,
            right: Box::new(right),
        })
        .unwrap()
}

/// Parse a standalone expression source (used for interpolation innards).
fn expression_from(source: &str) -> Expression {
    let tokens = lexer::lex(source);
    let mut parser = Parser {
        depth: 0,
        position: 0,
        tokens,
    };
    let expression = parser.expression();
    parser.expect_end();
    expression
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
        if self.peek_is_keyword("struct") {
            return self.struct_definition();
        }
        if self.peek_is_keyword("while") {
            return self.while_statement();
        }
        let statement = self.simple_statement();
        self.postfix_modifier(statement)
    }

    fn simple_statement(&mut self) -> Statement {
        if self.peek_is_keyword("break") {
            self.position += 1;
            return Statement::Break;
        }
        if self.peek_is_keyword("next") {
            self.position += 1;
            return Statement::Next;
        }
        if self.peek_is_keyword("return") {
            self.position += 1;
            let value = match self.peek_kind() {
                None | Some(TokenKind::Newline) => None,
                _ if self.peek_is_keyword("if") || self.peek_is_keyword("unless") => None,
                _ => Some(self.expression()),
            };
            return Statement::Return { value };
        }
        if self.peek_kind() == Some(TokenKind::Identifier)
            && self.peek_kind_at(1) == Some(TokenKind::Equal)
        {
            let name = self.advance().text.to_string();
            self.position += 1; // the `=`
            let value = self.expression();
            return Statement::Assignment { name, value };
        }
        if self.peek_kind() == Some(TokenKind::Identifier)
            && let Some(operator) = match self.peek_kind_at(1) {
                Some(TokenKind::MinusEqual) => Some(BinaryOperator::Subtract),
                Some(TokenKind::PercentEqual) => Some(BinaryOperator::Modulo),
                Some(TokenKind::PlusEqual) => Some(BinaryOperator::Add),
                Some(TokenKind::SlashEqual) => Some(BinaryOperator::Divide),
                Some(TokenKind::StarEqual) => Some(BinaryOperator::Multiply),
                _ => None,
            }
        {
            // `x += e` desugars to `x = x + e`.
            let name = self.advance().text.to_string();
            self.position += 1; // the compound operator
            let value = Expression::Binary {
                left: Box::new(Expression::Variable(name.clone())),
                operator,
                right: Box::new(self.expression()),
            };
            return Statement::Assignment { name, value };
        }
        if self.peek_kind() == Some(TokenKind::Identifier)
            && let Some(command) = self.command_call()
        {
            return command;
        }
        Statement::Expression(self.expression())
    }

    /// Paren-less command calls, statement position only: `puts "hello"`.
    /// Forms Ruby resolves by whitespace guessing are errors here instead.
    fn command_call(&mut self) -> Option<Statement> {
        let name = self.tokens[self.position].text;
        let next = *self.tokens.get(self.position + 1)?;
        let starts_command = match next.kind {
            TokenKind::Bang
            | TokenKind::Identifier
            | TokenKind::Integer
            | TokenKind::String
            | TokenKind::WordArray => true,
            TokenKind::Keyword => matches!(next.text, "false" | "nil" | "true"),
            TokenKind::Minus if next.leading_space => {
                // `foo - 1` is subtraction; `foo -1` would be a guess.
                let after = self.tokens.get(self.position + 2)?;
                if after.leading_space {
                    return None;
                }
                panic!(
                    "ambiguous without parens — write {name}(-{}) or {name} - {}",
                    after.text, after.text
                );
            }
            TokenKind::LeftBracket if next.leading_space => {
                panic!(
                    "ambiguous without parens — write {name}([...]) to pass an array or {name}[...] to index"
                );
            }
            TokenKind::LeftParen if next.leading_space => {
                panic!("ambiguous without parens — write {name}(...) with no space to call");
            }
            _ => false,
        };
        if !starts_command {
            return None;
        }
        let name = self.advance().text.to_string();
        let mut arguments = vec![self.expression()];
        while self.peek_kind() == Some(TokenKind::Comma) {
            self.position += 1; // the `,`
            arguments.push(self.expression());
        }
        if self.peek_is_keyword("do") {
            panic!("blocks on paren-less calls aren't supported yet — write {name}(...) do");
        }
        Some(Statement::Expression(Expression::Call { arguments, name }))
    }

    /// Ruby's postfix guards: `puts(x) if ready`, `return 0 unless valid`.
    fn postfix_modifier(&mut self, statement: Statement) -> Statement {
        let negated = if self.peek_is_keyword("if") {
            false
        } else if self.peek_is_keyword("unless") {
            true
        } else {
            return statement;
        };
        self.position += 1; // the `if` / `unless`
        let condition = Box::new(self.expression());
        let body = vec![statement];
        let (then_body, else_body) = if negated {
            (Vec::new(), body)
        } else {
            (body, Vec::new())
        };
        Statement::Expression(Expression::If {
            condition,
            else_body,
            then_body,
        })
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

    /// `struct Name` then one field name per line, closed by `end`.
    fn struct_definition(&mut self) -> Statement {
        self.position += 1; // the `struct`
        let token = self.advance();
        if token.kind != TokenKind::Identifier {
            panic!("expected struct name after struct, got {token:?}");
        }
        if !token.text.chars().next().unwrap().is_ascii_uppercase() {
            panic!(
                "struct names start with a capital letter, got {}",
                token.text
            );
        }
        let name = token.text.to_string();
        self.expect_statement_boundary();
        self.skip_newlines();
        let mut fields: Vec<String> = Vec::new();
        while !self.peek_is_keyword("end") {
            if self.position >= self.tokens.len() {
                panic!("expected end to close struct {name}");
            }
            let token = self.advance();
            if token.kind != TokenKind::Identifier {
                panic!("expected a field name in struct {name}, got {token:?}");
            }
            if fields.contains(&token.text.to_string()) {
                panic!("duplicate field {} in struct {name}", token.text);
            }
            fields.push(token.text.to_string());
            self.expect_statement_boundary();
            self.skip_newlines();
        }
        if fields.is_empty() {
            panic!("struct {name} needs at least one field");
        }
        self.position += 1; // the `end`
        Statement::StructDefinition { fields, name }
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

    /// `case subject ... when a, b [then one-liner | body] ... else ... end`.
    /// Matching is by equality (no ranges or classes to `===` against yet).
    fn case_expression(&mut self) -> Expression {
        let subject = Box::new(self.expression());
        self.expect_statement_boundary();
        self.skip_newlines();
        let mut branches = Vec::new();
        while self.peek_is_keyword("when") {
            self.position += 1; // the `when`
            let mut values = vec![self.expression()];
            while self.peek_kind() == Some(TokenKind::Comma) {
                self.position += 1;
                values.push(self.expression());
            }
            let body;
            if self.peek_is_keyword("then") {
                self.position += 1; // the `then`
                body = vec![self.simple_statement()];
                self.expect_statement_boundary();
                self.skip_newlines();
            } else {
                self.expect_statement_boundary();
                self.skip_newlines();
                body = self.body_until(&["when", "else", "end"], "when");
            }
            branches.push(CaseBranch { body, values });
        }
        if branches.is_empty() {
            panic!(
                "case needs at least one when, got {:?}",
                self.tokens.get(self.position)
            );
        }
        let else_body = if self.peek_is_keyword("else") {
            self.position += 1; // the `else`
            self.expect_statement_boundary();
            self.skip_newlines();
            self.body_until(&["end"], "else")
        } else {
            Vec::new()
        };
        self.position += 1; // the `end`
        Expression::Case {
            branches,
            else_body,
            subject,
        }
    }

    /// `unless c ... else ... end`, desugared to an `if` with swapped branches.
    fn unless_expression(&mut self) -> Expression {
        let condition = Box::new(self.expression());
        self.expect_statement_boundary();
        self.skip_newlines();
        let unless_body = self.body_until(&["else", "end"], "unless");
        let else_body = if self.peek_is_keyword("else") {
            self.position += 1; // the `else`
            self.expect_statement_boundary();
            self.skip_newlines();
            self.body_until(&["end"], "else")
        } else {
            Vec::new()
        };
        self.position += 1; // the `end`
        Expression::If {
            condition,
            else_body: unless_body,
            then_body: else_body,
        }
    }

    /// Parse a `do |params| ... end` block.
    fn block(&mut self) -> Block {
        self.position += 1; // the `do`
        let mut parameters = Vec::new();
        if self.peek_kind() == Some(TokenKind::Pipe) {
            self.position += 1; // the opening `|`
            loop {
                let token = self.advance();
                if token.kind != TokenKind::Identifier {
                    panic!("expected block parameter, got {token:?}");
                }
                if parameters.contains(&token.text.to_string()) {
                    panic!("duplicate block parameter name {}", token.text);
                }
                parameters.push(token.text.to_string());
                match self.peek_kind() {
                    Some(TokenKind::Comma) => self.position += 1,
                    Some(TokenKind::Pipe) => {
                        self.position += 1; // the closing `|`
                        break;
                    }
                    _ => panic!(
                        "expected , or | in block parameters, got {:?}",
                        self.tokens.get(self.position)
                    ),
                }
            }
        }
        self.expect_statement_boundary();
        self.skip_newlines();
        let body = self.body_until(&["end"], "do");
        self.position += 1; // the `end`
        Block { body, parameters }
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

    /// Parse a comma-separated parameter list (with optional trailing
    /// `name = default` entries), consuming the closing paren.
    fn parameters(&mut self) -> Vec<Parameter> {
        let mut parameters: Vec<Parameter> = Vec::new();
        if self.peek_kind() == Some(TokenKind::Identifier) {
            loop {
                let token = self.advance();
                if token.kind != TokenKind::Identifier {
                    panic!("expected parameter name, got {token:?}");
                }
                let name = token.text.to_string();
                let default = if self.peek_kind() == Some(TokenKind::Equal) {
                    self.position += 1; // the `=`
                    Some(self.expression())
                } else {
                    if parameters
                        .iter()
                        .any(|parameter| parameter.default.is_some())
                    {
                        panic!(
                            "required parameter {name} cannot follow a parameter with a default"
                        );
                    }
                    None
                };
                if parameters.iter().any(|parameter| parameter.name == name) {
                    panic!("duplicate parameter name {name}");
                }
                parameters.push(Parameter { default, name });
                if self.peek_kind() != Some(TokenKind::Comma) {
                    break;
                }
                self.position += 1; // the `,`
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
        self.depth += 1;
        if self.depth > MAXIMUM_NESTING {
            panic!("expression nesting deeper than {MAXIMUM_NESTING} levels");
        }
        let expression = self.logical_or();
        self.depth -= 1;
        expression
    }

    fn logical_or(&mut self) -> Expression {
        let mut left = self.logical_and();
        while self.peek_kind() == Some(TokenKind::PipePipe) {
            self.position += 1;
            let right = self.logical_and();
            left = Expression::Logical {
                left: Box::new(left),
                operator: LogicalOperator::Or,
                right: Box::new(right),
            };
        }
        left
    }

    fn logical_and(&mut self) -> Expression {
        let mut left = self.comparison();
        while self.peek_kind() == Some(TokenKind::AmpersandAmpersand) {
            self.position += 1;
            let right = self.comparison();
            left = Expression::Logical {
                left: Box::new(left),
                operator: LogicalOperator::And,
                right: Box::new(right),
            };
        }
        left
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
        if self.peek_kind() == Some(TokenKind::Bang) {
            self.position += 1;
            return Expression::Unary {
                operand: Box::new(self.unary()),
                operator: UnaryOperator::Not,
            };
        }
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
                // Fluent style: a chain may continue on the next line
                // when the line leads with a dot.
                Some(TokenKind::Newline) => {
                    let mut offset = 0;
                    while self.peek_kind_at(offset) == Some(TokenKind::Newline) {
                        offset += 1;
                    }
                    if self.peek_kind_at(offset) != Some(TokenKind::Dot) {
                        return expression;
                    }
                    self.position += offset;
                }
                Some(TokenKind::Dot) => {
                    self.position += 1; // the `.`
                    let token = self.advance();
                    if token.kind != TokenKind::Identifier {
                        panic!("expected method name after dot, got {token:?}");
                    }
                    let (arguments, keyword_arguments) =
                        if self.peek_kind() == Some(TokenKind::LeftParen) {
                            self.position += 1; // the `(`
                            self.call_arguments()
                        } else {
                            (Vec::new(), Vec::new())
                        };
                    let block = if self.peek_is_keyword("do") {
                        Some(self.block())
                    } else {
                        None
                    };
                    expression = Expression::MethodCall {
                        arguments,
                        block,
                        keyword_arguments,
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
                    let (arguments, keyword_arguments) = self.call_arguments();
                    if !keyword_arguments.is_empty() {
                        panic!(
                            "keyword arguments are only supported on struct new and with so far"
                        );
                    }
                    Expression::Call {
                        arguments,
                        name: token.text.to_string(),
                    }
                } else {
                    Expression::Variable(token.text.to_string())
                }
            }
            TokenKind::String => string_expression(token.text),
            TokenKind::WordArray => {
                let words = &token.text[3..token.text.len() - 1];
                Expression::ArrayLiteral(
                    words
                        .split_whitespace()
                        .map(|word| Expression::String(word.to_string()))
                        .collect(),
                )
            }
            TokenKind::LeftBrace => {
                let mut pairs = Vec::new();
                if self.peek_kind() != Some(TokenKind::RightBrace) {
                    loop {
                        let key = self.expression();
                        if self.peek_kind() != Some(TokenKind::FatArrow) {
                            panic!(
                                "expected => in hash literal, got {:?}",
                                self.tokens.get(self.position)
                            );
                        }
                        self.position += 1; // the `=>`
                        let value = self.expression();
                        pairs.push((key, value));
                        if self.peek_kind() != Some(TokenKind::Comma) {
                            break;
                        }
                        self.position += 1; // the `,`
                    }
                }
                if self.peek_kind() != Some(TokenKind::RightBrace) {
                    panic!(
                        "expected closing brace, got {:?}",
                        self.tokens.get(self.position)
                    );
                }
                self.position += 1;
                Expression::HashLiteral(pairs)
            }
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
                "case" => self.case_expression(),
                "false" => Expression::Boolean(false),
                "nil" => Expression::Nil,
                "if" => self.if_expression(),
                "true" => Expression::Boolean(true),
                "unless" => self.unless_expression(),
                _ => panic!("unexpected keyword {:?}", token.text),
            },
            _ => panic!("unexpected token {token:?}"),
        }
    }

    /// Parse a comma-separated argument list, consuming the closing paren.
    /// Keyword arguments (`label: value`) may only follow positional ones.
    fn call_arguments(&mut self) -> (Vec<Expression>, Vec<(String, Expression)>) {
        let mut positional = Vec::new();
        let mut keyword: Vec<(String, Expression)> = Vec::new();
        if self.peek_kind() != Some(TokenKind::RightParen) {
            loop {
                if self.peek_kind() == Some(TokenKind::Identifier)
                    && self.peek_kind_at(1) == Some(TokenKind::Colon)
                {
                    let label = self.advance().text.to_string();
                    self.position += 1; // the `:`
                    if keyword.iter().any(|(existing, _)| *existing == label) {
                        panic!("duplicate keyword argument {label}");
                    }
                    keyword.push((label, self.expression()));
                } else {
                    if !keyword.is_empty() {
                        panic!("positional arguments cannot follow keyword arguments");
                    }
                    positional.push(self.expression());
                }
                if self.peek_kind() != Some(TokenKind::Comma) {
                    break;
                }
                self.position += 1; // the `,`
            }
        }
        if self.peek_kind() != Some(TokenKind::RightParen) {
            panic!(
                "expected closing paren after arguments, got {:?}",
                self.tokens.get(self.position)
            );
        }
        self.position += 1;
        (positional, keyword)
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
                parameters: vec![Parameter {
                    default: None,
                    name: "name".to_string(),
                }],
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
