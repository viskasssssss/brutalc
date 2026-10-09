use core::fmt;

use crate::lex::{self, Token, TokenType};

pub enum Expression {
    Atom(lex::Token),
    Operation(lex::Token, Vec<Expression>),
    Illegal
}

// for debug
impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Atom(i) => write!(f, "{:?}", i.token_type),
            Expression::Operation(head, rest) => {
                write!(f, "({:?}", head.token_type)?;
                for s in rest {
                    write!(f, " {}", s)?
                }
                write!(f, ")")
            }
            Expression::Illegal => write!(f, "[ILLEGAL]"),
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    pub result: Expression,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            result: Expression::Illegal
        }
    }

    pub fn parse(&mut self) {
        self.result = self.parse_expression(0.0);
    }

    fn parse_expression(&mut self, min_bp: f32) -> Expression {
        let it = self.next();

        let mut lhs = if is_atom(&it) {
            Expression::Atom(it)
        } else if it.token_type == TokenType::LParen {
            let lhs = self.parse_expression(0.0);
            assert_eq!(self.next().token_type, TokenType::RParen);
            lhs
        } else {
            panic!("Bad token: {:?}", it)
        };

        loop {
            let peek = self.peek();

            if peek.token_type == TokenType::EOF {
                break;
            } else if peek.token_type == TokenType::RParen {
                break;
            }

            if !is_operator(peek) {
                panic!("Bad token: {:?}", peek);
            }

            let (l_bp, _) = infix_binding_power(peek);

            if l_bp < min_bp {
                break;
            }

            let op = self.next();
            let (_, r_bp) = infix_binding_power(&op);

            let rhs = self.parse_expression(r_bp);
            lhs = Expression::Operation(op, vec![lhs, rhs]);
        }

        println!("parsed expression: {}!", lhs);
        lhs
    }

    fn next(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        token
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .expect("Unexpected end of token stream")
    }
}

fn is_atom(t: &Token) -> bool {
    matches!(
        &t.token_type,
        TokenType::IntLiteral(_) | TokenType::FloatLiteral(_)
    )
}

fn is_operator(t: &Token) -> bool {
    matches!(
        &t.token_type,
        TokenType::Plus | TokenType::Minus |
        TokenType::Asterisk | TokenType::Slash |
        TokenType::Caret
    )
}

fn infix_binding_power(op: &Token) -> (f32, f32) {
    match &op.token_type {
        TokenType::Plus | TokenType::Minus => (1.0, 1.1),
        TokenType::Asterisk | TokenType::Slash => (2.0, 2.1),
        TokenType::Caret => (3.0, 3.1),
        _ => panic!("Unknown operator: {:?}", op.token_type)
    }
}