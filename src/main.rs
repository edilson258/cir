use std::fs;
use std::process::exit;

fn main() {
    let path = "main.c";
    let content = fs::read_to_string(path)
        .map_err(|err| {
            eprintln!(
                "ERROR [{}:{}]: Couldn't read file content: {:?}",
                file!(),
                line!(),
                err
            );
            exit(1);
        })
        .unwrap();

    let content = content.chars().collect::<Vec<char>>();
    let mut lexer = lexer::Lexer::new(&content);
    let tokens = lexer.lex();

    let mut parser = parser::Parser::new(tokens);
    parser.parse();
}

pub mod types {
    #[derive(Clone, Debug)]
    pub struct Token {
        pub kind: TokenKind,
        pub value: String,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum TokenKind {
        StrLit, 
        StrVal, 
        Numeric,
        PluSymb,
        MinSymb,
        MulSymb,
        DivSymb,
        OpenPar,
        ClosPar,
        OpenBlk,
        ClosBlk,
        Colon,
        Comma,
        Equal,
        Semicolon,
        Dot,
        Hash,
        LessThan,
        GraThan,
    }
}

pub mod ast {
    #[derive(Clone, Debug, PartialEq)]
    pub enum ASTNode {
        Include(String),
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct AST {
        body: Vec<ASTNode>
    }

    impl AST {
        pub fn new() -> Self {
            Self { body: Vec::new() }
        }

        pub fn push(&mut self, token: ASTNode) {
            self.body.push(token);
        }

        pub fn dump(&mut self) {
            println!("{:#?}", self);
        }
    }
}

pub mod parser {
    use exit;
    use ast::{AST, ASTNode};
    use types::{Token, TokenKind as TK};

    pub struct Parser {
        tokens: Vec<Token>,
        ast: AST,
    }

    impl Parser {
        pub fn new(tokens: Vec<Token>) -> Self {
            Self { 
                tokens,
                ast: AST::new()
            }
        }

        pub fn parse(&mut self) {
            while !self.eof() {
                self.parse_stmt();
            }

            self.ast.dump();
        }

        fn parse_stmt(&mut self) {
            let at = self.eat();

            /*
             * Handles:
             *   # ...
             */
            if at.kind == TK::Hash {
                self.parse_deretive();
                return;
            }

            println!("ERROR:{}: Unable to parse {:?}", line!(), at);
        }

        /* Handles:
         *   #include ...
         *   #define ...
         */
        fn parse_deretive(&mut self) {
            let at = self.eat_kind(TK::StrLit);
            match at.value.as_str() {
                "include" => self.parse_deretive_include(),
                "define" => self.parse_deretive_define(),
                _ => {
                    eprintln!("ERROR:{}: Invalid preprocessing: {}", line!(), at.value);
                    exit(1);
                }
            }
        }

        /* Handles:
         *   #include ...
         */
        fn parse_deretive_include(&mut self) {
            let at = self.eat();

            let mut filepath = String::new();

            match at.kind {
                TK::LessThan => {
                    while self.at().kind != TK::GraThan {
                        let x = self.eat();
                        match x.kind {
                            TK::StrLit  |
                            TK::DivSymb |
                            TK::Dot     => {
                                filepath.extend(x.value.chars());
                            },
                            _ => {
                                eprintln!("ERROR:{}: Invalid file path", line!());
                                exit(1);
                            }
                        }
                    }
                    self.eat_kind(TK::GraThan);
                },
                TK::StrVal => {
                    filepath.extend(at.value.chars());
                },
                _ => {
                    eprintln!("ERROR:{}: Invalid preprocessing: {}", line!(), at.value);
                    exit(1);
                }
            }

            let incl_node = ASTNode::Include(filepath);
            self.ast.push(incl_node);
        }

        fn parse_deretive_define(&mut self) {
            todo!();
        }
        
        fn eat_kind(&mut self, kind: TK) -> Token {
            if self.at().kind != kind {
                eprintln!("ERROR:{}: Expected: {:?} but found: {:?}", line!(), kind, self.at().kind);
                exit(1);
            } 
            self.eat()
        }

        fn at(&mut self) -> Token {
            if self.eof() {
                eprintln!("ERROR:{}: Unexpected EOF", line!());
                exit(1);
            }
            self.tokens.first().unwrap().clone()
        }

        fn eat(&mut self) -> Token {
            if self.eof() {
                eprintln!("ERROR:{}: Unexpected EOF", line!());
                exit(1);
            }
            self.tokens.remove(0)
        }
        
