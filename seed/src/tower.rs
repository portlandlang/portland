//! Rational and Complex (ADR 0085): the upper two floors of Ruby's numeric
//! tower as values. A rational is two 64-bit integers, reduced, with a
//! positive denominator; a complex is two real parts — integers, floats,
//! or rationals — mixed as Ruby mixes them. The arithmetic follows CRuby's
//! `rational.c` and `complex.c` step for step, so the digits are Ruby's:
//! integer with rational is rational, rational with float is float,
//! anything with complex is complex, and a part that comes out whole after
//! a complex division is an integer again (`rb_rational_canonicalize`).

use crate::ast::BinaryOperator;
use crate::value::Value;

// — rationals —

fn gcd(mut left: i128, mut right: i128) -> i128 {
    left = left.abs();
    right = right.abs();
    while right != 0 {
        let rest = left % right;
        left = right;
        right = rest;
    }
    left
}

/// A wide intermediate back to the 64-bit integers (ADR 0070), or the
/// refusal that names the operation.
fn narrow(value: i128, written: &str) -> i64 {
    i64::try_from(value).unwrap_or_else(|_| panic!("{written} overflows the 64-bit integers"))
}

/// A reduced rational from wide parts; a zero denominator refuses as
/// `1 / 0` does, in Ruby's words.
fn rational_wide(numerator: i128, denominator: i128, written: &str) -> Value {
    if denominator == 0 {
        panic!("divided by 0");
    }
    let divisor = gcd(numerator, denominator).max(1);
    let (mut numerator, mut denominator) = (numerator / divisor, denominator / divisor);
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    Value::Rational {
        numerator: narrow(numerator, written),
        denominator: narrow(denominator, written),
    }
}

pub fn rational(numerator: i64, denominator: i64) -> Value {
    let written = format!("Rational({numerator}, {denominator})");
    rational_wide(i128::from(numerator), i128::from(denominator), &written)
}

pub fn complex(real: Value, imaginary: Value) -> Value {
    Value::Complex {
        real: Box::new(real),
        imaginary: Box::new(imaginary),
    }
}

/// An integer or a rational, as a wide fraction.
fn fraction(value: &Value) -> (i128, i128) {
    match value {
        Value::Integer(number) => (i128::from(*number), 1),
        Value::Rational {
            numerator,
            denominator,
        } => (i128::from(*numerator), i128::from(*denominator)),
        other => unreachable!("an exact number, got {other:?}"),
    }
}

pub fn is_real(value: &Value) -> bool {
    matches!(
        value,
        Value::Integer(_) | Value::Float(_) | Value::Rational { .. }
    )
}

fn is_exact(value: &Value) -> bool {
    matches!(value, Value::Integer(_) | Value::Rational { .. })
}

/// Ruby's `k_exact_zero_p`: an integer or rational zero — not `0.0`.
pub fn exact_zero(value: &Value) -> bool {
    matches!(
        value,
        Value::Integer(0) | Value::Rational { numerator: 0, .. }
    )
}

/// Ruby's `f_zero_p`: any zero, `0.0` included.
fn any_zero(value: &Value) -> bool {
    match value {
        Value::Float(number) => *number == 0.0,
        other => exact_zero(other),
    }
}

pub fn to_f64(value: &Value) -> f64 {
    match value {
        Value::Integer(number) => *number as f64,
        Value::Float(number) => *number,
        Value::Rational {
            numerator,
            denominator,
        } => *numerator as f64 / *denominator as f64,
        other => unreachable!("a real number, got {other:?}"),
    }
}

/// `rb_rational_canonicalize`: a whole rational is its integer again —
/// what a complex division's parts pass through.
pub fn canonicalize(value: Value) -> Value {
    match value {
        Value::Rational {
            numerator,
            denominator: 1,
        } => Value::Integer(numerator),
        other => other,
    }
}

/// Ruby's sign for a part's `inspect`: a float by its sign bit, so `-0.0`
/// reads negative; anything else by its value.
fn negative_part(value: &Value) -> bool {
    match value {
        Value::Float(number) => number.is_sign_negative() && !number.is_nan(),
        Value::Integer(number) => *number < 0,
        Value::Rational { numerator, .. } => *numerator < 0,
        _ => false,
    }
}

fn written(left: &Value, glyph: &str, right: &Value) -> String {
    format!("{} {glyph} {}", left.inspect(), right.inspect())
}

// — real arithmetic, Ruby's contagion —

fn real_add(left: &Value, right: &Value) -> Value {
    match (left, right) {
        (Value::Integer(left_value), Value::Integer(right_value)) => {
            Value::Integer(left_value.checked_add(*right_value).unwrap_or_else(|| {
                panic!(
                    "{} overflows the 64-bit integers",
                    written(left, "+", right)
                )
            }))
        }
        (Value::Float(_), _) | (_, Value::Float(_)) => Value::Float(to_f64(left) + to_f64(right)),
        _ => {
            let (left_numerator, left_denominator) = fraction(left);
            let (right_numerator, right_denominator) = fraction(right);
            rational_wide(
                left_numerator * right_denominator + right_numerator * left_denominator,
                left_denominator * right_denominator,
                &written(left, "+", right),
            )
        }
    }
}

fn real_sub(left: &Value, right: &Value) -> Value {
    match (left, right) {
        (Value::Integer(left_value), Value::Integer(right_value)) => {
            Value::Integer(left_value.checked_sub(*right_value).unwrap_or_else(|| {
                panic!(
                    "{} overflows the 64-bit integers",
                    written(left, "-", right)
                )
            }))
        }
        (Value::Float(_), _) | (_, Value::Float(_)) => Value::Float(to_f64(left) - to_f64(right)),
        _ => {
            let (left_numerator, left_denominator) = fraction(left);
            let (right_numerator, right_denominator) = fraction(right);
            rational_wide(
                left_numerator * right_denominator - right_numerator * left_denominator,
                left_denominator * right_denominator,
                &written(left, "-", right),
            )
        }
    }
}

fn real_mul(left: &Value, right: &Value) -> Value {
    match (left, right) {
        (Value::Integer(left_value), Value::Integer(right_value)) => {
            Value::Integer(left_value.checked_mul(*right_value).unwrap_or_else(|| {
                panic!(
                    "{} overflows the 64-bit integers",
                    written(left, "*", right)
                )
            }))
        }
        (Value::Float(_), _) | (_, Value::Float(_)) => Value::Float(to_f64(left) * to_f64(right)),
        _ => {
            let (left_numerator, left_denominator) = fraction(left);
            let (right_numerator, right_denominator) = fraction(right);
            rational_wide(
                left_numerator * right_numerator,
                left_denominator * right_denominator,
                &written(left, "*", right),
            )
        }
    }
}

/// Ruby's `quo`: exact division answering a rational, float division
/// where a float is involved; an exact zero divisor refuses.
fn real_quo(left: &Value, right: &Value) -> Value {
    match (left, right) {
        (Value::Float(_), _) | (_, Value::Float(_)) => Value::Float(to_f64(left) / to_f64(right)),
        _ => {
            if exact_zero(right) {
                panic!("divided by 0");
            }
            let (left_numerator, left_denominator) = fraction(left);
            let (right_numerator, right_denominator) = fraction(right);
            rational_wide(
                left_numerator * right_denominator,
                left_denominator * right_numerator,
                &written(left, "/", right),
            )
        }
    }
}

/// The floor of a real, as an integer.
fn real_floor(value: &Value) -> Value {
    match value {
        Value::Integer(_) => value.clone(),
        Value::Float(number) => float_integer(number.floor(), value),
        Value::Rational {
            numerator,
            denominator,
        } => Value::Integer(numerator.div_euclid(*denominator)),
        other => unreachable!("a real number, got {other:?}"),
    }
}

