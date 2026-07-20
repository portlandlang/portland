//! A tree-walking interpreter — the reference semantics for the Stage 0
//! subset before any codegen exists. Crude in the seed: type errors panic.

use std::collections::HashMap;

use crate::ast::{
    BinaryOperator, Block, Expression, LogicalOperator, Parameter, Program, Statement,
    UnaryOperator,
};
use crate::parser;
use crate::value::Value;

/// Parse and evaluate a source string, returning the last statement'word value.
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

/// Deep enough for real programs, shallow enough to fail as a clean Portland
/// error before the Rust stack runs out (overflow hangs rather than crashes
/// on macOS 26).
const MAXIMUM_CALL_DEPTH: usize = 10_000;
const MAXIMUM_EXPRESSION_DEPTH: usize = 100_000;

pub struct Interpreter<W: std::io::Write = std::io::Stdout> {
    arguments: Vec<String>,
    call_depth: usize,
    expression_depth: usize,
    methods: HashMap<String, Method>,
    output: W,
    pending: Option<Pending>,
    structs: HashMap<String, Vec<String>>,
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
            call_depth: 0,
            expression_depth: 0,
            methods: HashMap::new(),
            output,
            pending: None,
            structs: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    /// Command-line arguments exposed to the program via `argv()`.
    pub fn set_arguments(&mut self, arguments: Vec<String>) {
        self.arguments = arguments;
    }

