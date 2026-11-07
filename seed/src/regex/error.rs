//! The refusals a pattern can earn, in Ruby's own words. Most sentences are
//! Onigmo's (`regerror.c`); the escape ones are CRuby's (`re.c`), which
//! checks `\x`, `\u`, `\c`, and `\M-` before Onigmo sees the pattern. Ruby
//! renders a `RegexpError` as `<sentence>: /<source>/`; this type carries the
//! sentence alone, and the surface adds the source.

use std::fmt;

/// A refusal with Ruby's sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegexError {
    pub message: String,
}

impl RegexError {
    pub fn new(message: impl Into<String>) -> RegexError {
        RegexError {
            message: message.into(),
        }
    }

    /// Onigmo's `%n` sentences name the group or property in question.
    pub fn named(template: &str, name: &str) -> RegexError {
        RegexError::new(template.replace("%n", name))
    }
}

impl fmt::Display for RegexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RegexError {}

// Onigmo, regerror.c.
pub const MATCH_STACK_LIMIT_OVER: &str = "match-stack limit over";
pub const PARSE_DEPTH_LIMIT_OVER: &str = "parse depth limit over";
pub const END_PATTERN_AT_LEFT_BRACE: &str = "end pattern at left brace";
pub const END_PATTERN_AT_LEFT_BRACKET: &str = "end pattern at left bracket";
pub const EMPTY_CHAR_CLASS: &str = "empty char-class";
pub const PREMATURE_END_OF_CHAR_CLASS: &str = "premature end of char-class";
pub const END_PATTERN_AT_ESCAPE: &str = "end pattern at escape";
pub const END_PATTERN_AT_META: &str = "end pattern at meta";
pub const END_PATTERN_AT_CONTROL: &str = "end pattern at control";
pub const META_CODE_SYNTAX: &str = "invalid meta-code syntax";
pub const CONTROL_CODE_SYNTAX: &str = "invalid control-code syntax";
pub const CHAR_CLASS_VALUE_AT_END_OF_RANGE: &str = "char-class value at end of range";
pub const CHAR_CLASS_VALUE_AT_START_OF_RANGE: &str = "char-class value at start of range";
pub const UNMATCHED_RANGE_SPECIFIER_IN_CHAR_CLASS: &str =
    "unmatched range specifier in char-class";
pub const TARGET_OF_REPEAT_OPERATOR_NOT_SPECIFIED: &str =
    "target of repeat operator is not specified";
pub const TARGET_OF_REPEAT_OPERATOR_INVALID: &str = "target of repeat operator is invalid";
pub const NESTED_REPEAT_OPERATOR: &str = "nested repeat operator";
pub const UNMATCHED_CLOSE_PARENTHESIS: &str = "unmatched close parenthesis";
pub const END_PATTERN_WITH_UNMATCHED_PARENTHESIS: &str =
    "end pattern with unmatched parenthesis";
pub const END_PATTERN_IN_GROUP: &str = "end pattern in group";
pub const UNDEFINED_GROUP_OPTION: &str = "undefined group option";
pub const INVALID_POSIX_BRACKET_TYPE: &str = "invalid POSIX bracket type";
pub const INVALID_LOOK_BEHIND_PATTERN: &str = "invalid pattern in look-behind";
pub const INVALID_REPEAT_RANGE_PATTERN: &str = "invalid repeat range {lower,upper}";
pub const INVALID_CONDITION_PATTERN: &str = "invalid conditional pattern";
pub const TOO_BIG_NUMBER: &str = "too big number";
pub const TOO_BIG_NUMBER_FOR_REPEAT_RANGE: &str = "too big number for repeat range";
pub const UPPER_SMALLER_THAN_LOWER_IN_REPEAT_RANGE: &str =
    "upper is smaller than lower in repeat range";
pub const EMPTY_RANGE_IN_CHAR_CLASS: &str = "empty range in char class";
pub const MISMATCH_CODE_LENGTH_IN_CLASS_RANGE: &str =
    "mismatch multibyte code length in char-class range";
pub const TOO_MANY_MULTI_BYTE_RANGES: &str = "too many multibyte code ranges are specified";
pub const TOO_SHORT_MULTI_BYTE_STRING: &str = "too short multibyte code string";
pub const TOO_BIG_BACKREF_NUMBER: &str = "too big backref number";
pub const INVALID_BACKREF: &str = "invalid backref number/name";
pub const NUMBERED_BACKREF_OR_CALL_NOT_ALLOWED: &str =
    "numbered backref/call is not allowed. (use name)";
pub const TOO_SHORT_DIGITS: &str = "too short digits";
pub const TOO_LONG_WIDE_CHAR_VALUE: &str = "too long wide-char value";
pub const EMPTY_GROUP_NAME: &str = "group name is empty";
pub const INVALID_GROUP_NAME: &str = "invalid group name <%n>";
pub const INVALID_CHAR_IN_GROUP_NAME: &str = "invalid char in group name <%n>";
pub const UNDEFINED_NAME_REFERENCE: &str = "undefined name <%n> reference";
pub const UNDEFINED_GROUP_REFERENCE: &str = "undefined group <%n> reference";
pub const MULTIPLEX_DEFINED_NAME: &str = "multiplex defined name <%n>";
pub const MULTIPLEX_DEFINITION_NAME_CALL: &str = "multiplex definition name <%n> call";
pub const NEVER_ENDING_RECURSION: &str = "never ending recursion";
pub const INVALID_CHAR_PROPERTY_NAME: &str = "invalid character property name {%n}";
pub const TOO_MANY_CAPTURE_GROUPS: &str = "too many capture groups are specified";
pub const INVALID_CODE_POINT_VALUE: &str = "invalid code point value";
pub const TOO_BIG_WIDE_CHAR_VALUE: &str = "too big wide-char value";

// CRuby, re.c.
pub const TOO_SHORT_ESCAPE_SEQUENCE: &str = "too short escape sequence";
pub const INVALID_HEX_ESCAPE: &str = "invalid hex escape";
pub const DUPLICATE_META_ESCAPE: &str = "duplicate meta escape";
pub const TOO_SHORT_META_ESCAPE: &str = "too short meta escape";
pub const TOO_SHORT_CONTROL_ESCAPE: &str = "too short control escape";
pub const DUPLICATE_CONTROL_ESCAPE: &str = "duplicate control escape";
pub const UNEXPECTED_ESCAPE_SEQUENCE: &str = "unexpected escape sequence";
pub const INVALID_ESCAPE_CODE: &str = "invalid escape code";
pub const INVALID_MULTIBYTE_ESCAPE: &str = "invalid multibyte escape";
pub const INVALID_UNICODE_RANGE: &str = "invalid Unicode range";
pub const INVALID_UNICODE_LIST: &str = "invalid Unicode list";
pub const INVALID_UNICODE_ESCAPE: &str = "invalid Unicode escape";
pub const INVALID_MULTIBYTE_CHARACTER: &str = "invalid multibyte character";