fn float_integer(rounded: f64, original: &Value) -> Value {
    if !rounded.is_finite() {
        panic!("{}", original.inspect());
    }
    if rounded < i64::MIN as f64 || rounded > i64::MAX as f64 {
        panic!("{} overflows the 64-bit integers", original.inspect());
    }
    Value::Integer(rounded as i64)
}

/// Floored modulo, Ruby's `%`: `left - floor(left / right) * right`.
fn real_modulo(left: &Value, right: &Value) -> Value {
    if is_exact(left) && is_exact(right) && exact_zero(right) {
        panic!("divided by 0");
    }
    let quotient = real_floor(&real_quo(left, right));
    real_sub(left, &real_mul(&quotient, right))
}

fn real_cmp(left: &Value, right: &Value) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (Value::Float(_), _) | (_, Value::Float(_)) => to_f64(left).partial_cmp(&to_f64(right)),
        _ => {
            let (left_numerator, left_denominator) = fraction(left);
            let (right_numerator, right_denominator) = fraction(right);
            Some((left_numerator * right_denominator).cmp(&(right_numerator * left_denominator)))
        }
    }
}

fn real_eq(left: &Value, right: &Value) -> bool {
    real_cmp(left, right) == Some(std::cmp::Ordering::Equal)
}

fn real_negate(value: &Value) -> Value {
    match value {
        Value::Integer(number) => Value::Integer(
            number
                .checked_neg()
                .unwrap_or_else(|| panic!("-({number}) overflows the 64-bit integers")),
        ),
        Value::Float(number) => Value::Float(-number),
        Value::Rational {
            numerator,
            denominator,
        } => rational_wide(
            -i128::from(*numerator),
            i128::from(*denominator),
            &format!("-{}", value.inspect()),
        ),
        other => unreachable!("a real number, got {other:?}"),
    }
}

fn real_abs(value: &Value) -> Value {
    if negative_part(value) {
        real_negate(value)
    } else {
        value.clone()
    }
}

/// A float's exact rational (`float_to_r`): the mantissa over its power of
/// two, within the 64-bit integers or refused past them; an infinity or NaN
/// has none, in Ruby's words.
pub fn float_to_rational(number: f64) -> Value {
    if !number.is_finite() {
        panic!("{}", Value::Float(number).inspect());
    }
    if number == 0.0 {
        return rational(0, 1);
    }
    let bits = number.to_bits();
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction_bits = bits & ((1u64 << 52) - 1);
    let (mantissa, exponent) = if exponent_bits == 0 {
        (fraction_bits, -1074)
    } else {
        (fraction_bits | (1u64 << 52), exponent_bits - 1075)
    };
    let mantissa = i128::from(mantissa) * if number < 0.0 { -1 } else { 1 };
    let written = format!("{}.to_r", Value::Float(number).inspect());
    if exponent >= 0 {
        let scale = 1i128
            .checked_shl(exponent as u32)
            .unwrap_or_else(|| panic!("{written} overflows the 64-bit integers"));
        let scaled = mantissa
            .checked_mul(scale)
            .unwrap_or_else(|| panic!("{written} overflows the 64-bit integers"));
        return rational_wide(scaled, 1, &written);
    }
    let shift = (-exponent) as u32;
    if shift >= 127 {
        panic!("{written} overflows the 64-bit integers");
    }
    rational_wide(mantissa, 1i128 << shift, &written)
}

/// Any real as a rational, for `Rational(a, b)`'s parts.
fn to_rational(value: &Value, context: &str) -> Value {
    match value {
        Value::Integer(number) => rational(*number, 1),
        Value::Rational { .. } => value.clone(),
        Value::Float(number) => float_to_rational(*number),
        other => panic!("{context} takes numbers, got {}", other.inspect()),
    }
}

/// `nurat_rationalize_internal`: the simplest rational between two
/// bounds, by continued fractions — Ruby's walk, step for step.
fn simplest_between(mut low: Value, mut high: Value) -> Value {
    let one = Value::Integer(1);
    let mut p0 = Value::Integer(0);
    let mut p1 = Value::Integer(1);
    let mut q0 = Value::Integer(1);
    let mut q1 = Value::Integer(0);
    let ceiling;
    loop {
        let c = real_ceil(&low);
        if real_cmp(&c, &high) == Some(std::cmp::Ordering::Less) {
            ceiling = c;
            break;
        }
        let k = real_sub(&c, &one);
        let p2 = real_add(&real_mul(&k, &p1), &p0);
        let q2 = real_add(&real_mul(&k, &q1), &q0);
        let t = real_quo(&one, &real_sub(&high, &k));
        high = real_quo(&one, &real_sub(&low, &k));
        low = t;
        p0 = p1;
        q0 = q1;
        p1 = p2;
        q1 = q2;
    }
    let p = real_add(&real_mul(&ceiling, &p1), &p0);
    let q = real_add(&real_mul(&ceiling, &q1), &q0);
    real_quo(&p, &q)
}

fn real_ceil(value: &Value) -> Value {
    match value {
        Value::Integer(_) => value.clone(),
        Value::Float(number) => float_integer(number.ceil(), value),
        Value::Rational {
            numerator,
            denominator,
        } => Value::Integer(-(-numerator).div_euclid(*denominator)),
        other => unreachable!("a real number, got {other:?}"),
    }
}

/// `rationalize` (Ruby's): with an epsilon, the simplest rational within it
/// of the value; without one, a rational's self, an integer's `to_r`, and
/// for a float the simplest rational the float's own precision allows
/// (`rb_flt_rationalize`).
pub fn rationalize(value: &Value, epsilon: Option<&Value>) -> Value {
    if let Some(epsilon) = epsilon {
        let exact = to_rational(value, "rationalize");
        let span = real_abs(&to_rational(epsilon, "rationalize"));
        let low = real_sub(&exact, &span);
        let high = real_add(&exact, &span);
        if real_eq(&low, &high) {
            return exact;
        }
        return simplest_between(low, high);
    }
    match value {
        Value::Integer(number) => rational(*number, 1),
        Value::Rational { .. } => value.clone(),
        Value::Float(number) => {
            if *number == 0.0 || !number.is_finite() {
                return float_to_rational(*number);
            }
            let bits = number.to_bits();
            let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
            let fraction_bits = bits & ((1u64 << 52) - 1);
            let (mantissa, exponent) = if exponent_bits == 0 {
                (fraction_bits, -1074)
            } else {
                (fraction_bits | (1u64 << 52), exponent_bits - 1075)
            };
            if exponent >= 0 {
                return float_to_rational(*number);
            }
            let written = format!("{}.rationalize", value.inspect());
            let shift = (1 - exponent) as u32;
            if shift >= 127 {
                panic!("{written} overflows the 64-bit integers");
            }
            let denominator = 1i128 << shift;
            let doubled = i128::from(mantissa) * 2 * if *number < 0.0 { -1 } else { 1 };
            let low = rational_wide(doubled - 1, denominator, &written);
            let high = rational_wide(doubled + 1, denominator, &written);
            simplest_between(low, high)
        }
        other => unreachable!("a real number, got {other:?}"),
    }
}

/// `Rational#round` without digits: half away from zero
/// (`nurat_round_half_up`), on the magnitude with the sign put back.
fn rational_round(numerator: i64, denominator: i64) -> Value {
    let magnitude = i128::from(numerator).abs();
    let denominator = i128::from(denominator);
    let rounded = (magnitude * 2 + denominator).div_euclid(denominator * 2);
    let signed = if numerator < 0 { -rounded } else { rounded };
    Value::Integer(narrow(
        signed,
        &format!("Rational({numerator}, {denominator}).round"),
    ))
}