    pub fn program(&mut self, program: &Program) -> Option<Value> {
        // A caught panic (REPL) unwinds past the decrements; start fresh.
        self.call_depth = 0;
        self.expression_depth = 0;
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
            Statement::StructDefinition { fields, name } => {
                self.structs.insert(name.clone(), fields.clone());
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
        self.expression_depth += 1;
        if self.expression_depth > MAXIMUM_EXPRESSION_DEPTH {
            panic!("expression evaluation deeper than {MAXIMUM_EXPRESSION_DEPTH} levels");
        }
        let result = self.expression_inner(expression);
        self.expression_depth -= 1;
        result
    }

    fn expression_inner(&mut self, expression: &Expression) -> Option<Value> {
        match expression {
            Expression::ArrayLiteral(elements) => Some(Value::Array(
                elements
                    .iter()
                    .map(|element| self.value_of(element))
                    .collect(),
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
                    (Value::Integer(left), BinaryOperator::Add, Value::Integer(right)) => {
                        Some(Value::Integer(left + right))
                    }
                    (Value::Integer(left), BinaryOperator::Divide, Value::Integer(right)) => {
                        // Truncates toward zero (Rust semantics); Ruby floors.
                        // Revisit when Portland'word integer semantics are specified.
                        Some(Value::Integer(left / right))
                    }
                    (Value::Integer(left), BinaryOperator::Modulo, Value::Integer(right)) => {
                        // Truncated remainder (Rust semantics); Ruby'word % is floored.
                        // Revisit when Portland'word integer semantics are specified.
                        Some(Value::Integer(left % right))
                    }
                    (Value::Integer(left), BinaryOperator::Multiply, Value::Integer(right)) => {
                        Some(Value::Integer(left * right))
                    }
                    (Value::Integer(left), BinaryOperator::Subtract, Value::Integer(right)) => {
                        Some(Value::Integer(left - right))
                    }
                    (Value::String(left), BinaryOperator::Add, Value::String(right)) => {
                        Some(Value::String(left + &right))
                    }
                    (Value::Array(left), BinaryOperator::Add, Value::Array(right)) => {
                        let mut combined = left;
                        combined.extend(right);
                        Some(Value::Array(combined))
                    }
                    (Value::String(text), BinaryOperator::Multiply, Value::Integer(count)) => {
                        let count = usize::try_from(count)
                            .unwrap_or_else(|_| panic!("cannot repeat a string {count} times"));
                        Some(Value::String(text.repeat(count)))
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
                    (Value::Integer(left), BinaryOperator::Greater, Value::Integer(right)) => {
                        Some(Value::Boolean(left > right))
                    }
                    (
                        Value::Integer(left),
                        BinaryOperator::GreaterOrEqual,
                        Value::Integer(right),
                    ) => Some(Value::Boolean(left >= right)),
                    (Value::Integer(left), BinaryOperator::Less, Value::Integer(right)) => {
                        Some(Value::Boolean(left < right))
                    }
                    (Value::Integer(left), BinaryOperator::LessOrEqual, Value::Integer(right)) => {
                        Some(Value::Boolean(left <= right))
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
                keyword_arguments,
                name,
                receiver,
            } => {
                // `Name.new(...)` constructs a struct; the name is not a value.
                if name == "new" {
                    return Some(self.construct_struct(receiver, arguments, keyword_arguments));
                }
                let receiver = self.value_of(receiver);
                let arguments: Vec<Value> = arguments
                    .iter()
                    .map(|argument| self.value_of(argument))
                    .collect();
                let keyword_arguments: Vec<(String, Value)> = keyword_arguments
                    .iter()
                    .map(|(label, expression)| (label.clone(), self.value_of(expression)))
                    .collect();
                Some(self.method_call(receiver, name, arguments, keyword_arguments, block.as_ref()))
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
                let arguments: Vec<Value> = arguments
                    .iter()
                    .map(|argument| self.value_of(argument))
                    .collect();
                self.call(name, arguments)
            }
        }
    }

    /// `Name.new(field: value, ...)` — validate against the definition and
    /// build the value with fields in definition order.
    fn construct_struct(
        &mut self,
        receiver: &Expression,
        arguments: &[Expression],
        keyword_arguments: &[(String, Expression)],
    ) -> Value {
        let Expression::Variable(struct_name) = receiver else {
            panic!("new needs a struct name receiver, got {receiver:?}")
        };
        let fields = self
            .structs
            .get(struct_name)
            .unwrap_or_else(|| panic!("undefined struct {struct_name}"))
            .clone();
        if !arguments.is_empty() {
            panic!("{struct_name}.new takes keyword arguments, not positional ones");
        }
        let provided: Vec<(String, Value)> = keyword_arguments
            .iter()
            .map(|(label, expression)| (label.clone(), self.value_of(expression)))
            .collect();
        for (label, _) in &provided {
            if !fields.contains(label) {
                panic!("{struct_name} has no field {label}");
            }
        }
        let ordered: Vec<(String, Value)> = fields
            .iter()
            .map(|field| {
                let value = provided
                    .iter()
                    .find(|(label, _)| label == field)
                    .unwrap_or_else(|| panic!("{struct_name}.new is missing field {field}"))
                    .1
                    .clone();
                (field.clone(), value)
            })
            .collect();
        Value::Struct {
            fields: ordered,
            name: struct_name.clone(),
        }
    }

    /// Built-in methods on values — read-only on purpose; mutation is a
    /// language-design decision the seed doesn't get to make.
    fn method_call(
        &mut self,
        receiver: Value,
        name: &str,
        arguments: Vec<Value>,
        keyword_arguments: Vec<(String, Value)>,
        block: Option<&Block>,
    ) -> Value {
        // `with` builds an updated copy of an immutable struct.
        if name == "with" {
            let Value::Struct { fields, name } = receiver else {
                panic!("with is only for structs, got {receiver:?}")
            };
            if !arguments.is_empty() {
                panic!("{name}.with takes keyword arguments, not positional ones");
            }
            for (label, _) in &keyword_arguments {
                if !fields.iter().any(|(field, _)| field == label) {
                    panic!("{name} has no field {label}");
                }
            }
            let updated = fields
                .into_iter()
                .map(|(field, value)| {
                    let value = keyword_arguments
                        .iter()
                        .find(|(label, _)| *label == field)
                        .map(|(_, new_value)| new_value.clone())
                        .unwrap_or(value);
                    (field, value)
                })
                .collect();
            return Value::Struct {
                fields: updated,
                name,
            };
        }
        if !keyword_arguments.is_empty() {
            panic!("keyword arguments are only for struct new and with so far");
        }
        // Struct field access reads like a method call: token.kind
        if let Value::Struct {
            fields,
            name: struct_name,
        } = &receiver
            && arguments.is_empty()
            && block.is_none()
        {
            if let Some((_, value)) = fields.iter().find(|(field, _)| field == name) {
                return value.clone();
            }
            if name != "to_s" {
                panic!("{struct_name} has no field {name}");
            }
        }
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
                (Value::Integer(from), "downto", [Value::Integer(to)]) => {
                    let mut current = *from;
                    while current >= *to {
                        self.run_block(block, vec![Value::Integer(current)]);
                        current -= 1;
                    }
                    receiver
                }
                (Value::Integer(from), "upto", [Value::Integer(to)]) => {
                    for current in *from..=*to {
                        self.run_block(block, vec![Value::Integer(current)]);
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
            (Value::Integer(number), "abs", []) => Value::Integer(number.abs()),
            (Value::Integer(number), "even?", []) => Value::Boolean(number % 2 == 0),
            (Value::Integer(number), "negative?", []) => Value::Boolean(*number < 0),
            (Value::Integer(number), "odd?", []) => Value::Boolean(number % 2 != 0),
            (Value::Integer(number), "positive?", []) => Value::Boolean(*number > 0),
            (Value::Integer(number), "zero?", []) => Value::Boolean(*number == 0),
            (Value::String(text), "chars", []) => Value::Array(
                text.chars()
                    .map(|character| Value::String(character.to_string()))
                    .collect(),
            ),
            (Value::String(text), "downcase", []) => Value::String(text.to_lowercase()),
            (Value::String(text), "empty?", []) => Value::Boolean(text.is_empty()),
            (Value::String(text), "end_with?", [Value::String(suffix)]) => {
                Value::Boolean(text.ends_with(suffix))
            }
            (Value::String(text), "include?", [Value::String(needle)]) => {
                Value::Boolean(text.contains(needle))
            }
            (Value::String(text), "length", []) => Value::Integer(text.chars().count() as i64),
            (Value::String(text), "reverse", []) => Value::String(text.chars().rev().collect()),
            (Value::String(text), "slice", [Value::Integer(start), Value::Integer(length)]) => {
                let start = usize::try_from(*start)
                    .unwrap_or_else(|_| panic!("slice start must not be negative, got {start}"));
                let length = usize::try_from(*length)
                    .unwrap_or_else(|_| panic!("slice length must not be negative, got {length}"));
                Value::String(text.chars().skip(start).take(length).collect())
            }
            (Value::String(text), "split", [Value::String(separator)]) => Value::Array(
                text.split(separator.as_str())
                    .map(|piece| Value::String(piece.to_string()))
                    .collect(),
            ),
            (Value::String(text), "start_with?", [Value::String(prefix)]) => {
                Value::Boolean(text.starts_with(prefix))
            }
            (Value::String(text), "to_i", []) => {
                let trimmed = text.trim();
                let value = trimmed
                    .parse()
                    .unwrap_or_else(|_| panic!("to_i cannot parse {text:?} — no nil to return"));
                Value::Integer(value)
            }
            (Value::String(text), "upcase", []) => Value::String(text.to_uppercase()),
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
                "block expects {} argument(word), got {}",
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
                "{name} expects {expected} argument(word), got {}",
                arguments.len()
            );
        }

        self.call_depth += 1;
        if self.call_depth > MAXIMUM_CALL_DEPTH {
            panic!("call stack deeper than {MAXIMUM_CALL_DEPTH} frames (infinite recursion?)");
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
        self.call_depth -= 1;
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
        assert_eq!(
            evaluate("value = 1\nvalue += 2\nvalue\n"),
            Some(Value::Integer(3))
        );
        assert_eq!(
            evaluate("value = 10\nvalue -= 3\nvalue\n"),
            Some(Value::Integer(7))
        );
        assert_eq!(
            evaluate("value = 4\nvalue *= 3\nvalue\n"),
            Some(Value::Integer(12))
        );
        assert_eq!(
            evaluate("value = 9\nvalue /= 2\nvalue\n"),
            Some(Value::Integer(4))
        );
        assert_eq!(
            evaluate("value = 9\nvalue %= 4\nvalue\n"),
            Some(Value::Integer(1))
        );
        assert_eq!(
            evaluate("word = \"port\"\nword += \"land\"\nword\n"),
            Some(Value::String("portland".to_string()))
        );
    }

    #[test]
    fn compound_assignment_takes_a_postfix_guard() {
        assert_eq!(
            evaluate("value = 1\nvalue += 10 if false\nvalue\n"),
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
        assert_eq!(
            evaluate("value = 5 if true\nvalue\n"),
            Some(Value::Integer(5))
        );
    }

    #[test]
    fn postfix_unless_negates_the_guard() {
        assert_eq!(output_of("puts(1) unless false"), "1\n");
        assert_eq!(output_of("puts(1) unless true"), "");
    }

    #[test]
    fn return_with_a_postfix_guard_is_a_guard_clause() {
        let source = "def clamp(number)\n  return 0 if number < 0\n  number\nend\n";
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
        let source =
            "number = 0\nwhile true\n  number = number + 1\n  break if number == 4\nend\nnumber\n";
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
        let source = "def sign(number)\n  if number < 0\n    return \"negative\"\n  end\n  \"non-negative\"\nend\n";
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
        let source = "def find_first_multiple(of)\n  number = 1\n  while true\n    if number % of == 0\n      return number\n    end\n    number = number + 1\n  end\nend\nfind_first_multiple(7)\n";
        assert_eq!(evaluate(source), Some(Value::Integer(7)));
    }

    #[test]
    fn next_skips_to_the_following_iteration() {
        let source = "number = 0\ntotal = 0\nwhile number < 5\n  number += 1\n  next if number.even?\n  total += number\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(9)));
    }

    #[test]
    #[should_panic(expected = "next outside of a loop")]
    fn panics_on_a_top_level_next() {
        evaluate("next");
    }

    #[test]
    fn break_exits_a_while_loop() {
        let source = "number = 0\nwhile true\n  number = number + 1\n  if number == 3\n    break\n  end\nend\nnumber\n";
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
        evaluate("[1, 2].each do |number|\n  break\nend\n");
    }

    #[test]
    fn each_iterates_with_a_closure_over_the_enclosing_scope() {
        let source =
            "total = 0\n[1, 2, 3].each do |number|\n  total = total + number\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(6)));
    }

    #[test]
    fn each_returns_its_receiver() {
        let source = "[1, 2].each do |number|\n  number\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![Value::Integer(1), Value::Integer(2)]))
        );
    }

    #[test]
    fn map_builds_a_new_array() {
        let source = "[1, 2, 3].map do |number|\n  number * number\nend\n";
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
        let source = "[1, 2, 3, 4].select do |number|\n  number.even?\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![Value::Integer(2), Value::Integer(4)]))
        );
    }

    #[test]
    fn reject_drops_matching_elements() {
        let source = "[1, 2, 3, 4].reject do |number|\n  number.even?\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![Value::Integer(1), Value::Integer(3)]))
        );
    }

    #[test]
    fn reduce_folds_from_an_initial_value() {
        let source = "[1, 2, 3].reduce(10) do |sum, number|\n  sum + number\nend\n";
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
        evaluate("[1].select do |number|\n  number\nend\n");
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
    fn upto_and_downto_iterate_inclusively() {
        assert_eq!(
            output_of("1.upto(3) do |number|\n  puts(number)\nend\n"),
            "1\n2\n3\n"
        );
        assert_eq!(
            output_of("3.downto(1) do |number|\n  puts(number)\nend\n"),
            "3\n2\n1\n"
        );
        assert_eq!(
            output_of("1.upto(0) do |number|\n  puts(number)\nend\n"),
            ""
        );
    }

    #[test]
    fn times_counts_from_zero() {
        let source = "sum = 0\n3.times do |index|\n  sum = sum + index\nend\nsum\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    #[test]
    fn times_block_may_ignore_its_argument() {
        let source = "count = 0\n3.times do\n  count = count + 1\nend\ncount\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    #[test]
    fn block_parameters_are_block_local() {
        let source = "number = 100\n[1, 2].each do |number|\n  number\nend\nnumber\n";
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
        evaluate("\"pdx\".each do |character|\n  character\nend\n");
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
        assert_eq!(evaluate("value = p(42)\nvalue\n"), Some(Value::Integer(42)));
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

    const TOKEN_STRUCT: &str = "struct Token\n  kind\n  text\nend\n";

    #[test]
    fn constructs_a_struct_and_reads_its_fields() {
        let source = format!(
            "{TOKEN_STRUCT}token = Token.new(kind: \"integer\", text: \"42\")\ntoken.kind + \" \" + token.text\n"
        );
        assert_eq!(
            evaluate(&source),
            Some(Value::String("integer 42".to_string()))
        );
    }

    #[test]
    fn struct_keyword_arguments_are_order_independent() {
        let source = format!(
            "{TOKEN_STRUCT}Token.new(text: \"42\", kind: \"integer\") == Token.new(kind: \"integer\", text: \"42\")\n"
        );
        assert_eq!(evaluate(&source), Some(Value::Boolean(true)));
    }

    #[test]
    fn structs_compare_by_value() {
        let source = format!(
            "{TOKEN_STRUCT}Token.new(kind: \"a\", text: \"b\") == Token.new(kind: \"a\", text: \"c\")\n"
        );
        assert_eq!(evaluate(&source), Some(Value::Boolean(false)));
    }

    #[test]
    fn with_builds_an_updated_copy_without_mutating() {
        let source = format!(
            "{TOKEN_STRUCT}a = Token.new(kind: \"integer\", text: \"42\")\nb = a.with(text: \"43\")\na.text + b.text\n"
        );
        assert_eq!(evaluate(&source), Some(Value::String("4243".to_string())));
    }

    #[test]
    fn structs_render_readably() {
        let source = format!("{TOKEN_STRUCT}puts(Token.new(kind: \"integer\", text: \"42\"))\n");
        assert_eq!(
            output_of(&source),
            "Token(kind: \"integer\", text: \"42\")\n"
        );
    }

    #[test]
    fn structs_interpolate_via_to_s() {
        let source =
            format!("{TOKEN_STRUCT}t = Token.new(kind: \"plus\", text: \"+\")\n\"saw #{{t}}\"\n");
        assert_eq!(
            evaluate(&source),
            Some(Value::String(
                "saw Token(kind: \"plus\", text: \"+\")".to_string()
            ))
        );
    }

    #[test]
    #[should_panic(expected = "missing field text")]
    fn panics_on_a_missing_struct_field() {
        evaluate("struct Token\n  kind\n  text\nend\nToken.new(kind: \"value\")\n");
    }

    #[test]
    #[should_panic(expected = "has no field speed")]
    fn panics_on_an_unknown_struct_field() {
        evaluate("struct Token\n  kind\nend\nToken.new(kind: \"value\", speed: 9)\n");
    }

    #[test]
    #[should_panic(expected = "undefined struct")]
    fn panics_on_constructing_an_undefined_struct() {
        evaluate("Nope.new(kind: 1)");
    }

    #[test]
    #[should_panic(expected = "keyword arguments, not positional")]
    fn panics_on_positional_struct_construction() {
        evaluate("struct Token\n  kind\nend\nToken.new(\"integer\")\n");
    }

    #[test]
    #[should_panic(expected = "start with a capital letter")]
    fn panics_on_a_lowercase_struct_name() {
        evaluate("struct token\n  kind\nend\n");
    }

    #[test]
    #[should_panic(expected = "has no field speed")]
    fn panics_on_with_of_an_unknown_field() {
        evaluate("struct Token\n  kind\nend\nToken.new(kind: \"value\").with(speed: 9)\n");
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
        evaluate("def f(same, same)\n  same\nend\n");
    }

    #[test]
    #[should_panic(expected = "duplicate block parameter name")]
    fn panics_on_duplicate_block_parameters() {
        evaluate("[1].each do |same, same|\n  same\nend\n");
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
            evaluate(r"'it\'word escaped, and so is \\'"),
            Some(Value::String("it'word escaped, and so is \\".to_string()))
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
        let source = "def size(number)\n  case number\n  when 0 then \"none\"\n  when 1, 2 then \"few\"\n  else\n    \"many\"\n  end\nend\n";
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
        let source = "def describe(number)\n  if number < 0\n    \"negative\"\n  elsif number == 0\n    \"zero\"\n  elsif number < 10\n    \"small\"\n  else\n    \"big\"\n  end\nend\n";
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
        let source = "def sign(number)\n  if number < 0\n    \"negative\"\n  else\n    \"non-negative\"\n  end\nend\nsign(0 - 5)\n";
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
        let source = "number = 3\nwhile number > 0\n  puts(number)\n  number = number - 1\nend\n";
        assert_eq!(output_of(source), "3\n2\n1\n");
    }

    #[test]
    fn while_computes_a_factorial() {
        let source = "def factorial(number)\n  result = 1\n  while number > 1\n    result = result * number\n    number = number - 1\n  end\n  result\nend\nfactorial(10)\n";
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
        let source = "def pair(base, twice = base * 2)\n  [base, twice]\nend\npair(3)\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(vec![Value::Integer(3), Value::Integer(6)]))
        );
    }

    #[test]
    #[should_panic(expected = "expects 1 to 2 argument(word)")]
    fn panics_when_over_the_optional_arity() {
        evaluate("def f(required, extra = 1)\n  required\nend\nf(1, 2, 3)\n");
    }

    #[test]
    #[should_panic(expected = "cannot follow a parameter with a default")]
    fn panics_on_a_required_parameter_after_a_default() {
        evaluate("def f(a = 1, b)\n  b\nend\n");
    }

    #[test]
    #[should_panic(expected = "undefined variable")]
    fn method_bodies_cannot_see_outer_locals() {
        evaluate("value = 1\ndef f\n  value\nend\nf()\n");
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
        evaluate("1 + puts(\"value\")");
    }
}
