//! A tree-walking interpreter — the reference semantics for the Stage 0
//! subset before any codegen exists. Crude in the seed: type errors panic.

use std::collections::HashMap;

use crate::ast::{
    BinaryOperator, Block, Expression, GuardAction, LogicalOperator, Parameter, Pattern, Program,
    Statement, UnaryOperator,
};
use crate::parser;
use crate::value::Value;

/// Parse and evaluate a source string, returning the last statement's value.
pub fn evaluate(source: &str) -> Option<Value> {
    let program = parser::parse(source);
    let mut interpreter = Interpreter::new();
    interpreter.program(&program)
}

/// The integers a range covers. Endless and beginless ranges have no
/// element list, so asking for one is an error rather than a hang.
fn range_elements(range: &Value) -> Vec<Value> {
    let Value::Range {
        end,
        exclusive,
        start,
    } = range
    else {
        unreachable!("called with a range")
    };
    let (Some(start), Some(end)) = (start, end) else {
        panic!("a range without both ends has no elements to walk — give it a start and an end");
    };
    let last = if *exclusive { end - 1 } else { *end };
    (*start..=last).map(Value::Integer).collect()
}

/// Resolve a range against a collection length into clamped `[from, to)`
/// offsets (ADR 0019 §2). Negative bounds count from the end; everything
/// out of range clamps, so a slice is always a collection.
fn slice_bounds(range: &Value, length: usize) -> (usize, usize) {
    let Value::Range {
        end,
        exclusive,
        start,
    } = range
    else {
        unreachable!("called with a range")
    };
    let length = length as i64;
    let resolve = |bound: i64| if bound < 0 { length + bound } else { bound };
    let from = start.map(resolve).unwrap_or(0).clamp(0, length);
    let to = match end {
        None => length,
        Some(bound) => {
            let bound = resolve(*bound);
            let bound = if *exclusive { bound } else { bound + 1 };
            bound.clamp(0, length)
        }
    };
    // A reversed or inverted range is empty, not an error.
    (from as usize, to.max(from) as usize)
}

/// Widen a numeric value for mixed arithmetic (ADR 0018).
fn as_float(value: &Value) -> f64 {
    match value {
        Value::Float(number) => *number,
        Value::Integer(number) => *number as f64,
        other => panic!("expected a number, got {other:?}"),
    }
}

/// Integer division, floored — Ruby's rule, not Rust's truncation
/// (ADR 0018): `-7 / 2` is `-4`, because the quotient rounds toward
/// negative infinity rather than toward zero.
fn floored_divide(left: i64, right: i64) -> i64 {
    let quotient = left / right;
    if left % right != 0 && (left < 0) != (right < 0) {
        quotient - 1
    } else {
        quotient
    }
}

/// Modulo whose result takes the sign of the divisor — Ruby's rule
/// (ADR 0018): `-7 % 2` is `1`, and `7 % -2` is `-1`.
fn floored_modulo(left: i64, right: i64) -> i64 {
    let remainder = left % right;
    if remainder != 0 && (remainder < 0) != (right < 0) {
        remainder + right
    } else {
        remainder
    }
}

#[derive(Clone)]
struct Method {
    body: Vec<Statement>,
    /// The namespace this method was written in (ADR 0021). Bare names in
    /// its body resolve outward from here.
    home: Vec<String>,
    keyword_parameters: Vec<Parameter>,
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
    /// The file currently executing; `require_relative` resolves against it.
    current_file: Option<std::path::PathBuf>,
    expression_depth: usize,
    loaded: std::collections::HashSet<std::path::PathBuf>,
    methods: HashMap<String, std::rc::Rc<Method>>,
    /// The namespace currently being defined or executed (ADR 0021).
    module_path: Vec<String>,
    output: W,
    pending: Option<Pending>,
    /// The receiver while a struct method runs: `(struct name, instance)`.
    self_receiver: Option<(String, Value)>,
    structs: HashMap<String, StructInfo>,
    variables: HashMap<String, Binding>,
}

#[derive(Clone)]
struct StructInfo {
    fields: Vec<String>,
    methods: HashMap<String, std::rc::Rc<Method>>,
}

/// A named binding: immutable unless declared `mutable` (ADR 0001).
#[derive(Clone)]
struct Binding {
    mutable: bool,
    value: Value,
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
            current_file: None,
            loaded: std::collections::HashSet::new(),
            expression_depth: 0,
            methods: HashMap::new(),
            module_path: Vec::new(),
            output,
            pending: None,
            self_receiver: None,
            structs: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    /// Command-line arguments exposed to the program via `argv()`.
    /// The path of the file being run, for `require_relative` resolution.
    pub fn set_current_file(&mut self, path: std::path::PathBuf) {
        self.current_file = Some(path);
    }

    pub fn set_arguments(&mut self, arguments: Vec<String>) {
        self.arguments = arguments;
    }

    /// A bare name qualified by the namespace being defined (ADR 0021).
    fn qualified(&self, name: &str) -> String {
        if self.module_path.is_empty() {
            return name.to_string();
        }
        format!("{}::{name}", self.module_path.join("::"))
    }

