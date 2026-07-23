//! Runtime values. Tiny on purpose — grows with the Stage 0 subset.

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    /// Rc because Portland values are immutable: sharing is invisible, and
    /// cloning a value must never mean copying a whole collection.
    Array(std::rc::Rc<Vec<Value>>),
    Boolean(bool),
    /// Insertion-ordered pairs; lookup is linear. Note: derived equality is
    /// order-sensitive, unlike Ruby's — acceptable crudeness for the seed.
    Hash(std::rc::Rc<Vec<(Value, Value)>>),
    Integer(i64),
    /// Absence (ADR 0005/0006): the empty case of a maybe. No methods, not
    /// falsy — the seed enforces both with runtime panics where the real
    /// compiler will refuse to build.
    Nil,
    String(String),
    /// Immutable named record; fields stay in definition order.
    Struct {
        fields: Vec<(String, Value)>,
        name: String,
    },
}

impl Value {
    pub fn array(elements: Vec<Value>) -> Value {
        Value::Array(std::rc::Rc::new(elements))
    }

    pub fn hash(pairs: Vec<(Value, Value)>) -> Value {
        Value::Hash(std::rc::Rc::new(pairs))
    }

    /// The developer-facing rendering: strings keep their quotes, like irb.
    pub fn inspect(&self) -> String {
        match self {
            Value::Array(elements) => {
                let inner: Vec<String> = elements.iter().map(|element| element.inspect()).collect();
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
            Value::Nil => "nil".to_string(),
            Value::String(value) => format!("{value:?}"),
            Value::Struct { fields, name } => {
                let inner: Vec<String> = fields
                    .iter()
                    .map(|(field, value)| format!("{field}: {}", value.inspect()))
                    .collect();
                format!("{name}({})", inner.join(", "))
            }
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
            Value::Nil => write!(formatter, "nil"),
            Value::String(value) => write!(formatter, "{value}"),
            Value::Struct { .. } => write!(formatter, "{}", self.inspect()),
        }
    }
}