// — powers —

/// A real to a real power, Ruby's table: an exact base to an integer power
/// stays exact (a negative power inverting, a zero base refusing); a
/// whole rational exponent is its integer; anything else is the floats'
/// power — which, for a negative base and a fractional exponent, is a
/// complex (`rb_dbl_complex_new_polar_pi`).
fn real_pow(left: &Value, right: &Value) -> Value {
    let right = match right {
        Value::Rational {
            numerator,
            denominator: 1,
        } => Value::Integer(*numerator),
        other => other.clone(),
    };
    if let Value::Integer(exponent) = right
        && is_exact(left)
    {
        if exponent == 0 {
            return match left {
                Value::Rational { .. } => rational(1, 1),
                _ => Value::Integer(1),
            };
        }
        if exponent < 0 && exact_zero(left) {
            panic!("divided by 0");
        }
        let (numerator, denominator) = fraction(left);
        let magnitude = u32::try_from(exponent.unsigned_abs()).unwrap_or_else(|_| {
            panic!(
                "{} overflows the 64-bit integers",
                written(left, "**", &right)
            )
        });
        let shown = written(left, "**", &right);
        let raised_numerator = numerator
            .checked_pow(magnitude)
            .unwrap_or_else(|| panic!("{shown} overflows the 64-bit integers"));
        let raised_denominator = denominator
            .checked_pow(magnitude)
            .unwrap_or_else(|| panic!("{shown} overflows the 64-bit integers"));
        if exponent < 0 {
            return rational_wide(raised_denominator, raised_numerator, &shown);
        }
        return match left {
            Value::Integer(_) => Value::Integer(narrow(raised_numerator, &shown)),
            _ => rational_wide(raised_numerator, raised_denominator, &shown),
        };
    }
    let base = to_f64(left);
    let exponent = to_f64(&right);
    if base < 0.0 && exponent.fract() != 0.0 && exponent.is_finite() {
        return polar_pi((-base).powf(exponent), exponent);
    }
    Value::Float(base.powf(exponent))
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    #[link_name = "__cospi"]
    fn cospi(x: f64) -> f64;
    #[link_name = "__sinpi"]
    fn sinpi(x: f64) -> f64;
}

/// `cos(πx)` and `sin(πx)`: the platform's own where it has them, as
/// CRuby uses them, so `(-8.0) ** (1.0 / 3)` reads `1.0+1.73…i` and not
/// `1.0000000000000002+…`.
fn cos_pi(x: f64) -> f64 {
    #[cfg(target_os = "macos")]
    {
        unsafe { cospi(x) }
    }
    #[cfg(not(target_os = "macos"))]
    {
        (x * std::f64::consts::PI).cos()
    }
}

fn sin_pi(x: f64) -> f64 {
    #[cfg(target_os = "macos")]
    {
        unsafe { sinpi(x) }
    }
    #[cfg(not(target_os = "macos"))]
    {
        (x * std::f64::consts::PI).sin()
    }
}

/// `rb_dbl_complex_new_polar_pi`: a magnitude turned by `angle` half-turns
/// — exact on the axes, a complex of floats elsewhere.
fn polar_pi(mut magnitude: f64, angle: f64) -> Value {
    let whole = angle.trunc();
    let fraction = angle - whole;
    let positive_half = fraction == 0.5;
    if positive_half || fraction == -0.5 {
        let half_whole = whole / 2.0;
        if ((half_whole - half_whole.trunc()) != fraction) ^ positive_half {
            magnitude = -magnitude;
        }
        return complex(Value::Float(0.0), Value::Float(magnitude));
    }
    if fraction == 0.0 {
        let half_whole = whole / 2.0;
        if half_whole - half_whole.trunc() != 0.0 {
            magnitude = -magnitude;
        }
        return Value::Float(magnitude);
    }
    complex(
        Value::Float(magnitude * cos_pi(angle)),
        Value::Float(magnitude * sin_pi(angle)),
    )
}

// — complex numbers —

fn complex_parts(value: &Value) -> (Value, Value) {
    match value {
        Value::Complex { real, imaginary } => (real.as_ref().clone(), imaginary.as_ref().clone()),
        other => (other.clone(), Value::Integer(0)),
    }
}

/// `safe_mul`: a zero times an infinity is a zero, not NaN — the float
/// stands in as its sign.
fn safe_mul(left: &Value, right: &Value, left_zero: bool, right_zero: bool) -> Value {
    let unit = |value: &Value| -> Value {
        match value {
            Value::Float(number) if !number.is_nan() => {
                Value::Float(if number.is_sign_negative() { -1.0 } else { 1.0 })
            }
            other => other.clone(),
        }
    };
    let left = if !left_zero && right_zero {
        unit(left)
    } else {
        left.clone()
    };
    let right = if !right_zero && left_zero {
        unit(right)
    } else {
        right.clone()
    };
    real_mul(&left, &right)
}

/// `comp_mul`: the product of two complexes, part by part.
fn complex_mul_parts(
    left_real: &Value,
    left_imaginary: &Value,
    right_real: &Value,
    right_imaginary: &Value,
) -> (Value, Value) {
    let (left_real_zero, left_imaginary_zero) = (exact_zero(left_real), exact_zero(left_imaginary));
    let (right_real_zero, right_imaginary_zero) =
        (exact_zero(right_real), exact_zero(right_imaginary));
    let real = real_sub(
        &safe_mul(left_real, right_real, left_real_zero, right_real_zero),
        &safe_mul(
            left_imaginary,
            right_imaginary,
            left_imaginary_zero,
            right_imaginary_zero,
        ),
    );
    let imaginary = real_add(
        &safe_mul(
            left_real,
            right_imaginary,
            left_real_zero,
            right_imaginary_zero,
        ),
        &safe_mul(
            left_imaginary,
            right_real,
            left_imaginary_zero,
            right_real_zero,
        ),
    );
    (real, imaginary)
}

fn has_float(values: &[&Value]) -> bool {
    values.iter().any(|value| matches!(value, Value::Float(_)))
}

/// `f_divide` with `quo`: a real divisor divides each part; a complex one
/// scales by its larger component first, and exact answers come back
/// canonical.
fn complex_div(left: &Value, right: &Value) -> Value {
    let (left_real, left_imaginary) = complex_parts(left);
    if is_real(right) {
        let floats = has_float(&[&left_real, &left_imaginary, right]);
        let real = real_quo(&left_real, right);
        let imaginary = real_quo(&left_imaginary, right);
        return if floats {
            complex(real, imaginary)
        } else {
            complex(canonicalize(real), canonicalize(imaginary))
        };
    }
    let (right_real, right_imaginary) = complex_parts(right);
    let floats = has_float(&[&left_real, &left_imaginary, &right_real, &right_imaginary]);
    let one = Value::Integer(1);
    let (real, imaginary) = if real_cmp(&real_abs(&right_real), &real_abs(&right_imaginary))
        == Some(std::cmp::Ordering::Greater)
    {
        let ratio = real_quo(&right_imaginary, &right_real);
        let scale = real_mul(&right_real, &real_add(&one, &real_mul(&ratio, &ratio)));
        (
            real_quo(
                &real_add(&left_real, &real_mul(&left_imaginary, &ratio)),
                &scale,
            ),
            real_quo(
                &real_sub(&left_imaginary, &real_mul(&left_real, &ratio)),
                &scale,
            ),
        )
    } else {
        let ratio = real_quo(&right_real, &right_imaginary);
        let scale = real_mul(&right_imaginary, &real_add(&one, &real_mul(&ratio, &ratio)));
        (
            real_quo(
                &real_add(&real_mul(&left_real, &ratio), &left_imaginary),
                &scale,
            ),
            real_quo(
                &real_sub(&real_mul(&left_imaginary, &ratio), &left_real),
                &scale,
            ),
        )
    };
    if floats {
        complex(real, imaginary)
    } else {
        complex(canonicalize(real), canonicalize(imaginary))
    }
}

