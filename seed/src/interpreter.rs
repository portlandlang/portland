//! A tree-walking interpreter — the reference semantics for the Stage 0
//! subset before any codegen exists. Crude in the seed: type errors panic.

use std::collections::HashMap;

use crate::ast::{
    BinaryOperator, Block, Expression, LogicalOperator, Parameter, Program, Statement,
    UnaryOperator,
};
use crate::parser;
use crate::value::Value;

/// Parse and evaluate a source string, returning the last statement's value.
pub fn evaluate(source: &str) -> Option<Value> {
    let program = parser::parse(source);
    let mut interpreter = Interpreter::new();
    interpreter.program(&program)
}

#[derive(Clone)]
struct Method {
    body: Vec<Statement>,
    parameters: Vec<Parameter>,
}

/// A `return`, `break`, or `next` in flight, unwinding to whatever handles it.
enum Pending {
    Break,
    Next,
    Return(Option<Value>),
}

pub struct Interpreter<W: std::io::Write = std::io::Stdout> {
    arguments: Vec<String>,
    methods: HashMap<String, Method>,
    output: W,
    pending: Option<Pending>,
    variables: HashMap<String, Value>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::with_output(std::io::stdout())
    }
}

impl<W: std::io::Write> Interpreter<W> {
    pub fn with_output(output: W) -> Self {
        Self {
            arguments: Vec::new(),
            methods: HashMap::new(),
            output,
            pending: None,
            variables: HashMap::new(),
        }
    }

    /// Command-line arguments exposed to the program via `argv()`.
    pub fn set_arguments(&mut self, arguments: Vec<String>) {
        self.arguments = arguments;
    }

    pub fn program(&mut self, program: &Program) -> Option<Value> {
        let last = self.run_body(&program.statements);
        match self.pending.take() {
            None => last,
            Some(Pending::Break) => panic!("break outside of a loop"),
            Some(Pending::Next) => panic!("next outside of a loop"),
            Some(Pending::Return(_)) => panic!("return outside of a method"),
        }
    }

    /// Run statements until done or a `return`/`break` starts unwinding.
    fn run_body(&mut self, statements: &[Statement]) -> Option<Value> {
        let mut last = None;
        for statement in statements {
            last = self.statement(statement);
            if self.pending.is_some() {
                break;
            }
        }
        last
    }

    fn statement(&mut self, statement: &Statement) -> Option<Value> {
        match statement {
            Statement::Assignment { name, value } => {
                let value = self.value_of(value);
                self.variables.insert(name.clone(), value.clone());
                Some(value)
            }
            Statement::Expression(expression) => self.expression(expression),
            Statement::MethodDefinition {
                body,
                name,
                parameters,
            } => {
                let method = Method {
                    body: body.clone(),
                    parameters: parameters.clone(),
                };
                self.methods.insert(name.clone(), method);
                None
            }
            Statement::Break => {
                self.pending = Some(Pending::Break);
                None
            }
            Statement::Next => {
                self.pending = Some(Pending::Next);
                None
            }
            Statement::Return { value } => {
                let value = value.as_ref().map(|expression| self.value_of(expression));
                self.pending = Some(Pending::Return(value));
                None
            }
            Statement::While { body, condition } => {
                loop {
                    let condition = self.value_of(condition);
                    let Value::Boolean(condition) = condition else {
                        panic!("while condition must be true or false, got {condition:?}")
                    };
                    if !condition {
                        break;
                    }
                    self.run_body(body);
                    match self.pending {
                        None => {}
                        Some(Pending::Break) => {
                            self.pending = None;
                            break;
                        }
                        Some(Pending::Next) => self.pending = None,
                        // A return keeps unwinding to the enclosing method.
                        Some(Pending::Return(_)) => break,
                    }
                }
                None
            }
        }
    }

