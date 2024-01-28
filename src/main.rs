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
    let ast = parser.parse();

    let mut rt = runtime::Interpreter::new(ast);
    rt.eval();
}

pub mod libc {
    pub struct LibC {
        pub filepaths: Vec<String>,
        pub stdio: Stdio
    }

    impl LibC {
        pub fn new() -> Self {
            let filepaths: Vec<String> = vec![
                "stdio.h".to_string()
            ];
            let stdio = Stdio::new();

            Self { filepaths, stdio }
        }
    }

    pub struct Stdio {
        pub funcnames: Vec<String>
    }

    impl Stdio {
        pub fn new() -> Self {
            let funcnames: Vec<String> = vec![
                "printf".to_string()
            ];

            Self { funcnames }
        }

        pub fn printf(&mut self, x: &str) {
            print!("{x}");
        }
    }
}

pub mod runtime {
    use ast::{AST, ASTNode};
    use libc::LibC;

    #[derive(Debug)]
    struct Function {
        name: String,
        location: String
    }

    #[derive(Debug)]
    struct Env {
        functions: Vec<Function>
    }

    impl Env {
        pub fn new() -> Self {
            Self {
                functions: vec![]
            }
        }

        pub fn push_function(&mut self, function: Function) {
            self.functions.push(function);
        }
    }

    pub struct Interpreter {
        ast: AST,
        env: Env,
        libc: LibC,
    }

    impl Interpreter {
        pub fn new(ast: AST) -> Self {
            Self { 
                ast, 
                env: Env::new(),
                libc: LibC::new(),
            }
        }

        pub fn eval(&mut self) {
            loop {
                let node = self.ast.next();
                if node.is_none() {
                    break;
                }

                self.eval_node_stmt(node.unwrap());
            }

            println!("{:#?}", self.env);
        }

        fn eval_node_stmt(&mut self, node: ASTNode) {
            match node {
                ASTNode::Include(filepath) => self.eval_node_include(filepath),
                _ => {
                    eprintln!("ERROR:{}: Evaluation of {:#?} not supported yet", line!(), node);
                }
            }
        }

        fn eval_node_include(&mut self, filepath: String) {
            if !self.libc.filepaths.contains(&filepath) {
                eprintln!("ERROR: File {filepath} not found. only looking for libc files for now");
                return;
            }

            for func in &self.libc.stdio.funcnames {
                self.env.push_function(Function {
                    name: func.to_string(),
                    location: String::from(format!("{}/{}", "libc", func)),
                })
            }
        }

    }
}

pub mod types {
    #[derive(Clone, Debug, PartialEq)]
    pub enum Type {
        INT,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct FuncParam {
        ttype: Type,
        name: String,
    }

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
    use types::{FuncParam, Type};

    #[derive(Clone, Debug, PartialEq)]
    pub enum ASTNode {
        Include(String),
        FuncDecl {
            name: String,
            params: Vec<FuncParam>,
            ret_type: Type,
            body: Vec<Box<ASTNode>>,
        },
        FunCall {
            name: String,
            args: Vec<Box<ASTNode>>,
        },
        Return(Box<ASTNode>),
        StrLit(String),
        StrVal(String),
        IntLit(i32),
        Semicolon,
        EOF,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct AST {
        body: Vec<ASTNode>,
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

        pub fn next(&mut self) -> Option<ASTNode> {
            if self.body.is_empty() {
                return None;
            }
            Some(self.body.remove(0))
        }
    }
}

pub mod parser {
    use ast::{ASTNode, AST};
    use exit;
    use types::{FuncParam, Token, TokenKind as TK, Type};

    pub struct Parser {
        tokens: Vec<Token>,
        ast: AST,
    }

    impl Parser {
        pub fn new(tokens: Vec<Token>) -> Self {
            Self {
                tokens,
                ast: AST::new(),
            }
        }

        pub fn parse(&mut self) -> AST {
            while !self.eof() {
                let node = self.parse_stmt();
                self.ast.push(node);
            }

            self.ast.push(ASTNode::EOF);
            self.ast.clone()
        }

        fn parse_stmt(&mut self) -> ASTNode {
            let at = self.at();

            /* Handles:
             *   # ...
             */
            if at.kind == TK::Hash {
                self.eat(); // remove `#`
                return self.parse_deretive();
            }

            /* Handles:
             *   int ...
             */
            if at.kind == TK::StrLit && self.is_decl(&at.value) {
                self.eat(); // remove `int` or ...
                return self.parse_decl(at);
            }

            if at.value.as_str() == "return" {
                self.eat(); // remove `return`
                return self.parse_return();
            }

            self.parse_expr()
        }

        fn parse_expr(&mut self) -> ASTNode {
            self.parse_func_call()
        }

