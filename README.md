# C interpreter in Rust (CIR)

A simple rust project that receives a C program file and runs it (without generating a native binary).

## Interpretation Steps

1. `Lexer`
2. `Parser`
3. `Runtime` 

### Lexer

Receives the input C program file and splits the source code into tokens.

### Parser

Takes the tokens returned by the `Lexer` and generates the AST (Abstract Syntax Tree)

### Runtime

Last step which gets the AST representing the source code evaluates and runs it.
