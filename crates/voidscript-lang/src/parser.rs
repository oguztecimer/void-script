use crate::ast::*;
use crate::error::VoidScriptError;
use crate::token::{SpannedToken, Token};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Program, VoidScriptError> {
        let stmts = self.parse_program()?;
        Ok(Program { statements: stmts })
    }

    // --- Helpers ---

    fn current(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos].token
        } else {
            &Token::Eof
        }
    }

    fn current_line(&self) -> u32 {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].line
        } else {
            0
        }
    }

    fn advance(&mut self) -> &Token {
        let tok = if self.pos < self.tokens.len() {
            &self.tokens[self.pos].token
        } else {
            &Token::Eof
        };
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), VoidScriptError> {
        if self.current() == expected {
            self.advance();
            Ok(())
        } else {
            Err(VoidScriptError::syntax(
                self.current_line(),
                format!("Expected {:?}, got {:?}", expected, self.current()),
            ))
        }
    }

    fn skip_newlines(&mut self) {
        while self.current() == &Token::Newline {
            self.advance();
        }
    }

    // --- Program & Blocks ---

    fn parse_program(&mut self) -> Result<Vec<Statement>, VoidScriptError> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while self.current() != &Token::Eof {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, VoidScriptError> {
        self.expect(&Token::Indent)?;
        let mut stmts = Vec::new();
        self.skip_newlines();
        while self.current() != &Token::Dedent && self.current() != &Token::Eof {
            let stmt = self.parse_statement()?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        if self.current() == &Token::Dedent {
            self.advance();
        }
        Ok(stmts)
    }

    // --- Statements ---

    fn parse_statement(&mut self) -> Result<Statement, VoidScriptError> {
        let line = self.current_line();
        let stmt_kind = match self.current().clone() {
            Token::Def => self.parse_function_def()?,
            Token::If => self.parse_if()?,
            Token::While => self.parse_while()?,
            Token::For => self.parse_for()?,
            Token::Return => self.parse_return()?,
            Token::Break => {
                self.advance();
                self.skip_newlines();
                StmtKind::Break
            }
            Token::Continue => {
                self.advance();
                self.skip_newlines();
                StmtKind::Continue
            }
            Token::Pass => {
                self.advance();
                self.skip_newlines();
                StmtKind::Pass
            }
            _ => self.parse_expr_or_assignment()?,
        };
        Ok(Statement {
            kind: stmt_kind,
            line,
        })
    }

    fn parse_function_def(&mut self) -> Result<StmtKind, VoidScriptError> {
        self.advance(); // skip 'def'
        let name = match self.current().clone() {
            Token::Identifier(n) => {
                self.advance();
                n
            }
            _ => {
                return Err(VoidScriptError::syntax(
                    self.current_line(),
                    "Expected function name",
                ))
            }
        };
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.current() != &Token::RParen {
            match self.current().clone() {
                Token::Identifier(p) => {
                    self.advance();
                    params.push(p);
                }
                _ => {
                    return Err(VoidScriptError::syntax(
                        self.current_line(),
                        "Expected parameter name",
                    ))
                }
            }
            if self.current() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        self.expect(&Token::Colon)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(StmtKind::FunctionDef { name, params, body })
    }

    fn parse_if(&mut self) -> Result<StmtKind, VoidScriptError> {
        self.advance(); // skip 'if'
        let condition = self.parse_expression()?;
        self.expect(&Token::Colon)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        self.skip_newlines();

        let mut elif_clauses = Vec::new();
        while self.current() == &Token::Elif {
            self.advance();
            let elif_cond = self.parse_expression()?;
            self.expect(&Token::Colon)?;
            self.skip_newlines();
            let elif_body = self.parse_block()?;
            elif_clauses.push((elif_cond, elif_body));
            self.skip_newlines();
        }

        let else_body = if self.current() == &Token::Else {
            self.advance();
            self.expect(&Token::Colon)?;
            self.skip_newlines();
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(StmtKind::If {
            condition,
            body,
            elif_clauses,
            else_body,
        })
    }

    fn parse_while(&mut self) -> Result<StmtKind, VoidScriptError> {
        self.advance(); // skip 'while'
        let condition = self.parse_expression()?;
        self.expect(&Token::Colon)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(StmtKind::While { condition, body })
    }

    fn parse_for(&mut self) -> Result<StmtKind, VoidScriptError> {
        self.advance(); // skip 'for'
        let var = match self.current().clone() {
            Token::Identifier(v) => {
                self.advance();
                v
            }
            _ => {
                return Err(VoidScriptError::syntax(
                    self.current_line(),
                    "Expected variable name after 'for'",
                ))
            }
        };
        self.expect(&Token::In)?;
        let iterable = self.parse_expression()?;
        self.expect(&Token::Colon)?;
        self.skip_newlines();
        let body = self.parse_block()?;
        Ok(StmtKind::For {
            var,
            iterable,
            body,
        })
    }

    fn parse_return(&mut self) -> Result<StmtKind, VoidScriptError> {
        self.advance(); // skip 'return'
        let value = if self.current() == &Token::Newline || self.current() == &Token::Eof {
            None
        } else {
            Some(self.parse_expression()?)
        };
        Ok(StmtKind::Return { value })
    }

    fn parse_expr_or_assignment(&mut self) -> Result<StmtKind, VoidScriptError> {
        let expr = self.parse_expression()?;

        // Check for assignment
        match self.current() {
            Token::Assign => {
                self.advance();
                let target = self.expr_to_assign_target(&expr)?;
                let value = self.parse_expression()?;
                Ok(StmtKind::Assign { target, value })
            }
            Token::PlusAssign => {
                self.advance();
                let target = self.expr_to_assign_target(&expr)?;
                let value = self.parse_expression()?;
                Ok(StmtKind::AugAssign {
                    target,
                    op: AugOp::Add,
                    value,
                })
            }
            Token::MinusAssign => {
                self.advance();
                let target = self.expr_to_assign_target(&expr)?;
                let value = self.parse_expression()?;
                Ok(StmtKind::AugAssign {
                    target,
                    op: AugOp::Sub,
                    value,
                })
            }
            Token::StarAssign => {
                self.advance();
                let target = self.expr_to_assign_target(&expr)?;
                let value = self.parse_expression()?;
                Ok(StmtKind::AugAssign {
                    target,
                    op: AugOp::Mul,
                    value,
                })
            }
            Token::SlashAssign => {
                self.advance();
                let target = self.expr_to_assign_target(&expr)?;
                let value = self.parse_expression()?;
                Ok(StmtKind::AugAssign {
                    target,
                    op: AugOp::Div,
                    value,
                })
            }
            _ => Ok(StmtKind::Expr(expr)),
        }
    }

    fn expr_to_assign_target(&self, expr: &Expr) -> Result<AssignTarget, VoidScriptError> {
        match &expr.kind {
            ExprKind::Name(n) => Ok(AssignTarget::Name(n.clone())),
            ExprKind::Index { object, index } => Ok(AssignTarget::Index {
                object: (**object).clone(),
                index: (**index).clone(),
            }),
            ExprKind::Attribute { object, attr } => {
                // Treat attribute as index with string key for assignment
                Ok(AssignTarget::Index {
                    object: (**object).clone(),
                    index: Expr {
                        kind: ExprKind::StringLit(attr.clone()),
                        line: expr.line,
                    },
                })
            }
            _ => Err(VoidScriptError::syntax(
                expr.line,
                "Invalid assignment target",
            )),
        }
    }

    // --- Expressions (Pratt parser) ---
    //
    // Binding power table (higher = tighter):
    //   or:              1,  2   (left-assoc)
    //   and:             3,  4   (left-assoc)
    //   not (prefix):        5
    //   comparisons:     7,  8   (==, !=, <, >, <=, >=, is, in, not in)
    //   add / sub:       9, 10   (left-assoc)
    //   mul / div / mod: 11, 12  (left-assoc)
    //   unary minus:        13
    //   call / [] / .:  15, 16   (postfix)

    fn parse_expression(&mut self) -> Result<Expr, VoidScriptError> {
        self.parse_expr(0)
    }

    /// Pratt parser core: parse an expression whose operators all have
    /// left-binding-power >= `min_bp`.
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, VoidScriptError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let Some((l_bp, r_bp)) = self.infix_bp() else {
                break;
            };
            if l_bp < min_bp {
                break;
            }
            lhs = self.parse_infix(lhs, r_bp)?;
        }

        Ok(lhs)
    }

    /// Return `(left_bp, right_bp)` if the current token is an
    /// infix or postfix operator, `None` otherwise.
    fn infix_bp(&self) -> Option<(u8, u8)> {
        match self.current() {
            Token::Or => Some((1, 2)),
            Token::And => Some((3, 4)),
            Token::Eq | Token::NotEq | Token::Lt | Token::Gt
            | Token::LtEq | Token::GtEq | Token::Is | Token::In => Some((7, 8)),
            Token::Not if matches!(self.peek_ahead(1), Some(&Token::In)) => Some((7, 8)),
            Token::Plus | Token::Minus => Some((9, 10)),
            Token::Star | Token::Slash | Token::DoubleSlash | Token::Percent => Some((11, 12)),
            Token::LParen | Token::LBracket | Token::Dot => Some((15, 16)),
            _ => None,
        }
    }

    fn peek_ahead(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset).map(|t| &t.token)
    }

    // --- Prefix (nud) ---

    fn parse_prefix(&mut self) -> Result<Expr, VoidScriptError> {
        let line = self.current_line();
        match self.current().clone() {
            // Prefix operators
            Token::Not => {
                self.advance();
                let operand = self.parse_expr(5)?;
                Ok(Expr { kind: ExprKind::UnaryOp { op: UnaryOp::Not, operand: Box::new(operand) }, line })
            }
            Token::Minus => {
                self.advance();
                let operand = self.parse_expr(13)?;
                Ok(Expr { kind: ExprKind::UnaryOp { op: UnaryOp::Neg, operand: Box::new(operand) }, line })
            }
            // Atoms
            Token::Integer(n)   => { self.advance(); Ok(Expr { kind: ExprKind::Integer(n), line }) }
            Token::Float(f)     => { self.advance(); Ok(Expr { kind: ExprKind::Float(f), line }) }
            Token::StringLit(s) => { self.advance(); Ok(Expr { kind: ExprKind::StringLit(s), line }) }
            Token::True         => { self.advance(); Ok(Expr { kind: ExprKind::Bool(true), line }) }
            Token::False        => { self.advance(); Ok(Expr { kind: ExprKind::Bool(false), line }) }
            Token::None         => { self.advance(); Ok(Expr { kind: ExprKind::NoneLit, line }) }
            Token::Identifier(n) => { self.advance(); Ok(Expr { kind: ExprKind::Name(n), line }) }
            // Grouping / compound literals
            Token::LParen   => self.parse_paren_expr(),
            Token::LBracket => self.parse_list_expr(),
            Token::LBrace   => self.parse_dict_expr(),
            other => Err(VoidScriptError::syntax(line, format!("Unexpected token: {other:?}"))),
        }
    }

    // --- Infix / postfix (led) ---

    fn parse_infix(&mut self, lhs: Expr, r_bp: u8) -> Result<Expr, VoidScriptError> {
        let line = lhs.line;
        match self.current().clone() {
            // Boolean
            Token::Or  => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(Expr { kind: ExprKind::BoolOp { op: BoolOpKind::Or,  left: Box::new(lhs), right: Box::new(r) }, line }) }
            Token::And => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(Expr { kind: ExprKind::BoolOp { op: BoolOpKind::And, left: Box::new(lhs), right: Box::new(r) }, line }) }

            // Arithmetic
            Token::Plus        => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_binop(lhs, BinOp::Add, r)) }
            Token::Minus       => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_binop(lhs, BinOp::Sub, r)) }
            Token::Star        => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_binop(lhs, BinOp::Mul, r)) }
            Token::Slash       => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_binop(lhs, BinOp::Div, r)) }
            Token::DoubleSlash => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_binop(lhs, BinOp::FloorDiv, r)) }
            Token::Percent     => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_binop(lhs, BinOp::Mod, r)) }

            // Comparisons
            Token::Eq    => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_cmp(lhs, CmpOp::Eq, r)) }
            Token::NotEq => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_cmp(lhs, CmpOp::NotEq, r)) }
            Token::Lt    => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_cmp(lhs, CmpOp::Lt, r)) }
            Token::Gt    => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_cmp(lhs, CmpOp::Gt, r)) }
            Token::LtEq  => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_cmp(lhs, CmpOp::LtEq, r)) }
            Token::GtEq  => { self.advance(); let r = self.parse_expr(r_bp)?; Ok(self.make_cmp(lhs, CmpOp::GtEq, r)) }

            // `is` / `is not`
            Token::Is => {
                self.advance();
                let negated = self.current() == &Token::Not && { self.advance(); true };
                if self.current() == &Token::None {
                    self.advance();
                    Ok(Expr { kind: ExprKind::IsNone { expr: Box::new(lhs), negated }, line })
                } else {
                    let r = self.parse_expr(r_bp)?;
                    Ok(self.make_cmp(lhs, if negated { CmpOp::NotEq } else { CmpOp::Eq }, r))
                }
            }

            // `in`
            Token::In => {
                self.advance();
                let r = self.parse_expr(r_bp)?;
                Ok(self.make_cmp(lhs, CmpOp::Eq, r))
            }

            // `not in` (compound)
            Token::Not => {
                self.advance(); // not
                self.advance(); // in  (guaranteed by infix_bp guard)
                let r = self.parse_expr(r_bp)?;
                Ok(self.make_cmp(lhs, CmpOp::NotEq, r))
            }

            // Postfix: call
            Token::LParen => {
                self.advance();
                let mut args = Vec::new();
                while self.current() != &Token::RParen {
                    args.push(self.parse_expression()?);
                    if self.current() == &Token::Comma { self.advance(); }
                }
                self.expect(&Token::RParen)?;
                Ok(Expr { kind: ExprKind::Call { func: Box::new(lhs), args }, line })
            }

            // Postfix: index
            Token::LBracket => {
                self.advance();
                let index = self.parse_expression()?;
                self.expect(&Token::RBracket)?;
                Ok(Expr { kind: ExprKind::Index { object: Box::new(lhs), index: Box::new(index) }, line })
            }

            // Postfix: attribute
            Token::Dot => {
                self.advance();
                match self.current().clone() {
                    Token::Identifier(attr) => {
                        self.advance();
                        Ok(Expr { kind: ExprKind::Attribute { object: Box::new(lhs), attr }, line })
                    }
                    _ => Err(VoidScriptError::syntax(self.current_line(), "Expected attribute name after '.'")),
                }
            }

            _ => unreachable!("infix_bp returned Some but no arm matched"),
        }
    }

    // --- AST construction helpers ---

    fn make_binop(&self, left: Expr, op: BinOp, right: Expr) -> Expr {
        let line = left.line;
        Expr { kind: ExprKind::BinOp { left: Box::new(left), op, right: Box::new(right) }, line }
    }

    fn make_cmp(&self, left: Expr, op: CmpOp, right: Expr) -> Expr {
        let line = left.line;
        Expr { kind: ExprKind::Compare { left: Box::new(left), op, right: Box::new(right) }, line }
    }

    // --- Compound literals ---

    /// `(expr)` or `(a, b, ...)` tuple
    fn parse_paren_expr(&mut self) -> Result<Expr, VoidScriptError> {
        let line = self.current_line();
        self.advance(); // skip '('
        let expr = self.parse_expression()?;
        if self.current() == &Token::Comma {
            let mut items = vec![expr];
            while self.current() == &Token::Comma {
                self.advance();
                if self.current() == &Token::RParen { break; }
                items.push(self.parse_expression()?);
            }
            self.expect(&Token::RParen)?;
            Ok(Expr {
                kind: ExprKind::Call {
                    func: Box::new(Expr { kind: ExprKind::Name("__tuple__".to_string()), line }),
                    args: items,
                },
                line,
            })
        } else {
            self.expect(&Token::RParen)?;
            Ok(expr)
        }
    }

    /// `[]`, `[a, b]`, or `[expr for var in iter if cond]`
    fn parse_list_expr(&mut self) -> Result<Expr, VoidScriptError> {
        let line = self.current_line();
        self.advance(); // skip '['

        if self.current() == &Token::RBracket {
            self.advance();
            return Ok(Expr { kind: ExprKind::List(vec![]), line });
        }

        let first = self.parse_expression()?;

        // List comprehension?
        if self.current() == &Token::For {
            self.advance();
            let var = match self.current().clone() {
                Token::Identifier(v) => { self.advance(); v }
                _ => return Err(VoidScriptError::syntax(self.current_line(), "Expected variable in list comprehension")),
            };
            self.expect(&Token::In)?;
            let iter = self.parse_expression()?;
            let condition = if self.current() == &Token::If {
                self.advance();
                Some(Box::new(self.parse_expression()?))
            } else {
                Option::None
            };
            self.expect(&Token::RBracket)?;
            return Ok(Expr { kind: ExprKind::ListComp { expr: Box::new(first), var, iter: Box::new(iter), condition }, line });
        }

        // Regular list
        let mut items = vec![first];
        while self.current() == &Token::Comma {
            self.advance();
            if self.current() == &Token::RBracket { break; }
            items.push(self.parse_expression()?);
        }
        self.expect(&Token::RBracket)?;
        Ok(Expr { kind: ExprKind::List(items), line })
    }

    /// `{}` or `{k: v, ...}`
    fn parse_dict_expr(&mut self) -> Result<Expr, VoidScriptError> {
        let line = self.current_line();
        self.advance(); // skip '{'

        let mut args = Vec::new();
        if self.current() != &Token::RBrace {
            loop {
                let key = self.parse_expression()?;
                self.expect(&Token::Colon)?;
                let val = self.parse_expression()?;
                args.push(key);
                args.push(val);
                if self.current() == &Token::Comma {
                    self.advance();
                    if self.current() == &Token::RBrace { break; }
                } else {
                    break;
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr {
            kind: ExprKind::Call {
                func: Box::new(Expr { kind: ExprKind::Name("__dict__".to_string()), line }),
                args,
            },
            line,
        })
    }
}
