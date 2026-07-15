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
        name: String,
        parameters: Vec<Parameter>,
    },
    Return {
        value: Option<Expression>,
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
        name: String,
        receiver: Box<Expression>,
    },
    String(String),
    Unary {
        operand: Box<Expression>,
        operator: UnaryOperator,
    },
    Variable(String),
}
