//! Runtime values. Tiny on purpose — grows with the Stage 0 subset.

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Boolean(bool),
    Integer(i64),
    String(String),
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Boolean(value) => write!(formatter, "{value}"),
            Value::Integer(value) => write!(formatter, "{value}"),
            Value::String(value) => write!(formatter, "{value}"),
        }
    }
}