    fn expression(&mut self, expression: &Expression) -> Option<Value> {
        match expression {
            Expression::ArrayLiteral(elements) => Some(Value::Array(
                elements.iter().map(|e| self.value_of(e)).collect(),
            )),
            Expression::Boolean(value) => Some(Value::Boolean(*value)),
            Expression::Case {
                branches,
                else_body,
                subject,
            } => {
                let subject = self.value_of(subject);
                for branch in branches {
                    for value in &branch.values {
                        if self.value_of(value) == subject {
                            return self.run_body(&branch.body);
                        }
                    }
                }
                self.run_body(else_body)
            }
            Expression::HashLiteral(entries) => {
                let mut pairs: Vec<(Value, Value)> = Vec::new();
                for (key_expression, value_expression) in entries {
                    let key = self.value_of(key_expression);
                    let value = self.value_of(value_expression);
                    // Ruby rule: a duplicate key keeps its position, last value wins.
                    match pairs.iter_mut().find(|(existing, _)| *existing == key) {
                        Some(pair) => pair.1 = value,
                        None => pairs.push((key, value)),
                    }
                }
                Some(Value::Hash(pairs))
            }
            Expression::Index { index, receiver } => {
                let receiver = self.value_of(receiver);
                let index = self.value_of(index);
                match (&receiver, &index) {
                    (Value::Array(elements), Value::Integer(index)) => {
                        // Ruby-style negative indices; out of range panics — no nil to return.
                        let length = elements.len() as i64;
                        let position = if *index < 0 { length + index } else { *index };
                        if position < 0 || position >= length {
                            panic!("index {index} out of range for array of length {length}");
                        }
                        Some(elements[position as usize].clone())
                    }
                    (Value::String(text), Value::Integer(index)) => {
                        // Indexing a string yields a one-character string.
                        let length = text.chars().count() as i64;
                        let position = if *index < 0 { length + index } else { *index };
                        if position < 0 || position >= length {
                            panic!("index {index} out of range for string of length {length}");
                        }
                        let character = text.chars().nth(position as usize).unwrap();
                        Some(Value::String(character.to_string()))
                    }
                    (Value::Hash(pairs), key) => Some(
                        pairs
                            .iter()
                            .find(|(existing, _)| existing == key)
                            .unwrap_or_else(|| {
                                panic!("key {key} not found in hash — no nil; check key? first")
                            })
                            .1
                            .clone(),
                    ),
                    _ => panic!("cannot index {receiver:?} with {index:?}"),
                }
            }
            Expression::If {
                condition,
                else_body,
                then_body,
            } => {
                let condition = self.value_of(condition);
                // Strict booleans, no truthiness — Portland has no nil to be falsy.
                let Value::Boolean(condition) = condition else {
                    panic!("if condition must be true or false, got {condition:?}")
                };
                let body = if condition { then_body } else { else_body };
                self.run_body(body)
            }
            Expression::Integer(value) => Some(Value::Integer(*value)),
            Expression::String(value) => Some(Value::String(value.clone())),
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.value_of(left);
                let right = self.value_of(right);
                match (left, operator, right) {
                    (Value::Integer(a), BinaryOperator::Add, Value::Integer(b)) => {
                        Some(Value::Integer(a + b))
                    }
                    (Value::Integer(a), BinaryOperator::Divide, Value::Integer(b)) => {
                        // Truncates toward zero (Rust semantics); Ruby floors.
                        // Revisit when Portland's integer semantics are specified.
                        Some(Value::Integer(a / b))
                    }
                    (Value::Integer(a), BinaryOperator::Modulo, Value::Integer(b)) => {
                        // Truncated remainder (Rust semantics); Ruby's % is floored.
                        // Revisit when Portland's integer semantics are specified.
                        Some(Value::Integer(a % b))
                    }
                    (Value::Integer(a), BinaryOperator::Multiply, Value::Integer(b)) => {
                        Some(Value::Integer(a * b))
                    }
                    (Value::Integer(a), BinaryOperator::Subtract, Value::Integer(b)) => {
                        Some(Value::Integer(a - b))
                    }
                    (Value::String(a), BinaryOperator::Add, Value::String(b)) => {
                        Some(Value::String(a + &b))
                    }
                    (Value::Array(a), BinaryOperator::Add, Value::Array(b)) => {
                        let mut combined = a;
                        combined.extend(b);
                        Some(Value::Array(combined))
                    }
                    (Value::String(s), BinaryOperator::Multiply, Value::Integer(count)) => {
                        let count = usize::try_from(count)
                            .unwrap_or_else(|_| panic!("cannot repeat a string {count} times"));
                        Some(Value::String(s.repeat(count)))
                    }
                    (Value::Array(elements), BinaryOperator::Multiply, Value::Integer(count)) => {
                        let count = usize::try_from(count)
                            .unwrap_or_else(|_| panic!("cannot repeat an array {count} times"));
                        let mut repeated = Vec::with_capacity(elements.len() * count);
                        for _ in 0..count {
                            repeated.extend(elements.iter().cloned());
                        }
                        Some(Value::Array(repeated))
                    }
                    (Value::Integer(a), BinaryOperator::Greater, Value::Integer(b)) => {
                        Some(Value::Boolean(a > b))
                    }
                    (Value::Integer(a), BinaryOperator::GreaterOrEqual, Value::Integer(b)) => {
                        Some(Value::Boolean(a >= b))
                    }
                    (Value::Integer(a), BinaryOperator::Less, Value::Integer(b)) => {
                        Some(Value::Boolean(a < b))
                    }
                    (Value::Integer(a), BinaryOperator::LessOrEqual, Value::Integer(b)) => {
                        Some(Value::Boolean(a <= b))
                    }
                    // Equality is defined for every value pair; mixed types are just unequal.
                    (left, BinaryOperator::Equals, right) => Some(Value::Boolean(left == right)),
                    (left, BinaryOperator::NotEquals, right) => Some(Value::Boolean(left != right)),
                    (left, operator, right) => {
                        panic!("cannot apply {operator:?} to {left:?} and {right:?}")
                    }
                }
            }
            Expression::MethodCall {
                arguments,
                block,
                name,
                receiver,
            } => {
                let receiver = self.value_of(receiver);
                let arguments: Vec<Value> = arguments.iter().map(|a| self.value_of(a)).collect();
                Some(self.method_call(receiver, name, arguments, block.as_ref()))
            }
            Expression::Logical {
                left,
                operator,
                right,
            } => {
                let left = self.boolean_of(left, "&& and || operands");
                // Short-circuit: the right side only runs when it can matter.
                let result = match (operator, left) {
                    (LogicalOperator::And, false) => false,
                    (LogicalOperator::Or, true) => true,
                    _ => self.boolean_of(right, "&& and || operands"),
                };
                Some(Value::Boolean(result))
            }
            Expression::Unary { operand, operator } => {
                let operand = self.value_of(operand);
                match (operator, operand) {
                    (UnaryOperator::Negate, Value::Integer(value)) => Some(Value::Integer(-value)),
                    (UnaryOperator::Not, Value::Boolean(value)) => Some(Value::Boolean(!value)),
                    (operator, operand) => {
                        panic!("cannot apply {operator:?} to {operand:?}")
                    }
                }
            }
            Expression::Variable(name) => Some(
                self.variables
                    .get(name)
                    .unwrap_or_else(|| panic!("undefined variable {name}"))
                    .clone(),
            ),
            Expression::Call { arguments, name } => {
                let arguments: Vec<Value> = arguments.iter().map(|a| self.value_of(a)).collect();
                self.call(name, arguments)
            }
        }
    }

    /// Built-in methods on values — read-only on purpose; mutation is a
    /// language-design decision the seed doesn't get to make.
    fn method_call(
        &mut self,
        receiver: Value,
        name: &str,
        arguments: Vec<Value>,
        block: Option<&Block>,
    ) -> Value {
        if let Some(block) = block {
            return match (&receiver, name, arguments.as_slice()) {
                (Value::Array(elements), "each", []) => {
                    for element in elements.clone() {
                        self.run_block(block, vec![element]);
                    }
                    receiver
                }
                (Value::Hash(pairs), "each", []) => {
                    for (key, value) in pairs.clone() {
                        self.run_block(block, vec![key, value]);
                    }
                    receiver
                }
                (Value::Array(elements), "map", []) => {
                    let mut results = Vec::new();
                    for element in elements.clone() {
                        let result = self
                            .run_block(block, vec![element])
                            .unwrap_or_else(|| panic!("map block produced no value"));
                        results.push(result);
                    }
                    Value::Array(results)
                }
                (Value::Array(elements), "each_with_index", []) => {
                    for (index, element) in elements.clone().into_iter().enumerate() {
                        self.run_block(block, vec![element, Value::Integer(index as i64)]);
                    }
                    receiver
                }
                (Value::Array(elements), "reduce", [initial]) => {
                    let mut accumulator = initial.clone();
                    for element in elements.clone() {
                        accumulator = self
                            .run_block(block, vec![accumulator, element])
                            .unwrap_or_else(|| panic!("reduce block produced no value"));
                    }
                    accumulator
                }
                (Value::Array(elements), "reject", []) => {
                    let mut kept = Vec::new();
                    for element in elements.clone() {
                        if !self.block_boolean(block, element.clone(), "reject") {
                            kept.push(element);
                        }
                    }
                    Value::Array(kept)
                }
                (Value::Array(elements), "select", []) => {
                    let mut kept = Vec::new();
                    for element in elements.clone() {
                        if self.block_boolean(block, element.clone(), "select") {
                            kept.push(element);
                        }
                    }
                    Value::Array(kept)
                }
                (Value::Integer(count), "times", []) => {
                    for index in 0..*count {
                        self.run_block(block, vec![Value::Integer(index)]);
                    }
                    receiver
                }
                (receiver, name, _) => {
                    panic!("undefined block-taking method {name} for {receiver:?}")
                }
            };
        }

        match (&receiver, name, arguments.as_slice()) {
            (Value::Array(elements), "empty?", []) => Value::Boolean(elements.is_empty()),
            (Value::Array(elements), "first", []) => elements
                .first()
                .unwrap_or_else(|| panic!("first on an empty array — no nil; check empty? first"))
                .clone(),
            (Value::Array(elements), "join", [Value::String(separator)]) => Value::String(
                elements
                    .iter()
                    .map(|element| element.to_string())
                    .collect::<Vec<_>>()
                    .join(separator),
            ),
            (Value::Array(elements), "last", []) => elements
                .last()
                .unwrap_or_else(|| panic!("last on an empty array — no nil; check empty? first"))
                .clone(),
            (Value::Array(elements), "length", []) => Value::Integer(elements.len() as i64),
            (Value::Hash(pairs), "empty?", []) => Value::Boolean(pairs.is_empty()),
            (Value::Hash(pairs), "key?", [key]) => {
                Value::Boolean(pairs.iter().any(|(existing, _)| existing == key))
            }
            (Value::Hash(pairs), "keys", []) => {
                Value::Array(pairs.iter().map(|(key, _)| key.clone()).collect())
            }
            (Value::Hash(pairs), "length", []) => Value::Integer(pairs.len() as i64),
            (Value::Hash(pairs), "values", []) => {
                Value::Array(pairs.iter().map(|(_, value)| value.clone()).collect())
            }
            (Value::Array(elements), "include?", [needle]) => {
                Value::Boolean(elements.contains(needle))
            }
            (Value::Array(elements), "max", []) => Self::integers_of(elements, "max")
                .into_iter()
                .max()
                .map(Value::Integer)
                .unwrap_or_else(|| panic!("max on an empty array — no nil; check empty? first")),
            (Value::Array(elements), "min", []) => Self::integers_of(elements, "min")
                .into_iter()
                .min()
                .map(Value::Integer)
                .unwrap_or_else(|| panic!("min on an empty array — no nil; check empty? first")),
            (Value::Array(elements), "slice", [Value::Integer(start), Value::Integer(length)]) => {
                let start = usize::try_from(*start)
                    .unwrap_or_else(|_| panic!("slice start must not be negative, got {start}"));
                let length = usize::try_from(*length)
                    .unwrap_or_else(|_| panic!("slice length must not be negative, got {length}"));
                Value::Array(elements.iter().skip(start).take(length).cloned().collect())
            }
            (Value::Array(elements), "sort", []) => {
                let mut sorted = Self::integers_of(elements, "sort");
                sorted.sort_unstable();
                Value::Array(sorted.into_iter().map(Value::Integer).collect())
            }
            (Value::Array(elements), "sum", []) => {
                Value::Integer(Self::integers_of(elements, "sum").into_iter().sum())
            }
            (Value::Integer(n), "abs", []) => Value::Integer(n.abs()),
            (Value::Integer(n), "even?", []) => Value::Boolean(n % 2 == 0),
            (Value::Integer(n), "negative?", []) => Value::Boolean(*n < 0),
            (Value::Integer(n), "odd?", []) => Value::Boolean(n % 2 != 0),
            (Value::Integer(n), "positive?", []) => Value::Boolean(*n > 0),
            (Value::Integer(n), "zero?", []) => Value::Boolean(*n == 0),
            (Value::String(s), "chars", []) => Value::Array(
                s.chars()
                    .map(|character| Value::String(character.to_string()))
                    .collect(),
            ),
            (Value::String(s), "downcase", []) => Value::String(s.to_lowercase()),
            (Value::String(s), "empty?", []) => Value::Boolean(s.is_empty()),
            (Value::String(s), "end_with?", [Value::String(suffix)]) => {
                Value::Boolean(s.ends_with(suffix))
            }
            (Value::String(s), "include?", [Value::String(needle)]) => {
                Value::Boolean(s.contains(needle))
            }
            (Value::String(s), "length", []) => Value::Integer(s.chars().count() as i64),
            (Value::String(s), "reverse", []) => Value::String(s.chars().rev().collect()),
            (Value::String(s), "slice", [Value::Integer(start), Value::Integer(length)]) => {
                let start = usize::try_from(*start)
                    .unwrap_or_else(|_| panic!("slice start must not be negative, got {start}"));
                let length = usize::try_from(*length)
                    .unwrap_or_else(|_| panic!("slice length must not be negative, got {length}"));
                Value::String(s.chars().skip(start).take(length).collect())
            }
            (Value::String(s), "split", [Value::String(separator)]) => Value::Array(
                s.split(separator.as_str())
                    .map(|piece| Value::String(piece.to_string()))
                    .collect(),
            ),
            (Value::String(s), "start_with?", [Value::String(prefix)]) => {
                Value::Boolean(s.starts_with(prefix))
            }
            (Value::String(s), "to_i", []) => {
                let trimmed = s.trim();
                let value = trimmed
                    .parse()
                    .unwrap_or_else(|_| panic!("to_i cannot parse {s:?} — no nil to return"));
                Value::Integer(value)
            }
            (Value::String(s), "upcase", []) => Value::String(s.to_uppercase()),
            (receiver, "to_s", []) => Value::String(receiver.to_string()),
            (receiver, name, arguments) => {
                panic!("undefined method {name} for {receiver:?} with {arguments:?}")
            }
        }
    }

    fn integers_of(elements: &[Value], method: &str) -> Vec<i64> {
        elements
            .iter()
            .map(|element| match element {
                Value::Integer(value) => *value,
                other => panic!("{method} needs an array of integers, found {other:?}"),
            })
            .collect()
    }

    /// Run a block whose result must be a strict boolean (select/reject).
    fn block_boolean(&mut self, block: &Block, element: Value, method: &str) -> bool {
        match self.run_block(block, vec![element]) {
            Some(Value::Boolean(value)) => value,
            other => panic!("{method} block must produce true or false, got {other:?}"),
        }
    }

    /// Run a block as a closure: it sees the enclosing scope; only its
    /// parameters are block-local (shadowed, then restored).
    fn run_block(&mut self, block: &Block, arguments: Vec<Value>) -> Option<Value> {
        if block.parameters.len() > arguments.len() {
            panic!(
                "block expects {} argument(s), got {}",
                block.parameters.len(),
                arguments.len()
            );
        }
        let shadowed: Vec<(String, Option<Value>)> = block
            .parameters
            .iter()
            .map(|parameter| (parameter.clone(), self.variables.get(parameter).cloned()))
            .collect();
        for (parameter, argument) in block.parameters.iter().zip(arguments) {
            self.variables.insert(parameter.clone(), argument);
        }
        let result = self.run_body(&block.body);
        if self.pending.is_some() {
            self.pending = None;
            panic!("return, break, and next inside blocks are not supported in the seed yet");
        }
        for (parameter, original) in shadowed {
            match original {
                Some(value) => self.variables.insert(parameter, value),
                None => self.variables.remove(&parameter),
            };
        }
        result
    }

    /// Evaluate an expression that must produce a value.
    fn value_of(&mut self, expression: &Expression) -> Value {
        self.expression(expression)
            .unwrap_or_else(|| panic!("{expression:?} produced no value"))
    }

    /// Evaluate an expression that must be a strict boolean.
    fn boolean_of(&mut self, expression: &Expression, context: &str) -> bool {
        match self.value_of(expression) {
            Value::Boolean(value) => value,
            other => panic!("{context} must be true or false, got {other:?}"),
        }
    }

    fn call(&mut self, name: &str, arguments: Vec<Value>) -> Option<Value> {
        if !self.methods.contains_key(name) && name == "puts" {
            // Like Ruby: bare puts() prints a blank line.
            if arguments.is_empty() {
                writeln!(self.output).expect("failed to write output");
            }
            for argument in &arguments {
                writeln!(self.output, "{argument}").expect("failed to write output");
            }
            return None;
        }
        if !self.methods.contains_key(name) && name == "p" {
            for argument in &arguments {
                let inspected = argument.inspect();
                writeln!(self.output, "{inspected}").expect("failed to write output");
            }
            // Like Ruby: p returns its argument, so it drops into any expression.
            let mut arguments = arguments;
            return match arguments.len() {
                0 => None,
                1 => Some(arguments.remove(0)),
                _ => Some(Value::Array(arguments)),
            };
        }

        // Crude IO builtins so real programs are possible before the object
        // model exists; names and shapes are placeholders, not decisions.
        if !self.methods.contains_key(name) {
            match (name, arguments.as_slice()) {
                ("argv", []) => {
                    let arguments = self
                        .arguments
                        .iter()
                        .map(|argument| Value::String(argument.clone()))
                        .collect();
                    return Some(Value::Array(arguments));
                }
                ("read_file", [Value::String(path)]) => {
                    let content = std::fs::read_to_string(path)
                        .unwrap_or_else(|error| panic!("read_file {path:?}: {error}"));
                    return Some(Value::String(content));
                }
                ("write_file", [Value::String(path), Value::String(content)]) => {
                    std::fs::write(path, content)
                        .unwrap_or_else(|error| panic!("write_file {path:?}: {error}"));
                    return None;
                }
                _ => {}
            }
        }

        let method = self
            .methods
            .get(name)
            .unwrap_or_else(|| panic!("undefined method {name}"))
            .clone();
        let required = method
            .parameters
            .iter()
            .take_while(|parameter| parameter.default.is_none())
            .count();
        let total = method.parameters.len();
        if arguments.len() < required || arguments.len() > total {
            let expected = if required == total {
                format!("{required}")
            } else {
                format!("{required} to {total}")
            };
            panic!(
                "{name} expects {expected} argument(s), got {}",
                arguments.len()
            );
        }

        // Methods get a fresh scope: parameters only, no outer locals.
        // Bind left to right so a default can reference earlier parameters.
        let mut scope: HashMap<String, Value> = HashMap::new();
        std::mem::swap(&mut self.variables, &mut scope);
        let mut supplied = arguments.into_iter();
        for parameter in &method.parameters {
            let value = match supplied.next() {
                Some(value) => value,
                None => {
                    let default = parameter.default.as_ref().unwrap();
                    self.value_of(default)
                }
            };
            self.variables.insert(parameter.name.clone(), value);
        }
        let mut result = self.run_body(&method.body);
        std::mem::swap(&mut self.variables, &mut scope);
        match self.pending.take() {
            None => {}
            Some(Pending::Return(value)) => result = value,
            Some(Pending::Break) => panic!("break outside of a loop"),
            Some(Pending::Next) => panic!("next outside of a loop"),
        }
        result
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_an_integer_literal() {
        assert_eq!(evaluate("42"), Some(Value::Integer(42)));
    }

    #[test]
    fn evaluates_integer_addition() {
        assert_eq!(evaluate("1 + 2 + 3"), Some(Value::Integer(6)));
    }

    #[test]
    fn evaluates_string_concatenation() {
        assert_eq!(
            evaluate(r#""port" + "land""#),
            Some(Value::String("portland".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "cannot apply")]
    fn panics_on_adding_a_string_to_an_integer() {
        evaluate(r#"1 + "one""#);
    }

    #[test]
    fn evaluates_compound_assignment() {
        assert_eq!(evaluate("x = 1\nx += 2\nx\n"), Some(Value::Integer(3)));
        assert_eq!(evaluate("x = 10\nx -= 3\nx\n"), Some(Value::Integer(7)));
        assert_eq!(evaluate("x = 4\nx *= 3\nx\n"), Some(Value::Integer(12)));
        assert_eq!(evaluate("x = 9\nx /= 2\nx\n"), Some(Value::Integer(4)));
        assert_eq!(evaluate("x = 9\nx %= 4\nx\n"), Some(Value::Integer(1)));
        assert_eq!(
            evaluate("s = \"port\"\ns += \"land\"\ns\n"),
            Some(Value::String("portland".to_string()))
        );
    }

    #[test]
    fn compound_assignment_takes_a_postfix_guard() {
        assert_eq!(
            evaluate("x = 1\nx += 10 if false\nx\n"),
            Some(Value::Integer(1))
        );
    }

    #[test]
    fn hash_each_yields_key_and_value() {
        let source =
            "{\"amy\" => 3, \"bo\" => 5}.each do |name, age|\n  puts(\"#{name} is #{age}\")\nend\n";
        assert_eq!(output_of(source), "amy is 3\nbo is 5\n");
    }

    #[test]
    fn evaluates_logical_operators() {
        assert_eq!(evaluate("true && false"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("true && true"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("false || true"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("false || false"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("1 < 2 && 2 < 3"), Some(Value::Boolean(true)));
        assert_eq!(
            evaluate("false && false || true"),
            Some(Value::Boolean(true))
        );
    }

    #[test]
    fn logical_operators_short_circuit() {
        // The right side would panic (undefined method) if evaluated.
        assert_eq!(evaluate("false && nope()"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("true || nope()"), Some(Value::Boolean(true)));
    }

    #[test]
    fn evaluates_not() {
        assert_eq!(evaluate("!true"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("!(1 == 2)"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("!!true"), Some(Value::Boolean(true)));
    }

    #[test]
    #[should_panic(expected = "must be true or false")]
    fn panics_on_a_non_boolean_logical_operand() {
        evaluate("1 && true");
    }

    #[test]
    #[should_panic(expected = "cannot apply")]
    fn panics_on_not_of_an_integer() {
        evaluate("!1");
    }

    #[test]
    fn postfix_if_guards_a_statement() {
        assert_eq!(output_of("puts(1) if true"), "1\n");
        assert_eq!(output_of("puts(1) if false"), "");
        assert_eq!(evaluate("x = 5 if true\nx\n"), Some(Value::Integer(5)));
    }

    #[test]
    fn postfix_unless_negates_the_guard() {
        assert_eq!(output_of("puts(1) unless false"), "1\n");
        assert_eq!(output_of("puts(1) unless true"), "");
    }

    #[test]
    fn return_with_a_postfix_guard_is_a_guard_clause() {
        let source = "def clamp(n)\n  return 0 if n < 0\n  n\nend\n";
        assert_eq!(
            evaluate(&format!("{source}clamp(-5)\n")),
            Some(Value::Integer(0))
        );
        assert_eq!(
            evaluate(&format!("{source}clamp(3)\n")),
            Some(Value::Integer(3))
        );
    }

    #[test]
    fn break_with_a_postfix_guard() {
        let source = "n = 0\nwhile true\n  n = n + 1\n  break if n == 4\nend\nn\n";
        assert_eq!(evaluate(source), Some(Value::Integer(4)));
    }

    #[test]
    fn unless_block_form_runs_on_false() {
        assert_eq!(
            evaluate("unless false\n  \"ran\"\nend\n"),
            Some(Value::String("ran".to_string()))
        );
        assert_eq!(
            evaluate("unless true\n  \"skipped\"\nelse\n  \"else ran\"\nend\n"),
            Some(Value::String("else ran".to_string()))
        );
    }

    #[test]
    fn return_exits_a_method_early() {
        let source =
            "def sign(n)\n  if n < 0\n    return \"negative\"\n  end\n  \"non-negative\"\nend\n";
        assert_eq!(
            evaluate(&format!("{source}sign(-1)\n")),
            Some(Value::String("negative".to_string()))
        );
        assert_eq!(
            evaluate(&format!("{source}sign(1)\n")),
            Some(Value::String("non-negative".to_string()))
        );
    }

    #[test]
    fn bare_return_produces_no_value() {
        let source = "def noop\n  return\n  \"unreached\"\nend\nnoop()\n";
        assert_eq!(evaluate(source), None);
    }

    #[test]
    fn return_unwinds_through_a_while_loop() {
        let source = "def find_first_multiple(of)\n  n = 1\n  while true\n    if n % of == 0\n      return n\n    end\n    n = n + 1\n  end\nend\nfind_first_multiple(7)\n";
        assert_eq!(evaluate(source), Some(Value::Integer(7)));
    }

    #[test]
    fn next_skips_to_the_following_iteration() {
        let source = "n = 0\ntotal = 0\nwhile n < 5\n  n += 1\n  next if n.even?\n  total += n\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(9)));
    }

    #[test]
    #[should_panic(expected = "next outside of a loop")]
    fn panics_on_a_top_level_next() {
        evaluate("next");
    }

    #[test]
    fn break_exits_a_while_loop() {
        let source = "n = 0\nwhile true\n  n = n + 1\n  if n == 3\n    break\n  end\nend\nn\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    #[test]
    #[should_panic(expected = "return outside of a method")]
    fn panics_on_a_top_level_return() {
        evaluate("return 1");
    }

    #[test]
    #[should_panic(expected = "break outside of a loop")]
    fn panics_on_a_top_level_break() {
        evaluate("break");
    }

    #[test]
    #[should_panic(expected = "break outside of a loop")]
    fn panics_on_a_break_in_a_method_without_a_loop() {
        evaluate("def f\n  break\nend\nf()\n");
    }

    #[test]
    #[should_panic(expected = "not supported in the seed yet")]
    fn panics_on_a_break_inside_a_block() {
        evaluate("[1, 2].each do |n|\n  break\nend\n");
    }

    #[test]
    fn each_iterates_with_a_closure_over_the_enclosing_scope() {
        let source = "total = 0\n[1, 2, 3].each do |n|\n  total = total + n\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(6)));
    }

    #[test]
    fn each_returns_its_receiver() {
        let source = "[1, 2].each do |n|\n  n\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![Value::Integer(1), Value::Integer(2)]))
        );
    }

    #[test]
    fn map_builds_a_new_array() {
        let source = "[1, 2, 3].map do |n|\n  n * n\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![
                Value::Integer(1),
                Value::Integer(4),
                Value::Integer(9),
            ]))
        );
    }

    #[test]
    fn each_with_index_yields_element_then_index() {
        let source =
            "%w[a b].each_with_index do |letter, index|\n  puts(\"#{index}: #{letter}\")\nend\n";
        assert_eq!(output_of(source), "0: a\n1: b\n");
    }

    #[test]
    fn sorts_integer_arrays() {
        assert_eq!(
            evaluate("[3, 1, 2].sort"),
            Some(Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ]))
        );
    }

    #[test]
    fn slices_strings_and_arrays() {
        assert_eq!(
            evaluate("\"portland\".slice(4, 4)"),
            Some(Value::String("land".to_string()))
        );
        assert_eq!(
            evaluate("[1, 2, 3, 4].slice(1, 2)"),
            Some(Value::Array(vec![Value::Integer(2), Value::Integer(3)]))
        );
        assert_eq!(
            evaluate("[1, 2].slice(1, 99)"),
            Some(Value::Array(vec![Value::Integer(2)]))
        );
    }

    #[test]
    fn select_keeps_matching_elements() {
        let source = "[1, 2, 3, 4].select do |n|\n  n.even?\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![Value::Integer(2), Value::Integer(4)]))
        );
    }

    #[test]
    fn reject_drops_matching_elements() {
        let source = "[1, 2, 3, 4].reject do |n|\n  n.even?\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![Value::Integer(1), Value::Integer(3)]))
        );
    }

    #[test]
    fn reduce_folds_from_an_initial_value() {
        let source = "[1, 2, 3].reduce(10) do |sum, n|\n  sum + n\nend\n";
        assert_eq!(evaluate(source), Some(Value::Integer(16)));
        let concat = "%w[rose city].reduce(\"\") do |all, word|\n  all + word\nend\n";
        assert_eq!(
            evaluate(concat),
            Some(Value::String("rosecity".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "select block must produce true or false")]
    fn panics_on_a_non_boolean_select_block() {
        evaluate("[1].select do |n|\n  n\nend\n");
    }

    #[test]
    fn converts_strings_to_integers() {
        assert_eq!(evaluate("\"42\".to_i"), Some(Value::Integer(42)));
        assert_eq!(evaluate("\" -7 \".to_i"), Some(Value::Integer(-7)));
    }

    #[test]
    #[should_panic(expected = "to_i cannot parse")]
    fn panics_on_an_unparsable_to_i() {
        evaluate("\"pdx\".to_i");
    }

    #[test]
    fn reads_a_real_fixture_file() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/haiku.txt");
        let source = format!("read_file(\"{path}\").include?(\"carpet\")");
        assert_eq!(evaluate(&source), Some(Value::Boolean(true)));
    }

    #[test]
    fn writes_then_reads_a_file() {
        let path = "/tmp/portland-seed-write-test.txt";
        let source = format!("write_file(\"{path}\", \"teal carpet\")\nread_file(\"{path}\")\n");
        assert_eq!(
            evaluate(&source),
            Some(Value::String("teal carpet".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "read_file")]
    fn panics_on_reading_a_missing_file() {
        evaluate("read_file(\"no_such_file.txt\")");
    }

    #[test]
    fn argv_is_empty_by_default() {
        assert_eq!(evaluate("argv().length"), Some(Value::Integer(0)));
    }

    #[test]
    fn times_counts_from_zero() {
        let source = "sum = 0\n3.times do |i|\n  sum = sum + i\nend\nsum\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    #[test]
    fn times_block_may_ignore_its_argument() {
        let source = "count = 0\n3.times do\n  count = count + 1\nend\ncount\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    #[test]
    fn block_parameters_are_block_local() {
        let source = "n = 100\n[1, 2].each do |n|\n  n\nend\nn\n";
        assert_eq!(evaluate(source), Some(Value::Integer(100)));
    }

    #[test]
    fn blocks_print_through_the_interpreter_output() {
        assert_eq!(
            output_of("[\"rose\", \"city\"].each do |word|\n  puts(word.upcase)\nend\n"),
            "ROSE\nCITY\n"
        );
    }

    #[test]
    #[should_panic(expected = "undefined block-taking method")]
    fn panics_on_an_unknown_block_method() {
        evaluate("\"pdx\".each do |c|\n  c\nend\n");
    }

    #[test]
    fn evaluates_hash_literals_and_lookup() {
        let source = "ages = {\"amy\" => 3, \"bo\" => 5}\nages[\"bo\"]\n";
        assert_eq!(evaluate(source), Some(Value::Integer(5)));
        assert_eq!(
            evaluate("{1 => \"one\"}[1]"),
            Some(Value::String("one".to_string()))
        );
    }

    #[test]
    fn duplicate_hash_keys_keep_position_and_last_value_wins() {
        assert_eq!(
            evaluate("{\"a\" => 1, \"a\" => 2}[\"a\"]"),
            Some(Value::Integer(2))
        );
    }

    #[test]
    #[should_panic(expected = "not found in hash")]
    fn panics_on_a_missing_hash_key() {
        evaluate("{\"a\" => 1}[\"b\"]");
    }

    #[test]
    fn calls_builtin_methods_on_hashes() {
        let hash = "{\"a\" => 1, \"b\" => 2}";
        assert_eq!(evaluate(&format!("{hash}.length")), Some(Value::Integer(2)));
        assert_eq!(evaluate("{}.empty?"), Some(Value::Boolean(true)));
        assert_eq!(
            evaluate(&format!("{hash}.key?(\"a\")")),
            Some(Value::Boolean(true))
        );
        assert_eq!(
            evaluate(&format!("{hash}.key?(\"z\")")),
            Some(Value::Boolean(false))
        );
        assert_eq!(
            evaluate(&format!("{hash}.keys")),
            Some(Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ]))
        );
        assert_eq!(
            evaluate(&format!("{hash}.values")),
            Some(Value::Array(vec![Value::Integer(1), Value::Integer(2)]))
        );
    }

    #[test]
    fn puts_prints_hashes_readably() {
        assert_eq!(
            output_of("puts({\"a\" => 1, \"b\" => 2})"),
            "{a => 1, b => 2}\n"
        );
    }

    #[test]
    fn evaluates_array_literals() {
        assert_eq!(
            evaluate("[1, 2 + 3, \"pdx\"]"),
            Some(Value::Array(vec![
                Value::Integer(1),
                Value::Integer(5),
                Value::String("pdx".to_string()),
            ]))
        );
        assert_eq!(evaluate("[]"), Some(Value::Array(vec![])));
    }

    #[test]
    fn indexes_arrays() {
        assert_eq!(evaluate("[10, 20, 30][1]"), Some(Value::Integer(20)));
        assert_eq!(evaluate("[10, 20, 30][-1]"), Some(Value::Integer(30)));
        assert_eq!(
            evaluate("cities = [\"pdx\", \"sea\"]\ncities[0]\n"),
            Some(Value::String("pdx".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn panics_on_an_out_of_range_index() {
        evaluate("[1, 2][5]");
    }

    #[test]
    fn concatenates_arrays_with_plus() {
        assert_eq!(
            evaluate("[1] + [2, 3]"),
            Some(Value::Array(vec![
                Value::Integer(1),
                Value::Integer(2),
                Value::Integer(3),
            ]))
        );
    }

    #[test]
    fn calls_builtin_methods_on_arrays() {
        assert_eq!(evaluate("[1, 2, 3].length"), Some(Value::Integer(3)));
        assert_eq!(evaluate("[].empty?"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("[7, 8].first"), Some(Value::Integer(7)));
        assert_eq!(evaluate("[7, 8].last"), Some(Value::Integer(8)));
        assert_eq!(
            evaluate("[\"a\", \"b\"].join(\"-\")"),
            Some(Value::String("a-b".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "no nil; check empty? first")]
    fn panics_on_first_of_an_empty_array() {
        evaluate("[].first");
    }

    #[test]
    fn p_prints_inspected_values_and_returns_its_argument() {
        assert_eq!(output_of("p(\"hi\")"), "\"hi\"\n");
        assert_eq!(output_of("p([1, \"two\"])"), "[1, \"two\"]\n");
        assert_eq!(evaluate("x = p(42)\nx\n"), Some(Value::Integer(42)));
        assert_eq!(evaluate("p(1) + 1"), Some(Value::Integer(2)));
    }

    #[test]
    fn puts_prints_arrays_readably() {
        assert_eq!(output_of("puts([1, 2, 3])"), "[1, 2, 3]\n");
    }

    #[test]
    fn calls_stdlib_breadth_methods() {
        assert_eq!(
            evaluate(r#""pdx".chars"#),
            Some(Value::Array(vec![
                Value::String("p".to_string()),
                Value::String("d".to_string()),
                Value::String("x".to_string()),
            ]))
        );
        assert_eq!(
            evaluate(r#""a,b".split(",")"#),
            Some(Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ]))
        );
        assert_eq!(
            evaluate(r#""portland".include?("land")"#),
            Some(Value::Boolean(true))
        );
        assert_eq!(
            evaluate(r#""portland".start_with?("port")"#),
            Some(Value::Boolean(true))
        );
        assert_eq!(
            evaluate(r#""portland".end_with?("port")"#),
            Some(Value::Boolean(false))
        );
        assert_eq!(evaluate("4.even?"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("4.odd?"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("[1, 2, 3].sum"), Some(Value::Integer(6)));
        assert_eq!(evaluate("[3, 1, 2].max"), Some(Value::Integer(3)));
        assert_eq!(evaluate("[3, 1, 2].min"), Some(Value::Integer(1)));
        assert_eq!(evaluate("[1, 2].include?(2)"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("[1, 2].include?(9)"), Some(Value::Boolean(false)));
    }

    #[test]
    fn indexes_strings_by_character() {
        assert_eq!(
            evaluate(r#""pdx"[0]"#),
            Some(Value::String("p".to_string()))
        );
        assert_eq!(
            evaluate(r#""pdx"[-1]"#),
            Some(Value::String("x".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "out of range for string")]
    fn panics_on_an_out_of_range_string_index() {
        evaluate(r#""pdx"[9]"#);
    }

    #[test]
    #[should_panic(expected = "no nil; check empty? first")]
    fn panics_on_max_of_an_empty_array() {
        evaluate("[].max");
    }

    #[test]
    fn calls_builtin_methods_on_values() {
        assert_eq!(evaluate(r#""portland".length"#), Some(Value::Integer(8)));
        assert_eq!(
            evaluate(r#""pdx".upcase"#),
            Some(Value::String("PDX".to_string()))
        );
        assert_eq!(evaluate(r#""".empty?"#), Some(Value::Boolean(true)));
        assert_eq!(evaluate("-5.abs"), Some(Value::Integer(5)));
        assert_eq!(evaluate("(1 - 1).zero?"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("42.to_s"), Some(Value::String("42".to_string())));
    }

    #[test]
    #[should_panic(expected = "duplicate parameter name")]
    fn panics_on_duplicate_method_parameters() {
        evaluate("def f(a, a)\n  a\nend\n");
    }

    #[test]
    #[should_panic(expected = "duplicate block parameter name")]
    fn panics_on_duplicate_block_parameters() {
        evaluate("[1].each do |a, a|\n  a\nend\n");
    }

    #[test]
    fn method_chains_may_continue_on_the_next_line() {
        let source = "\"pdx\"\n  .upcase\n  .reverse\n";
        assert_eq!(evaluate(source), Some(Value::String("XDP".to_string())));
    }

    #[test]
    fn chains_method_calls() {
        assert_eq!(
            evaluate(r#""pdx".upcase.reverse"#),
            Some(Value::String("XDP".to_string()))
        );
        assert_eq!(
            evaluate(r#""portland".reverse.length + 1"#),
            Some(Value::Integer(9))
        );
    }

    #[test]
    fn method_calls_work_on_variables_and_results() {
        let source = "city = \"portland\"\ncity.upcase\n";
        assert_eq!(
            evaluate(source),
            Some(Value::String("PORTLAND".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "undefined method shout")]
    fn panics_on_an_undefined_value_method() {
        evaluate(r#""pdx".shout"#);
    }

    #[test]
    fn interpolates_expressions_into_strings() {
        assert_eq!(
            evaluate(r#""1 + 1 = #{1 + 1}""#),
            Some(Value::String("1 + 1 = 2".to_string()))
        );
        assert_eq!(
            evaluate("name = \"world\"\n\"hello #{name}!\"\n"),
            Some(Value::String("hello world!".to_string()))
        );
        assert_eq!(
            evaluate(r##""#{1}, #{2}, and #{1 + 2}""##),
            Some(Value::String("1, 2, and 3".to_string()))
        );
        assert_eq!(
            evaluate(r##""#{"pdx".upcase} rules""##),
            Some(Value::String("PDX rules".to_string()))
        );
    }

    #[test]
    fn escaped_interpolation_stays_literal() {
        assert_eq!(
            evaluate(r#""\#{not run}""#),
            Some(Value::String("#{not run}".to_string()))
        );
    }

    #[test]
    fn interpolation_handles_nested_braces() {
        assert_eq!(
            evaluate(r#""age: #{ {"amy" => 3}["amy"] }""#),
            Some(Value::String("age: 3".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "unterminated string")]
    fn panics_on_an_unterminated_interpolation() {
        evaluate(r##""#{1 + 1""##);
    }

    #[test]
    fn evaluates_word_arrays() {
        assert_eq!(
            evaluate("%w[rose city]"),
            Some(Value::Array(vec![
                Value::String("rose".to_string()),
                Value::String("city".to_string()),
            ]))
        );
        assert_eq!(evaluate("%w[]"), Some(Value::Array(vec![])));
        assert_eq!(
            evaluate("%w[a b c].length + 1 % 2"),
            Some(Value::Integer(4))
        );
    }

    #[test]
    fn repeats_strings_and_arrays_with_star() {
        assert_eq!(
            evaluate("\"ab\" * 3"),
            Some(Value::String("ababab".to_string()))
        );
        assert_eq!(
            evaluate("[0] * 3"),
            Some(Value::Array(vec![
                Value::Integer(0),
                Value::Integer(0),
                Value::Integer(0),
            ]))
        );
    }

    #[test]
    #[should_panic(expected = "cannot repeat")]
    fn panics_on_repeating_a_string_a_negative_number_of_times() {
        evaluate("\"ab\" * -1");
    }

    #[test]
    fn single_quoted_strings_are_literal() {
        assert_eq!(
            evaluate(r##"'no #{interpolation} or \n escapes'"##),
            Some(Value::String(
                "no #{interpolation} or \\n escapes".to_string()
            ))
        );
        assert_eq!(
            evaluate(r"'it\'s escaped, and so is \\'"),
            Some(Value::String("it's escaped, and so is \\".to_string()))
        );
    }

    #[test]
    fn decodes_string_escapes() {
        assert_eq!(
            evaluate(r#""line1\nline2""#),
            Some(Value::String("line1\nline2".to_string()))
        );
        assert_eq!(
            evaluate(r#""say \"hi\"\t\\ done""#),
            Some(Value::String("say \"hi\"\t\\ done".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "unknown escape sequence")]
    fn panics_on_an_unknown_escape() {
        evaluate(r#""\q""#);
    }

    #[test]
    fn evaluates_unary_minus() {
        assert_eq!(evaluate("-5"), Some(Value::Integer(-5)));
        assert_eq!(evaluate("-(1 + 2)"), Some(Value::Integer(-3)));
        assert_eq!(evaluate("10 + -5"), Some(Value::Integer(5)));
        assert_eq!(evaluate("--5"), Some(Value::Integer(5)));
        assert_eq!(evaluate("-2 * 3"), Some(Value::Integer(-6)));
    }

    #[test]
    #[should_panic(expected = "cannot apply")]
    fn panics_on_negating_a_string() {
        evaluate(r#"-"pdx""#);
    }

    #[test]
    fn evaluates_boolean_literals() {
        assert_eq!(evaluate("true"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("false"), Some(Value::Boolean(false)));
    }

    #[test]
    fn evaluates_integer_comparisons() {
        assert_eq!(evaluate("1 < 2"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("1 >= 2"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("1 + 1 == 2"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("3 * 3 <= 8"), Some(Value::Boolean(false)));
    }

    #[test]
    fn evaluates_equality_across_types() {
        assert_eq!(evaluate(r#""pdx" == "pdx""#), Some(Value::Boolean(true)));
        assert_eq!(evaluate(r#"1 == "1""#), Some(Value::Boolean(false)));
        assert_eq!(evaluate(r#"1 != "1""#), Some(Value::Boolean(true)));
    }

    #[test]
    #[should_panic(expected = "cannot apply")]
    fn panics_on_ordering_strings() {
        evaluate(r#""a" < "b""#);
    }

    #[test]
    fn evaluates_case_with_aligned_thens() {
        let source = "def size(n)\n  case n\n  when 0 then \"none\"\n  when 1, 2 then \"few\"\n  else\n    \"many\"\n  end\nend\n";
        assert_eq!(
            evaluate(&format!("{source}size(0)\n")),
            Some(Value::String("none".to_string()))
        );
        assert_eq!(
            evaluate(&format!("{source}size(2)\n")),
            Some(Value::String("few".to_string()))
        );
        assert_eq!(
            evaluate(&format!("{source}size(9)\n")),
            Some(Value::String("many".to_string()))
        );
    }

    #[test]
    fn evaluates_case_with_multiline_branches() {
        let source =
            "label = case \"b\"\nwhen \"a\"\n  \"first\"\nwhen \"b\"\n  \"second\"\nend\nlabel\n";
        assert_eq!(evaluate(source), Some(Value::String("second".to_string())));
    }

    #[test]
    fn case_without_a_match_or_else_produces_nothing() {
        assert_eq!(evaluate("case 9\nwhen 1 then 100\nend\n"), None);
    }

    #[test]
    #[should_panic(expected = "case needs at least one when")]
    fn panics_on_a_case_without_when() {
        evaluate("case 1\nelse\n  2\nend\n");
    }

    #[test]
    fn evaluates_the_then_branch() {
        let source = "if 1 < 2\n  \"yes\"\nelse\n  \"no\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("yes".to_string())));
    }

    #[test]
    fn evaluates_the_else_branch() {
        let source = "if 1 > 2\n  \"yes\"\nelse\n  \"no\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("no".to_string())));
    }

    #[test]
    fn if_without_else_produces_nothing_when_false() {
        assert_eq!(evaluate("if false\n  1\nend\n"), None);
    }

    #[test]
    fn if_is_an_expression() {
        let source = "label = if 2 > 1\n  \"big\"\nelse\n  \"small\"\nend\nlabel\n";
        assert_eq!(evaluate(source), Some(Value::String("big".to_string())));
    }

    #[test]
    fn evaluates_elsif_chains() {
        let source = "def describe(n)\n  if n < 0\n    \"negative\"\n  elsif n == 0\n    \"zero\"\n  elsif n < 10\n    \"small\"\n  else\n    \"big\"\n  end\nend\n";
        assert_eq!(
            evaluate(&format!("{source}describe(-1)\n")),
            Some(Value::String("negative".to_string()))
        );
        assert_eq!(
            evaluate(&format!("{source}describe(0)\n")),
            Some(Value::String("zero".to_string()))
        );
        assert_eq!(
            evaluate(&format!("{source}describe(7)\n")),
            Some(Value::String("small".to_string()))
        );
        assert_eq!(
            evaluate(&format!("{source}describe(42)\n")),
            Some(Value::String("big".to_string()))
        );
    }

    #[test]
    fn if_works_inside_methods() {
        let source = "def sign(n)\n  if n < 0\n    \"negative\"\n  else\n    \"non-negative\"\n  end\nend\nsign(0 - 5)\n";
        assert_eq!(
            evaluate(source),
            Some(Value::String("negative".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "must be true or false")]
    fn panics_on_a_non_boolean_condition() {
        evaluate("if 1\n  2\nend\n");
    }

    #[test]
    fn while_loops_until_the_condition_is_false() {
        let source = "n = 3\nwhile n > 0\n  puts(n)\n  n = n - 1\nend\n";
        assert_eq!(output_of(source), "3\n2\n1\n");
    }

    #[test]
    fn while_computes_a_factorial() {
        let source = "def factorial(n)\n  result = 1\n  while n > 1\n    result = result * n\n    n = n - 1\n  end\n  result\nend\nfactorial(10)\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3_628_800)));
    }

    #[test]
    fn while_with_a_false_condition_never_runs() {
        assert_eq!(output_of("while false\n  puts(1)\nend\n"), "");
    }

    #[test]
    #[should_panic(expected = "while condition must be true or false")]
    fn panics_on_a_non_boolean_while_condition() {
        evaluate("while 1\n  2\nend\n");
    }

    #[test]
    fn evaluates_subtraction_left_to_right() {
        assert_eq!(evaluate("10 - 2 - 3"), Some(Value::Integer(5)));
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        assert_eq!(evaluate("2 + 3 * 4"), Some(Value::Integer(14)));
        assert_eq!(evaluate("(2 + 3) * 4"), Some(Value::Integer(20)));
    }

    #[test]
    fn evaluates_division() {
        assert_eq!(evaluate("42 / 6"), Some(Value::Integer(7)));
        assert_eq!(evaluate("7 / 2"), Some(Value::Integer(3)));
    }

    #[test]
    #[should_panic(expected = "divide by zero")]
    fn panics_on_dividing_by_zero() {
        evaluate("1 / 0");
    }

    #[test]
    fn evaluates_modulo() {
        assert_eq!(evaluate("10 % 3"), Some(Value::Integer(1)));
        assert_eq!(evaluate("15 % 5 == 0"), Some(Value::Boolean(true)));
    }

    #[test]
    #[should_panic(expected = "cannot apply")]
    fn panics_on_multiplying_strings() {
        evaluate(r#""ab" * "cd""#);
    }

    #[test]
    fn evaluates_an_empty_program_to_nothing() {
        assert_eq!(evaluate(""), None);
    }

    #[test]
    fn evaluates_assignment_and_variable_reference() {
        assert_eq!(
            evaluate("total = 1 + 2\ntotal + 10\n"),
            Some(Value::Integer(13))
        );
    }

    #[test]
    fn assignment_evaluates_to_its_value() {
        assert_eq!(
            evaluate("greeting = \"hi\""),
            Some(Value::String("hi".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "undefined variable")]
    fn panics_on_an_undefined_variable() {
        evaluate("nope");
    }

    #[test]
    fn defines_and_calls_a_method() {
        let source = "def greet(name)\n  \"hello \" + name\nend\ngreet(\"world\")\n";
        assert_eq!(
            evaluate(source),
            Some(Value::String("hello world".to_string()))
        );
    }

    #[test]
    fn calls_a_method_with_no_parameters() {
        let source = "def pdx\n  \"portland\"\nend\npdx()\n";
        assert_eq!(
            evaluate(source),
            Some(Value::String("portland".to_string()))
        );
    }

    #[test]
    fn method_bodies_return_their_last_expression() {
        let source = "def compute\n  1\n  2 + 3\nend\ncompute()\n";
        assert_eq!(evaluate(source), Some(Value::Integer(5)));
    }

    #[test]
    fn methods_can_call_other_methods() {
        let source = "def inner\n  21\nend\ndef outer\n  inner() + inner()\nend\nouter()\n";
        assert_eq!(evaluate(source), Some(Value::Integer(42)));
    }

    #[test]
    #[should_panic(expected = "expects 1 argument")]
    fn panics_on_an_arity_mismatch() {
        evaluate("def greet(name)\n  name\nend\ngreet()\n");
    }

    #[test]
    fn default_parameter_values_fill_missing_arguments() {
        let source = "def greet(name = \"world\")\n  \"hello #{name}\"\nend\n";
        assert_eq!(
            evaluate(&format!("{source}greet()\n")),
            Some(Value::String("hello world".to_string()))
        );
        assert_eq!(
            evaluate(&format!("{source}greet(\"pdx\")\n")),
            Some(Value::String("hello pdx".to_string()))
        );
    }

    #[test]
    fn defaults_can_reference_earlier_parameters() {
        let source = "def pair(a, b = a * 2)\n  [a, b]\nend\npair(3)\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![Value::Integer(3), Value::Integer(6)]))
        );
    }

    #[test]
    #[should_panic(expected = "expects 1 to 2 argument(s)")]
    fn panics_when_over_the_optional_arity() {
        evaluate("def f(a, b = 1)\n  a\nend\nf(1, 2, 3)\n");
    }

    #[test]
    #[should_panic(expected = "cannot follow a parameter with a default")]
    fn panics_on_a_required_parameter_after_a_default() {
        evaluate("def f(a = 1, b)\n  b\nend\n");
    }

    #[test]
    #[should_panic(expected = "undefined variable")]
    fn method_bodies_cannot_see_outer_locals() {
        evaluate("x = 1\ndef f\n  x\nend\nf()\n");
    }

    #[test]
    #[should_panic(expected = "undefined method")]
    fn panics_on_an_undefined_method() {
        evaluate("nope()");
    }

    /// Run a source and capture what it printed.
    fn output_of(source: &str) -> String {
        let program = parser::parse(source);
        let mut interpreter = Interpreter::with_output(Vec::new());
        interpreter.program(&program);
        String::from_utf8(interpreter.output).unwrap()
    }

    #[test]
    fn puts_prints_a_line() {
        assert_eq!(output_of("puts(\"hello portland\")"), "hello portland\n");
    }

    #[test]
    fn bare_puts_prints_a_blank_line() {
        assert_eq!(output_of("puts()"), "\n");
    }

    #[test]
    fn puts_prints_each_argument_on_its_own_line() {
        assert_eq!(output_of("puts(1, 2 + 3)"), "1\n5\n");
    }

    #[test]
    fn puts_works_inside_methods() {
        let source = "def shout(word)\n  puts(word + \"!\")\nend\nshout(\"pdx\")\n";
        assert_eq!(output_of(source), "pdx!\n");
    }

    #[test]
    #[should_panic(expected = "produced no value")]
    fn puts_produces_no_value() {
        evaluate("1 + puts(\"x\")");
    }
}
