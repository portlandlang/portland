//! A tree-walking interpreter — the reference semantics for the Stage 0
//! subset before any codegen exists. Crude in the seed: type errors panic.

use crate::ast::{Expression, Program, Statement};
use crate::parser;
use crate::value::Value;

/// Parse and evaluate a source string, returning the last statement's value.
pub fn evaluate(source: &str) -> Option<Value> {
    let program = parser::parse(source);
    let mut interpreter = Interpreter::new();
    interpreter.program(&program)
}

pub struct Interpreter {}

impl Interpreter {
    pub fn new() -> Self {
        Self {}
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
            Statement::Expression(expression) => Some(self.expression(expression)),
            other => panic!("cannot evaluate {other:?} yet"),
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
            other => panic!("cannot evaluate {other:?} yet"),
        }
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
}
