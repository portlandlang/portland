//! `Math` (ADR 0083): Ruby's module functions as builtins, spelled as Ruby
//! spells them, answering what Ruby 4.0.9 answers. A domain error is a
//! failure in Ruby's own sentence — `Numerical argument is out of domain -
//! sqrt` — since the caller's number was the problem (ADR 0027); the wrong
//! count or kind of argument refuses, since the program was.
//!
//! Rust's `f64` carries most of the family; the error and gamma functions
//! and `frexp`/`ldexp` come from the C library the seed already links.

use crate::value::Value;

unsafe extern "C" {
    fn erf(x: f64) -> f64;
    fn erfc(x: f64) -> f64;
    fn tgamma(x: f64) -> f64;
    fn lgamma_r(x: f64, sign: *mut libc::c_int) -> f64;
    fn frexp(x: f64, exponent: *mut libc::c_int) -> f64;
    fn ldexp(x: f64, exponent: libc::c_int) -> f64;
}

/// `Math.<name>(arguments)`: a Float (or `[Float, Integer]` for `frexp` and
/// `lgamma`), or a failure for a domain error.
pub fn call(name: &str, arguments: &[Value]) -> Value {
    match name {
        "sqrt" => one(arguments, |x| domain(x < 0.0, name, x.sqrt())),
        "cbrt" => one(arguments, |x| Value::Float(x.cbrt())),
        "sin" => one(arguments, |x| Value::Float(x.sin())),
        "cos" => one(arguments, |x| Value::Float(x.cos())),
        "tan" => one(arguments, |x| Value::Float(x.tan())),
        "asin" => one(arguments, |x| domain(x.abs() > 1.0, name, x.asin())),
        "acos" => one(arguments, |x| domain(x.abs() > 1.0, name, x.acos())),
        "atan" => one(arguments, |x| Value::Float(x.atan())),
        "sinh" => one(arguments, |x| Value::Float(x.sinh())),
        "cosh" => one(arguments, |x| Value::Float(x.cosh())),
        "tanh" => one(arguments, |x| Value::Float(x.tanh())),
        "asinh" => one(arguments, |x| Value::Float(x.asinh())),
        "acosh" => one(arguments, |x| domain(x < 1.0, name, x.acosh())),
        "atanh" => one(arguments, |x| domain(x.abs() > 1.0, name, x.atanh())),
        "exp" => one(arguments, |x| Value::Float(x.exp())),
        "expm1" => one(arguments, |x| Value::Float(x.exp_m1())),
        "log2" => one(arguments, |x| domain(x < 0.0, name, x.log2())),
        "log10" => one(arguments, |x| domain(x < 0.0, name, x.log10())),
        "log1p" => one(arguments, |x| domain(x < -1.0, name, x.ln_1p())),
        "erf" => one(arguments, |x| Value::Float(unsafe { erf(x) })),
        "erfc" => one(arguments, |x| Value::Float(unsafe { erfc(x) })),
        "gamma" => one(arguments, gamma),
        "lgamma" => one(arguments, lgamma),
        "frexp" => one(arguments, |x| {
            let mut exponent: libc::c_int = 0;
            let fraction = unsafe { frexp(x, &mut exponent) };
            Value::array(vec![
                Value::Float(fraction),
                Value::Integer(i64::from(exponent)),
            ])
        }),
        "hypot" => two(arguments, |x, y| Value::Float(x.hypot(y))),
        "atan2" => two(arguments, |y, x| Value::Float(y.atan2(x))),
        "log" => log(arguments),
        "ldexp" => ldexp_value(arguments),
        _ => panic!("Math has no {name}"),
    }
}

fn one(arguments: &[Value], answer: impl Fn(f64) -> Value) -> Value {
    match arguments {
        [x] => answer(float_of(x)),
        _ => panic!(
            "wrong number of arguments (given {}, expected 1)",
            arguments.len()
        ),
    }
}

fn two(arguments: &[Value], answer: impl Fn(f64, f64) -> Value) -> Value {
    match arguments {
        [x, y] => answer(float_of(x), float_of(y)),
        _ => panic!(
            "wrong number of arguments (given {}, expected 2)",
            arguments.len()
        ),
    }
}

