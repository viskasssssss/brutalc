#[derive(Debug, Clone, Default, PartialEq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    EOF,
    Plus, Minus, Asterisk, Slash, Caret,
    LParen, RParen,
    Illegal,

    IntLiteral(i64),
    FloatLiteral(f64),
}

pub const SINGLE_OP_CODES: &[(char, TokenType)] = &[
    ('+', TokenType::Plus),
    ('-', TokenType::Minus),
    ('*', TokenType::Asterisk),
    ('/', TokenType::Slash),
    ('^', TokenType::Caret),
    ('(', TokenType::LParen),
    (')', TokenType::RParen),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
}

#[derive(Debug, Default)]
pub struct LexerResult {
    pub tokens: Vec<Token>,
    //span: Span
}

pub struct Lexer<'a> {
    span: Span,
    pos: usize,
    current: char,
    contents: &'a str,
    pub result: LexerResult
}

impl<'a> Lexer<'a> {
    pub fn new(contents: &'a str) -> Self {
        let first_char = contents.chars().next().unwrap_or('\0');
        
        Lexer {
            span: Span { line: 1, column: 1 },
            pos: 0,
            current: first_char,
            contents,
            result: LexerResult::default()
        }
    }

    fn advance(&mut self) {
        self.pos += self.current.len_utf8();

        self.current = self.contents
            .get(self.pos..)
            .and_then(|s| s.chars().next())
            .unwrap_or('\0');

        self.span.column += 1;
    }

    fn eat_number(&mut self) -> Token {
        let mut v: String = String::from("");
        let mut dot_count: u32 = 0;
        while self.current.is_ascii_digit() || self.current == '.' {
            if self.current == '.' {
                dot_count += 1;
            }
            v.push(self.current);
            self.advance();
        }

        if dot_count > 0 {
            if dot_count > 1 {
                eprintln!(
                    "Floating point value can't contain more than 1 dot: '{}'\n\tline: {} column: {}",
                    v, self.span.line, self.span.column
                );
                return Token { token_type: TokenType::Illegal, span: self.span.clone() };
            }
            match v.parse::<f64>() {
                Ok(num) => {
                    return Token { token_type: TokenType::FloatLiteral(num), span: self.span.clone() };
                },
                Err(e) => {
                    eprintln!(
                        "Illegal float value: '{}'\n\t{}\n\tline: {} column: {}", 
                        v, e, self.span.line, self.span.column
                    );
                    return Token { token_type: TokenType::Illegal, span: self.span.clone() };
                },
            }
        }

        match v.parse::<i64>() {
            Ok(num) => {
                return Token { token_type: TokenType::IntLiteral(num), span: self.span.clone() };
            },
            Err(e) => {
                eprintln!(
                    "Illegal integer value: '{}'\n\t{}\n\tline: {} column: {}", 
                    v, e, self.span.line, self.span.column
                );
                return Token { token_type: TokenType::Illegal, span: self.span.clone() };
            },
        }
    }

    fn try_eat_operator(&mut self) -> Option<Token> {
        let token_type = SINGLE_OP_CODES
            .iter()
            .find(|(ch, _)| *ch == self.current)
            .map(|(_, token)| token);

        match token_type {
            Some(t) => {
                return Some(Token { token_type: t.clone(), span: self.span.clone() });
            },
            None => None,
        }
    }

    fn eat_next(&mut self) {
        if self.current == '\n' {
            self.advance();
            self.span.line += 1;
            self.span.column = 1;
            return;
        }

        if self.current.is_whitespace() {
            self.advance();
            return;
        }

        let op = self.try_eat_operator();
        if let Some(v) = op {
            self.result.tokens.push(v);
            self.advance();
            return;
        }

        if self.current.is_ascii_digit() {
            let res = self.eat_number();
            self.result.tokens.push(res);
            return;
        }

        eprintln!(
            "Unknown character: '{}'\n\tline: {} column: {}",
            self.current, self.span.line, self.span.column
        );
        self.result.tokens.push(
            Token { token_type: TokenType::Illegal, span: self.span.clone() }
        );
        self.advance();
    }

    pub fn lex(&mut self) {
        while self.current != '\0' {
            self.eat_next();
        }

        self.result.tokens.push(Token { token_type: TokenType::EOF, span: self.span.clone() });
    }
}