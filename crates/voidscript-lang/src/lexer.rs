use crate::token::{SpannedToken, Token};

pub struct Lexer {
    source: String,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
        }
    }

    pub fn tokenize(&self) -> Vec<SpannedToken> {
        let mut tokens: Vec<SpannedToken> = Vec::new();
        let mut indent_stack: Vec<usize> = vec![0];

        let lines: Vec<&str> = self.source.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = (line_idx + 1) as u32;

            // Count leading spaces
            let trimmed = line.trim_start_matches(' ');
            let indent = line.len() - trimmed.len();

            // Skip blank lines and comment-only lines
            let content = trimmed.trim();
            if content.is_empty() || content.starts_with('#') {
                continue;
            }

            // Handle indentation changes
            let current_indent = *indent_stack.last().unwrap();
            if indent > current_indent {
                indent_stack.push(indent);
                tokens.push(SpannedToken {
                    token: Token::Indent,
                    line: line_num,
                    col: 1,
                });
            } else if indent < current_indent {
                while let Some(&top) = indent_stack.last() {
                    if top > indent {
                        indent_stack.pop();
                        tokens.push(SpannedToken {
                            token: Token::Dedent,
                            line: line_num,
                            col: 1,
                        });
                    } else {
                        break;
                    }
                }
            }

            // Tokenize the line content
            let line_tokens = self.tokenize_line(trimmed, line_num, (indent + 1) as u32);
            tokens.extend(line_tokens);

            // Emit newline
            tokens.push(SpannedToken {
                token: Token::Newline,
                line: line_num,
                col: line.len() as u32 + 1,
            });
        }

        // Close remaining indents at EOF
        let final_line = (lines.len() + 1) as u32;
        while indent_stack.len() > 1 {
            indent_stack.pop();
            tokens.push(SpannedToken {
                token: Token::Dedent,
                line: final_line,
                col: 1,
            });
        }

        tokens.push(SpannedToken {
            token: Token::Eof,
            line: final_line,
            col: 1,
        });

        tokens
    }

    fn tokenize_line(&self, line: &str, line_num: u32, col_offset: u32) -> Vec<SpannedToken> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let col = col_offset + i as u32;
            let ch = chars[i];

            // Skip whitespace
            if ch == ' ' || ch == '\t' {
                i += 1;
                continue;
            }

            // Comment - skip rest of line
            if ch == '#' {
                break;
            }

            // String literals
            if ch == '"' || ch == '\'' {
                let quote = ch;
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != quote {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        match chars[i] {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            '\\' => s.push('\\'),
                            '\'' => s.push('\''),
                            '"' => s.push('"'),
                            other => {
                                s.push('\\');
                                s.push(other);
                            }
                        }
                    } else {
                        s.push(chars[i]);
                    }
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // skip closing quote
                }
                tokens.push(SpannedToken {
                    token: Token::StringLit(s),
                    line: line_num,
                    col,
                });
                continue;
            }

            // Numbers
            if ch.is_ascii_digit() {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i < chars.len() && chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                    i += 1; // skip '.'
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    let num_str: String = chars[start..i].iter().collect();
                    let f: f64 = num_str.parse().unwrap_or(0.0);
                    tokens.push(SpannedToken {
                        token: Token::Float(f),
                        line: line_num,
                        col,
                    });
                } else {
                    let num_str: String = chars[start..i].iter().collect();
                    let n: i64 = num_str.parse().unwrap_or(0);
                    tokens.push(SpannedToken {
                        token: Token::Integer(n),
                        line: line_num,
                        col,
                    });
                }
                continue;
            }

            // Identifiers and keywords
            if ch.is_ascii_alphabetic() || ch == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let token = match word.as_str() {
                    "def" => Token::Def,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "elif" => Token::Elif,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "for" => Token::For,
                    "in" => Token::In,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    "is" => Token::Is,
                    "True" => Token::True,
                    "False" => Token::False,
                    "None" => Token::None,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "pass" => Token::Pass,
                    _ => Token::Identifier(word),
                };
                tokens.push(SpannedToken {
                    token,
                    line: line_num,
                    col,
                });
                continue;
            }

            // Multi-character operators
            let next_ch = if i + 1 < chars.len() {
                Some(chars[i + 1])
            } else {
                Option::None
            };

            match (ch, next_ch) {
                ('=', Some('=')) => {
                    tokens.push(SpannedToken { token: Token::Eq, line: line_num, col });
                    i += 2;
                    continue;
                }
                ('!', Some('=')) => {
                    tokens.push(SpannedToken { token: Token::NotEq, line: line_num, col });
                    i += 2;
                    continue;
                }
                ('<', Some('=')) => {
                    tokens.push(SpannedToken { token: Token::LtEq, line: line_num, col });
                    i += 2;
                    continue;
                }
                ('>', Some('=')) => {
                    tokens.push(SpannedToken { token: Token::GtEq, line: line_num, col });
                    i += 2;
                    continue;
                }
                ('/', Some('/')) => {
                    tokens.push(SpannedToken { token: Token::DoubleSlash, line: line_num, col });
                    i += 2;
                    continue;
                }
                ('+', Some('=')) => {
                    tokens.push(SpannedToken { token: Token::PlusAssign, line: line_num, col });
                    i += 2;
                    continue;
                }
                ('-', Some('=')) => {
                    tokens.push(SpannedToken { token: Token::MinusAssign, line: line_num, col });
                    i += 2;
                    continue;
                }
                ('*', Some('=')) => {
                    tokens.push(SpannedToken { token: Token::StarAssign, line: line_num, col });
                    i += 2;
                    continue;
                }
                ('/', Some('=')) => {
                    tokens.push(SpannedToken { token: Token::SlashAssign, line: line_num, col });
                    i += 2;
                    continue;
                }
                _ => {}
            }

            // Single-character tokens
            let token = match ch {
                '+' => Token::Plus,
                '-' => Token::Minus,
                '*' => Token::Star,
                '/' => Token::Slash,
                '%' => Token::Percent,
                '<' => Token::Lt,
                '>' => Token::Gt,
                '=' => Token::Assign,
                '(' => Token::LParen,
                ')' => Token::RParen,
                '[' => Token::LBracket,
                ']' => Token::RBracket,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                ':' => Token::Colon,
                ',' => Token::Comma,
                '.' => Token::Dot,
                _ => {
                    // Unknown character, skip
                    i += 1;
                    continue;
                }
            };

            tokens.push(SpannedToken {
                token,
                line: line_num,
                col,
            });
            i += 1;
        }

        tokens
    }
}