    /// Resolve a name outward from a namespace: innermost first, then each
    /// enclosing level, then the top (ADR 0021). Lexical scope always
    /// includes every enclosing level, however the namespace was declared —
    /// which is what makes `module A::B` and nested blocks identical.
    fn resolve<'a, T>(
        home: &[String],
        name: &str,
        table: &'a HashMap<String, T>,
    ) -> Option<(String, &'a T)> {
        for depth in (0..=home.len()).rev() {
            let candidate = if depth == 0 {
                name.to_string()
            } else {
                format!("{}::{name}", home[..depth].join("::"))
            };
            if let Some(found) = table.get(&candidate) {
                return Some((candidate, found));
            }
        }
        None
    }

    /// A method reachable from where we stand — own namespace first, then
    /// outward, then the top level (ADR 0021).
    fn lookup_method(&self, name: &str) -> Option<std::rc::Rc<Method>> {
        Self::resolve(&self.module_path, name, &self.methods).map(|(_, found)| found.clone())
    }

    /// Is this name (or path) a namespace rather than a value? Used to tell
    /// `Statistics.mean(x)` from `receiver.method(x)`.
    fn is_namespace(&self, path: &str) -> bool {
        let prefix = format!("{path}::");
        self.methods.keys().any(|name| name.starts_with(&prefix))
            || self.structs.keys().any(|name| name.starts_with(&prefix))
            || self.variables.keys().any(|name| name.starts_with(&prefix))
    }

    /// The namespace a receiver expression names, if it names one. A local
    /// of the same name wins — values beat namespaces.
    fn namespace_receiver(&self, receiver: &Expression) -> Option<String> {
        let written = match receiver {
            Expression::Variable(name) => {
                if self.variables.contains_key(name) {
                    return None;
                }
                name.clone()
            }
            Expression::Path(path) => path.join("::"),
            _ => return None,
        };
        for depth in (0..=self.module_path.len()).rev() {
            let candidate = if depth == 0 {
                written.clone()
            } else {
                format!("{}::{written}", self.module_path[..depth].join("::"))
            };
            if self.is_namespace(&candidate) {
                return Some(candidate);
            }
        }
        None
    }

    /// Bind `_` to the last value the REPL printed. Mutable, so re-binding
    /// each entry is legal under ADR 0001 rather than an immutable clash.
    pub fn set_last_value(&mut self, value: Value) {
        self.variables.insert(
            "_".to_string(),
            Binding {
                mutable: true,
                value,
            },
        );
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
            Statement::Assignment {
                mutable,
                name,
                value,
            } => {
                let Some(value) = self.expression(value) else {
                    if self.pending.is_some() {
                        // An or-guard diverted (`x = f() or return`): no
                        // binding happens; the pending signal unwinds.
                        return None;
                    }
                    panic!("assignment to {name} produced no value");
                };
                // A binding written directly in a module body is a constant
                // of that namespace (ADR 0021): `Foo::LIMIT`. Inside a
                // method the module path is the method's home, but locals
                // there are ordinary — only top-of-body bindings qualify.
                if self.module_path.is_empty() || self.call_depth > 0 {
                    self.assign(name, value.clone(), *mutable);
                } else {
                    self.variables.insert(
                        self.qualified(name),
                        Binding {
                            mutable: *mutable,
                            value: value.clone(),
                        },
                    );
                }
                Some(value)
            }
            Statement::Expression(expression) => self.expression(expression),
            Statement::MethodDefinition {
                body,
                keyword_parameters,
                name,
                parameters,
            } => {
                if self.variables.contains_key(name) {
                    panic!("method {name} shadows local {name} — rename one");
                }
                let method = Method {
                    body: body.clone(),
                    home: self.module_path.clone(),
                    keyword_parameters: keyword_parameters.clone(),
                    parameters: parameters.clone(),
                };
                self.methods
                    .insert(self.qualified(name), std::rc::Rc::new(method));
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
            // Namespaces are flattened at definition time into qualified
            // names (ADR 0021): `module Foo` holding `struct Bar` registers
            // `Foo::Bar`. Both declaration spellings land here identically.
            Statement::ModuleDefinition { body, path } => {
                let depth = self.module_path.len();
                self.module_path.extend(path.iter().cloned());
                self.run_body(body);
                self.module_path.truncate(depth);
                None
            }
            Statement::StructDefinition {
                fields,
                methods,
                name,
                nested,
            } => {
                let mut method_table = HashMap::new();
                for method in methods {
                    let Statement::MethodDefinition {
                        body,
                        keyword_parameters,
                        name: method_name,
                        parameters,
                    } = method
                    else {
                        unreachable!()
                    };
                    method_table.insert(
                        method_name.clone(),
                        std::rc::Rc::new(Method {
                            body: body.clone(),
                            home: self.module_path.clone(),
                            keyword_parameters: keyword_parameters.clone(),
                            parameters: parameters.clone(),
                        }),
                    );
                }
                self.structs.insert(
                    self.qualified(name),
                    StructInfo {
                        fields: fields.clone(),
                        methods: method_table,
                    },
                );
                // A type nested in a type lives under it: `Outer::Inner`.
                if !nested.is_empty() {
                    self.module_path.push(name.clone());
                    self.run_body(nested);
                    self.module_path.pop();
                }
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
                    // Each iteration is a fresh scope for its own locals
                    // (the block rule of ADR 0001, applied to loops): names
                    // first assigned inside the body die at iteration end,
                    // so plain immutable bindings work per-iteration.
                    let preexisting: std::collections::HashSet<String> =
                        self.variables.keys().cloned().collect();
                    self.run_body(body);
                    self.variables.retain(|name, _| preexisting.contains(name));
                    match self.pending {
                        None => {}
                        Some(Pending::Break) => {
                            self.pending = None;
                            break;
                        }
                        Some(Pending::Next) => self.pending = None,
                        // A return keeps unwinding to the enclosing method.
                        Some(Pending::Return(_)) => return None,
                    }
                }
                // A finished while is nil, always (ADR 0012, Ruby's rule).
                Some(Value::Nil)
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
            Expression::ArrayLiteral(elements) => Some(Value::array(
                elements
                    .iter()
                    .map(|element| self.value_of(element))
                    .collect(),
            )),
            Expression::Boolean(value) => Some(Value::Boolean(*value)),
            Expression::Float(value) => Some(Value::Float(*value)),
            // `Foo::BAR` names a constant; `Foo::Bar` names a type and is
            // only meaningful as a receiver, which is handled before we get
            // here (ADR 0021).
            Expression::Path(path) => {
                let joined = path.join("::");
                if let Some(binding) = self.variables.get(&joined) {
                    return Some(binding.value.clone());
                }
                if self.structs.contains_key(&joined) {
                    panic!("{joined} is a type, not a value");
                }
                panic!("undefined name {joined}");
            }
            Expression::Range {
                end,
                exclusive,
                start,
            } => {
                let mut bound = |side: &Option<Box<Expression>>| {
                    side.as_ref().map(|side| match self.value_of(side) {
                        Value::Integer(number) => number,
                        other => panic!("range bounds must be integers so far, got {other:?}"),
                    })
                };
                // Evaluated left to right, like every other binary form.
                let start = bound(start);
                let end = bound(end);
                Some(Value::Range {
                    end,
                    exclusive: *exclusive,
                    start,
                })
            }
            Expression::Nil => Some(Value::Nil),
            Expression::SelfValue => {
                let (_, receiver) = self
                    .self_receiver
                    .as_ref()
                    .unwrap_or_else(|| panic!("self only exists inside a struct method"));
                Some(receiver.clone())
            }
            Expression::Append { name, value } => {
                let current = self
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("undefined variable {name}"))
                    .value
                    .clone();
                let value = self.value_of(value);
                match (current, value) {
                    (Value::String(mut base), Value::String(suffix)) => {
                        base.push_str(&suffix);
                        Some(Value::String(base))
                    }
                    (Value::Array(elements), element) => {
                        let mut elements = elements.as_ref().clone();
                        elements.push(element);
                        Some(Value::array(elements))
                    }
                    (current, value) => {
                        panic!(
                            "cannot append {value:?} to {current:?} — << takes a string or an array on the left"
                        )
                    }
                }
            }
            Expression::IndexUpdate { index, name, value } => {
                let current = self
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("undefined variable {name}"))
                    .value
                    .clone();
                let index = self.value_of(index);
                let value = self.value_of(value);
                match (current, index) {
                    (Value::Array(elements), Value::Integer(index)) => {
                        let mut elements = elements.as_ref().clone();
                        let length = elements.len() as i64;
                        let position = if index < 0 { length + index } else { index };
                        if position == length {
                            elements.push(value);
                        } else if position >= 0 && position < length {
                            elements[position as usize] = value;
                        } else {
                            panic!("index {index} out of range for assignment to {name}");
                        }
                        Some(Value::array(elements))
                    }
                    (Value::Hash(pairs), key) => {
                        let mut pairs = pairs.as_ref().clone();
                        match pairs.iter_mut().find(|(existing, _)| *existing == key) {
                            Some(pair) => pair.1 = value,
                            None => pairs.push((key, value)),
                        }
                        Some(Value::hash(pairs))
                    }
                    (current, index) => {
                        panic!("cannot index-assign {current:?} with {index:?}")
                    }
                }
            }
            // Only reached when an or-guard's left side was absent: divert
            // control and produce no value; the unwinding machinery takes over.
            Expression::Guard(action) => {
                match action {
                    GuardAction::Break => self.pending = Some(Pending::Break),
                    GuardAction::Next => self.pending = Some(Pending::Next),
                    GuardAction::Return(value) => {
                        let value = value.as_ref().map(|expression| self.value_of(expression));
                        self.pending = Some(Pending::Return(value));
                    }
                }
                None
            }
            Expression::MatchAssert { pattern, subject } => {
                let subject = self.value_of(subject);
                let Some(captures) = self.match_pattern(pattern, &subject) else {
                    panic!(
                        "pattern mismatch: {} — the => form panics when it can't destructure",
                        subject.inspect()
                    );
                };
                self.bind_captures(captures);
                Some(Value::Nil)
            }
            Expression::MatchTest { pattern, subject } => {
                let subject = self.value_of(subject);
                match self.match_pattern(pattern, &subject) {
                    Some(captures) => {
                        self.bind_captures(captures);
                        Some(Value::Boolean(true))
                    }
                    None => Some(Value::Boolean(false)),
                }
            }
            Expression::CaseIn {
                branches,
                else_body,
                subject,
            } => {
                let subject = self.value_of(subject);
                for branch in branches {
                    let Some(captures) = self.match_pattern(&branch.pattern, &subject) else {
                        continue;
                    };
                    // Captures bind into the enclosing scope and persist,
                    // Ruby-style — fenced by the assignment rules (ADR 0013
                    // §3). They bind before the guard runs so the guard can
                    // see them — but a failed guard rolls its captures back
                    // (tidier than Ruby's leak, and immutability needs it).
                    let saved: Vec<(String, Option<Binding>)> = captures
                        .iter()
                        .map(|(name, _)| (name.clone(), self.variables.get(name).cloned()))
                        .collect();
                    self.bind_captures(captures);
                    if let Some(guard) = &branch.guard
                        && !self.boolean_of(guard, "pattern guard")
                    {
                        for (name, original) in saved {
                            match original {
                                Some(binding) => self.variables.insert(name, binding),
                                None => self.variables.remove(&name),
                            };
                        }
                        continue;
                    }
                    if branch.body.is_empty() {
                        return Some(Value::Nil);
                    }
                    return self.run_body(&branch.body);
                }
                if else_body.is_empty() {
                    // The runtime preview of compile-checked exhaustiveness
                    // (ADR 0013 §1): the real compiler refuses to build this.
                    panic!(
                        "no pattern matched {} — add an in branch or an else",
                        subject.inspect()
                    );
                }
                self.run_body(else_body)
            }
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
                Some(Value::hash(pairs))
            }
            Expression::Index { index, receiver } => {
                let receiver = self.value_of(receiver);
                let index = self.value_of(index);
                match (&receiver, &index) {
                    (Value::Array(elements), Value::Integer(index)) => {
                        // Partial operations return maybes (ADR 0010): negative
                        // indices stay Ruby-style, out of range is nil.
                        let length = elements.len() as i64;
                        let position = if *index < 0 { length + index } else { *index };
                        if position < 0 || position >= length {
                            return Some(Value::Nil);
                        }
                        Some(Value::present(elements[position as usize].clone()))
                    }
                    (Value::String(text), Value::Integer(index)) => {
                        // Indexing a string yields a one-character string.
                        let length = text.chars().count() as i64;
                        let position = if *index < 0 { length + index } else { *index };
                        if position < 0 || position >= length {
                            return Some(Value::Nil);
                        }
                        let character = text.chars().nth(position as usize).unwrap();
                        Some(Value::String(character.to_string()))
                    }
                    // A slice is always a collection, never a maybe
                    // (ADR 0019 §2): the start clamps the way Ruby already
                    // clamps the end, so out of range is empty, not nil.
                    (Value::Array(elements), Value::Range { .. }) => {
                        let (from, to) = slice_bounds(&index, elements.len());
                        Some(Value::array(elements[from..to].to_vec()))
                    }
                    (Value::String(text), Value::Range { .. }) => {
                        let characters: Vec<char> = text.chars().collect();
                        let (from, to) = slice_bounds(&index, characters.len());
                        Some(Value::String(characters[from..to].iter().collect()))
                    }
                    (Value::Hash(pairs), key) => Some(
                        pairs
                            .iter()
                            .find(|(existing, _)| existing == key)
                            .map_or(Value::Nil, |(_, value)| Value::present(value.clone())),
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
                if body.is_empty() {
                    // A branch that doesn't happen produces nil (ADR 0012).
                    return Some(Value::Nil);
                }
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
                        Some(Value::Integer(floored_divide(left, right)))
                    }
                    (Value::Integer(left), BinaryOperator::Modulo, Value::Integer(right)) => {
                        Some(Value::Integer(floored_modulo(left, right)))
                    }
                    (Value::Integer(left), BinaryOperator::Multiply, Value::Integer(right)) => {
                        Some(Value::Integer(left * right))
                    }
                    (Value::Integer(left), BinaryOperator::Subtract, Value::Integer(right)) => {
                        Some(Value::Integer(left - right))
                    }
                    // Mixed arithmetic promotes to float, Ruby's rule
                    // (ADR 0018). `/` is real division once a float is
                    // involved — only integer `/` integer floors. Equality
                    // compares across the two numeric types, so `1.0 == 1`.
                    (ref left_value, _, ref right_value)
                        if matches!(
                            (left_value, right_value),
                            (Value::Float(_), Value::Float(_))
                                | (Value::Float(_), Value::Integer(_))
                                | (Value::Integer(_), Value::Float(_))
                        ) =>
                    {
                        let (left, right) = (as_float(left_value), as_float(right_value));
                        Some(match operator {
                            BinaryOperator::Add => Value::Float(left + right),
                            BinaryOperator::Divide => Value::Float(left / right),
                            BinaryOperator::Modulo => {
                                Value::Float(left - right * (left / right).floor())
                            }
                            BinaryOperator::Multiply => Value::Float(left * right),
                            BinaryOperator::Subtract => Value::Float(left - right),
                            BinaryOperator::Greater => Value::Boolean(left > right),
                            BinaryOperator::GreaterOrEqual => Value::Boolean(left >= right),
                            BinaryOperator::Less => Value::Boolean(left < right),
                            BinaryOperator::LessOrEqual => Value::Boolean(left <= right),
                            BinaryOperator::Equals => Value::Boolean(left == right),
                            BinaryOperator::NotEquals => Value::Boolean(left != right),
                        })
                    }
                    (Value::String(left), BinaryOperator::Add, Value::String(right)) => {
                        Some(Value::String(left + &right))
                    }
                    (Value::Array(left), BinaryOperator::Add, Value::Array(right)) => {
                        let mut combined = left.as_ref().clone();
                        combined.extend(right.iter().cloned());
                        Some(Value::array(combined))
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
                        Some(Value::array(repeated))
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
                safe,
            } => {
                // `Name.new(...)` constructs a struct; the name is not a value.
                if name == "new" {
                    return Some(self.construct_struct(receiver, arguments, keyword_arguments));
                }
                // `Statistics.mean(x)` — the receiver names a namespace, so
                // this invokes a function inside it rather than dispatching
                // on a value (ADR 0021).
                if let Some(namespace) = self.namespace_receiver(receiver) {
                    let qualified = format!("{namespace}::{name}");
                    if self.methods.contains_key(&qualified) {
                        let arguments: Vec<Value> = arguments
                            .iter()
                            .map(|argument| self.value_of(argument))
                            .collect();
                        let keyword_arguments: Vec<(String, Value)> = keyword_arguments
                            .iter()
                            .map(|(label, expression)| (label.clone(), self.value_of(expression)))
                            .collect();
                        return self.call(&qualified, arguments, keyword_arguments);
                    }
                    panic!("{namespace} has no {name}");
                }
                let receiver = self.value_of(receiver);
                // `&.`: an absent receiver short-circuits — arguments never run.
                if *safe && matches!(receiver, Value::Nil) {
                    return Some(Value::Nil);
                }
                let arguments: Vec<Value> = arguments
                    .iter()
                    .map(|argument| self.value_of(argument))
                    .collect();
                let keyword_arguments: Vec<(String, Value)> = keyword_arguments
                    .iter()
                    .map(|(label, expression)| (label.clone(), self.value_of(expression)))
                    .collect();
                self.method_call(receiver, name, arguments, keyword_arguments, block.as_ref())
            }
            Expression::Logical {
                left,
                operator,
                right,
            } => {
                // Typed `or` (ADR 0007): booleans get logical or; a maybe gets
                // unwrap-or-else. Short-circuit either way — the right side
                // only runs when it can matter. The static halves (dead right
                // sides, the Boolean? never-guess) are out of the seed's reach.
                let left = self.value_of(left);
                match (operator, left) {
                    (LogicalOperator::Or, Value::Nil) => self.expression(right),
                    // Present-but-wrapped: unwrap one layer — a stored nil
                    // beats the default, exactly fetch's rule (ADR 0010).
                    (LogicalOperator::Or, Value::Some(inner)) => Some(*inner),
                    (LogicalOperator::Or, Value::Boolean(true)) => Some(Value::Boolean(true)),
                    (LogicalOperator::Or, Value::Boolean(false)) => {
                        Some(Value::Boolean(self.boolean_of(right, "|| operands")))
                    }
                    (LogicalOperator::Or, present) => Some(present),
                    (LogicalOperator::And, Value::Boolean(false)) => Some(Value::Boolean(false)),
                    (LogicalOperator::And, Value::Boolean(true)) => {
                        Some(Value::Boolean(self.boolean_of(right, "&& operands")))
                    }
                    (LogicalOperator::And, other) => {
                        panic!("&& needs true or false, got {other:?}")
                    }
                }
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
            Expression::Variable(name) => {
                // A bare name is a local if one exists, otherwise a
                // zero-argument call — unambiguous because shadowing is an error.
                if let Some(binding) = self.variables.get(name) {
                    Some(binding.value.clone())
                } else if let Some((_, binding)) =
                    Self::resolve(&self.module_path, name, &self.variables)
                {
                    // A constant of an enclosing namespace (ADR 0021).
                    Some(binding.value.clone())
                } else if let Some(method) = self.own_struct_method(name) {
                    // Bare own-method calls inside a struct method (#27).
                    let (struct_name, receiver) = self.self_receiver.clone().unwrap();
                    self.call_struct_method(struct_name, receiver, method, Vec::new(), Vec::new())
                } else if self.lookup_method(name).is_some() || Self::builtin_name(name) {
                    self.call(name, Vec::new(), Vec::new())
                } else {
                    panic!("undefined variable or method {name}")
                }
            }
            Expression::Call {
                arguments,
                keyword_arguments,
                name,
            } => {
                let arguments: Vec<Value> = arguments
                    .iter()
                    .map(|argument| self.value_of(argument))
                    .collect();
                let keyword_arguments: Vec<(String, Value)> = keyword_arguments
                    .iter()
                    .map(|(label, expression)| (label.clone(), self.value_of(expression)))
                    .collect();
                self.call(name, arguments, keyword_arguments)
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
        // A struct name is either bare (resolved outward from where we
        // stand) or a `::` path (already absolute) — ADR 0021.
        let written = match receiver {
            Expression::Variable(name) => name.clone(),
            Expression::Path(path) => path.join("::"),
            other => panic!("new needs a struct name receiver, got {other:?}"),
        };
        let (struct_name, info) = Self::resolve(&self.module_path, &written, &self.structs)
            .map(|(found, info)| (found, info.clone()))
            .unwrap_or_else(|| panic!("undefined struct {written}"));
        let struct_name = &struct_name;
        let fields = info.fields.clone();
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
    ) -> Option<Value> {
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
            return Some(Value::Struct {
                fields: updated,
                name,
            });
        }
        // Struct methods dispatch before field access (#27): token.integer?
        if let Value::Struct {
            name: struct_name, ..
        } = &receiver
        {
            let method = self
                .structs
                .get(struct_name)
                .and_then(|info| info.methods.get(name))
                .cloned();
            if let Some(method) = method {
                let struct_name = struct_name.clone();
                return self.call_struct_method(
                    struct_name,
                    receiver,
                    method,
                    arguments,
                    keyword_arguments,
                );
            }
        }
        if !keyword_arguments.is_empty() {
            panic!("keyword arguments are only for struct new, with, and struct methods so far");
        }
        if let Value::Struct {
            fields,
            name: struct_name,
        } = &receiver
            && arguments.is_empty()
            && block.is_none()
        {
            if let Some((_, value)) = fields.iter().find(|(field, _)| field == name) {
                return Some(value.clone());
            }
            // to_s and the maybe predicates fall through to the generic arms.
            if !matches!(name, "nil?" | "some?" | "to_s") {
                panic!("{struct_name} has no field {name}");
            }
        }
        // A range behaves as its elements do: `(1..n).each` is the counted
        // loop (ADR 0019). `include?` answers without walking, so endless
        // and beginless ranges can answer it too.
        if let Value::Range {
            end,
            exclusive,
            start,
        } = &receiver
        {
            if name == "include?"
                && let [Value::Integer(probe)] = arguments.as_slice()
            {
                let above = start.is_none_or(|start| *probe >= start);
                let below = end.is_none_or(|end| {
                    if *exclusive {
                        *probe < end
                    } else {
                        *probe <= end
                    }
                });
                return Some(Value::Boolean(above && below));
            }
            if name == "to_a" && arguments.is_empty() {
                return Some(Value::array(range_elements(&receiver)));
            }
            if block.is_some() || matches!(name, "length" | "sum" | "first" | "last") {
                let elements = range_elements(&receiver);
                return self.method_call(
                    Value::array(elements),
                    name,
                    arguments,
                    Vec::new(),
                    block,
                );
            }
        }
        if let Some(block) = block {
            return match (&receiver, name, arguments.as_slice()) {
                (Value::Array(elements), "each", []) => {
                    for element in elements.iter().cloned() {
                        self.run_block(block, vec![element]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                    }
                    Some(receiver)
                }
                (Value::Hash(pairs), "each", []) => {
                    for (key, value) in pairs.iter().cloned() {
                        self.run_block(block, vec![key, value]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                    }
                    Some(receiver)
                }
                (Value::Array(elements), "map", []) => {
                    let mut results = Vec::new();
                    for element in elements.iter().cloned() {
                        let result = self.run_block(block, vec![element]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                        let result =
                            result.unwrap_or_else(|| panic!("map block produced no value"));
                        results.push(result);
                    }
                    Some(Value::array(results))
                }
                (Value::Array(elements), "each_with_index", []) => {
                    for (index, element) in elements.iter().cloned().enumerate() {
                        self.run_block(block, vec![element, Value::Integer(index as i64)]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                    }
                    Some(receiver)
                }
                (Value::Array(elements), "reduce", [initial]) => {
                    let mut accumulator = initial.clone();
                    for element in elements.iter().cloned() {
                        let result = self.run_block(block, vec![accumulator.clone(), element]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                        accumulator =
                            result.unwrap_or_else(|| panic!("reduce block produced no value"));
                    }
                    Some(accumulator)
                }
                (Value::Array(elements), "reject", []) => {
                    let mut kept = Vec::new();
                    for element in elements.iter().cloned() {
                        let verdict = self.run_block(block, vec![element.clone()]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                        match verdict {
                            Some(Value::Boolean(true)) => {}
                            Some(Value::Boolean(false)) => kept.push(element),
                            other => {
                                panic!("reject block must produce true or false, got {other:?}")
                            }
                        }
                    }
                    Some(Value::array(kept))
                }
                (Value::Array(elements), "select", []) => {
                    let mut kept = Vec::new();
                    for element in elements.iter().cloned() {
                        let verdict = self.run_block(block, vec![element.clone()]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                        match verdict {
                            Some(Value::Boolean(true)) => kept.push(element),
                            Some(Value::Boolean(false)) => {}
                            other => {
                                panic!("select block must produce true or false, got {other:?}")
                            }
                        }
                    }
                    Some(Value::array(kept))
                }
                (Value::Integer(count), "times", []) => {
                    for index in 0..*count {
                        self.run_block(block, vec![Value::Integer(index)]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                    }
                    Some(receiver)
                }
                (Value::Integer(from), "downto", [Value::Integer(to)]) => {
                    let mut current = *from;
                    while current >= *to {
                        self.run_block(block, vec![Value::Integer(current)]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                        current -= 1;
                    }
                    Some(receiver)
                }
                (Value::Integer(from), "upto", [Value::Integer(to)]) => {
                    for current in *from..=*to {
                        self.run_block(block, vec![Value::Integer(current)]);
                        if let Some(interrupted) = self.block_interrupt() {
                            return interrupted;
                        }
                    }
                    Some(receiver)
                }
                (receiver, name, _) => {
                    panic!("undefined block-taking method {name} for {receiver:?}")
                }
            };
        }

        Some(match (&receiver, name, arguments.as_slice()) {
            // The maybe predicates (ADR 0006/0009) come first: they are the
            // one thing callable on any value including nil, because
            // statically they belong to the maybe, not to nil.
            (_, "nil?", []) => Value::Boolean(matches!(receiver, Value::Nil)),
            (_, "some?", []) => Value::Boolean(!matches!(receiver, Value::Nil)),
            (Value::Nil, name, _) => {
                panic!("nil has no method {name} — handle the nil case first")
            }
            (Value::Array(elements), "empty?", []) => Value::Boolean(elements.is_empty()),
            (Value::Array(elements), "first", []) => {
                elements.first().cloned().map_or(Value::Nil, Value::present)
            }
            (Value::Array(elements), "join", [Value::String(separator)]) => Value::String(
                elements
                    .iter()
                    .map(|element| element.to_string())
                    .collect::<Vec<_>>()
                    .join(separator),
            ),
            (Value::Array(elements), "last", []) => {
                elements.last().cloned().map_or(Value::Nil, Value::present)
            }
            (Value::Array(elements), "length", []) => Value::Integer(elements.len() as i64),
            (Value::Hash(pairs), "empty?", []) => Value::Boolean(pairs.is_empty()),
            (Value::Hash(pairs), "key?", [key]) => {
                Value::Boolean(pairs.iter().any(|(existing, _)| existing == key))
            }
            (Value::Hash(pairs), "keys", []) => {
                Value::array(pairs.iter().map(|(key, _)| key.clone()).collect())
            }
            (Value::Hash(pairs), "length", []) => Value::Integer(pairs.len() as i64),
            (Value::Hash(pairs), "values", []) => {
                Value::array(pairs.iter().map(|(_, value)| value.clone()).collect())
            }
            (Value::Array(elements), "include?", [needle]) => {
                Value::Boolean(elements.contains(needle))
            }
            (Value::Array(elements), "max", []) => Self::integers_of(elements, "max")
                .into_iter()
                .max()
                .map_or(Value::Nil, Value::Integer),
            (Value::Array(elements), "min", []) => Self::integers_of(elements, "min")
                .into_iter()
                .min()
                .map_or(Value::Nil, Value::Integer),
            (Value::Array(elements), "slice", [Value::Integer(start), Value::Integer(length)]) => {
                let start = usize::try_from(*start)
                    .unwrap_or_else(|_| panic!("slice start must not be negative, got {start}"));
                let length = usize::try_from(*length)
                    .unwrap_or_else(|_| panic!("slice length must not be negative, got {length}"));
                Value::array(elements.iter().skip(start).take(length).cloned().collect())
            }
            (Value::Array(elements), "sort", []) => {
                let mut sorted = Self::integers_of(elements, "sort");
                sorted.sort_unstable();
                Value::array(sorted.into_iter().map(Value::Integer).collect())
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
            (Value::String(text), "chars", []) => Value::array(
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
            (Value::String(text), "split", [Value::String(separator)]) => Value::array(
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
            (Value::String(text), "to_f", []) => {
                // Ruby's `to_f` never fails — unparseable text is 0.0.
                Value::Float(text.trim().parse().unwrap_or(0.0))
            }
            (Value::Integer(number), "to_f", []) => Value::Float(*number as f64),
            (Value::Float(number), "to_f", []) => Value::Float(*number),
            // Ruby's Float#to_i truncates toward zero — it is not the
            // floored division of ADR 0018.
            (Value::Float(number), "to_i", []) => Value::Integer(*number as i64),
            (Value::Float(number), "abs", []) => Value::Float(number.abs()),
            (Value::String(text), "upcase", []) => Value::String(text.to_uppercase()),
            (receiver, "to_s", []) => Value::String(receiver.to_string()),
            (receiver, name, arguments) => {
                panic!("undefined method {name} for {receiver:?} with {arguments:?}")
            }
        })
    }

    fn builtin_name(name: &str) -> bool {
        matches!(
            name,
            "argv"
                | "p"
                | "panic"
                | "puts"
                | "read_file"
                | "require_relative"
                | "some"
                | "write_file"
        )
    }

    /// Load another Portland file, Ruby-style: the path is resolved against
    /// the requiring file's directory, `.pdx` is implied, and a file loads
    /// only once (returns false when already loaded).
    fn require_relative(&mut self, path: &str) -> bool {
        let base = self
            .current_file
            .as_ref()
            .and_then(|file| file.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let mut resolved = base.join(path);
        if resolved.extension().is_none() {
            resolved.set_extension("pdx");
        }
        let resolved = resolved
            .canonicalize()
            .unwrap_or_else(|error| panic!("require_relative {path:?}: {error}"));
        if self.loaded.contains(&resolved) {
            return false;
        }
        self.loaded.insert(resolved.clone());
        let source = std::fs::read_to_string(&resolved)
            .unwrap_or_else(|error| panic!("require_relative {path:?}: {error}"));
        let program = parser::parse(&source);
        let previous_file = self.current_file.replace(resolved);
        self.run_body(&program.statements);
        self.current_file = previous_file;
        true
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

    /// After a block ran: `Some(outcome)` when the iteration must stop now.
    /// A `break` is consumed here and the call produces nil (ADR 0012);
    /// a `return` stays pending and unwinds to the enclosing method.
    fn block_interrupt(&mut self) -> Option<Option<Value>> {
        match self.pending {
            None => None,
            Some(Pending::Break) => {
                self.pending = None;
                Some(Some(Value::Nil))
            }
            _ => Some(None),
        }
    }

    /// Run a block as a closure: it sees the enclosing scope; its parameters
    /// are block-local (shadowed, then restored), and names first assigned
    /// inside the block die at `end` (ADR 0001's third closure rule).
    fn run_block(&mut self, block: &Block, arguments: Vec<Value>) -> Option<Value> {
        if block.parameters.len() > arguments.len() {
            panic!(
                "block expects {} argument(s), got {}",
                block.parameters.len(),
                arguments.len()
            );
        }
        let preexisting: std::collections::HashSet<String> =
            self.variables.keys().cloned().collect();
        let shadowed: Vec<(String, Option<Binding>)> = block
            .parameters
            .iter()
            .map(|parameter| (parameter.clone(), self.variables.get(parameter).cloned()))
            .collect();
        for (parameter, argument) in block.parameters.iter().zip(arguments) {
            if self.methods.contains_key(parameter) || Self::builtin_name(parameter) {
                panic!("block parameter {parameter} shadows method {parameter} — rename one");
            }
            // ADR 0017: `it` is implicit, so a collision with an existing
            // local is confusing rather than merely shadowing. Named
            // parameters keep the ordinary shadow-and-restore rule.
            if parameter == "it" && self.variables.contains_key("it") {
                panic!("`it` is a local here and a block parameter there — rename one");
            }
            self.variables.insert(
                parameter.clone(),
                Binding {
                    mutable: false,
                    value: argument,
                },
            );
        }
        let result = self.run_body(&block.body);
        // `next` ends just this invocation of the block; `break` and `return`
        // stay pending for the iterating caller to handle.
        if matches!(self.pending, Some(Pending::Next)) {
            self.pending = None;
        }
        // Fresh block-locals die at end; outer bindings survive.
        self.variables.retain(|name, _| preexisting.contains(name));
        for (parameter, original) in shadowed {
            match original {
                Some(binding) => self.variables.insert(parameter, binding),
                None => self.variables.remove(&parameter),
            };
        }
        result
    }

    /// Bind or rebind a name, enforcing ADR 0001: `mutable` declares a new
    /// rebindable name exactly once; a bare assignment creates an immutable
    /// binding or rebinds an existing mutable one.
    fn assign(&mut self, name: &str, value: Value, declare_mutable: bool) {
        // The no-shadow rule: a name is a local or a method, never both.
        if self.lookup_method(name).is_some() || Self::builtin_name(name) {
            panic!("local {name} shadows method {name} — rename one");
        }
        if declare_mutable {
            if self.variables.contains_key(name) {
                panic!("{name} is already declared — mutable declares a new name once");
            }
            self.variables.insert(
                name.to_string(),
                Binding {
                    mutable: true,
                    value,
                },
            );
            return;
        }
        match self.variables.get(name) {
            Some(binding) if !binding.mutable => {
                panic!(
                    "{name} is immutable — declare it `mutable {name} = ...` if it needs to change"
                );
            }
            Some(_) => {
                self.variables.insert(
                    name.to_string(),
                    Binding {
                        mutable: true,
                        value,
                    },
                );
            }
            None => {
                self.variables.insert(
                    name.to_string(),
                    Binding {
                        mutable: false,
                        value,
                    },
                );
            }
        }
    }

    /// Bind pattern captures into the enclosing scope: same rules as a bare
    /// assignment (immutable clash = error, suggesting the ^ pin).
    fn bind_captures(&mut self, captures: Vec<(String, Value)>) {
        for (name, value) in captures {
            self.assign(&name, value, false);
        }
    }

    /// Try one pattern against a value: `Some(captures)` on a match (empty
    /// for literal patterns), `None` on a miss.
    fn match_pattern(
        &mut self,
        pattern: &Pattern,
        subject: &Value,
    ) -> Option<Vec<(String, Value)>> {
        match pattern {
            Pattern::Alternative(options) => options
                .iter()
                .find_map(|option| self.match_pattern(option, subject)),
            // A range pattern tests membership, not equality (ADR 0019 §1).
            Pattern::Range {
                end,
                exclusive,
                start,
            } => {
                let Value::Integer(probe) = subject else {
                    return None;
                };
                let above = start.is_none_or(|start| *probe >= start);
                let below = end.is_none_or(|end| {
                    if *exclusive {
                        *probe < end
                    } else {
                        *probe <= end
                    }
                });
                (above && below).then(Vec::new)
            }
            Pattern::Array { elements, rest } => {
                let Value::Array(values) = subject else {
                    return None;
                };
                match rest {
                    None if values.len() != elements.len() => return None,
                    Some(_) if values.len() < elements.len() => return None,
                    _ => {}
                }
                let mut captures = Vec::new();
                for (element, value) in elements.iter().zip(values.iter()) {
                    captures.extend(self.match_pattern(element, value)?);
                }
                if let Some(Some(name)) = rest {
                    let remainder = values[elements.len()..].to_vec();
                    captures.push((name.clone(), Value::array(remainder)));
                }
                Some(captures)
            }
            Pattern::Capture(name) => Some(vec![(name.clone(), subject.clone())]),
            Pattern::Pin(name) => {
                let value = self
                    .variables
                    .get(name)
                    .unwrap_or_else(|| panic!("undefined variable {name} in a ^ pin"))
                    .value
                    .clone();
                if value == *subject {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            Pattern::Literal(expression) => {
                if self.value_of(expression) == *subject {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            Pattern::Struct {
                fields: pattern_fields,
                name,
            } => {
                // Builtin type patterns (#27): `in String`, `in Integer`, …
                // — the type predicate, pattern-flavored. No reflection API.
                if matches!(
                    name.as_str(),
                    "Array" | "Boolean" | "Hash" | "Integer" | "String"
                ) {
                    if !pattern_fields.is_empty() {
                        panic!("the builtin type pattern {name} takes no fields");
                    }
                    let matched = matches!(
                        (name.as_str(), subject),
                        ("Array", Value::Array(_))
                            | ("Boolean", Value::Boolean(_))
                            | ("Hash", Value::Hash(_))
                            | ("Integer", Value::Integer(_))
                            | ("String", Value::String(_))
                    );
                    return if matched { Some(Vec::new()) } else { None };
                }
                let Value::Struct {
                    fields,
                    name: struct_name,
                } = subject
                else {
                    return None;
                };
                if struct_name != name {
                    return None;
                }
                let mut captures = Vec::new();
                for (label, sub_pattern) in pattern_fields {
                    let value = fields
                        .iter()
                        .find(|(field, _)| field == label)
                        .unwrap_or_else(|| panic!("{name} has no field {label}"))
                        .1
                        .clone();
                    match sub_pattern {
                        // `field:` shorthand binds under the field's name.
                        None => captures.push((label.clone(), value)),
                        Some(sub_pattern) => {
                            captures.extend(self.match_pattern(sub_pattern, &value)?)
                        }
                    }
                }
                Some(captures)
            }
        }
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

    /// The struct method `name` on the current receiver, if both exist.
    fn own_struct_method(&self, name: &str) -> Option<std::rc::Rc<Method>> {
        let (struct_name, _) = self.self_receiver.as_ref()?;
        self.structs
            .get(struct_name)
            .and_then(|info| info.methods.get(name))
            .cloned()
    }

    /// Run one struct method: a fresh scope of the receiver's fields (bare,
    /// immutable) plus parameters; `self` is the receiver (#27).
    fn call_struct_method(
        &mut self,
        struct_name: String,
        receiver: Value,
        method: std::rc::Rc<Method>,
        arguments: Vec<Value>,
        keyword_arguments: Vec<(String, Value)>,
    ) -> Option<Value> {
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
                "{struct_name} method expects {expected} argument(s), got {}",
                arguments.len()
            );
        }
        self.call_depth += 1;
        if self.call_depth > MAXIMUM_CALL_DEPTH {
            panic!("call stack deeper than {MAXIMUM_CALL_DEPTH} frames (infinite recursion?)");
        }
        // Namespace constants (qualified names) survive into a fresh
        // scope; bare locals do not (ADR 0021).
        let mut scope: HashMap<String, Binding> = self
            .variables
            .iter()
            .filter(|(name, _)| name.contains("::"))
            .map(|(name, binding)| (name.clone(), binding.clone()))
            .collect();
        std::mem::swap(&mut self.variables, &mut scope);
        let field_names: Vec<String> = if let Value::Struct { fields, .. } = &receiver {
            for (field, value) in fields {
                self.variables.insert(
                    field.clone(),
                    Binding {
                        mutable: false,
                        value: value.clone(),
                    },
                );
            }
            fields.iter().map(|(field, _)| field.clone()).collect()
        } else {
            panic!("struct method on a non-struct receiver: {receiver:?}");
        };
        let mut supplied = arguments.into_iter();
        for parameter in &method.parameters {
            if field_names.contains(&parameter.name) {
                panic!(
                    "parameter {} shadows a field of {struct_name} — rename one",
                    parameter.name
                );
            }
            let value = match supplied.next() {
                Some(value) => value,
                None => {
                    let default = parameter.default.as_ref().unwrap();
                    self.value_of(default)
                }
            };
            self.variables.insert(
                parameter.name.clone(),
                Binding {
                    mutable: parameter.mutable,
                    value,
                },
            );
        }
        let mut keyword_arguments = keyword_arguments;
        for parameter in &method.keyword_parameters {
            if field_names.contains(&parameter.name) {
                panic!(
                    "parameter {} shadows a field of {struct_name} — rename one",
                    parameter.name
                );
            }
            let position = keyword_arguments
                .iter()
                .position(|(label, _)| *label == parameter.name);
            let value = match position {
                Some(index) => keyword_arguments.remove(index).1,
                None => match &parameter.default {
                    Some(default) => self.value_of(default),
                    None => panic!(
                        "{struct_name} method missing keyword argument {}",
                        parameter.name
                    ),
                },
            };
            self.variables.insert(
                parameter.name.clone(),
                Binding {
                    mutable: parameter.mutable,
                    value,
                },
            );
        }
        if let Some((label, _)) = keyword_arguments.first() {
            panic!("{struct_name} method got unknown keyword argument {label}");
        }
        let previous_self = self.self_receiver.replace((struct_name, receiver));
        let caller_module = std::mem::replace(&mut self.module_path, method.home.clone());
        let mut result = self.run_body(&method.body);
        self.module_path = caller_module;
        self.call_depth -= 1;
        self.self_receiver = previous_self;
        std::mem::swap(&mut self.variables, &mut scope);
        match self.pending.take() {
            None => {}
            Some(Pending::Return(value)) => result = value,
            Some(Pending::Break) => panic!("break outside of a loop"),
            Some(Pending::Next) => panic!("next outside of a loop"),
        }
        result
    }

    fn call(
        &mut self,
        name: &str,
        arguments: Vec<Value>,
        keyword_arguments: Vec<(String, Value)>,
    ) -> Option<Value> {
        // Inside a struct method, bare calls reach own methods first (#27).
        if let Some(method) = self.own_struct_method(name) {
            let (struct_name, receiver) = self.self_receiver.clone().unwrap();
            return self.call_struct_method(
                struct_name,
                receiver,
                method,
                arguments,
                keyword_arguments,
            );
        }
        if self.lookup_method(name).is_none() && !keyword_arguments.is_empty() {
            panic!("{name} takes no keyword arguments");
        }
        if self.lookup_method(name).is_none() && name == "puts" {
            // Like Ruby: bare puts() prints a blank line.
            if arguments.is_empty() {
                writeln!(self.output).expect("failed to write output");
            }
            for argument in &arguments {
                // Crude preview of the compile error: nil has no rendering.
                if matches!(argument, Value::Nil) {
                    panic!(
                        "puts got nil — handle the nil case first (p renders nil for debugging)"
                    );
                }
                writeln!(self.output, "{argument}").expect("failed to write output");
            }
            return None;
        }
        if self.lookup_method(name).is_none() && name == "some" {
            let mut arguments = arguments;
            match arguments.len() {
                1 => return Some(Value::present(arguments.remove(0))),
                other => panic!("some takes one argument, got {other}"),
            }
        }
        if self.lookup_method(name).is_none() && name == "panic" {
            // The only crash is one you typed (ADR 0010).
            match arguments.as_slice() {
                [Value::String(message)] => panic!("{message}"),
                _ => panic!("panic needs one string message"),
            }
        }
        if self.lookup_method(name).is_none() && name == "p" {
            for argument in &arguments {
                let inspected = argument.inspect();
                writeln!(self.output, "{inspected}").expect("failed to write output");
            }
            // Like Ruby: p returns its argument, so it drops into any expression.
            let mut arguments = arguments;
            return match arguments.len() {
                0 => None,
                1 => Some(arguments.remove(0)),
                _ => Some(Value::array(arguments)),
            };
        }

        // Crude IO builtins so real programs are possible before the object
        // model exists; names and shapes are placeholders, not decisions.
        if self.lookup_method(name).is_none() {
            match (name, arguments.as_slice()) {
                ("argv", []) => {
                    let arguments = self
                        .arguments
                        .iter()
                        .map(|argument| Value::String(argument.clone()))
                        .collect();
                    return Some(Value::array(arguments));
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
                ("require_relative", [Value::String(path)]) => {
                    return Some(Value::Boolean(self.require_relative(path)));
                }
                _ => {}
            }
        }

        let method = self
            .lookup_method(name)
            .unwrap_or_else(|| panic!("undefined method {name}"));
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

        self.call_depth += 1;
        if self.call_depth > MAXIMUM_CALL_DEPTH {
            panic!("call stack deeper than {MAXIMUM_CALL_DEPTH} frames (infinite recursion?)");
        }
        // Methods get a fresh scope: parameters only, no outer locals.
        // Bind left to right so a default can reference earlier parameters.
        // Namespace constants (qualified names) survive into a fresh
        // scope; bare locals do not (ADR 0021).
        let mut scope: HashMap<String, Binding> = self
            .variables
            .iter()
            .filter(|(name, _)| name.contains("::"))
            .map(|(name, binding)| (name.clone(), binding.clone()))
            .collect();
        std::mem::swap(&mut self.variables, &mut scope);
        let mut supplied = arguments.into_iter();
        for parameter in &method.parameters {
            if self.methods.contains_key(&parameter.name) || Self::builtin_name(&parameter.name) {
                panic!(
                    "parameter {} shadows method {} — rename one",
                    parameter.name, parameter.name
                );
            }
            let value = match supplied.next() {
                Some(value) => value,
                None => {
                    let default = parameter.default.as_ref().unwrap();
                    self.value_of(default)
                }
            };
            self.variables.insert(
                parameter.name.clone(),
                Binding {
                    mutable: parameter.mutable,
                    value,
                },
            );
        }
        // Keyword parameters bind after positionals, in declaration order,
        // so their defaults can reference any earlier parameter.
        let mut keyword_arguments = keyword_arguments;
        for parameter in &method.keyword_parameters {
            if self.methods.contains_key(&parameter.name) || Self::builtin_name(&parameter.name) {
                panic!(
                    "parameter {} shadows method {} — rename one",
                    parameter.name, parameter.name
                );
            }
            let position = keyword_arguments
                .iter()
                .position(|(label, _)| *label == parameter.name);
            let value = match position {
                Some(index) => keyword_arguments.remove(index).1,
                None => match &parameter.default {
                    Some(default) => self.value_of(default),
                    None => panic!("{name} missing keyword argument {}", parameter.name),
                },
            };
            self.variables.insert(
                parameter.name.clone(),
                Binding {
                    mutable: parameter.mutable,
                    value,
                },
            );
        }
        if let Some((label, _)) = keyword_arguments.first() {
            panic!("{name} got unknown keyword argument {label}");
        }
        // A top-level method body has no receiver, even when called from
        // inside a struct method.
        let previous_self = self.self_receiver.take();
        // Bare names resolve from where the method was written (ADR 0021).
        let caller_module = std::mem::replace(&mut self.module_path, method.home.clone());
        let mut result = self.run_body(&method.body);
        self.module_path = caller_module;
        self.call_depth -= 1;
        self.self_receiver = previous_self;
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
            evaluate("mutable value = 1\nvalue += 2\nvalue\n"),
            Some(Value::Integer(3))
        );
        assert_eq!(
            evaluate("mutable value = 10\nvalue -= 3\nvalue\n"),
            Some(Value::Integer(7))
        );
        assert_eq!(
            evaluate("mutable value = 4\nvalue *= 3\nvalue\n"),
            Some(Value::Integer(12))
        );
        assert_eq!(
            evaluate("mutable value = 9\nvalue /= 2\nvalue\n"),
            Some(Value::Integer(4))
        );
        assert_eq!(
            evaluate("mutable value = 9\nvalue %= 4\nvalue\n"),
            Some(Value::Integer(1))
        );
        assert_eq!(
            evaluate("mutable word = \"port\"\nword += \"land\"\nword\n"),
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
    #[should_panic(expected = "&& needs true or false")]
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
        let source = "mutable number = 0\nwhile true\n  number = number + 1\n  break if number == 4\nend\nnumber\n";
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
        let source = "def find_first_multiple(of)\n  mutable number = 1\n  while true\n    if number % of == 0\n      return number\n    end\n    number = number + 1\n  end\nend\nfind_first_multiple(7)\n";
        assert_eq!(evaluate(source), Some(Value::Integer(7)));
    }

    #[test]
    fn next_skips_to_the_following_iteration() {
        let source = "mutable number = 0\nmutable total = 0\nwhile number < 5\n  number += 1\n  next if number.even?\n  total += number\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(9)));
    }

    #[test]
    #[should_panic(expected = "next outside of a loop")]
    fn panics_on_a_top_level_next() {
        evaluate("next");
    }

    #[test]
    fn break_exits_a_while_loop() {
        let source = "mutable number = 0\nwhile true\n  number = number + 1\n  if number == 3\n    break\n  end\nend\nnumber\n";
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
    fn break_stops_block_iteration() {
        let source = "mutable total = 0\n[1, 2, 3, 4].each do |number|\n  break if number == 3\n  total += number\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    /// ADR 0016: `{ ... }` and `do ... end` are dead-identical.
    #[test]
    fn brace_blocks_match_do_end_blocks() {
        assert_eq!(
            evaluate("[1, 2, 3].map { |number| number * 2 }"),
            evaluate("[1, 2, 3].map do |number|\n  number * 2\nend\n")
        );
    }

    #[test]
    fn brace_blocks_chain_and_nest() {
        assert_eq!(
            evaluate(r#"["a", "b"].map { |word| word.upcase }.join("-")"#),
            Some(Value::String("A-B".to_string()))
        );
        assert_eq!(
            evaluate("[[1, 2], [3]].map { |pair| pair.map { |number| number * 2 } }.length"),
            Some(Value::Integer(2))
        );
    }

    #[test]
    fn brace_blocks_hold_several_statements() {
        let source = "[1, 2].map { |number|\n  doubled = number * 2\n  doubled + 1\n}\n";
        assert_eq!(
            evaluate(source),
            Some(Value::Array(std::rc::Rc::new(vec![
                Value::Integer(3),
                Value::Integer(5)
            ])))
        );
    }

    /// A `{` in expression position is still a hash literal, not a block.
    #[test]
    fn brace_blocks_do_not_shadow_hash_literals() {
        assert_eq!(evaluate(r#"{"a" => 1}.length"#), Some(Value::Integer(1)));
    }

    /// ADR 0017: naming `it` declares the block's implicit parameter.
    #[test]
    fn it_is_the_implicit_block_parameter() {
        assert_eq!(
            evaluate(r#"["a", "b"].map { it.upcase }.join("-")"#),
            Some(Value::String("A-B".to_string()))
        );
        // Same rule in a do/end block — the forms are dead-identical.
        assert_eq!(
            evaluate("[1, 2, 3].select do\n  it.odd?\nend.length\n"),
            Some(Value::Integer(2))
        );
    }

    /// An enclosing block with named parameters puts no `it` in scope, so
    /// the inner block may claim it.
    #[test]
    fn it_nests_under_a_named_outer_block() {
        assert_eq!(
            evaluate("[[1, 2]].map { |pair| pair.map { it * 2 } }.length"),
            Some(Value::Integer(1))
        );
    }

    #[test]
    #[should_panic(expected = "use one or the other")]
    fn panics_when_it_mixes_with_declared_parameters() {
        evaluate(r#"["a"].map { |word| it.upcase }"#);
    }

    /// Nesting is shadowing, and shadows are errors.
    #[test]
    #[should_panic(expected = "already a nested block's parameter")]
    fn panics_on_nested_it() {
        evaluate("[[1]].map { it.map { it } }");
    }

    #[test]
    #[should_panic(expected = "`it` is a local here and a block parameter there")]
    fn panics_when_it_collides_with_a_local() {
        evaluate("it = 5\n[\"a\"].map { it.upcase }\n");
    }

    /// But an uncontested local named `it` is perfectly fine.
    #[test]
    fn a_local_named_it_is_allowed_when_no_block_claims_it() {
        assert_eq!(evaluate("it = 5\nit + 1\n"), Some(Value::Integer(6)));
    }

    /// ADR 0016: the one ambiguous position names every reading. With a
    /// `=>` inside, the hash reading survives, so all three are offered.
    #[test]
    #[should_panic(expected = "could be three things")]
    fn panics_on_a_brace_after_a_command_call() {
        evaluate("def render(x)\n  x\nend\ndef config\n  1\nend\nrender config { \"a\" => 1 }\n");
    }

    /// A `|` rules the hash out, so only the two block owners are offered.
    #[test]
    #[should_panic(expected = "is a block — but whose?")]
    fn panics_on_a_block_brace_after_a_command_call() {
        evaluate("def render(x)\n  x\nend\ndef config\n  1\nend\nrender config { |item| item }\n");
    }

    /// No `=>` rules the hash out too, even without block parameters.
    #[test]
    #[should_panic(expected = "is a block — but whose?")]
    fn panics_on_a_parameterless_block_brace_after_a_command_call() {
        evaluate("def render(x)\n  x\nend\ndef config\n  1\nend\nrender config { config }\n");
    }

    /// And the parens the error suggests do resolve it.
    #[test]
    fn parens_resolve_the_brace_ambiguity() {
        let source = "def render(x)\n  x\nend\nputs render({\"a\" => 1}).length\n";
        assert_eq!(evaluate(source), None);
        assert_eq!(
            evaluate(r#"["a"].map { |word| word.upcase }.join("-")"#),
            Some(Value::String("A".to_string()))
        );
    }

    #[test]
    fn a_broken_call_produces_nil() {
        assert_eq!(
            evaluate("[1, 2].each do |number|\n  break\nend\n"),
            Some(Value::Nil)
        );
    }

    #[test]
    fn next_skips_a_block_iteration() {
        let source = "mutable total = 0\n[1, 2, 3, 4].each do |number|\n  next if number.even?\n  total += number\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(4)));
    }

    #[test]
    fn return_unwinds_through_a_block_to_the_enclosing_method() {
        let source = "def first_even(numbers)\n  numbers.each do |number|\n    return number if number.even?\n  end\n  0 - 1\nend\n";
        assert_eq!(
            evaluate(&format!("{source}first_even([1, 3, 4, 5])\n")),
            Some(Value::Integer(4))
        );
        assert_eq!(
            evaluate(&format!("{source}first_even([1, 3])\n")),
            Some(Value::Integer(-1))
        );
    }

    #[test]
    fn break_stops_times() {
        let source = "mutable count = 0\n5.times do |index|\n  break if index == 2\n  count += 1\nend\ncount\n";
        assert_eq!(evaluate(source), Some(Value::Integer(2)));
    }

    #[test]
    fn break_stops_select() {
        let source = "[1, 2, 3].select do |number|\n  break\n  true\nend\n";
        assert_eq!(evaluate(source), Some(Value::Nil));
    }

    #[test]
    #[should_panic(expected = "return outside of a method")]
    fn panics_on_a_top_level_return_from_a_block() {
        evaluate("[1].each do |number|\n  return number\nend\n");
    }

    #[test]
    fn each_iterates_with_a_closure_over_the_enclosing_scope() {
        let source =
            "mutable total = 0\n[1, 2, 3].each do |number|\n  total = total + number\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(6)));
    }

    #[test]
    fn each_returns_its_receiver() {
        let source = "[1, 2].each do |number|\n  number\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::array(vec![Value::Integer(1), Value::Integer(2)]))
        );
    }

    #[test]
    fn map_builds_a_new_array() {
        let source = "[1, 2, 3].map do |number|\n  number * number\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::array(vec![
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
            Some(Value::array(vec![
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
            Some(Value::array(vec![Value::Integer(2), Value::Integer(3)]))
        );
        assert_eq!(
            evaluate("[1, 2].slice(1, 99)"),
            Some(Value::array(vec![Value::Integer(2)]))
        );
    }

    #[test]
    fn select_keeps_matching_elements() {
        let source = "[1, 2, 3, 4].select do |number|\n  number.even?\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::array(vec![Value::Integer(2), Value::Integer(4)]))
        );
    }

    #[test]
    fn reject_drops_matching_elements() {
        let source = "[1, 2, 3, 4].reject do |number|\n  number.even?\nend\n";
        assert_eq!(
            evaluate(source),
            Some(Value::array(vec![Value::Integer(1), Value::Integer(3)]))
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
        let source = "mutable sum = 0\n3.times do |index|\n  sum = sum + index\nend\nsum\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    #[test]
    fn times_block_may_ignore_its_argument() {
        let source = "mutable count = 0\n3.times do\n  count = count + 1\nend\ncount\n";
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
            Some(Value::array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ]))
        );
        assert_eq!(
            evaluate(&format!("{hash}.values")),
            Some(Value::array(vec![Value::Integer(1), Value::Integer(2)]))
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
            Some(Value::array(vec![
                Value::Integer(1),
                Value::Integer(5),
                Value::String("pdx".to_string()),
            ]))
        );
        assert_eq!(evaluate("[]"), Some(Value::array(vec![])));
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
    fn concatenates_arrays_with_plus() {
        assert_eq!(
            evaluate("[1] + [2, 3]"),
            Some(Value::array(vec![
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
            Some(Value::array(vec![
                Value::String("p".to_string()),
                Value::String("d".to_string()),
                Value::String("x".to_string()),
            ]))
        );
        assert_eq!(
            evaluate(r#""a,b".split(",")"#),
            Some(Value::array(vec![
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

    const TOKEN_STRUCT: &str = "struct Token\n  kind\n  text\nend\n";

    #[test]
    fn maybe_predicates_work_on_structs() {
        let source =
            format!("{TOKEN_STRUCT}token = Token.new(kind: \"a\", text: \"b\")\ntoken.some?");
        assert_eq!(evaluate(&source), Some(Value::Boolean(true)));
        let source =
            format!("{TOKEN_STRUCT}token = Token.new(kind: \"a\", text: \"b\")\ntoken.nil?");
        assert_eq!(evaluate(&source), Some(Value::Boolean(false)));
    }

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
            Some(Value::array(vec![
                Value::String("rose".to_string()),
                Value::String("city".to_string()),
            ]))
        );
        assert_eq!(evaluate("%w[]"), Some(Value::array(vec![])));
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
            Some(Value::array(vec![
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
    fn if_without_else_produces_nil_when_false() {
        assert_eq!(evaluate("if false\n  1\nend\n"), Some(Value::Nil));
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
        let source =
            "mutable number = 3\nwhile number > 0\n  puts(number)\n  number = number - 1\nend\n";
        assert_eq!(output_of(source), "3\n2\n1\n");
    }

    #[test]
    fn while_computes_a_factorial() {
        let source = "def factorial(mutable number)\n  mutable result = 1\n  while number > 1\n    result = result * number\n    number = number - 1\n  end\n  result\nend\nfactorial(10)\n";
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

    /// ADR 0021: `module` namespaces; `::` names, `.` invokes.
    #[test]
    fn modules_namespace_their_contents() {
        let source = "module Statistics\n  LIMIT = 10\n\n  struct Summary\n    mean\n  end\n\n  def mean(values)\n    values.sum / values.length\n  end\nend\nStatistics.mean([1, 2, 3]) + Statistics::LIMIT + Statistics::Summary.new(mean: 5).mean\n";
        assert_eq!(evaluate(source), Some(Value::Integer(17)));
    }

    /// The two declaration spellings are identical — including lexical
    /// visibility of enclosing names, which is where Ruby differs.
    #[test]
    fn both_module_forms_mean_the_same_thing() {
        let nested = "module Outer\n  LIMIT = 4\n  module Inner\n    def reach\n      LIMIT\n    end\n  end\nend\nOuter::Inner.reach\n";
        let path = "module Outer\n  LIMIT = 4\nend\nmodule Outer::Inner\n  def reach\n    LIMIT\n  end\nend\nOuter::Inner.reach\n";
        assert_eq!(evaluate(nested), Some(Value::Integer(4)));
        // Ruby raises NameError for the path form here; Portland does not.
        assert_eq!(evaluate(path), Some(Value::Integer(4)));
    }

    /// Bare names resolve outward from where a method was *written*.
    #[test]
    fn names_resolve_outward_from_their_home() {
        let source = "module Shapes\n  struct Circle\n    radius\n  end\n\n  def unit\n    Circle.new(radius: 1)\n  end\nend\nShapes.unit.radius\n";
        assert_eq!(evaluate(source), Some(Value::Integer(1)));
    }

    /// ADR 0021 §5: a type nests in a type.
    #[test]
    fn types_nest_in_types() {
        let source = "struct Invoice\n  total\n\n  struct Line\n    amount\n  end\nend\nInvoice::Line.new(amount: 9).amount\n";
        assert_eq!(evaluate(source), Some(Value::Integer(9)));
    }

    #[test]
    #[should_panic(expected = "`::` names, `.` invokes")]
    fn panics_when_a_path_is_used_to_invoke() {
        evaluate("module S\n  def mean(v)\n    1\n  end\nend\nS::mean([1])\n");
    }

    #[test]
    #[should_panic(expected = "modules don't nest inside structs")]
    fn panics_on_a_module_inside_a_struct() {
        evaluate("struct Foo\n  bar\n  module Helpers\n  end\nend\n");
    }

    #[test]
    #[should_panic(expected = "module names start with a capital letter")]
    fn panics_on_a_lowercase_module_name() {
        evaluate("module stats\nend\n");
    }

    /// ADR 0019: `..` inclusive, `...` exclusive, either end optional.
    #[test]
    fn evaluates_range_literals() {
        assert_eq!(
            evaluate("(1..5).to_s"),
            Some(Value::String("1..5".to_string()))
        );
        assert_eq!(
            evaluate("(1...5).to_s"),
            Some(Value::String("1...5".to_string()))
        );
        assert_eq!(
            evaluate("(1..).to_s"),
            Some(Value::String("1..".to_string()))
        );
        assert_eq!(
            evaluate("(..5).to_s"),
            Some(Value::String("..5".to_string()))
        );
    }

    /// A slice is always a collection, never a maybe (ADR 0019 §2). The
    /// last two rows are the deliberate divergence: Ruby answers nil.
    #[test]
    fn range_slices_are_collections_not_maybes() {
        assert_eq!(evaluate("[1, 2, 3][1..99].length"), Some(Value::Integer(2)));
        assert_eq!(evaluate("[1, 2, 3][3..].length"), Some(Value::Integer(0)));
        assert_eq!(evaluate("[1, 2, 3][2..1].length"), Some(Value::Integer(0)));
        assert_eq!(evaluate("[1, 2, 3][..1].length"), Some(Value::Integer(2)));
        assert_eq!(
            evaluate(r#""hello"[1..3]"#),
            Some(Value::String("ell".to_string()))
        );
        // Ruby: nil. Portland: empty — the start clamps like the end.
        assert_eq!(evaluate("[1, 2, 3][4..].length"), Some(Value::Integer(0)));
        assert_eq!(evaluate("[1, 2, 3][-99..].length"), Some(Value::Integer(3)));
        assert_eq!(
            evaluate(r#""hello"[9..]"#),
            Some(Value::String(String::new()))
        );
    }

    /// `(1..n).each` is the counted loop — the main reason ranges are worth
    /// having this early.
    #[test]
    fn ranges_iterate() {
        assert_eq!(evaluate("(1..4).sum"), Some(Value::Integer(10)));
        assert_eq!(evaluate("(1..3).to_a.length"), Some(Value::Integer(3)));
        assert_eq!(evaluate("(1...5).to_a.length"), Some(Value::Integer(4)));
        assert_eq!(
            evaluate("mutable total = 0\n(1..4).each { total += it }\ntotal\n"),
            Some(Value::Integer(10))
        );
    }

    /// `include?` answers without walking, so the unbounded forms can too.
    #[test]
    fn ranges_answer_membership_without_walking() {
        assert_eq!(evaluate("(1..5).include?(3)"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("(1...5).include?(5)"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("(1..).include?(999)"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("(..5).include?(-99)"), Some(Value::Boolean(true)));
    }

    /// Range patterns test membership, not equality (ADR 0019 §1) — and a
    /// beginless-through-endless chain is the shape that will one day
    /// prove exhaustive without an `else`.
    #[test]
    fn range_patterns_match_by_membership() {
        let source = "def size(number)\n  case number\n  in ..0 then \"none\"\n  in 1..9 then \"some\"\n  in 10.. then \"lots\"\n  end\nend\nsize(-5) + size(3) + size(50)\n";
        assert_eq!(
            evaluate(source),
            Some(Value::String("nonesomelots".to_string()))
        );
    }

    /// ADR 0019 §3: a range spans a newline only where one reading exists.
    #[test]
    #[should_panic(expected = "endless range at end of line")]
    fn panics_on_an_ambiguous_endless_range() {
        evaluate("span = 1..\np span\n");
    }

    /// And the unambiguous positions need no parens at all.
    #[test]
    fn endless_ranges_close_on_a_token_that_cannot_continue_them() {
        assert_eq!(evaluate("[1, 2, 3][1..].length"), Some(Value::Integer(2)));
        assert_eq!(
            evaluate("case 50\nin 10.. then \"big\"\nend\n"),
            Some(Value::String("big".to_string()))
        );
    }

    /// ADR 0018: IEEE doubles, Ruby's printing, mixed arithmetic promotes.
    /// Every expectation below was checked against Ruby 4.0.6.
    #[test]
    fn evaluates_floats() {
        assert_eq!(evaluate("2.75"), Some(Value::Float(2.75)));
        assert_eq!(evaluate("-1.5"), Some(Value::Float(-1.5)));
        assert_eq!(evaluate("1.5.to_s"), Some(Value::String("1.5".to_string())));
    }

    /// A float always shows its point, so `1.0` never renders as `1`.
    #[test]
    fn floats_keep_their_point_when_printed() {
        assert_eq!(Value::Float(1.0).inspect(), "1.0");
        assert_eq!(Value::Float(1.0).to_string(), "1.0");
        assert_eq!(Value::Float(2.75).inspect(), "2.75");
    }

    #[test]
    fn mixed_arithmetic_promotes_to_float() {
        assert_eq!(evaluate("7 / 2.0"), Some(Value::Float(3.5)));
        assert_eq!(evaluate("7.0 / 2"), Some(Value::Float(3.5)));
        assert_eq!(evaluate("2.5 + 1"), Some(Value::Float(3.5)));
        assert_eq!(evaluate("1 + 2.5"), Some(Value::Float(3.5)));
        assert_eq!(evaluate("2.5 * 2"), Some(Value::Float(5.0)));
        // Integer division still floors — only a float makes `/` real.
        assert_eq!(evaluate("7 / 2"), Some(Value::Integer(3)));
    }

    /// Float modulo takes the sign of the divisor, like the integer one.
    #[test]
    fn float_modulo_matches_ruby() {
        assert_eq!(evaluate("7.5 % 2"), Some(Value::Float(1.5)));
        assert_eq!(evaluate("-7.5 % 2"), Some(Value::Float(0.5)));
        assert_eq!(evaluate("7.5 % -2"), Some(Value::Float(-0.5)));
    }

    #[test]
    fn floats_compare_across_the_numeric_types() {
        assert_eq!(evaluate("1.0 == 1"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("1.5 < 2"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("2.5 > 3"), Some(Value::Boolean(false)));
    }

    /// The lexer only makes a float when a digit follows the dot, which is
    /// what leaves `1..5` available to ranges (ADR 0019).
    #[test]
    fn a_dot_without_a_digit_is_not_a_float() {
        assert_eq!(evaluate("1.to_s"), Some(Value::String("1".to_string())));
    }

    /// Ruby floors; Rust truncates. ADR 0018 picks Ruby, so the two
    /// mixed-sign quotients round toward negative infinity.
    #[test]
    fn division_floors_like_ruby() {
        assert_eq!(evaluate("-7 / 2"), Some(Value::Integer(-4)));
        assert_eq!(evaluate("7 / -2"), Some(Value::Integer(-4)));
        assert_eq!(evaluate("-7 / -2"), Some(Value::Integer(3)));
        assert_eq!(evaluate("-6 / 2"), Some(Value::Integer(-3)));
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

    /// The remainder takes the sign of the divisor (ADR 0018) — Ruby's
    /// rule, and the identity `(a / b) * b + (a % b) == a` holds.
    #[test]
    fn modulo_takes_the_sign_of_the_divisor_like_ruby() {
        assert_eq!(evaluate("-7 % 2"), Some(Value::Integer(1)));
        assert_eq!(evaluate("7 % -2"), Some(Value::Integer(-1)));
        assert_eq!(evaluate("-7 % -2"), Some(Value::Integer(-1)));
        assert_eq!(evaluate("-6 % 2"), Some(Value::Integer(0)));
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
    fn command_calls_work_at_statement_position() {
        assert_eq!(output_of("puts \"hello\""), "hello\n");
        assert_eq!(output_of("puts \"a\", \"b\""), "a\nb\n");
        assert_eq!(output_of("puts 1 + 2"), "3\n");
        assert_eq!(output_of("name = \"pdx\"\nputs name\n"), "pdx\n");
        assert_eq!(output_of("puts true"), "true\n");
        assert_eq!(output_of("puts %w[rose city]"), "[rose, city]\n");
        assert_eq!(
            output_of("def shout(word)\n  puts(word.upcase)\nend\nshout \"pdx\"\n"),
            "PDX\n"
        );
    }

    #[test]
    fn command_calls_take_postfix_guards() {
        assert_eq!(output_of("puts \"hi\" if false"), "");
        assert_eq!(output_of("puts \"hi\" unless false"), "hi\n");
    }

    #[test]
    #[should_panic(expected = "ambiguous without parens")]
    fn panics_on_a_glued_minus_argument() {
        evaluate("puts -1");
    }

    #[test]
    #[should_panic(expected = "ambiguous without parens")]
    fn panics_on_a_spaced_bracket_argument() {
        evaluate("puts [1]");
    }

    #[test]
    #[should_panic(expected = "ambiguous without parens")]
    fn panics_on_a_spaced_paren_argument() {
        evaluate("puts (1)");
    }

    #[test]
    #[should_panic(expected = "blocks on paren-less calls")]
    fn panics_on_a_block_after_a_command_call() {
        evaluate("puts \"x\" do\n  1\nend\n");
    }

    #[test]
    fn spaced_minus_stays_subtraction() {
        assert_eq!(evaluate("total = 10\ntotal - 1\n"), Some(Value::Integer(9)));
    }

    #[test]
    fn bare_zero_argument_calls_resolve_to_methods() {
        let source = "def pdx\n  \"rose city\"\nend\npdx\n";
        assert_eq!(
            evaluate(source),
            Some(Value::String("rose city".to_string()))
        );
    }

    #[test]
    fn bare_question_mark_methods_read_as_prose() {
        let source = "def ready?\n  true\nend\nputs \"go\" if ready?\n";
        assert_eq!(output_of(source), "go\n");
    }

    #[test]
    fn bare_builtins_resolve_too() {
        assert_eq!(evaluate("argv.length"), Some(Value::Integer(0)));
    }

    #[test]
    fn locals_still_win_when_no_method_exists() {
        assert_eq!(
            evaluate("greeting = \"hi\"\ngreeting\n"),
            Some(Value::String("hi".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "local greet shadows method greet")]
    fn panics_when_a_local_shadows_a_method() {
        evaluate("def greet\n  1\nend\ngreet = 2\n");
    }

    #[test]
    #[should_panic(expected = "method greet shadows local greet")]
    fn panics_when_a_method_shadows_a_local() {
        evaluate("greet = 2\ndef greet\n  1\nend\n");
    }

    #[test]
    #[should_panic(expected = "parameter greet shadows method greet")]
    fn panics_when_a_parameter_shadows_a_method() {
        evaluate("def greet\n  1\nend\ndef call_it(greet)\n  greet\nend\ncall_it(5)\n");
    }

    #[test]
    #[should_panic(expected = "shadows method puts")]
    fn panics_when_a_local_shadows_a_builtin() {
        evaluate("puts = 1");
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
            Some(Value::array(vec![Value::Integer(3), Value::Integer(6)]))
        );
    }

    #[test]
    #[should_panic(expected = "expects 1 to 2 argument(s)")]
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
    fn evaluates_a_nil_literal() {
        assert_eq!(evaluate("nil"), Some(Value::Nil));
    }

    #[test]
    fn nil_equals_only_nil() {
        assert_eq!(evaluate("nil == nil"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("nil == 1"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("1 != nil"), Some(Value::Boolean(true)));
    }

    #[test]
    fn p_renders_nil() {
        assert_eq!(output_of("p(nil)"), "nil\n");
    }

    #[test]
    #[should_panic(expected = "handle the nil case")]
    fn puts_rejects_nil() {
        evaluate("puts(nil)");
    }

    #[test]
    fn nil_predicate_answers_absence() {
        assert_eq!(evaluate("nil.nil?"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("1.nil?"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("\"rose\".nil?"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("false.nil?"), Some(Value::Boolean(false)));
    }

    #[test]
    fn some_predicate_answers_presence() {
        assert_eq!(evaluate("nil.some?"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("1.some?"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("[].some?"), Some(Value::Boolean(true)));
    }

    #[test]
    #[should_panic(expected = "nil has no method upcase")]
    fn nil_refuses_ordinary_methods() {
        evaluate("nil.upcase");
    }

    #[test]
    fn or_on_nil_yields_the_right_side() {
        assert_eq!(evaluate("nil || 5"), Some(Value::Integer(5)));
        assert_eq!(
            evaluate("nil || \"teal\""),
            Some(Value::String("teal".to_string()))
        );
        assert_eq!(evaluate("nil || nil"), Some(Value::Nil));
    }

    #[test]
    fn or_on_a_present_value_yields_it_and_skips_the_right_side() {
        // Real Portland flags a never-absent left as dead code at compile
        // time; the seed's runtime can only do the unwrap-or-else half.
        assert_eq!(evaluate("3 || 5"), Some(Value::Integer(3)));
        assert_eq!(evaluate("3 || nope()"), Some(Value::Integer(3)));
    }

    #[test]
    fn or_on_booleans_stays_logical() {
        assert_eq!(evaluate("false || true"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("true || false"), Some(Value::Boolean(true)));
    }

    #[test]
    #[should_panic(expected = "must be true or false")]
    fn or_on_false_needs_a_boolean_right_side() {
        evaluate("false || 5");
    }

    #[test]
    #[should_panic(expected = "&& needs true or false")]
    fn and_refuses_nil() {
        evaluate("nil && true");
    }

    #[test]
    fn word_operators_are_dead_identical_to_their_sigils() {
        assert_eq!(evaluate("nil or 5"), Some(Value::Integer(5)));
        assert_eq!(evaluate("3 or nope()"), Some(Value::Integer(3)));
        assert_eq!(evaluate("false and true"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("true and true"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("not true"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("not not false"), Some(Value::Boolean(false)));
    }

    #[test]
    fn word_or_has_sigil_precedence_not_rubys() {
        // Ruby parses `x = nil or 7` as `(x = nil) or 7`; Portland's or is
        // dead-identical to ||, so this is `x = (nil or 7)` (ADR 0007).
        assert_eq!(evaluate("x = nil or 7\nx"), Some(Value::Integer(7)));
    }

    #[test]
    fn or_return_guards_a_method() {
        let absent = "def bump(x)\n  value = x or return 0\n  value + 1\nend\nbump(nil)\n";
        assert_eq!(evaluate(absent), Some(Value::Integer(0)));
        let present = "def bump(x)\n  value = x or return 0\n  value + 1\nend\nbump(41)\n";
        assert_eq!(evaluate(present), Some(Value::Integer(42)));
    }

    #[test]
    fn bare_or_return_produces_no_value() {
        let source = "def check(x)\n  value = x or return\n  value\nend\ncheck(nil)\n";
        assert_eq!(evaluate(source), None);
    }

    #[test]
    fn or_break_leaves_the_loop() {
        let source =
            "mutable total = 0\nwhile true\n  total += 1\n  x = nil or break\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(1)));
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn or_panic_asserts_with_a_message() {
        evaluate("x = nil or panic \"boom\"");
    }

    #[test]
    fn or_panic_is_skipped_when_present() {
        assert_eq!(
            evaluate("x = 5 or panic \"boom\"\nx"),
            Some(Value::Integer(5))
        );
    }

    #[test]
    #[should_panic(expected = "kaboom")]
    fn panic_builtin_works_at_statement_position() {
        evaluate("panic \"kaboom\"");
    }

    #[test]
    fn partial_operations_return_nil_instead_of_panicking() {
        assert_eq!(evaluate("[].first"), Some(Value::Nil));
        assert_eq!(evaluate("[].last"), Some(Value::Nil));
        assert_eq!(evaluate("[].min"), Some(Value::Nil));
        assert_eq!(evaluate("[].max"), Some(Value::Nil));
        assert_eq!(evaluate("[1, 2][9]"), Some(Value::Nil));
        assert_eq!(evaluate("[1, 2][-9]"), Some(Value::Nil));
        assert_eq!(evaluate("\"pdx\"[9]"), Some(Value::Nil));
        assert_eq!(evaluate("{\"a\" => 1}[\"zzz\"]"), Some(Value::Nil));
    }

    #[test]
    fn partial_operations_still_answer_when_they_can() {
        assert_eq!(evaluate("[7, 8].first"), Some(Value::Integer(7)));
        assert_eq!(evaluate("[1, 2][-1]"), Some(Value::Integer(2)));
        assert_eq!(evaluate("{\"a\" => 1}[\"a\"]"), Some(Value::Integer(1)));
    }

    #[test]
    fn lookups_compose_with_the_or_guard() {
        assert_eq!(evaluate("[].first or 42"), Some(Value::Integer(42)));
        assert_eq!(
            evaluate("theme = {\"a\" => 1}[\"theme\"] or \"teal\"\ntheme"),
            Some(Value::String("teal".to_string()))
        );
    }

    #[test]
    fn safe_navigation_passes_nil_through() {
        assert_eq!(evaluate("nil&.upcase"), Some(Value::Nil));
        // The arguments never evaluate when the receiver is absent.
        assert_eq!(evaluate("nil&.include?(nope())"), Some(Value::Nil));
    }

    #[test]
    fn safe_navigation_calls_through_on_a_present_receiver() {
        assert_eq!(
            evaluate("\"pdx\"&.upcase"),
            Some(Value::String("PDX".to_string()))
        );
    }

    #[test]
    fn safe_navigation_chains_into_the_or_guard() {
        let source = "name = {\"a\" => \"pdx\"}[\"b\"]&.upcase or \"ROSE\"\nname";
        assert_eq!(evaluate(source), Some(Value::String("ROSE".to_string())));
    }

    #[test]
    fn some_is_identity_on_plain_present_values() {
        // Never ceremonial (ADR 0005): wrapping a plain value is a no-op.
        assert_eq!(evaluate("some(5)"), Some(Value::Integer(5)));
        assert_eq!(
            evaluate("some(\"pdx\")"),
            Some(Value::String("pdx".to_string()))
        );
    }

    #[test]
    fn some_wraps_only_where_nesting_is_load_bearing() {
        assert_eq!(
            evaluate("some(nil)"),
            Some(Value::Some(Box::new(Value::Nil)))
        );
        assert_eq!(
            evaluate("some(some(nil))"),
            Some(Value::Some(Box::new(Value::Some(Box::new(Value::Nil)))))
        );
    }

    #[test]
    fn first_distinguishes_empty_from_containing_nil() {
        // Exhibit A come home: [].first found nothing; [nil].first found
        // an absent value. The wrapper keeps them apart (ADR 0005).
        assert_eq!(evaluate("[].first"), Some(Value::Nil));
        assert_eq!(
            evaluate("[nil].first"),
            Some(Value::Some(Box::new(Value::Nil)))
        );
    }

    #[test]
    fn hash_lookup_distinguishes_missing_key_from_stored_nil() {
        assert_eq!(evaluate("{\"a\" => nil}[\"zzz\"]"), Some(Value::Nil));
        assert_eq!(
            evaluate("{\"a\" => nil}[\"a\"]"),
            Some(Value::Some(Box::new(Value::Nil)))
        );
    }

    #[test]
    fn or_unwraps_one_layer_preserving_fetch_semantics() {
        // A stored nil is present: the or-guard hands over the inner nil,
        // not the default (ADR 0010's fetch table).
        assert_eq!(
            evaluate("{\"a\" => nil}[\"a\"] or \"default\""),
            Some(Value::Nil)
        );
        assert_eq!(
            evaluate("{\"a\" => nil}[\"zzz\"] or \"default\""),
            Some(Value::String("default".to_string()))
        );
    }

    #[test]
    fn some_of_nil_is_present() {
        assert_eq!(evaluate("some(nil).nil?"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("some(nil).some?"), Some(Value::Boolean(true)));
    }

    #[test]
    fn p_renders_the_nested_case() {
        assert_eq!(output_of("p(some(nil))"), "some(nil)\n");
    }

    #[test]
    fn branchless_if_produces_nil() {
        assert_eq!(evaluate("if false\n  5\nend"), Some(Value::Nil));
        assert_eq!(evaluate("x = if false\n  5\nend\nx"), Some(Value::Nil));
        assert_eq!(evaluate("if true\n  5\nend"), Some(Value::Integer(5)));
        // An else that exists but is empty is the same absence of an answer.
        assert_eq!(evaluate("if true\nelse\n  5\nend"), Some(Value::Nil));
    }

    #[test]
    fn branchless_if_composes_with_the_or_guard() {
        assert_eq!(
            evaluate("greeting = if false\n  \"gm\"\nend\ngreeting or \"hello\""),
            Some(Value::String("hello".to_string()))
        );
    }

    #[test]
    fn while_produces_nil() {
        assert_eq!(
            evaluate("mutable n = 0\nwhile n < 3\n  n += 1\nend"),
            Some(Value::Nil)
        );
    }

    #[test]
    fn a_broken_out_call_produces_nil() {
        let source = "found = [1, 2, 3].each do |number|\n  break\nend\nfound";
        assert_eq!(evaluate(source), Some(Value::Nil));
    }

    #[test]
    fn keyword_arguments_on_methods() {
        let source = "def greet(name:, greeting: \"hi\")\n  \"#{greeting} #{name}\"\nend\ngreet(name: \"pdx\")\n";
        assert_eq!(evaluate(source), Some(Value::String("hi pdx".to_string())));
        let source = "def greet(name:, greeting: \"hi\")\n  \"#{greeting} #{name}\"\nend\ngreet(greeting: \"yo\", name: \"pdx\")\n";
        assert_eq!(evaluate(source), Some(Value::String("yo pdx".to_string())));
    }

    #[test]
    fn keyword_and_positional_parameters_mix() {
        let source =
            "def tag(word, separator: \"-\")\n  word + separator + word\nend\ntag(\"go\")\n";
        assert_eq!(evaluate(source), Some(Value::String("go-go".to_string())));
        let source = "def tag(word, separator: \"-\")\n  word + separator + word\nend\ntag(\"go\", separator: \"+\")\n";
        assert_eq!(evaluate(source), Some(Value::String("go+go".to_string())));
    }

    #[test]
    fn keyword_arguments_work_on_command_calls() {
        let source = "def shout(word:)\n  puts(word.upcase)\nend\nshout word: \"pdx\"\n";
        assert_eq!(output_of(source), "PDX\n");
    }

    #[test]
    fn keyword_defaults_may_reference_earlier_parameters() {
        let source = "def frame(word, edge: word)\n  edge + word + edge\nend\nframe(\"o\")\n";
        assert_eq!(evaluate(source), Some(Value::String("ooo".to_string())));
    }

    #[test]
    #[should_panic(expected = "missing keyword argument name")]
    fn panics_on_a_missing_required_keyword() {
        evaluate("def greet(name:)\n  name\nend\ngreet()\n");
    }

    #[test]
    #[should_panic(expected = "unknown keyword argument extra")]
    fn panics_on_an_unknown_keyword() {
        evaluate("def greet(name:)\n  name\nend\ngreet(name: \"x\", extra: 1)\n");
    }

    #[test]
    #[should_panic(expected = "expects")]
    fn keyword_parameters_do_not_take_positional_values() {
        evaluate("def greet(name:)\n  name\nend\ngreet(\"x\")\n");
    }

    #[test]
    fn case_in_matches_literals_and_nil() {
        let source = "case [].first\nin 1 then \"one\"\nin nil then \"empty\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("empty".to_string())));
        let source = "case 1\nin 1 then \"one\"\nin nil then \"empty\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("one".to_string())));
    }

    #[test]
    fn case_in_captures_bind_and_persist() {
        // A bare lowercase pattern captures, Ruby-style (ADR 0013 §3), and
        // the binding persists past the case, like Ruby's.
        let source = "case 41\nin nil then 0\nin found then found + 1\nend\nfound\n";
        assert_eq!(evaluate(source), Some(Value::Integer(41)));
        let source = "case 41\nin nil then 0\nin found then found + 1\nend\n";
        assert_eq!(evaluate(source), Some(Value::Integer(42)));
    }

    #[test]
    fn case_in_alternatives() {
        let source = "case 2\nin 1 | 2 | 3 then \"few\"\nelse\n  \"many\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("few".to_string())));
        let source = "case 9\nin 1 | 2 | 3 then \"few\"\nelse\n  \"many\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("many".to_string())));
    }

    #[test]
    #[should_panic(expected = "no pattern matched")]
    fn case_in_without_a_match_panics() {
        // The runtime preview of compile-checked exhaustiveness (ADR 0013 §1).
        evaluate("case 9\nin 1 then \"one\"\nend\n");
    }

    #[test]
    #[should_panic(expected = "shadows method")]
    fn case_in_captures_obey_no_shadow() {
        evaluate("def taken\n  1\nend\ncase 5\nin taken then taken\nend\n");
    }

    const NODE_STRUCTS: &str = "struct ReturnNode\n  value\nend\nstruct BreakNode\n  label\nend\n";

    #[test]
    fn case_in_struct_patterns_match_by_type() {
        let source = format!(
            "{NODE_STRUCTS}node = BreakNode.new(label: \"b\")\ncase node\nin ReturnNode then \"return\"\nin BreakNode then \"break\"\nend\n"
        );
        assert_eq!(evaluate(&source), Some(Value::String("break".to_string())));
    }

    #[test]
    fn case_in_struct_patterns_refine_by_field_value() {
        let source = format!(
            "{NODE_STRUCTS}node = ReturnNode.new(value: nil)\ncase node\nin ReturnNode(value: nil) then \"(return)\"\nin ReturnNode(value:) then value\nend\n"
        );
        assert_eq!(
            evaluate(&source),
            Some(Value::String("(return)".to_string()))
        );
    }

    #[test]
    fn case_in_struct_patterns_bind_fields_shorthand_and_named() {
        let source = format!(
            "{NODE_STRUCTS}node = ReturnNode.new(value: 5)\ncase node\nin ReturnNode(value: nil) then \"(return)\"\nin ReturnNode(value:) then value + 1\nend\n"
        );
        assert_eq!(evaluate(&source), Some(Value::Integer(6)));
        let source = format!(
            "{NODE_STRUCTS}node = ReturnNode.new(value: 5)\ncase node\nin ReturnNode(value: held) then held * 2\nend\n"
        );
        assert_eq!(evaluate(&source), Some(Value::Integer(10)));
    }

    #[test]
    #[should_panic(expected = "no pattern matched")]
    fn case_in_struct_patterns_miss_on_wrong_type() {
        let source = format!(
            "{NODE_STRUCTS}node = BreakNode.new(label: \"b\")\ncase node\nin ReturnNode then \"return\"\nend\n"
        );
        evaluate(&source);
    }

    #[test]
    #[should_panic(expected = "keyword-only")]
    fn case_in_struct_patterns_reject_positional_fields() {
        let source = format!(
            "{NODE_STRUCTS}case ReturnNode.new(value: 1)\nin ReturnNode(5) then \"no\"\nend\n"
        );
        evaluate(&source);
    }

    #[test]
    fn case_in_pin_compares_instead_of_capturing() {
        let source = "expected = 2\ncase 2\nin ^expected then \"hit\"\nelse\n  \"miss\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("hit".to_string())));
        let source = "expected = 9\ncase 2\nin ^expected then \"hit\"\nelse\n  \"miss\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("miss".to_string())));
    }

    #[test]
    fn case_in_guards_refine_matches() {
        let source = "case 50\nin score if score > 10 then \"big\"\nin score then \"small\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("big".to_string())));
        let source = "case 5\nin score if score > 10 then \"big\"\nin score then \"small\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("small".to_string())));
    }

    #[test]
    fn case_in_array_patterns_match_exactly() {
        let source =
            "case [1, 2]\nin [] then \"empty\"\nin [a, b] then a + b\nelse\n  \"other\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
        let source =
            "case []\nin [] then \"empty\"\nin [a, b] then a + b\nelse\n  \"other\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("empty".to_string())));
        let source = "case [1, 2, 3]\nin [a, b] then a + b\nelse\n  \"other\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("other".to_string())));
    }

    #[test]
    fn case_in_array_patterns_take_a_rest() {
        let source = "case [1, 2, 3]\nin [first, *rest] then first + rest.length\nend\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
        let source = "case [1]\nin [first, *rest] then rest\nend\n";
        assert_eq!(evaluate(source), Some(Value::array(Vec::new())));
        let source = "case [1, 2, 3]\nin [first, *] then first\nend\n";
        assert_eq!(evaluate(source), Some(Value::Integer(1)));
    }

    #[test]
    fn one_line_in_is_a_boolean_test_that_binds() {
        assert_eq!(evaluate("5 in 1 | 5"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("5 in nil"), Some(Value::Boolean(false)));
        let source = format!(
            "{NODE_STRUCTS}node = ReturnNode.new(value: 7)\nhit = node in ReturnNode(value:)\nvalue + 1\n"
        );
        assert_eq!(evaluate(&source), Some(Value::Integer(8)));
    }

    #[test]
    fn one_line_in_reads_as_a_condition() {
        let source = format!(
            "{NODE_STRUCTS}node = BreakNode.new(label: \"b\")\nif node in BreakNode\n  \"break\"\nelse\n  \"other\"\nend\n"
        );
        assert_eq!(evaluate(&source), Some(Value::String("break".to_string())));
    }

    #[test]
    fn rightward_destructuring_binds_or_panics() {
        let source = "pair = [1, 2]\npair => [a, b]\na + b\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    #[test]
    #[should_panic(expected = "pattern mismatch")]
    fn rightward_destructuring_panics_on_mismatch() {
        evaluate("[1] => [a, b]\n");
    }

    #[test]
    fn while_iterations_are_fresh_scopes_for_their_own_locals() {
        // `current` is a plain immutable binding, born and dying once per
        // iteration; `index` and `total` persist because they're outer.
        let source = "mutable index = 0\nmutable total = 0\nwhile index < 3\n  current = index * 10\n  total += current\n  index += 1\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(30)));
    }

    #[test]
    #[should_panic(expected = "undefined variable or method current")]
    fn while_body_locals_die_at_loop_end() {
        evaluate("mutable index = 0\nwhile index < 2\n  current = 1\n  index += 1\nend\ncurrent\n");
    }

    #[test]
    fn mutable_declares_a_rebindable_name() {
        assert_eq!(
            evaluate("mutable count = 1\ncount = 5\ncount\n"),
            Some(Value::Integer(5))
        );
    }

    #[test]
    #[should_panic(expected = "count is immutable")]
    fn rebinding_an_immutable_name_panics() {
        evaluate("count = 1\ncount = 2\n");
    }

    #[test]
    #[should_panic(expected = "count is immutable")]
    fn compound_assignment_requires_mutable() {
        evaluate("count = 1\ncount += 1\n");
    }

    #[test]
    #[should_panic(expected = "already declared")]
    fn mutable_declares_once() {
        evaluate("mutable count = 1\nmutable count = 2\n");
    }

    #[test]
    fn blocks_rebind_outer_mutables_the_accumulator_pattern() {
        let source = "mutable total = 0\n[1, 2].each do |n|\n  total += n\nend\ntotal\n";
        assert_eq!(evaluate(source), Some(Value::Integer(3)));
    }

    #[test]
    #[should_panic(expected = "total is immutable")]
    fn blocks_may_not_rebind_outer_immutables() {
        evaluate("total = 0\n[1, 2].each do |n|\n  total += n\nend\n");
    }

    #[test]
    #[should_panic(expected = "undefined variable or method scratch")]
    fn fresh_block_locals_die_at_end() {
        evaluate("[1].each do |n|\n  scratch = n\nend\nscratch\n");
    }

    #[test]
    #[should_panic(expected = "number is immutable")]
    fn parameters_are_immutable_unless_marked() {
        evaluate("def bump(number)\n  number += 1\nend\nbump(1)\n");
    }

    #[test]
    fn mutable_parameters_may_rebind() {
        let source = "def bump(mutable number)\n  number += 1\n  number\nend\nbump(41)\n";
        assert_eq!(evaluate(source), Some(Value::Integer(42)));
    }

    #[test]
    #[should_panic(expected = "found is immutable")]
    fn captures_may_not_silently_rebind_immutables() {
        evaluate("found = 1\ncase 9\nin found then found\nend\n");
    }

    #[test]
    fn append_rebinds_strings_and_arrays() {
        assert_eq!(
            evaluate("mutable line = \"port\"\nline << \"land\"\nline\n"),
            Some(Value::String("portland".to_string()))
        );
        assert_eq!(
            evaluate("mutable list = [1]\nlist << 2\nlist\n"),
            Some(Value::array(vec![Value::Integer(1), Value::Integer(2)]))
        );
    }

    #[test]
    #[should_panic(expected = "line is immutable")]
    fn append_requires_a_mutable_binding() {
        evaluate("line = \"\"\nline << \"x\"\n");
    }

    #[test]
    fn append_cannot_spook_aliases() {
        // The ADR 0015 headline: << rebinds one name; other names holding
        // the old value are untouched (Ruby's aliased mutation dies here).
        let source = "mutable a = [1]\nb = a\na << 2\nb.length\n";
        assert_eq!(evaluate(source), Some(Value::Integer(1)));
    }

    #[test]
    fn index_assignment_updates_arrays_and_hashes() {
        assert_eq!(
            evaluate("mutable items = [1, 2]\nitems[0] = 9\nitems\n"),
            Some(Value::array(vec![Value::Integer(9), Value::Integer(2)]))
        );
        assert_eq!(
            evaluate("mutable items = [1]\nitems[1] = 2\nitems\n"),
            Some(Value::array(vec![Value::Integer(1), Value::Integer(2)]))
        );
        let source =
            "mutable counts = {\"a\" => 1}\ncounts[\"b\"] = 2\ncounts[\"a\"] = 5\ncounts[\"a\"]\n";
        assert_eq!(evaluate(source), Some(Value::Integer(5)));
    }

    #[test]
    #[should_panic(expected = "counts is immutable")]
    fn index_assignment_requires_a_mutable_binding() {
        evaluate("counts = {\"a\" => 1}\ncounts[\"a\"] = 2\n");
    }

    #[test]
    #[should_panic(expected = "out of range for assignment")]
    fn index_assignment_stays_in_bounds() {
        evaluate("mutable items = [1]\nitems[5] = 9\n");
    }

    const METHODED_TOKEN: &str = "struct Token\n  kind\n  text\n\n  def integer?\n    kind == \"integer\"\n  end\n\n  def describe\n    \"#{kind}: #{text}\"\n  end\n\n  def loud\n    describe.upcase\n  end\n\n  def mirror\n    self\n  end\n\n  def framed(edge: \"|\")\n    edge + text + edge\n  end\nend\n";

    #[test]
    fn struct_methods_see_fields_bare() {
        let source = format!("{METHODED_TOKEN}Token.new(kind: \"integer\", text: \"42\").integer?");
        assert_eq!(evaluate(&source), Some(Value::Boolean(true)));
        let source = format!("{METHODED_TOKEN}Token.new(kind: \"plus\", text: \"+\").describe");
        assert_eq!(
            evaluate(&source),
            Some(Value::String("plus: +".to_string()))
        );
    }

    #[test]
    fn struct_methods_call_their_own_methods_bare() {
        let source = format!("{METHODED_TOKEN}Token.new(kind: \"plus\", text: \"+\").loud");
        assert_eq!(
            evaluate(&source),
            Some(Value::String("PLUS: +".to_string()))
        );
    }

    #[test]
    fn self_is_the_receiver() {
        let source = format!(
            "{METHODED_TOKEN}token = Token.new(kind: \"a\", text: \"b\")\ntoken.mirror == token"
        );
        assert_eq!(evaluate(&source), Some(Value::Boolean(true)));
    }

    #[test]
    fn struct_methods_take_keyword_arguments() {
        let source =
            format!("{METHODED_TOKEN}Token.new(kind: \"a\", text: \"x\").framed(edge: \"!\")");
        assert_eq!(evaluate(&source), Some(Value::String("!x!".to_string())));
        let source = format!("{METHODED_TOKEN}Token.new(kind: \"a\", text: \"x\").framed");
        assert_eq!(evaluate(&source), Some(Value::String("|x|".to_string())));
    }

    #[test]
    fn with_and_new_still_work_on_methoded_structs() {
        let source = format!(
            "{METHODED_TOKEN}Token.new(kind: \"integer\", text: \"42\").with(text: \"43\").describe"
        );
        assert_eq!(
            evaluate(&source),
            Some(Value::String("integer: 43".to_string()))
        );
    }

    #[test]
    #[should_panic(expected = "kind is a field")]
    fn struct_methods_may_not_collide_with_fields() {
        evaluate("struct Bad\n  kind\n\n  def kind\n    1\n  end\nend\n");
    }

    #[test]
    #[should_panic(expected = "reserved")]
    fn struct_methods_may_not_claim_reserved_names() {
        evaluate("struct Bad\n  value\n\n  def with\n    1\n  end\nend\n");
    }

    #[test]
    fn builtin_type_patterns_match_by_kind() {
        assert_eq!(evaluate("5 in Integer"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("\"x\" in Integer"), Some(Value::Boolean(false)));
        assert_eq!(evaluate("[] in Array"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("\"x\" in String"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("{} in Hash"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("true in Boolean"), Some(Value::Boolean(true)));
        assert_eq!(evaluate("[].first in String"), Some(Value::Boolean(false)));
        let source = "case 5\nin String then \"text\"\nin Integer then \"number\"\nend\n";
        assert_eq!(evaluate(source), Some(Value::String("number".to_string())));
    }

    #[test]
    #[should_panic(expected = "takes no fields")]
    fn builtin_type_patterns_take_no_fields() {
        evaluate("5 in Integer(x:)\n");
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
