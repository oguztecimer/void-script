#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StmtKind,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    If {
        condition: Expr,
        body: Vec<Statement>,
        elif_clauses: Vec<(Expr, Vec<Statement>)>,
        else_body: Option<Vec<Statement>>,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    For {
        var: String,
        iterable: Expr,
        body: Vec<Statement>,
    },
    Assign {
        target: AssignTarget,
        value: Expr,
    },
    AugAssign {
        target: AssignTarget,
        op: AugOp,
        value: Expr,
    },
    Return {
        value: Option<Expr>,
    },
    Break,
    Continue,
    Pass,
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Name(String),
    Index { object: Expr, index: Expr },
}

#[derive(Debug, Clone)]
pub enum AugOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    // Literals
    Integer(i64),
    Float(f64),
    StringLit(String),
    Bool(bool),
    NoneLit,

    // Compound
    List(Vec<Expr>),
    ListComp {
        expr: Box<Expr>,
        var: String,
        iter: Box<Expr>,
        condition: Option<Box<Expr>>,
    },

    // Names
    Name(String),

    // Operations
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    BoolOp {
        op: BoolOpKind,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Compare {
        left: Box<Expr>,
        op: CmpOp,
        right: Box<Expr>,
    },
    IsNone {
        expr: Box<Expr>,
        negated: bool,
    },

    // Access
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Attribute {
        object: Box<Expr>,
        attr: String,
    },
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum BoolOpKind {
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum CmpOp {
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}
