//! Hand-written recursive descent, like every language that cares about
//! error messages and speed. Crude in the seed: parse failures just panic.

use crate::ast::{
    BinaryOperator, Block, CaseBranch, Expression, GuardAction, InBranch, LogicalOperator,
    Parameter, Pattern, Program, Statement, UnaryOperator,
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
                    safe: false,
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
        if self.peek_is_keyword("mutable") {
            self.position += 1; // the `mutable`
            let token = self.advance();
            if token.kind != TokenKind::Identifier {
                panic!("expected a name after mutable, got {token:?}");
            }
            let name = token.text.to_string();
            if self.peek_kind() != Some(TokenKind::Equal) {
                // Fused to first assignment (ADR 0001): no uninitialized holes.
                panic!("mutable is fused to the first assignment — write `mutable {name} = ...`");
            }
            self.position += 1; // the `=`
            let value = self.expression();
            return Statement::Assignment {
                mutable: true,
                name,
                value,
            };
        }
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
            return Statement::Assignment {
                mutable: false,
                name,
                value,
            };
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
            return Statement::Assignment {
                mutable: false,
                name,
                value,
            };
        }
        // `name << value` — rebinding append (ADR 0015), statement position
        // only, so Ruby's three-way `<<` lexer pileup never returns.
        if self.peek_kind() == Some(TokenKind::Identifier)
            && self.peek_kind_at(1) == Some(TokenKind::LessLess)
        {
            let name = self.advance().text.to_string();
            self.position += 1; // the `<<`
            let value = Box::new(self.expression());
            return Statement::Assignment {
                mutable: false,
                name: name.clone(),
                value: Expression::Append { name, value },
            };
        }
        if self.peek_kind() == Some(TokenKind::Identifier)
            && let Some(command) = self.command_call()
        {
            return command;
        }
        let expression = self.expression();
        // `name[index] = value` — a functional update rebound on the name.
        if self.peek_kind() == Some(TokenKind::Equal) {
            if let Expression::Index { index, receiver } = &expression
                && let Expression::Variable(name) = receiver.as_ref()
            {
                self.position += 1; // the `=`
                let value = Box::new(self.expression());
                return Statement::Assignment {
                    mutable: false,
                    name: name.clone(),
                    value: Expression::IndexUpdate {
                        index: index.clone(),
                        name: name.clone(),
                        value,
                    },
                };
            }
            panic!("only one level of index assignment is supported — assign to name[index]");
        }
        // Rightward destructuring (ADR 0013 §4): `expr => pattern` matches
        // or panics — the pattern-grammar answer to multiple assignment.
        if self.peek_kind() == Some(TokenKind::FatArrow) {
            self.position += 1; // the `=>`
            let pattern = self.pattern();
            return Statement::Expression(Expression::MatchAssert {
                pattern,
                subject: Box::new(expression),
            });
        }
        Statement::Expression(expression)
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
        let mut arguments = Vec::new();
        let mut keyword_arguments: Vec<(String, Expression)> = Vec::new();
        loop {
            if self.peek_kind() == Some(TokenKind::Identifier)
                && self.peek_kind_at(1) == Some(TokenKind::Colon)
            {
                let label = self.advance().text.to_string();
                self.position += 1; // the `:`
                if keyword_arguments
                    .iter()
                    .any(|(existing, _)| *existing == label)
                {
                    panic!("duplicate keyword argument {label}");
                }
                keyword_arguments.push((label, self.expression()));
            } else {
                if !keyword_arguments.is_empty() {
                    panic!("positional arguments cannot follow keyword arguments");
                }
                arguments.push(self.expression());
            }
            if self.peek_kind() != Some(TokenKind::Comma) {
                break;
            }
            self.position += 1; // the `,`
        }
        if self.peek_is_keyword("do") {
            panic!("blocks on paren-less calls aren't supported yet — write {name}(...) do");
        }
        Some(Statement::Expression(Expression::Call {
            arguments,
            keyword_arguments,
            name,
        }))
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
        let (parameters, keyword_parameters) = if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.position += 1;
            self.parameters()
        } else {
            (Vec::new(), Vec::new())
        };
        self.expect_statement_boundary();
        self.skip_newlines();
        let body = self.body_until(&["end"], &format!("def {name}"));
        self.position += 1; // the `end`
        Statement::MethodDefinition {
            body,
            keyword_parameters,
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
        let mut methods: Vec<Statement> = Vec::new();
        while !self.peek_is_keyword("end") {
            if self.position >= self.tokens.len() {
                panic!("expected end to close struct {name}");
            }
            // Fields first, then methods (#27).
            if self.peek_is_keyword("def") {
                let method = self.method_definition();
                let Statement::MethodDefinition {
                    name: method_name, ..
                } = &method
                else {
                    unreachable!()
                };
                if fields.contains(method_name) {
                    panic!(
                        "{method_name} is a field of {name} — a name is a field or a method, never both"
                    );
                }
                if matches!(method_name.as_str(), "new" | "nil?" | "some?" | "with") {
                    panic!("{method_name} is reserved on structs");
                }
                if methods.iter().any(|existing| {
                    matches!(existing, Statement::MethodDefinition { name, .. } if name == method_name)
                }) {
                    panic!("duplicate method {method_name} in struct {name}");
                }
                methods.push(method);
                self.skip_newlines();
                continue;
            }
            let token = self.advance();
            if token.kind != TokenKind::Identifier {
                panic!("expected a field name in struct {name}, got {token:?}");
            }
            if !methods.is_empty() {
                panic!("fields come before methods in struct {name}");
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
        Statement::StructDefinition {
            fields,
            methods,
            name,
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

    /// `case subject ... when a, b [then one-liner | body] ... else ... end`.
    /// Matching is by equality (no ranges or classes to `===` against yet).
    fn case_expression(&mut self) -> Expression {
        let subject = Box::new(self.expression());
        self.expect_statement_boundary();
        self.skip_newlines();
        if self.peek_is_keyword("in") {
            return self.case_in_branches(subject);
        }
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

    /// `case/in` branches (ADR 0013): each `in` takes one pattern, then a
    /// `then` one-liner or an indented body, exactly like `when`.
    fn case_in_branches(&mut self, subject: Box<Expression>) -> Expression {
        let mut branches = Vec::new();
        while self.peek_is_keyword("in") {
            self.position += 1; // the `in`
            let pattern = self.pattern();
            let guard = if self.peek_is_keyword("if") {
                self.position += 1; // the `if`
                Some(self.expression())
            } else {
                None
            };
            let body;
            if self.peek_is_keyword("then") {
                self.position += 1; // the `then`
                body = vec![self.simple_statement()];
                self.expect_statement_boundary();
                self.skip_newlines();
            } else {
                self.expect_statement_boundary();
                self.skip_newlines();
                body = self.body_until(&["in", "else", "end"], "in");
            }
            branches.push(InBranch {
                body,
                guard,
                pattern,
            });
        }
        if self.peek_is_keyword("when") {
            panic!("cannot mix when and in branches in one case");
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
        Expression::CaseIn {
            branches,
            else_body,
            subject,
        }
    }

    /// One pattern, with `|` alternatives binding loosest.
    fn pattern(&mut self) -> Pattern {
        let first = self.pattern_primary();
        if self.peek_kind() != Some(TokenKind::Pipe) {
            return first;
        }
        let mut alternatives = vec![first];
        while self.peek_kind() == Some(TokenKind::Pipe) {
            self.position += 1; // the `|`
            alternatives.push(self.pattern_primary());
        }
        Pattern::Alternative(alternatives)
    }

    fn pattern_primary(&mut self) -> Pattern {
        let token = self.advance();
        match token.kind {
            TokenKind::Integer => {
                let value = token.text.parse().expect("integer literal out of range");
                Pattern::Literal(Box::new(Expression::Integer(value)))
            }
            TokenKind::Minus if self.peek_kind() == Some(TokenKind::Integer) => {
                let value: i64 = self
                    .advance()
                    .text
                    .parse()
                    .expect("integer literal out of range");
                Pattern::Literal(Box::new(Expression::Integer(-value)))
            }
            TokenKind::String => Pattern::Literal(Box::new(string_expression(token.text))),
            TokenKind::Keyword => match token.text {
                "false" => Pattern::Literal(Box::new(Expression::Boolean(false))),
                "nil" => Pattern::Literal(Box::new(Expression::Nil)),
                "true" => Pattern::Literal(Box::new(Expression::Boolean(true))),
                other => panic!("unexpected keyword {other:?} in a pattern"),
            },
            TokenKind::Identifier => {
                let name = token.text.to_string();
                if name.starts_with(|character: char| character.is_ascii_uppercase()) {
                    return self.struct_pattern(name);
                }
                Pattern::Capture(name)
            }
            TokenKind::Caret => {
                let name = self.advance();
                if name.kind != TokenKind::Identifier {
                    panic!("expected a variable name after ^, got {name:?}");
                }
                Pattern::Pin(name.text.to_string())
            }
            TokenKind::LeftBracket => self.array_pattern(),
            other => panic!("unsupported pattern starting with {other:?}"),
        }
    }

    /// `[p, p]` exact, or `[p, *rest]` — a splat may only end the pattern
    /// (suffix-after-splat travels with the deferred find pattern).
    fn array_pattern(&mut self) -> Pattern {
        let mut elements = Vec::new();
        let mut rest: Option<Option<String>> = None;
        if self.peek_kind() != Some(TokenKind::RightBracket) {
            loop {
                if self.peek_kind() == Some(TokenKind::Star) {
                    self.position += 1; // the `*`
                    rest = if self.peek_kind() == Some(TokenKind::Identifier) {
                        Some(Some(self.advance().text.to_string()))
                    } else {
                        Some(None)
                    };
                    if self.peek_kind() == Some(TokenKind::Comma) {
                        panic!("a * rest must end an array pattern");
                    }
                    break;
                }
                elements.push(self.pattern());
                if self.peek_kind() != Some(TokenKind::Comma) {
                    break;
                }
                self.position += 1; // the `,`
            }
        }
        if self.peek_kind() != Some(TokenKind::RightBracket) {
            panic!(
                "expected closing bracket in array pattern, got {:?}",
                self.tokens.get(self.position)
            );
        }
        self.position += 1; // the `]`
        Pattern::Array { elements, rest }
    }

    /// `Name(field: pattern, other:)` — keyword-only (ADR 0013 §5); a bare
    /// `Name` with no parens matches by type alone.
    fn struct_pattern(&mut self, name: String) -> Pattern {
        if self.peek_kind() != Some(TokenKind::LeftParen) {
            return Pattern::Struct {
                fields: Vec::new(),
                name,
            };
        }
        self.position += 1; // the `(`
        let mut fields: Vec<(String, Option<Pattern>)> = Vec::new();
        if self.peek_kind() != Some(TokenKind::RightParen) {
            loop {
                let label_token = self.advance();
                if label_token.kind != TokenKind::Identifier
                    || self.peek_kind() != Some(TokenKind::Colon)
                {
                    panic!(
                        "struct patterns are keyword-only — write {name}(field: pattern) or {name}(field:)"
                    );
                }
                let label = label_token.text.to_string();
                self.position += 1; // the `:`
                if fields.iter().any(|(existing, _)| *existing == label) {
                    panic!("duplicate field {label} in a struct pattern");
                }
                let sub_pattern = match self.peek_kind() {
                    Some(TokenKind::Comma | TokenKind::RightParen) => None,
                    _ => Some(self.pattern()),
                };
                fields.push((label, sub_pattern));
                if self.peek_kind() != Some(TokenKind::Comma) {
                    break;
                }
                self.position += 1; // the `,`
            }
        }
        if self.peek_kind() != Some(TokenKind::RightParen) {
            panic!(
                "expected closing paren in struct pattern {name}, got {:?}",
                self.tokens.get(self.position)
            );
        }
        self.position += 1; // the `)`
        Pattern::Struct { fields, name }
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
    fn parameters(&mut self) -> (Vec<Parameter>, Vec<Parameter>) {
        let mut parameters: Vec<Parameter> = Vec::new();
        let mut keyword_parameters: Vec<Parameter> = Vec::new();
        if self.peek_kind() == Some(TokenKind::Identifier) || self.peek_is_keyword("mutable") {
            loop {
                let mutable = if self.peek_is_keyword("mutable") {
                    self.position += 1; // the `mutable`
                    true
                } else {
                    false
                };
                let token = self.advance();
                if token.kind != TokenKind::Identifier {
                    panic!("expected parameter name, got {token:?}");
                }
                let name = token.text.to_string();
                if parameters.iter().any(|parameter| parameter.name == name)
                    || keyword_parameters
                        .iter()
                        .any(|parameter| parameter.name == name)
                {
                    panic!("duplicate parameter name {name}");
                }
                if self.peek_kind() == Some(TokenKind::Colon) {
                    // `label:` (required) or `label: default` (optional) —
                    // keyword parameters, Ruby 3 style.
                    self.position += 1; // the `:`
                    let default = match self.peek_kind() {
                        Some(TokenKind::Comma | TokenKind::RightParen) => None,
                        _ => Some(self.expression()),
                    };
                    keyword_parameters.push(Parameter {
                        default,
                        mutable,
                        name,
                    });
                } else {
                    if !keyword_parameters.is_empty() {
                        panic!("positional parameter {name} cannot follow a keyword parameter");
                    }
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
                    parameters.push(Parameter {
                        default,
                        mutable,
                        name,
                    });
                }
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
        (parameters, keyword_parameters)
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
        let mut expression = self.logical_or();
        // One-line pattern test (ADR 0013 §4): `expr in pattern` is a
        // boolean, binding its captures on a hit. Binds loosest of all.
        if self.peek_is_keyword("in") {
            self.position += 1; // the `in`
            expression = Expression::MatchTest {
                pattern: self.pattern(),
                subject: Box::new(expression),
            };
        }
        self.depth -= 1;
        expression
    }

    fn logical_or(&mut self) -> Expression {
        let mut left = self.logical_and();
        // `or` is dead-identical to `||` (ADR 0007): same precedence, same
        // semantics — Ruby's looser-than-assignment `or` is the cut perlism.
        while self.peek_kind() == Some(TokenKind::PipePipe) || self.peek_is_keyword("or") {
            self.position += 1;
            let right = match self.guard_right() {
                Some(right) => right,
                None => self.logical_and(),
            };
            left = Expression::Logical {
                left: Box::new(left),
                operator: LogicalOperator::Or,
                right: Box::new(right),
            };
        }
        left
    }

    /// The diverging right side of an or-guard: `or return [value]`,
    /// `or break`, `or next`, and command-form `or panic "why"` (the one
    /// paren-less command allowed off statement position — ADR 0007's
    /// blessed assertion line).
    fn guard_right(&mut self) -> Option<Expression> {
        if self.peek_is_keyword("break") {
            self.position += 1;
            return Some(Expression::Guard(GuardAction::Break));
        }
        if self.peek_is_keyword("next") {
            self.position += 1;
            return Some(Expression::Guard(GuardAction::Next));
        }
        if self.peek_is_keyword("return") {
            self.position += 1;
            let value = match self.peek_kind() {
                None | Some(TokenKind::Newline) => None,
                _ if self.peek_is_keyword("if") || self.peek_is_keyword("unless") => None,
                _ => Some(Box::new(self.expression())),
            };
            return Some(Expression::Guard(GuardAction::Return(value)));
        }
        let panic_command = self
            .tokens
            .get(self.position)
            .is_some_and(|token| token.kind == TokenKind::Identifier && token.text == "panic")
            && self.peek_kind_at(1) == Some(TokenKind::String);
        if panic_command {
            self.position += 1;
            let message = self.expression();
            return Some(Expression::Call {
                arguments: vec![message],
                keyword_arguments: Vec::new(),
                name: "panic".to_string(),
            });
        }
        None
    }

    fn logical_and(&mut self) -> Expression {
        let mut left = self.comparison();
        while self.peek_kind() == Some(TokenKind::AmpersandAmpersand) || self.peek_is_keyword("and")
        {
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
        if self.peek_kind() == Some(TokenKind::Bang) || self.peek_is_keyword("not") {
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
                Some(kind @ (TokenKind::Dot | TokenKind::AmpersandDot)) => {
                    self.position += 1; // the `.` or `&.`
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
                        safe: kind == TokenKind::AmpersandDot,
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
                    Expression::Call {
                        arguments,
                        keyword_arguments,
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
                "self" => Expression::SelfValue,
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
                mutable: false,
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
                    mutable: false,
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
                keyword_arguments: vec![],
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
                keyword_arguments: vec![],
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
                        keyword_arguments: vec![],
                        name: "inner".to_string(),
                    }),
                    right: Box::new(Expression::Integer(2)),
                }],
                keyword_arguments: vec![],
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
                keyword_parameters: vec![],
                name: "greet".to_string(),
                parameters: vec![Parameter {
                    default: None,
                    mutable: false,
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
                keyword_parameters: vec![],
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