        fn eof(&self) -> bool {
            self.tokens.is_empty()
        }
    }
}

pub mod lexer {
    use types::{ Token, TokenKind };

    pub struct Lexer<'a> {
        stream: &'a [char],
        tokens: Vec<Token>,
    }

    impl<'a> Lexer<'a> {
        pub fn new(stream: &'a [char]) -> Self {
            Self {
                stream: stream,
                tokens: Vec::new(),
            }
        }

        pub fn lex(&mut self) -> Vec<Token> {
            loop {
                self.trim_left();

                if self.stream.is_empty() {
                    break;
                }

                // [a..z] + [0-9]
                if self.stream[0].is_alphabetic() {
                    let buf = self.chop_while(|c| c.is_alphabetic());
                    self.push_token(Token {
                        kind: TokenKind::StrLit,
                        value: buf.into_iter().collect::<String>(),
                    });
                    continue;
                }

                // [0-9]
                if self.stream[0].is_numeric() {
                    let buf = self.chop_while(|c| c.is_numeric());
                    self.push_token(Token {
                        kind: TokenKind::Numeric,
                        value: buf.into_iter().collect::<String>(),
                    });
                    continue;
                }

                if self.stream[0] == '"' {
                    self.chop(1); // remove `"`
                    let buf = self.chop_while(|c| *c != '"');
                    self.chop(1); // remove `"`
                    self.push_token(Token {
                        kind: TokenKind::StrVal,
                        value: buf.into_iter().collect::<String>(),
                    });
                    continue;
                }

                if self.extr_sgl_char_tkn() {
                    continue;
                }

                println!("ERROR:{}: Couldn't lex: `{:?}`", line!(), self.chop(1));
            }

            self.tokens.clone()
        }

        fn extr_sgl_char_tkn(&mut self) -> bool {
            for x in SINGLE_CHAR_TOKENS {
                if (x).value == self.stream[0] {
                    let buf = self.chop(1).into_iter().collect::<String>();
                    self.push_token(Token {
                        kind: (x).kind,
                        value: buf,
                    });
                    return true;
                }
            }
            false
        }

        fn chop_while<P>(&mut self, mut predicate: P) -> &'a [char]
        where
            P: FnMut(&char) -> bool,
        {
            let mut n = 0;
            while n < self.stream.len() && predicate(&self.stream[n]) {
                n += 1;
            }
            self.chop(n)
        }

        fn chop(&mut self, n: usize) -> &'a [char] {
            let buf = &self.stream[0..n];
            self.stream = &self.stream[n..];
            buf
        }

        fn trim_left(&mut self) -> usize {
            let mut n = 0;
            while !self.stream.is_empty() && self.stream[0].is_whitespace() {
                self.stream = &self.stream[1..];
                n += 1;
            }
            n
        }

        fn push_token(&mut self, lexeme: Token) {
            self.tokens.push(lexeme);
        }
    }

    struct SingleCharToken {
        kind: TokenKind,
        value: char,
    }

    const SINGLE_CHAR_TOKENS: [SingleCharToken; 16] = [
        SingleCharToken {
            kind: TokenKind::OpenPar,
            value: '(',
        },
        SingleCharToken {
            kind: TokenKind::ClosPar,
            value: ')',
        },
        SingleCharToken {
            kind: TokenKind::OpenBlk,
            value: '{',
        },
        SingleCharToken {
            kind: TokenKind::ClosBlk,
            value: '}',
        },
        SingleCharToken {
            kind: TokenKind::Colon,
            value: ':',
        },
        SingleCharToken {
            kind: TokenKind::Comma,
            value: ',',
        },
        SingleCharToken {
            kind: TokenKind::Semicolon,
            value: ';',
        },
        SingleCharToken {
            kind: TokenKind::Equal,
            value: '=',
        },
        SingleCharToken {
            kind: TokenKind::PluSymb,
            value: '+',
        },
        SingleCharToken {
            kind: TokenKind::MinSymb,
            value: '-',
        },
        SingleCharToken {
            kind: TokenKind::MulSymb,
            value: '*',
        },
        SingleCharToken {
            kind: TokenKind::DivSymb,
            value: '/',
        },
        SingleCharToken {
            kind: TokenKind::Dot,
            value: '.',
        },
        SingleCharToken {
            kind: TokenKind::Hash,
            value: '#',
        },
        SingleCharToken {
            kind: TokenKind::LessThan,
            value: '<',
        },
        SingleCharToken {
            kind: TokenKind::GraThan,
            value: '>',
        },
    ];
}
