//! Portland's AST — grown fresh, with Prism's node shapes as inspiration.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Statement {
    Assignment {
        name: String,
        value: Expression,
    },
    Break,
    Expression(Expression),
    Next,
    MethodDefinition {
        body: Vec<Statement>,
        /// `label:` (required) and `label: default` (optional) parameters,
        /// Ruby 3 style: strictly separate from positionals.
        keyword_parameters: Vec<Parameter>,
        name: String,
        parameters: Vec<Parameter>,
    },
    Return {
        value: Option<Expression>,
    },
    StructDefinition {
        fields: Vec<String>,
        name: String,
    },
    While {
        body: Vec<Statement>,
        condition: Expression,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperator {
    Add,
    Divide,
    Equals,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Modulo,
    Multiply,
    NotEquals,
    Subtract,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

/// What an or-guard does when the left side is absent (ADR 0007/0008):
/// `user = find_user(id) or return`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GuardAction {
    Break,
    Next,
    Return(Option<Box<Expression>>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub body: Vec<Statement>,
    pub parameters: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseBranch {
    pub body: Vec<Statement>,
    pub values: Vec<Expression>,
}

/// A `case/in` pattern (ADR 0013). Grows by rung: literals, captures, and
/// alternatives first; struct/array patterns, pin, and guards follow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pattern {
    /// `in 1 | 2 | 3` — first matching alternative wins.
    Alternative(Vec<Pattern>),
    /// A bare lowercase name: matches anything, binds it (Ruby's rule,
    /// fenced by no-shadow and exhaustiveness per ADR 0013 §3).
    Capture(String),
    /// A literal value to compare against: integers, strings, booleans, nil.
    Literal(Expression),
    /// `in ReturnNode(value: nil)` — match by struct type, refine or bind by
    /// field. Keyword-only (ADR 0013 §5); a field with no sub-pattern binds
    /// under its own name (`in Token(kind:)` binds `kind`). Bare
    /// `in BreakNode` is a type-only match (empty fields).
    Struct {
        fields: Vec<(String, Option<Pattern>)>,
        name: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InBranch {
    pub body: Vec<Statement>,
    pub pattern: Pattern,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parameter {
    pub default: Option<Expression>,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    ArrayLiteral(Vec<Expression>),
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    Boolean(bool),
    Case {
        branches: Vec<CaseBranch>,
        else_body: Vec<Statement>,
        subject: Box<Expression>,
    },
    CaseIn {
        branches: Vec<InBranch>,
        else_body: Vec<Statement>,
        subject: Box<Expression>,
    },
    /// A diverging or-guard right side; only ever built there.
    Guard(GuardAction),
    HashLiteral(Vec<(Expression, Expression)>),
    If {
        condition: Box<Expression>,
        else_body: Vec<Statement>,
        then_body: Vec<Statement>,
    },
    Index {
        index: Box<Expression>,
        receiver: Box<Expression>,
    },
    Call {
        arguments: Vec<Expression>,
        keyword_arguments: Vec<(String, Expression)>,
        name: String,
    },
    Integer(i64),
    /// Kept apart from Binary because these short-circuit.
    Logical {
        left: Box<Expression>,
        operator: LogicalOperator,
        right: Box<Expression>,
    },
    MethodCall {
        arguments: Vec<Expression>,
        block: Option<Block>,
        keyword_arguments: Vec<(String, Expression)>,
        name: String,
        receiver: Box<Expression>,
        /// `&.` — an absent receiver short-circuits to nil (ADR 0008).
        safe: bool,
    },
    /// Absence — the empty case of a maybe (ADR 0006). Not Ruby's nil: it has
    /// no methods and is not falsy; the seed panics where the real compiler
    /// will reject at compile time.
    Nil,
    String(String),
    Unary {
        operand: Box<Expression>,
        operator: UnaryOperator,
    },
    Variable(String),
}