/// A domain error is Ruby's failure; otherwise the answer.
fn domain(out: bool, name: &str, answer: f64) -> Value {
    if out {
        return failure(name);
    }
    Value::Float(answer)
}

fn failure(name: &str) -> Value {
    Value::Failure(Box::new(Value::String(format!(
        "Numerical argument is out of domain - {name}"
    ))))
}

/// Ruby's `Float()` on a Math argument: a number as a double, anything else
/// a refusal in Ruby's words.
fn float_of(value: &Value) -> f64 {
    match value {
        Value::Float(number) => *number,
        Value::Integer(number) => *number as f64,
        Value::Nil => panic!("can't convert nil into Float"),
        Value::Boolean(flag) => panic!("can't convert {flag} into Float"),
        other => panic!("can't convert {} into Float", other.type_name()),
    }
}

/// `Math.log(x)` and `Math.log(x, base)`: each operand checked for the
/// domain, and Ruby's one oddity kept — a base of 1 answers NaN.
fn log(arguments: &[Value]) -> Value {
    match arguments {
        [x] => {
            let x = float_of(x);
            domain(x < 0.0, "log", x.ln())
        }
        [x, base] => {
            let x = float_of(x);
            let base = float_of(base);
            if x < 0.0 || base < 0.0 {
                return failure("log");
            }
            if base == 1.0 {
                return Value::Float(f64::NAN);
            }
            Value::Float(x.ln() / base.ln())
        }
        _ => panic!(
            "wrong number of arguments (given {}, expected 1..2)",
            arguments.len()
        ),
    }
}

/// Ruby's gamma: exact factorials for the integers 1 through 23, where the
/// double still holds them, a domain error for the negative integers and
/// negative infinity, and the C library's answer elsewhere.
fn gamma(x: f64) -> Value {
    if x < 0.0 && (x == x.floor() || x.is_infinite()) {
        return failure("gamma");
    }
    if (1.0..=23.0).contains(&x) && x == x.floor() {
        let mut factorial = 1.0;
        let mut step = 2.0;
        while step < x {
            factorial *= step;
            step += 1.0;
        }
        return Value::Float(factorial);
    }
    Value::Float(unsafe { tgamma(x) })
}

/// Ruby's lgamma: `[log(|gamma(x)|), sign]`, a domain error only for
/// negative infinity.
fn lgamma(x: f64) -> Value {
    if x.is_infinite() && x < 0.0 {
        return failure("lgamma");
    }
    // Ruby answers the sign of the gamma that was infinite: -0.0's is -1,
    // which the C library does not say.
    if x == 0.0 {
        let sign = if x.is_sign_negative() { -1 } else { 1 };
        return Value::array(vec![Value::Float(f64::INFINITY), Value::Integer(sign)]);
    }
    let mut sign: libc::c_int = 1;
    let value = unsafe { lgamma_r(x, &mut sign) };
    Value::array(vec![Value::Float(value), Value::Integer(i64::from(sign))])
}

