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
    MethodDefinition {
        body: Vec<Statement>,
        name: String,
        parameters: Vec<String>,
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
pub enum UnaryOperator {
    Negate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub body: Vec<Statement>,
    pub parameters: Vec<String>,
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
