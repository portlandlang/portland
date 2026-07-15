//! A tree-walking interpreter — the reference semantics for the Stage 0
//! subset before any codegen exists. Crude in the seed: type errors panic.

use std::collections::HashMap;

use crate::ast::{BinaryOperator, Block, Expression, Program, Statement, UnaryOperator};
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
    parameters: Vec<String>,
}

/// A `return` or `break` in flight, unwinding to whatever handles it.
enum Pending {
    Break,
    Return(Option<Value>),
}

pub struct Interpreter<W: std::io::Write = std::io::Stdout> {
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
            methods: HashMap::new(),
            output,
            pending: None,
            variables: HashMap::new(),
        }
    }

    pub fn program(&mut self, program: &Program) -> Option<Value> {
        let last = self.run_body(&program.statements);
        match self.pending.take() {
            None => last,
            Some(Pending::Break) => panic!("break outside of a loop"),
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
            Expression::Index { index, receiver } => {
                let receiver = self.value_of(receiver);
                let index = self.value_of(index);
                let (Value::Array(elements), Value::Integer(index)) = (&receiver, &index) else {
                    panic!("cannot index {receiver:?} with {index:?}")
                };
                // Ruby-style negative indices; out of range panics — no nil to return.
                let length = elements.len() as i64;
                let position = if *index < 0 { length + index } else { *index };
                if position < 0 || position >= length {
                    panic!("index {index} out of range for array of length {length}");
                }
                Some(elements[position as usize].clone())
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
            Expression::Unary { operand, operator } => {
                let operand = self.value_of(operand);
                match (operator, operand) {
                    (UnaryOperator::Negate, Value::Integer(value)) => Some(Value::Integer(-value)),
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
            (Value::Integer(n), "abs", []) => Value::Integer(n.abs()),
            (Value::Integer(n), "negative?", []) => Value::Boolean(*n < 0),
            (Value::Integer(n), "positive?", []) => Value::Boolean(*n > 0),
            (Value::Integer(n), "zero?", []) => Value::Boolean(*n == 0),
            (Value::String(s), "downcase", []) => Value::String(s.to_lowercase()),
            (Value::String(s), "empty?", []) => Value::Boolean(s.is_empty()),
            (Value::String(s), "length", []) => Value::Integer(s.chars().count() as i64),
            (Value::String(s), "reverse", []) => Value::String(s.chars().rev().collect()),
            (Value::String(s), "upcase", []) => Value::String(s.to_uppercase()),
            (receiver, "to_s", []) => Value::String(receiver.to_string()),
            (receiver, name, arguments) => {
                panic!("undefined method {name} for {receiver:?} with {arguments:?}")
            }
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
            panic!("return and break inside blocks are not supported in the seed yet");
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

    fn call(&mut self, name: &str, arguments: Vec<Value>) -> Option<Value> {
        if !self.methods.contains_key(name) && name == "puts" {
            for argument in &arguments {
                writeln!(self.output, "{argument}").expect("failed to write output");
            }
            return None;
        }

        let method = self
            .methods
            .get(name)
            .unwrap_or_else(|| panic!("undefined method {name}"))
            .clone();
        if arguments.len() != method.parameters.len() {
            panic!(
                "{name} expects {} argument(s), got {}",
                method.parameters.len(),
                arguments.len()
            );
        }

        // Methods get a fresh scope: parameters only, no outer locals.
        let mut scope: HashMap<String, Value> =
            method.parameters.iter().cloned().zip(arguments).collect();
        std::mem::swap(&mut self.variables, &mut scope);
        let mut result = self.run_body(&method.body);
        std::mem::swap(&mut self.variables, &mut scope);
        match self.pending.take() {
            None => {}
            Some(Pending::Return(value)) => result = value,
            Some(Pending::Break) => panic!("break outside of a loop"),
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
    fn puts_prints_arrays_readably() {
        assert_eq!(output_of("puts([1, 2, 3])"), "[1, 2, 3]\n");
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
