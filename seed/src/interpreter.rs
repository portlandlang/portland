//! A tree-walking interpreter — the reference semantics for the Stage 0
//! subset before any codegen exists. Crude in the seed: type errors panic.

use std::collections::HashMap;

use crate::ast::{BinaryOperator, Expression, Program, Statement, UnaryOperator};
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

pub struct Interpreter<W: std::io::Write = std::io::Stdout> {
    methods: HashMap<String, Method>,
    output: W,
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
            variables: HashMap::new(),
        }
    }

    pub fn program(&mut self, program: &Program) -> Option<Value> {
        let mut last = None;
        for statement in &program.statements {
            last = self.statement(statement);
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
            Statement::While { body, condition } => {
                loop {
                    let condition = self.value_of(condition);
                    let Value::Boolean(condition) = condition else {
                        panic!("while condition must be true or false, got {condition:?}")
                    };
                    if !condition {
                        break;
                    }
                    for statement in body {
                        self.statement(statement);
                    }
                }
                None
            }
        }
    }

    fn expression(&mut self, expression: &Expression) -> Option<Value> {
        match expression {
            Expression::Boolean(value) => Some(Value::Boolean(*value)),
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
                let mut result = None;
                for statement in body {
                    result = self.statement(statement);
                }
                result
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
        let mut result = None;
        for statement in &method.body {
            result = self.statement(statement);
        }
        std::mem::swap(&mut self.variables, &mut scope);
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
