#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Integer(i64),
    Float(f64),
    StringLit(String),
    True,
    False,
    None,

    // Identifiers
    Identifier(String),

    // Keywords
    Def,
    Return,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    And,
    Or,
    Not,
    Is,
    Break,
    Continue,
    Pass,
    Enum,
    Match,
    Case,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    DoubleSlash,
    Percent,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Assign,
    Pipe,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Colon,
    Comma,
    Dot,

    // Structure
    Newline,
    Indent,
    Dedent,

    // Special
    Eof,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: u32,
    pub col: u32,
}