/// `rb_complex_abs`: a part that is zero leaves the other's magnitude,
/// else the hypotenuse.
pub fn complex_abs(value: &Value) -> Value {
    let (real, imaginary) = complex_parts(value);
    if any_zero(&real) {
        let magnitude = real_abs(&imaginary);
        return if matches!(real, Value::Float(_)) && !matches!(imaginary, Value::Float(_)) {
            Value::Float(to_f64(&magnitude))
        } else {
            magnitude
        };
    }
    if any_zero(&imaginary) {
        let magnitude = real_abs(&real);
        return if matches!(imaginary, Value::Float(_)) && !matches!(real, Value::Float(_)) {
            Value::Float(to_f64(&magnitude))
        } else {
            magnitude
        };
    }
    Value::Float(to_f64(&real).hypot(to_f64(&imaginary)))
}

fn complex_arg(value: &Value) -> Value {
    let (real, imaginary) = complex_parts(value);
    Value::Float(to_f64(&imaginary).atan2(to_f64(&real)))
}

/// `f_complex_polar_real`: a complex from a magnitude and an angle, exact
/// on the axes and at the half turns where the float angle lands on them.
pub fn complex_polar(magnitude: &Value, angle: &Value) -> Value {
    if any_zero(magnitude) || any_zero(angle) {
        return complex(magnitude.clone(), Value::Float(0.0));
    }
    if let Value::Float(arg) = angle {
        if *arg == std::f64::consts::PI {
            return complex(real_negate(magnitude), Value::Float(0.0));
        }
        if *arg == std::f64::consts::FRAC_PI_2 {
            return complex(Value::Float(0.0), magnitude.clone());
        }
        if *arg == -std::f64::consts::FRAC_PI_2 {
            return complex(Value::Float(0.0), real_negate(magnitude));
        }
        if let Value::Float(abs) = magnitude {
            return complex(Value::Float(abs * arg.cos()), Value::Float(abs * arg.sin()));
        }
        return complex(
            real_mul(magnitude, &Value::Float(arg.cos())),
            real_mul(magnitude, &Value::Float(arg.sin())),
        );
    }
    let arg = to_f64(angle);
    complex(
        real_mul(magnitude, &Value::Float(arg.cos())),
        real_mul(magnitude, &Value::Float(arg.sin())),
    )
}

/// `rb_complex_pow`: an exact zero exponent is one; an integer exponent
/// squares and multiplies as CRuby does (a negative one over the
/// reciprocal); a complex exponent goes through exp and log; any other
/// real exponent through the polar form.
fn complex_pow(left: &Value, right: &Value) -> Value {
    if is_real(right) && exact_zero(right) {
        return complex(Value::Integer(1), Value::Integer(0));
    }
    let right = match right {
        Value::Rational {
            numerator,
            denominator: 1,
        } => Value::Integer(*numerator),
        Value::Complex { real, imaginary } if exact_zero(imaginary) => real.as_ref().clone(),
        other => other.clone(),
    };
    if let Value::Complex {
        real: other_real,
        imaginary: other_imaginary,
    } = &right
    {
        let magnitude = to_f64(&complex_abs(left));
        let theta = to_f64(&complex_arg(left));
        let log = magnitude.ln();
        let new_magnitude = (to_f64(other_real) * log - to_f64(other_imaginary) * theta).exp();
        let new_theta = theta * to_f64(other_real) + to_f64(other_imaginary) * log;
        return complex_polar(&Value::Float(new_magnitude), &Value::Float(new_theta));
    }
    if let Value::Integer(exponent) = right {
        let (base, count) = if exponent < 0 {
            (
                complex_div(&Value::Integer(1), left),
                exponent.unsigned_abs(),
            )
        } else {
            (left.clone(), exponent.unsigned_abs())
        };
        let (mut xr, mut xi) = complex_parts(&base);
        let (mut zr, mut zi) = (xr.clone(), xi.clone());
        if exact_zero(&xi) {
            zr = real_pow(&zr, &Value::Integer(count as i64));
        } else if exact_zero(&xr) {
            zi = real_pow(&zi, &Value::Integer(count as i64));
            if count & 2 != 0 {
                zi = real_negate(&zi);
            }
            if count & 1 == 0 {
                std::mem::swap(&mut zr, &mut zi);
            }
        } else {
            let mut n = count;
            loop {
                n -= 1;
                if n == 0 {
                    break;
                }
                loop {
                    let (quotient, remainder) = (n / 2, n % 2);
                    if remainder != 0 {
                        break;
                    }
                    let squared_real = real_sub(&real_mul(&xr, &xr), &real_mul(&xi, &xi));
                    xi = real_mul(&real_mul(&Value::Integer(2), &xr), &xi);
                    xr = squared_real;
                    n = quotient;
                }
                let (next_real, next_imaginary) = complex_mul_parts(&zr, &zi, &xr, &xi);
                zr = next_real;
                zi = next_imaginary;
            }
        }
        return complex(zr, zi);
    }
    let magnitude = complex_abs(left);
    let theta = complex_arg(left);
    complex_polar(&real_pow(&magnitude, &right), &real_mul(&theta, &right))
}

fn complex_eq(left: &Value, right: &Value) -> bool {
    let (left_real, left_imaginary) = complex_parts(left);
    match right {
        Value::Complex { real, imaginary } => {
            real_eq(&left_real, real) && real_eq(&left_imaginary, imaginary)
        }
        other if is_real(other) => real_eq(&left_real, other) && any_zero(&left_imaginary),
        _ => false,
    }
}

/// `<=>` between complexes, or a complex and a real: an ordering only
/// where every imaginary part is zero, nil otherwise — Ruby's answer.
fn complex_cmp(left: &Value, right: &Value) -> Value {
    let (left_real, left_imaginary) = complex_parts(left);
    let (right_real, right_imaginary) = complex_parts(right);
    if !any_zero(&left_imaginary) || !any_zero(&right_imaginary) {
        return Value::Nil;
    }
    match real_cmp(&left_real, &right_real) {
        Some(ordering) => Value::Integer(ordering as i64),
        None => Value::Nil,
    }
}

// — the doors the interpreter knocks on —

fn involves_tower(left: &Value, right: &Value) -> bool {
    matches!(left, Value::Rational { .. } | Value::Complex { .. })
        || matches!(right, Value::Rational { .. } | Value::Complex { .. })
}

