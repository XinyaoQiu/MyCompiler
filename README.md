## MyComplier

A new compiler that can compile natural languages. Support simple arithmetic options based on numbers and floats.

### 1. Concrete Syntax

```
<expr>:
      | let <bindings> in <expr>
      | if <expr>: <expr> else: <expr>
      | <decls> in <expr>
      | <binop-expr>
<binop-expr>:
            | IDENTIFIER
            | NUMBER
            | FLOAT
            | true
            | false
            | !<binop-expr>
            | <prim1>(<expr>)
            | <expr> <prim2> <expr>
            | IDENTIFIER(<exprs>)
            | IDENTIFIER()
            | (<expr>)
<prim1>:
       | add1 | sub1
       | print | isbool | isnum | isfloat
       | cos | sqrt
<prim2>:
       | + | - | * | / | //
       | < | > | <= | >=
       | ==
       | && | ||
<decls>:
       | <decls> and <decl>
       | <decl>
<decl>:
      | def IDENTIFIER(<ids>): <expr>
      | def IDENTIFIER(): <expr>
<ids>:
     | IDENTIFIER
     | IDENTIFIER, <ids>
<exprs>:
       | <expr>
       | <expr>, <exprs>
<bindings>:
          | IDENTIFIER = <expr>
          | IDENTIFIER = <expr>, <bindings>
```
### 2. Usage
To install `rustc`, run

    sudo apt-get install rustc

To run the code, run

    cargo run

To compile a program and emit assembly code to stdout use

    snake INPUT_FILE

To compile a program, link it and run the produced binary use

    snake --run INPUT_FILE

To run the reference interpreter use

    snake --interp INPUT_FILE

To see this usage message run

    snake --help

### 3. Test
To test the examples in `./examples/`, run

    cargo test

To add new tests, add new `mk_test!` and `mk_fail_test!` in `./tests/examples.rs`

### 4. Additional information
See in `./new_proposal.pdf`