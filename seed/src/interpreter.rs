//! A tree-walking interpreter — the reference semantics for the Stage 0
//! subset before any codegen exists. Crude in the seed: type errors panic.

use std::collections::HashMap;

use crate::ast::{Expression, Program, Statement};
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

pub struct Interpreter {
    methods: HashMap<String, Method>,
    variables: HashMap<String, Value>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
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
                let value = self.expression(value);
                self.variables.insert(name.clone(), value.clone());
                Some(value)
            }
            Statement::Expression(expression) => Some(self.expression(expression)),
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
        }
    }

    fn expression(&mut self, expression: &Expression) -> Value {
        match expression {
            Expression::Integer(value) => Value::Integer(*value),
            Expression::String(value) => Value::String(value.clone()),
            Expression::Add { left, right } => {
                let left = self.expression(left);
                let right = self.expression(right);
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
                    (Value::String(a), Value::String(b)) => Value::String(a + &b),
                    (left, right) => panic!("cannot add {left:?} and {right:?}"),
                }
            }
            Expression::Variable(name) => self
                .variables
                .get(name)
                .unwrap_or_else(|| panic!("undefined variable {name}"))
                .clone(),
            Expression::Call { arguments, name } => {
                let arguments: Vec<Value> = arguments.iter().map(|a| self.expression(a)).collect();
                self.call(name, arguments)
            }
        }
    }

    fn call(&mut self, name: &str, arguments: Vec<Value>) -> Value {
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

        result.unwrap_or_else(|| panic!("{name} produced no value"))
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
    #[should_panic(expected = "cannot add")]
    fn panics_on_adding_a_string_to_an_integer() {
        evaluate(r#"1 + "one""#);
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
}
