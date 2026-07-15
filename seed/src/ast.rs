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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperator {
    Add,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    Call {
        arguments: Vec<Expression>,
        name: String,
    },
    Integer(i64),
    String(String),
    Variable(String),
}