        fn parse_func_call(&mut self) -> ASTNode {
            let func = self.parse_prim_expr();
            if self.at().kind == TK::OpenPar {
                self.eat(); // remove `(`

                let mut args: Vec<Box<ASTNode>> = vec![];
                while self.at().kind != TK::ClosPar {
                    args.push(Box::new(self.parse_expr()));
                }
                self.eat_kind(TK::ClosPar); // remove `)`

                return ASTNode::FunCall {
                    name: self.get_strlit_val(&func),
                    args,
                };
            }
            func
        }

        fn parse_return(&mut self) -> ASTNode {
            ASTNode::Return(Box::new(self.parse_expr()))
        }

        fn parse_decl(&mut self, prev_tok: Token) -> ASTNode {
            let at = self.eat_kind(TK::StrLit);

            /* TODO:
             *   - Validate at.value as identifier
             */

            match self.eat().kind {
                TK::OpenPar => {
                    return self.parse_decl_func(self.str2type(&prev_tok.value), at.value)
                }
                _ => {
                    eprintln!("ERROR:{}: Expected function declaration for now", line!());
                    exit(1);
                }
            }
        }

        fn parse_decl_func(&mut self, ret_type: Type, name: String) -> ASTNode {
            let params = self.parse_decl_func_params();
            let mut body: Vec<Box<ASTNode>> = vec![];

            self.eat_kind(TK::OpenBlk);
            while self.at().kind != TK::ClosBlk {
                body.push(Box::new(self.parse_stmt()));
            }
            self.eat_kind(TK::ClosBlk);

            ASTNode::FuncDecl {
                name,
                ret_type,
                params,
                body,
            }
        }

        fn parse_decl_func_params(&mut self) -> Vec<FuncParam> {
            let at = self.eat();

            match at.value.as_str() {
                "void" => {
                    self.eat_kind(TK::ClosPar);
                    return vec![];
                }
                ")" => return vec![],
                _ => {
                    eprintln!("ERROR:{}: Function params not supported yet", line!());
                    exit(1);
                }
            }
        }

        /* Handles:
         *   #include ...
         *   #define ...
         */
        fn parse_deretive(&mut self) -> ASTNode {
            let at = self.eat_kind(TK::StrLit);
            match at.value.as_str() {
                "include" => return self.parse_deretive_include(),
                "define" => return self.parse_deretive_define(),
                _ => {
                    eprintln!("ERROR:{}: Invalid preprocessing: {}", line!(), at.value);
                    exit(1);
                }
            }
        }

        /* Handles:
         *   #include ...
         */
        fn parse_deretive_include(&mut self) -> ASTNode {
            let at = self.eat();

            let mut filepath = String::new();

            match at.kind {
                TK::LessThan => {
                    while self.at().kind != TK::GraThan {
                        let x = self.eat();
                        match x.kind {
                            TK::StrLit | TK::DivSymb | TK::Dot => {
                                filepath.extend(x.value.chars());
                            }
                            _ => {
                                eprintln!("ERROR:{}: Invalid file path", line!());
                                exit(1);
                            }
                        }
                    }
                    self.eat_kind(TK::GraThan);
                }
                TK::StrVal => {
                    filepath.extend(at.value.chars());
                }
                _ => {
                    eprintln!("ERROR:{}: Invalid preprocessing: {}", line!(), at.value);
                    exit(1);
                }
            }

            ASTNode::Include(filepath)
        }

        fn parse_deretive_define(&mut self) -> ASTNode {
            todo!();
        }

        fn parse_prim_expr(&mut self) -> ASTNode {
            let at = self.eat();

            match at.kind {
                TK::StrLit => ASTNode::StrLit(at.value),
                TK::StrVal => ASTNode::StrVal(at.value),
                TK::Numeric => ASTNode::IntLit(at.value.parse::<i32>().unwrap()),
                TK::Semicolon => ASTNode::Semicolon,
                _ => {
                    eprintln!("ERROR:{}: Unsupported primary expression {:?}", line!(), at);
                    exit(1);
                }
            }
        }

        /*
         * HELPER Functions 👇
         * 
         */

        fn eat_kind(&mut self, kind: TK) -> Token {
            if self.at().kind != kind {
                eprintln!(
                    "ERROR:{}: Expected: {:?} but found: {:?}",
                    line!(),
                    kind,
                    self.at().kind
                );
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

        fn is_decl(&self, x: &str) -> bool {
            match x {
                "int" => return true,
                _ => return false,
            }
        }

        fn str2type(&self, s: &str) -> Type {
            match s {
                "int" => Type::INT,
                _ => {
                    eprintln!("ERROR:{}: Unknown type name {s}", line!());
                    exit(1);
                }
            }
        }

        fn get_strlit_val(&self, strlit: &ASTNode) -> String {
            if let ASTNode::StrLit(value) = strlit {
                return value.to_string();
            }
            eprintln!(
                "ERROR:{}: Expected `String Literal` but found: {:?}",
                line!(),
                strlit
            );
            exit(1);
        }
    }
}

pub mod lexer {
    use types::{Token, TokenKind};

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
