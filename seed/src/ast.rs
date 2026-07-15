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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Add {
        left: Box<Expression>,
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