/// A binary operator over the tower (ADR 0085): any pair with a rational or
/// a complex in it, an integer to a negative power, and a negative base
/// to a fractional float power. `None` leaves the pair to the integer and
/// float arms.
pub fn binary(left: &Value, operator: BinaryOperator, right: &Value) -> Option<Value> {
    let numeric = |value: &Value| is_real(value) || matches!(value, Value::Complex { .. });
    if !numeric(left) || !numeric(right) {
        return None;
    }
    if !involves_tower(left, right) {
        // The two floors below, where they reach up: `2 ** -1` is a
        // rational, `(-8.0) ** (1.0 / 3)` a complex.
        return match (left, operator, right) {
            (Value::Integer(_), BinaryOperator::Power, Value::Integer(exponent))
                if *exponent < 0 =>
            {
                Some(real_pow(left, right))
            }
            (_, BinaryOperator::Power, _)
                if to_f64(left) < 0.0
                    && (matches!(left, Value::Float(_)) || matches!(right, Value::Float(_)))
                    && to_f64(right).fract() != 0.0
                    && to_f64(right).is_finite() =>
            {
                Some(real_pow(left, right))
            }
            _ => None,
        };
    }
    let complex_involved =
        matches!(left, Value::Complex { .. }) || matches!(right, Value::Complex { .. });
    if complex_involved {
        let answer = match operator {
            BinaryOperator::Add => {
                let (left_real, left_imaginary) = complex_parts(left);
                let (right_real, right_imaginary) = complex_parts(right);
                complex(
                    real_add(&left_real, &right_real),
                    real_add(&left_imaginary, &right_imaginary),
                )
            }
            BinaryOperator::Subtract => {
                let (left_real, left_imaginary) = complex_parts(left);
                let (right_real, right_imaginary) = complex_parts(right);
                complex(
                    real_sub(&left_real, &right_real),
                    real_sub(&left_imaginary, &right_imaginary),
                )
            }
            BinaryOperator::Multiply => {
                if is_real(right) {
                    let (left_real, left_imaginary) = complex_parts(left);
                    complex(
                        real_mul(&left_real, right),
                        real_mul(&left_imaginary, right),
                    )
                } else if is_real(left) {
                    let (right_real, right_imaginary) = complex_parts(right);
                    complex(
                        real_mul(left, &right_real),
                        real_mul(left, &right_imaginary),
                    )
                } else {
                    let (left_real, left_imaginary) = complex_parts(left);
                    let (right_real, right_imaginary) = complex_parts(right);
                    let (real, imaginary) = complex_mul_parts(
                        &left_real,
                        &left_imaginary,
                        &right_real,
                        &right_imaginary,
                    );
                    complex(real, imaginary)
                }
            }
            BinaryOperator::Divide => complex_div(left, right),
            BinaryOperator::Power => complex_pow(left, right),
            BinaryOperator::Equals => Value::Boolean(complex_eq(left, right)),
            BinaryOperator::NotEquals => Value::Boolean(!complex_eq(left, right)),
            BinaryOperator::Spaceship => complex_cmp(left, right),
            _ => panic!(
                "cannot apply '{}' to {} and {}",
                operator.glyph(),
                left.shown(),
                right.shown()
            ),
        };
        return Some(answer);
    }
    let answer = match operator {
        BinaryOperator::Add => real_add(left, right),
        BinaryOperator::Subtract => real_sub(left, right),
        BinaryOperator::Multiply => real_mul(left, right),
        BinaryOperator::Divide => real_quo(left, right),
        BinaryOperator::Modulo => real_modulo(left, right),
        BinaryOperator::Power => real_pow(left, right),
        BinaryOperator::Equals => Value::Boolean(real_eq(left, right)),
        BinaryOperator::NotEquals => Value::Boolean(!real_eq(left, right)),
        BinaryOperator::Spaceship => match real_cmp(left, right) {
            Some(ordering) => Value::Integer(ordering as i64),
            None => Value::Nil,
        },
        BinaryOperator::Less => ordered(left, right, |ordering| ordering.is_lt()),
        BinaryOperator::LessOrEqual => ordered(left, right, |ordering| ordering.is_le()),
        BinaryOperator::Greater => ordered(left, right, |ordering| ordering.is_gt()),
        BinaryOperator::GreaterOrEqual => ordered(left, right, |ordering| ordering.is_ge()),
    };
    Some(answer)
}

fn ordered(left: &Value, right: &Value, test: impl Fn(std::cmp::Ordering) -> bool) -> Value {
    match real_cmp(left, right) {
        Some(ordering) => Value::Boolean(test(ordering)),
        None => panic!(
            "cannot apply an ordering to {} and {}",
            left.shown(),
            right.shown()
        ),
    }
}

/// Unary minus over the tower.
pub fn negate(value: &Value) -> Option<Value> {
    match value {
        Value::Rational { .. } => Some(real_negate(value)),
        Value::Complex { real, imaginary } => {
            Some(complex(real_negate(real), real_negate(imaginary)))
        }
        _ => None,
    }
}

/// `Rational(a)` and `Rational(a, b)`: each part any real, the quotient
/// reduced; `Rational(1, 0)` refuses as `1 / 0` does.
pub fn rational_function(arguments: &[Value]) -> Value {
    // A complex argument stands for its real part when its imaginary part is
    // an exact zero, as Ruby reads it; `Rational(1, 2+1i)` is Ruby's one
    // odd corner, the quotient itself, a complex.
    let real_or_complex = |value: &Value| -> Value {
        match value {
            Value::Complex { imaginary, .. } if exact_zero(imaginary) => {
                real_of_complex(value, "Rational")
            }
            other => other.clone(),
        }
    };
    match arguments {
        [value] => to_rational(&real_of_complex_or_self(value), "Rational"),
        [numerator, denominator] => {
            let numerator = to_rational(&real_of_complex_or_self(numerator), "Rational");
            let denominator = real_or_complex(denominator);
            if matches!(denominator, Value::Complex { .. }) {
                return binary(&numerator, BinaryOperator::Divide, &denominator)
                    .unwrap_or_else(|| panic!("Rational takes numbers, got {denominator:?}"));
            }
            let denominator = to_rational(&denominator, "Rational");
            real_quo(&numerator, &denominator)
        }
        _ => panic!("Rational takes 1 or 2 arguments, got {}", arguments.len()),
    }
}

/// A complex with an exact-zero imaginary part is its real part to
/// `Rational`; any other complex refuses in Ruby's words; a real is itself.
fn real_of_complex_or_self(value: &Value) -> Value {
    match value {
        Value::Complex { .. } => real_of_complex(value, "Rational"),
        other => other.clone(),
    }
}

/// `Complex(a)` and `Complex(a, b)`: two real parts.
pub fn complex_function(arguments: &[Value]) -> Value {
    let part = |value: &Value| -> Value {
        if is_real(value) || matches!(value, Value::Complex { .. }) {
            value.clone()
        } else {
            panic!("Complex takes numbers, got {}", value.inspect())
        }
    };
    match arguments {
        // A complex argument is itself; two of them are `a + b * i`, as
        // Ruby reads them.
        [value] => match part(value) {
            whole @ Value::Complex { .. } => whole,
            real => complex(real, Value::Integer(0)),
        },
        [real, imaginary] => {
            let (real, imaginary) = (part(real), part(imaginary));
            if is_real(&real) && is_real(&imaginary) {
                return complex(real, imaginary);
            }
            let unit = complex(Value::Integer(0), Value::Integer(1));
            let scaled = binary(&imaginary, BinaryOperator::Multiply, &unit)
                .unwrap_or_else(|| panic!("Complex takes numbers, got {imaginary:?}"));
            binary(&real, BinaryOperator::Add, &scaled)
                .unwrap_or_else(|| panic!("Complex takes numbers, got {real:?}"))
        }
        _ => panic!("Complex takes 1 or 2 arguments, got {}", arguments.len()),
    }
}

