//! Runtime values. Tiny on purpose — grows with the Stage 0 subset.

use std::fmt;

/// No `Eq`: floats are values now (ADR 0018), and IEEE equality is only
/// partial. Nothing keys a std collection by `Value`, so `PartialEq` is
/// all the seed needs.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Rc because Portland values are immutable: sharing is invisible, and
    /// cloning a value must never mean copying a whole collection.
    Array(std::rc::Rc<Vec<Value>>),
    Boolean(bool),
    /// IEEE 754 double (ADR 0018). Ruby's printing: always shows a point.
    Float(f64),
    /// Insertion-ordered pairs; lookup is linear. Note: derived equality is
    /// order-sensitive, unlike Ruby's — acceptable crudeness for the seed.
    Hash(std::rc::Rc<Vec<(Value, Value)>>),
    Integer(i64),
    /// Absence (ADR 0005/0006): the empty case of a maybe. No methods, not
    /// falsy — the seed enforces both with runtime panics where the real
    /// compiler will refuse to build.
    Nil,
    /// `1..5` inclusive, `1...5` exclusive; either end may be absent for
    /// the endless and beginless forms (ADR 0019). Integer-only in the
    /// seed — a documented crudeness.
    Range {
        end: Option<i64>,
        exclusive: bool,
        start: Option<i64>,
    },
    /// The nested case of the wrapper (ADR 0005): only ever wraps Nil or
    /// another Some — a plain present value is never boxed (`some(5)` is
    /// `5`). Built by `Value::present`; keeps `[nil].first` ≠ `[].first`.
    Some(Box<Value>),
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

    /// Lift a found value out of a successful partial lookup: plain values
    /// pass through untouched; only nil (and nested wrappers) gain a box,
    /// so presence-of-absence stays distinguishable from absence.
    pub fn present(value: Value) -> Value {
        match value {
            Value::Nil | Value::Some(_) => Value::Some(Box::new(value)),
            other => other,
        }
    }

    /// The developer-facing rendering: strings keep their quotes, like irb.
    pub fn inspect(&self) -> String {
        match self {
            Value::Array(elements) => {
                let inner: Vec<String> = elements.iter().map(|element| element.inspect()).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Boolean(value) => value.to_string(),
            Value::Float(value) => format_float(*value),
            Value::Hash(pairs) => {
                let inner: Vec<String> = pairs
                    .iter()
                    .map(|(key, value)| format!("{} => {}", key.inspect(), value.inspect()))
                    .collect();
                format!("{{{}}}", inner.join(", "))
            }
            Value::Integer(value) => value.to_string(),
            Value::Nil => "nil".to_string(),
            Value::Range { .. } => self.to_string(),
            Value::Some(inner) => format!("some({})", inner.inspect()),
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

/// Ruby's float rendering: a float always shows its point (`1.0`, never
/// `1`), and the infinities spell out (ADR 0018).
fn format_float(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    // Rust's Debug for f64 already keeps the point on whole numbers.
    format!("{value:?}")
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
            Value::Float(value) => write!(formatter, "{}", format_float(*value)),
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
            Value::Range {
                end,
                exclusive,
                start,
            } => {
                if let Some(start) = start {
                    write!(formatter, "{start}")?;
                }
                write!(formatter, "{}", if *exclusive { "..." } else { ".." })?;
                if let Some(end) = end {
                    write!(formatter, "{end}")?;
                }
                Ok(())
            }
            Value::Nil => write!(formatter, "nil"),
            Value::Some(_) => write!(formatter, "{}", self.inspect()),
            Value::String(value) => write!(formatter, "{value}"),
            Value::Struct { .. } => write!(formatter, "{}", self.inspect()),
        }
    }
}
