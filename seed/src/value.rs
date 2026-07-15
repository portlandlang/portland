//! Runtime values. Tiny on purpose — grows with the Stage 0 subset.

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Array(Vec<Value>),
    Boolean(bool),
    Integer(i64),
    String(String),
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Array(elements) => {
                write!(formatter, "[")?;
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, ", ")?;
                    }
                    write!(formatter, "{element}")?;
                }
                write!(formatter, "]")
            }
            Value::Boolean(value) => write!(formatter, "{value}"),
            Value::Integer(value) => write!(formatter, "{value}"),
            Value::String(value) => write!(formatter, "{value}"),
        }
    }
}