/// `Complex.polar(r, theta)` and `Complex.rectangular(a, b)`, the two type
/// functions; one argument leaves the other at zero.
pub fn complex_type_function(name: &str, arguments: &[Value]) -> Option<Value> {
    let real_parts: Vec<&Value> = arguments.iter().filter(|value| is_real(value)).collect();
    if real_parts.len() != arguments.len() {
        panic!(
            "Complex.{name} takes numbers, got {}",
            arguments
                .iter()
                .map(Value::inspect)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    match (name, arguments) {
        ("polar", [magnitude]) => Some(complex_polar(magnitude, &Value::Integer(0))),
        ("polar", [magnitude, angle]) => Some(complex_polar(magnitude, angle)),
        ("rectangular" | "rect", [real]) => Some(complex(real.clone(), Value::Integer(0))),
        ("rectangular" | "rect", [real, imaginary]) => {
            Some(complex(real.clone(), imaginary.clone()))
        }
        ("polar" | "rectangular" | "rect", _) => {
            panic!(
                "Complex.{name} takes 1 or 2 arguments, got {}",
                arguments.len()
            )
        }
        _ => None,
    }
}

/// The complex's real part when its imaginary part is an exact zero — what
/// `to_r`, `to_f`, `to_i`, and `rationalize` convert — else the refusal in
/// Ruby's words.
fn real_of_complex(value: &Value, target: &str) -> Value {
    let (real, imaginary) = complex_parts(value);
    if exact_zero(&imaginary) {
        return real;
    }
    panic!("can't convert {value} into {target}")
}

/// The tower's methods (ADR 0085): a rational's and a complex's own, and
/// the names every real answers — `numerator`, `to_r`, `to_c`, `real`,
/// `imaginary`, `arg`, `polar`, `rectangular`, `conjugate`, `abs2`, `i`.
/// `None` leaves the name to the general arms.
pub fn method(receiver: &Value, name: &str, arguments: &[Value]) -> Option<Value> {
    let numeric = is_real(receiver) || matches!(receiver, Value::Complex { .. });
    if !numeric {
        return None;
    }
    if let Value::Complex { real, imaginary } = receiver {
        let real = real.as_ref();
        let imaginary = imaginary.as_ref();
        let answer = match (name, arguments) {
            ("real", []) => real.clone(),
            ("imaginary" | "imag", []) => imaginary.clone(),
            ("real?", []) => Value::Boolean(false),
            ("integer?", []) => Value::Boolean(false),
            // Ruby's: the parts' denominators' lcm, and the parts scaled
            // onto it — for exact parts; a float part has no denominator.
            ("denominator", []) if is_exact(real) && is_exact(imaginary) => {
                let (_, real_denominator) = fraction(real);
                let (_, imaginary_denominator) = fraction(imaginary);
                let shared = real_denominator / gcd(real_denominator, imaginary_denominator)
                    * imaginary_denominator;
                Value::Integer(narrow(
                    shared,
                    &format!("{}.denominator", receiver.inspect()),
                ))
            }
            ("numerator", []) if is_exact(real) && is_exact(imaginary) => {
                let (real_numerator, real_denominator) = fraction(real);
                let (imaginary_numerator, imaginary_denominator) = fraction(imaginary);
                let shared = real_denominator / gcd(real_denominator, imaginary_denominator)
                    * imaginary_denominator;
                let written = format!("{}.numerator", receiver.inspect());
                complex(
                    Value::Integer(narrow(
                        real_numerator * (shared / real_denominator),
                        &written,
                    )),
                    Value::Integer(narrow(
                        imaginary_numerator * (shared / imaginary_denominator),
                        &written,
                    )),
                )
            }
            ("abs" | "magnitude", []) => complex_abs(receiver),
            ("abs2", []) => real_add(&real_mul(real, real), &real_mul(imaginary, imaginary)),
            ("arg" | "angle" | "phase", []) => complex_arg(receiver),
            ("polar", []) => Value::array(vec![complex_abs(receiver), complex_arg(receiver)]),
            ("rectangular" | "rect", []) => Value::array(vec![real.clone(), imaginary.clone()]),
            ("conjugate" | "conj", []) => complex(real.clone(), real_negate(imaginary)),
            ("to_c", []) => receiver.clone(),
            ("to_r", []) => to_rational(&real_of_complex(receiver, "Rational"), "to_r"),
            ("rationalize", []) => rationalize(&real_of_complex(receiver, "Rational"), None),
            ("rationalize", [epsilon]) => {
                rationalize(&real_of_complex(receiver, "Rational"), Some(epsilon))
            }
            ("to_f", []) => Value::Float(to_f64(&real_of_complex(receiver, "Float"))),
            ("to_i", []) => real_truncate(&real_of_complex(receiver, "Integer")),
            ("finite?", []) => {
                Value::Boolean(to_f64(real).is_finite() && to_f64(imaginary).is_finite())
            }
            ("infinite?", []) => {
                if to_f64(real).is_infinite() || to_f64(imaginary).is_infinite() {
                    Value::Integer(1)
                } else {
                    Value::Nil
                }
            }
            ("zero?", []) => Value::Boolean(any_zero(real) && any_zero(imaginary)),
            ("fdiv", [other]) if is_real(other) => complex(
                Value::Float(to_f64(real) / to_f64(other)),
                Value::Float(to_f64(imaginary) / to_f64(other)),
            ),
            ("quo", [other]) if is_real(other) || matches!(other, Value::Complex { .. }) => {
                complex_div(receiver, other)
            }
            ("pow", [other]) => complex_pow(receiver, other),
            _ => return None,
        };
        return Some(answer);
    }
    // The reals: integers, floats, rationals.
    let answer = match (name, arguments) {
        // NaN and the infinities have no rational: Ruby answers the float
        // itself as the numerator and 1 as the denominator, and so does this.
        ("numerator", []) => match receiver {
            Value::Integer(_) => receiver.clone(),
            Value::Float(number) if !number.is_finite() => receiver.clone(),
            Value::Float(number) => match float_to_rational(*number) {
                Value::Rational { numerator, .. } => Value::Integer(numerator),
                other => other,
            },
            Value::Rational { numerator, .. } => Value::Integer(*numerator),
            _ => return None,
        },
        ("denominator", []) => match receiver {
            Value::Integer(_) => Value::Integer(1),
            Value::Float(number) if !number.is_finite() => Value::Integer(1),
            Value::Float(number) => match float_to_rational(*number) {
                Value::Rational { denominator, .. } => Value::Integer(denominator),
                other => other,
            },
            Value::Rational { denominator, .. } => Value::Integer(*denominator),
            _ => return None,
        },
        ("to_r", []) => to_rational(receiver, "to_r"),
        ("to_c", []) => complex(receiver.clone(), Value::Integer(0)),
        ("rationalize", []) => rationalize(receiver, None),
        ("rationalize", [epsilon]) => rationalize(receiver, Some(epsilon)),
        ("real", []) => receiver.clone(),
        ("imaginary" | "imag", []) => Value::Integer(0),
        ("real?", []) => Value::Boolean(true),
        ("conjugate" | "conj", []) => receiver.clone(),
        ("arg" | "angle" | "phase", []) => match receiver {
            Value::Float(number) if number.is_nan() => receiver.clone(),
            _ if negative_part(receiver) => Value::Float(std::f64::consts::PI),
            _ => Value::Integer(0),
        },
        ("rectangular" | "rect", []) => Value::array(vec![receiver.clone(), Value::Integer(0)]),
        ("polar", []) => {
            let angle = match receiver {
                Value::Float(number) if number.is_nan() => receiver.clone(),
                _ if negative_part(receiver) => Value::Float(std::f64::consts::PI),
                _ => Value::Integer(0),
            };
            Value::array(vec![real_abs(receiver), angle])
        }
        ("abs2", []) => real_mul(receiver, receiver),
        ("i", []) => complex(Value::Integer(0), receiver.clone()),
        ("quo", [other]) if is_real(other) => real_quo(receiver, other),
        ("quo", [other @ Value::Complex { .. }]) => complex_div(receiver, other),
        _ => {
            let Value::Rational {
                numerator,
                denominator,
            } = receiver
            else {
                return None;
            };
            match (name, arguments) {
                ("to_i" | "truncate", []) => real_truncate(receiver),
                ("to_f", []) => Value::Float(to_f64(receiver)),
                ("floor", []) => real_floor(receiver),
                ("ceil", []) => real_ceil(receiver),
                ("round", []) => rational_round(*numerator, *denominator),
                ("abs" | "magnitude", []) => real_abs(receiver),
                ("zero?", []) => Value::Boolean(*numerator == 0),
                ("positive?", []) => Value::Boolean(*numerator > 0),
                ("negative?", []) => Value::Boolean(*numerator < 0),
                ("integer?", []) => Value::Boolean(false),
                ("finite?", []) => Value::Boolean(true),
                ("infinite?", []) => Value::Nil,
                ("fdiv", [other]) if is_real(other) => {
                    Value::Float(to_f64(receiver) / to_f64(other))
                }
                ("div", [other]) if is_real(other) => real_floor(&real_quo(receiver, other)),
                ("modulo", [other]) if is_real(other) => real_modulo(receiver, other),
                ("divmod", [other]) if is_real(other) => Value::array(vec![
                    real_floor(&real_quo(receiver, other)),
                    real_modulo(receiver, other),
                ]),
                ("remainder", [other]) if is_real(other) => {
                    let quotient = real_truncate(&real_quo(receiver, other));
                    real_sub(receiver, &real_mul(&quotient, other))
                }
                ("pow", [other]) if is_real(other) => real_pow(receiver, other),
                ("pow", [other @ Value::Complex { .. }]) => complex_pow(receiver, other),
                _ => return None,
            }
        }
    };
    Some(answer)
}

fn real_truncate(value: &Value) -> Value {
    match value {
        Value::Integer(_) => value.clone(),
        Value::Float(number) => float_integer(number.trunc(), value),
        Value::Rational {
            numerator,
            denominator,
        } => Value::Integer(numerator / denominator),
        other => unreachable!("a real number, got {other:?}"),
    }
}

/// `Rational#inspect` and `Complex#inspect` (`f_format` with `inspect`):
/// `(1/3)`, `(1+2i)`, `(1-(1/2)*i)` — the imaginary part signed by its
/// sign bit, starred when its text does not end in a digit.
pub fn inspect(value: &Value) -> String {
    match value {
        Value::Rational {
            numerator,
            denominator,
        } => format!("({numerator}/{denominator})"),
        Value::Complex { .. } => format!("({})", format_complex(value, Value::inspect)),
        other => other.inspect(),
    }
}

/// `to_s`: the same forms without the parentheses.
pub fn to_s(value: &Value) -> String {
    match value {
        Value::Rational {
            numerator,
            denominator,
        } => format!("{numerator}/{denominator}"),
        Value::Complex { .. } => format_complex(value, |part| part.to_string()),
        other => other.to_string(),
    }
}

fn format_complex(value: &Value, render: impl Fn(&Value) -> String) -> String {
    let (real, imaginary) = complex_parts(value);
    let sign = if negative_part(&imaginary) { "-" } else { "+" };
    let magnitude = match &imaginary {
        Value::Float(number) => render(&Value::Float(number.abs())),
        other => render(&real_abs(other)),
    };
    let star = if magnitude.ends_with(|character: char| character.is_ascii_digit()) {
        ""
    } else {
        "*"
    };
    format!("{}{sign}{magnitude}{star}i", render(&real))
}

/// Ruby's `==` where a rational or complex is involved: by value across
/// the reals, a complex equal to a real only with a zero imaginary part.
pub fn equals(left: &Value, right: &Value) -> Option<bool> {
    if !involves_tower(left, right) {
        return None;
    }
    let numeric = |value: &Value| is_real(value) || matches!(value, Value::Complex { .. });
    if !numeric(left) || !numeric(right) {
        return Some(false);
    }
    if matches!(left, Value::Complex { .. }) {
        return Some(complex_eq(left, right));
    }
    if matches!(right, Value::Complex { .. }) {
        return Some(complex_eq(right, left));
    }
    Some(real_eq(left, right))
}

// — reading a number from text: `String#to_r` and `String#to_c` —
//
// Ported from Ruby 4.0.9's own readers, so every answer is Ruby's: `to_r`
// is rational.c's `parse_rat` over `read_num`, and `to_c` is complex.c's
// `read_comp`, which copies the characters it accepts into a buffer and
// reads that buffer back by its shape. Both are lenient: leading spaces
// are skipped, reading stops at the first character that does not fit, and
// text with no number answers zero. Ruby's bignums are 64-bit here (ADR
// 0070), so a part past the edge refuses.

/// A byte at a position, or 0 past the end, as a C string reads.
fn byte_at(bytes: &[u8], position: usize) -> u8 {
    bytes.get(position).copied().unwrap_or(0)
}

fn skip_spaces(bytes: &[u8], mut position: usize) -> usize {
    while byte_at(bytes, position).is_ascii_whitespace() {
        position += 1;
    }
    position
}

/// rational.c's `rb_int_parse_cstr` with underscores: decimal digits, one
/// underscore at a time between them. Answers the value, how many digits
/// were read, and where reading stopped, or none when no digit starts here.
fn read_decimal(bytes: &[u8], start: usize, text: &str) -> Option<(i128, u32, usize)> {
    if !byte_at(bytes, start).is_ascii_digit() {
        return None;
    }
    let mut value: i128 = 0;
    let mut count = 0u32;
    let mut position = start;
    loop {
        let byte = byte_at(bytes, position);
        if byte.is_ascii_digit() {
            value = value
                .checked_mul(10)
                .and_then(|scaled| scaled.checked_add(i128::from(byte - b'0')))
                .unwrap_or_else(|| panic!("{text:?} overflows the 64-bit integers"));
            count += 1;
            position += 1;
        } else if byte == b'_' && byte_at(bytes, position + 1).is_ascii_digit() {
            position += 1;
        } else {
            break;
        }
    }
    Some((value, count, position))
}

/// rational.c's `read_num`: an integer part, a fraction, an exponent. The
/// value is the digits as one integer and `exponent` the power of ten to
/// divide it by; `None` when nothing number-like starts here.
fn read_rational_number(bytes: &[u8], start: usize, text: &str) -> Option<(i128, i128, usize)> {
    let mut position = start;
    let mut number: i128 = 0;
    let mut exponent: i128 = 0;
    let mut fraction_digits: i128 = 0;
    let mut ok = false;
    if position < bytes.len() && bytes[position] != b'.' {
        let (value, _, after) = read_decimal(bytes, position, text)?;
        number = value;
        position = after;
        ok = true;
    }
    if position < bytes.len() && bytes[position] == b'.' {
        position += 1;
        let Some((fraction, count, after)) = read_decimal(bytes, position, text) else {
            return Some((number, exponent, position));
        };
        position = after;
        let scale = 10i128
            .checked_pow(count)
            .unwrap_or_else(|| panic!("{text:?} overflows the 64-bit integers"));
        number = number
            .checked_mul(scale)
            .and_then(|scaled| scaled.checked_add(fraction))
            .unwrap_or_else(|| panic!("{text:?} overflows the 64-bit integers"));
        exponent = i128::from(count);
        fraction_digits = i128::from(count);
        ok = true;
    }
    if ok && position + 1 < bytes.len() && matches!(bytes[position], b'e' | b'E') {
        position += 1;
        let negative = match byte_at(bytes, position) {
            b'-' => {
                position += 1;
                true
            }
            b'+' => {
                position += 1;
                false
            }
            _ => false,
        };
        let Some((power, _, after)) = read_decimal(bytes, position, text) else {
            return Some((number, exponent, position));
        };
        position = after;
        if power != 0 {
            exponent = if negative {
                power + fraction_digits
            } else {
                fraction_digits - power
            };
        }
    }
    if ok {
        Some((number, exponent, position))
    } else {
        None
    }
}

/// Ten to a power, or a refusal past the integers.
fn power_of_ten(power: i128, text: &str) -> i128 {
    u32::try_from(power)
        .ok()
        .and_then(|power| 10i128.checked_pow(power))
        .unwrap_or_else(|| panic!("{text:?} overflows the 64-bit integers"))
}

/// `String#to_r`: rational.c's `parse_rat`, lenient.
pub fn string_to_r(text: &str) -> Value {
    let bytes = text.as_bytes();
    let mut position = skip_spaces(bytes, 0);
    let negative = match byte_at(bytes, position) {
        b'-' => {
            position += 1;
            true
        }
        b'+' => {
            position += 1;
            false
        }
        _ => false,
    };
    let Some((mut numerator, mut exponent, after)) = read_rational_number(bytes, position, text)
    else {
        return rational(0, 1);
    };
    position = after;
    let mut denominator: i128 = 1;
    if byte_at(bytes, position) == b'/'
        && let Some((below, below_exponent, _)) = read_rational_number(bytes, position + 1, text)
    {
        if below == 0 {
            panic!("divided by 0");
        }
        denominator = below;
        exponent -= below_exponent;
    }
    if exponent < 0 {
        numerator = numerator
            .checked_mul(power_of_ten(-exponent, text))
            .unwrap_or_else(|| panic!("{text:?} overflows the 64-bit integers"));
    } else if exponent > 0 {
        denominator = denominator
            .checked_mul(power_of_ten(exponent, text))
            .unwrap_or_else(|| panic!("{text:?} overflows the 64-bit integers"));
    }
    if negative {
        numerator = -numerator;
    }
    rational_wide(numerator, denominator, &format!("{text:?}.to_r"))
}

/// complex.c's reader: the text, where reading stands, and the buffer the
/// accepted characters are copied into.
struct ComplexReader<'text> {
    bytes: &'text [u8],
    position: usize,
    buffer: String,
}

impl ComplexReader<'_> {
    fn at(&self) -> u8 {
        byte_at(self.bytes, self.position)
    }

    fn take(&mut self) {
        self.buffer.push(char::from(self.at()));
        self.position += 1;
    }

    /// `read_sign`: a sign, copied, or nothing.
    fn read_sign(&mut self) -> u8 {
        let sign = self.at();
        if matches!(sign, b'+' | b'-') {
            self.take();
            sign
        } else {
            b'?'
        }
    }

    /// `read_digits`: digits with single underscores, which are not copied;
    /// a doubled or trailing underscore ends the run and reading backs up
    /// to the last digit, as Ruby's does.
    fn read_digits(&mut self) -> bool {
        if !self.at().is_ascii_digit() {
            return false;
        }
        let mut after_underscore = true;
        while self.at().is_ascii_digit() || self.at() == b'_' {
            if self.at() == b'_' {
                if after_underscore {
                    break;
                }
                after_underscore = true;
                self.position += 1;
            } else {
                self.take();
                after_underscore = false;
            }
        }
        if after_underscore {
            loop {
                self.position -= 1;
                if self.at() != b'_' {
                    break;
                }
            }
        }
        true
    }

    /// `read_num`: digits, a fraction, an exponent.
    fn read_num(&mut self) -> bool {
        if self.at() != b'.' && !self.read_digits() {
            return false;
        }
        if self.at() == b'.' {
            self.take();
            if !self.read_digits() {
                self.buffer.pop();
                return false;
            }
        }
        if matches!(self.at(), b'e' | b'E') {
            self.take();
            self.read_sign();
            if !self.read_digits() {
                self.buffer.pop();
                return false;
            }
        }
        true
    }

    /// `read_rat_nos`: a number, then an optional `/denominator`.
    fn read_rat_nos(&mut self) -> bool {
        if !self.read_num() {
            return false;
        }
        if self.at() == b'/' {
            self.take();
            if !self.read_digits() {
                self.buffer.pop();
                return false;
            }
        }
        true
    }

    /// Start a fresh number in the buffer, keeping where the old one began.
    fn fresh(&mut self) -> usize {
        self.buffer.len()
    }
}

