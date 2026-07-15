//! Portland's AST — grown fresh, with Prism's node shapes as inspiration.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Statement {
    Expression(Expression),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Add {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Integer(i64),
    String(String),
    Variable(String),
}
