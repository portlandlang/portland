//! Runtime values. Tiny on purpose — grows with the Stage 0 subset.

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Array(Vec<Value>),
    Boolean(bool),
    /// Insertion-ordered pairs; lookup is linear. Note: derived equality is
    /// order-sensitive, unlike Ruby's — acceptable crudeness for the seed.
    Hash(Vec<(Value, Value)>),
    Integer(i64),
    String(String),
}

impl Value {
    /// The developer-facing rendering: strings keep their quotes, like irb.
    pub fn inspect(&self) -> String {
        match self {
            Value::Array(elements) => {
                let inner: Vec<String> = elements.iter().map(|e| e.inspect()).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Boolean(value) => value.to_string(),
            Value::Hash(pairs) => {
                let inner: Vec<String> = pairs
                    .iter()
                    .map(|(key, value)| format!("{} => {}", key.inspect(), value.inspect()))
                    .collect();
                format!("{{{}}}", inner.join(", "))
            }
            Value::Integer(value) => value.to_string(),
            Value::String(value) => format!("{value:?}"),
        }
    }
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
            Value::Hash(pairs) => {
                write!(formatter, "{{")?;
                for (index, (key, value)) in pairs.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, ", ")?;
                    }
                    write!(formatter, "{key} => {value}")?;
                }
                write!(formatter, "}}")
            }
            Value::Integer(value) => write!(formatter, "{value}"),
            Value::String(value) => write!(formatter, "{value}"),
        }
    }
}