fn is_imaginary_unit(byte: u8) -> bool {
    matches!(byte, b'i' | b'I' | b'j' | b'J')
}

/// complex.c's `str2num`: the buffer read back by its shape — a slash a
/// rational, a point or an exponent a float, otherwise an integer.
fn buffer_number(written: &str, text: &str) -> Value {
    if written.contains('/') {
        return string_to_r(written);
    }
    if written.contains(['.', 'e', 'E']) {
        let trimmed = written.trim_end_matches(['e', 'E']);
        return Value::Float(trimmed.parse::<f64>().unwrap_or(0.0));
    }
    let digits = written.trim_start_matches(['+', '-']);
    if digits.is_empty() {
        return Value::Integer(0);
    }
    let magnitude: i128 = digits
        .parse()
        .unwrap_or_else(|_| panic!("{text:?} overflows the 64-bit integers"));
    let value = if written.starts_with('-') {
        -magnitude
    } else {
        magnitude
    };
    Value::Integer(narrow(value, &format!("{text:?}.to_c")))
}

/// `String#to_c`: complex.c's `read_comp`, lenient. Reading stops at a NUL,
/// as Ruby's C string does.
pub fn string_to_c(text: &str) -> Value {
    let text = text.split('\0').next().unwrap_or("");
    let bytes = text.as_bytes();
    let mut reader = ComplexReader {
        bytes,
        position: skip_spaces(bytes, 0),
        buffer: String::new(),
    };
    let sign = reader.read_sign();
    if is_imaginary_unit(reader.at()) {
        let unit = if sign == b'-' { -1 } else { 1 };
        return complex(Value::Integer(0), Value::Integer(unit));
    }
    if !reader.read_rat_nos() {
        return complex(buffer_number(&reader.buffer, text), Value::Integer(0));
    }
    let real = buffer_number(&reader.buffer, text);
    if is_imaginary_unit(reader.at()) {
        return complex(Value::Integer(0), real);
    }
    if reader.at() == b'@' {
        reader.position += 1;
        let start = reader.fresh();
        reader.read_sign();
        reader.read_rat_nos();
        let angle_text = reader.buffer[start..].to_string();
        if !angle_text.ends_with(|character: char| character.is_ascii_digit()) {
            return complex(real, Value::Integer(0));
        }
        return complex_polar(&real, &buffer_number(&angle_text, text));
    }
    if matches!(reader.at(), b'+' | b'-') {
        let start = reader.fresh();
        let sign = reader.read_sign();
        let imaginary = if is_imaginary_unit(reader.at()) {
            Value::Integer(if sign == b'-' { -1 } else { 1 })
        } else {
            if !reader.read_rat_nos() {
                return complex(real, Value::Integer(0));
            }
            buffer_number(&reader.buffer[start..], text)
        };
        if !is_imaginary_unit(reader.at()) {
            return complex(real, Value::Integer(0));
        }
        return complex(real, imaginary);
    }
    complex(real, Value::Integer(0))
}
