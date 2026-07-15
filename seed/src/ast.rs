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
    Expression(Expression),
    MethodDefinition {
        body: Vec<Statement>,
        name: String,
        parameters: Vec<String>,
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
pub enum Expression {
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
    Call {
        arguments: Vec<Expression>,
        name: String,
    },
    Integer(i64),
    String(String),
    Unary {
        operand: Box<Expression>,
        operator: UnaryOperator,
    },
    Variable(String),
}