/// `Math.ldexp(fraction, exponent)`: the exponent an Integer, or a Float
/// truncated to one, refused in Ruby's words past the C `int`.
fn ldexp_value(arguments: &[Value]) -> Value {
    let [fraction, exponent] = arguments else {
        panic!(
            "wrong number of arguments (given {}, expected 2)",
            arguments.len()
        );
    };
    let fraction = float_of(fraction);
    let exponent: i64 = match exponent {
        Value::Integer(number) => *number,
        Value::Float(number) if number.is_nan() => panic!("float NaN out of range of integer"),
        Value::Float(number) if number.is_infinite() => {
            panic!("float {number} out of range of integer")
        }
        Value::Float(number) => number.trunc() as i64,
        Value::Nil => panic!("no implicit conversion from nil to integer"),
        other => panic!(
            "no implicit conversion of {} into Integer",
            other.type_name()
        ),
    };
    let exponent = libc::c_int::try_from(exponent)
        .unwrap_or_else(|_| panic!("integer {exponent} too big to convert to 'int'"));
    Value::Float(unsafe { ldexp(fraction, exponent) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn float(value: f64) -> Value {
        Value::Float(value)
    }

    fn answer(name: &str, arguments: &[Value]) -> f64 {
        match call(name, arguments) {
            Value::Float(number) => number,
            other => panic!("{name} answered {other:?}"),
        }
    }

    fn sentence(name: &str, arguments: &[Value]) -> String {
        match call(name, arguments) {
            Value::Failure(inner) => match *inner {
                Value::String(text) => text,
                other => panic!("a failure of {other:?}"),
            },
            other => panic!("{name} answered {other:?}"),
        }
    }

    #[test]
    fn answers_rubys_values() {
        assert_eq!(answer("sqrt", &[Value::Integer(4)]), 2.0);
        assert_eq!(answer("cbrt", &[float(-27.0)]), -3.0);
        assert_eq!(
            answer("hypot", &[Value::Integer(3), Value::Integer(4)]),
            5.0
        );
        assert_eq!(answer("log", &[Value::Integer(8), Value::Integer(2)]), 3.0);
        assert_eq!(answer("log", &[Value::Integer(0)]), f64::NEG_INFINITY);
        assert!(answer("log", &[Value::Integer(8), Value::Integer(1)]).is_nan());
        assert_eq!(answer("log", &[Value::Integer(1), Value::Integer(0)]), -0.0);
        assert_eq!(answer("gamma", &[Value::Integer(5)]), 24.0);
        assert_eq!(
            answer("gamma", &[Value::Integer(23)]),
            1.1240007277776077e21
        );
        assert_eq!(answer("gamma", &[Value::Integer(0)]), f64::INFINITY);
        assert_eq!(answer("gamma", &[float(-0.0)]), f64::NEG_INFINITY);
        assert_eq!(answer("erf", &[Value::Integer(1)]), 0.8427007929497148);
        assert_eq!(
            answer("ldexp", &[float(0.6025390625), Value::Integer(11)]),
            1234.0
        );
        assert_eq!(answer("ldexp", &[float(1.0), float(2.5)]), 4.0);
        assert_eq!(answer("atanh", &[Value::Integer(1)]), f64::INFINITY);
        assert_eq!(
            call("frexp", &[Value::Integer(1234)]),
            Value::array(vec![float(0.6025390625), Value::Integer(11)])
        );
        assert_eq!(
            call("lgamma", &[float(-0.0)]),
            Value::array(vec![float(f64::INFINITY), Value::Integer(-1)])
        );
        assert_eq!(
            call("lgamma", &[Value::Integer(5)]),
            Value::array(vec![float(3.1780538303479453), Value::Integer(1)])
        );
    }

    #[test]
    fn domain_errors_are_failures_in_rubys_sentence() {
        assert_eq!(
            sentence("sqrt", &[Value::Integer(-1)]),
            "Numerical argument is out of domain - sqrt"
        );
        assert_eq!(
            sentence("log", &[Value::Integer(8), Value::Integer(-2)]),
            "Numerical argument is out of domain - log"
        );
        assert_eq!(
            sentence("gamma", &[float(f64::NEG_INFINITY)]),
            "Numerical argument is out of domain - gamma"
        );
        assert_eq!(
            sentence("acosh", &[float(0.5)]),
            "Numerical argument is out of domain - acosh"
        );
        assert_eq!(
            sentence("log1p", &[Value::Integer(-2)]),
            "Numerical argument is out of domain - log1p"
        );
        assert_eq!(answer("sqrt", &[float(-0.0)]), 0.0);
        assert!(answer("sqrt", &[float(f64::NAN)]).is_nan());
    }

    #[test]
    #[should_panic(expected = "can't convert String into Float")]
    fn a_string_refuses() {
        call("sqrt", &[Value::String("1".to_string())]);
    }

    #[test]
    #[should_panic(expected = "wrong number of arguments (given 1, expected 2)")]
    fn the_count_refuses() {
        call("hypot", &[Value::Integer(1)]);
    }
}
