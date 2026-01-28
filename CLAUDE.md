# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

SchemeIt is a Scheme interpreter written in Rust. It implements a subset of Scheme (Lisp family) with support for lambda functions, closures, lexical scoping, and first-class functions. Zero external dependencies - pure Rust stdlib implementation.

## Build Commands

```bash
cargo build              # Development build
cargo build --release    # Release build with optimizations
cargo run                # Start REPL
cargo run -- file.scm    # Execute a Scheme file
cargo run -- --benchmark # Run benchmark suite
cargo run -- --test      # Run development test function
cargo test               # Run Rust unit tests
cargo clippy             # Run linter
```

Requires Rust 1.90.0+

## Architecture

Classic interpreter pipeline with five modules:

1. **tokenize.rs** - Lexical analysis converting Scheme code to tokens (parens, symbols, integers, floats, strings). Uses VecDeque for token queue.

2. **parse.rs** - Builds S-expression AST from tokens. Recognizes 20+ built-in operations: arithmetic (`+`, `-`, `*`, `/`, `pow`, `exp`), list ops (`car`, `cdr`, `cons`, `list`), control flow (`if`, `cond`, `begin`), binding (`define`, `set!`, `let`, `lambda`), comparisons (`=`, `<`, `>`, `<=`, `>=`), and `quote`. `ConsCell` has custom iterative `Drop` to handle deeply nested lists without stack overflow.

3. **eval.rs** - Stack-based continuation machine for evaluation. Uses explicit `Continuation` stack instead of Rust recursion, enabling deep recursion (~50k depth) without stack overflow. Supports TCO (tail call optimization) - tail calls don't grow the continuation stack.

4. **env.rs** - Frame-based lexical scoping with outer frame links using `Rc<RefCell<>>` for shared mutable references. Frames nest for function calls and let bindings.

5. **error.rs** - Custom `InterpreterError` enum: `VariableNotFound`, `SyntaxError`, `RuntimeError`, `ValueError`, `ArgumentError`.

**main.rs** contains entry point (REPL, file execution, benchmark, test modes) and comprehensive unit tests.

**std.scm** contains the standard library in Scheme: `fib`, `fact`, `range`, `map`, `mapi`, `reduce`, `reducei`, `null?`, `make-account`.

## Key Patterns

- **S-expressions**: `SymbolicExpression` enum represents both code and data
- **Closure capture**: Lambdas capture environment at definition time (lexical scoping)
- **Frame chain**: Environments are linked frames forming scope chains
- **First-class functions**: Lambdas are values that can be passed and returned
- **Continuation machine**: `eval()` uses explicit `Vec<Continuation>` stack with `Control` enum (`Eval`/`ApplyValue`) - no Rust recursion
- **TCO**: Tail positions (if/cond branches, let/begin body, lambda body) transition directly to `Control::Eval` without pushing continuations
- **Iterative Drop**: `ConsCell` uses `ManuallyDrop` and unsafe code to iteratively drop tail chains, preventing stack overflow on large lists
