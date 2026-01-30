use std::collections::VecDeque;
use std::rc::Rc;

use crate::{
    env::Env,
    error::{InterpreterError, Result},
    parse::{ConsCell, Operation, SymbolicExpression},
};

// ============================================================================
// Helper Functions - Parsing and Utilities
// ============================================================================

/// Extract a symbol name from an expression, returning an error with the given message if not a symbol
fn extract_symbol(expr: &SymbolicExpression, err_msg: &str) -> Result<String> {
    match expr {
        SymbolicExpression::Symbol(s) => Ok(s.clone()),
        _ => Err(InterpreterError::ArgumentError(err_msg.into())),
    }
}

/// Parse lambda parameters from an expression containing the parameter list
fn parse_lambda_params(expr: &SymbolicExpression) -> Result<Vec<String>> {
    match expr {
        SymbolicExpression::Expression(params) => params
            .iter()
            .map(|p| extract_symbol(p, "lambda param must be symbol"))
            .collect(),
        _ => Err(InterpreterError::ArgumentError(
            "lambda requires parameter list".into(),
        )),
    }
}

/// Parse let bindings from an expression containing the bindings list
fn parse_let_bindings(expr: SymbolicExpression) -> Result<Vec<(String, SymbolicExpression)>> {
    match expr {
        SymbolicExpression::Expression(bs) => bs
            .into_iter()
            .map(|b| match b {
                SymbolicExpression::Expression(pair) if pair.len() >= 2 => {
                    let mut iter = pair.into_iter();
                    let name_expr = iter.next().unwrap();
                    let name = extract_symbol(&name_expr, "let binding name must be symbol")?;
                    let value = iter.next().unwrap();
                    Ok((name, value))
                }
                _ => Err(InterpreterError::ArgumentError("invalid let binding".into())),
            })
            .collect(),
        _ => Err(InterpreterError::ArgumentError(
            "let requires bindings".into(),
        )),
    }
}

/// Parse cond clauses from the argument expressions
fn parse_cond_clauses(
    args: Vec<SymbolicExpression>,
) -> Result<Vec<(SymbolicExpression, SymbolicExpression)>> {
    args.into_iter()
        .map(|arg| match arg {
            SymbolicExpression::Expression(clause) if clause.len() >= 2 => {
                let mut iter = clause.into_iter();
                let pred = iter.next().unwrap();
                let body = iter.next().unwrap();
                Ok((pred, body))
            }
            _ => Err(InterpreterError::ArgumentError("invalid cond clause".into())),
        })
        .collect()
}

// ============================================================================
// Numeric Operation Helper
// ============================================================================

/// Apply a binary numeric operation across all arguments, handling int/float coercion
fn apply_binary_numeric_op<F, G>(
    args: Vec<SymbolicExpression>,
    float_op: F,
    int_op: G,
    op_name: &str,
) -> Result<SymbolicExpression>
where
    F: Fn(f64, f64) -> f64,
    G: Fn(i128, i128) -> i128,
{
    args.into_iter()
        .reduce(|acc, elem| match (acc, elem) {
            (SymbolicExpression::Float(a), SymbolicExpression::Float(b)) => {
                SymbolicExpression::Float(float_op(a, b))
            }
            (SymbolicExpression::Float(a), SymbolicExpression::Int(b)) => {
                SymbolicExpression::Float(float_op(a, b as f64))
            }
            (SymbolicExpression::Int(a), SymbolicExpression::Float(b)) => {
                SymbolicExpression::Float(float_op(a as f64, b))
            }
            (SymbolicExpression::Int(a), SymbolicExpression::Int(b)) => {
                SymbolicExpression::Int(int_op(a, b))
            }
            _ => SymbolicExpression::Nil, // Error case, will be caught
        })
        .ok_or_else(|| InterpreterError::ArgumentError(format!("{} requires arguments", op_name)))
}

// ============================================================================
// Continuation Types
// ============================================================================

/// A continuation frame - represents pending work waiting for a value
#[derive(Clone)]
enum Continuation {
    /// Evaluating arguments for an operation
    OpArgs {
        env: Env,
        op: Operation,
        evaluated: Vec<SymbolicExpression>,
        remaining: VecDeque<SymbolicExpression>,
    },
    /// Evaluating arguments for a lambda call
    LambdaArgs {
        caller_env: Env,
        lambda_env: Env,
        parameters: Vec<String>,
        body: SymbolicExpression,
        evaluated: Vec<SymbolicExpression>,
        remaining: VecDeque<SymbolicExpression>,
    },
    /// Evaluating the function position of an application
    ApplyFunc {
        env: Env,
        args: VecDeque<SymbolicExpression>,
    },
    /// Evaluating let bindings
    LetBindings {
        env: Env,
        current_name: String,
        remaining_bindings: VecDeque<(String, SymbolicExpression)>,
        body: SymbolicExpression,
    },
    /// Evaluating expressions in begin (non-tail)
    BeginExprs {
        env: Env,
        remaining: VecDeque<SymbolicExpression>,
    },
    /// Evaluating expressions in module
    ModuleExprs {
        env: Env,
        remaining: VecDeque<SymbolicExpression>,
    },
    /// Define: waiting for value
    Define {
        env: Env,
        name: String,
    },
    /// Set!: waiting for value
    Set {
        env: Env,
        name: String,
    },
    /// If: waiting for predicate
    IfPredicate {
        env: Env,
        then_branch: SymbolicExpression,
        else_branch: SymbolicExpression,
    },
    /// Cond: waiting for current predicate
    CondPredicate {
        env: Env,
        current_body: SymbolicExpression,
        remaining_clauses: VecDeque<(SymbolicExpression, SymbolicExpression)>,
    },
}

/// What the evaluator should do next
enum Control {
    /// Evaluate this expression
    Eval { env: Env, expr: SymbolicExpression },
    /// Apply a value to the top continuation
    ApplyValue(SymbolicExpression),
}

fn eval_comparison_operation(
    evaluated_arguments: Vec<SymbolicExpression>,
    op: fn(&SymbolicExpression, &SymbolicExpression) -> bool,
) -> SymbolicExpression {
    let mut arg_iter = evaluated_arguments.iter();
    let previous = arg_iter.next().unwrap();

    for current in arg_iter {
        if !(op(previous, current)) {
            return SymbolicExpression::Bool(false);
        }
    }

    SymbolicExpression::Bool(true)
}

/// Apply a completed operation with all arguments evaluated
fn apply_operation(op: Operation, args: Vec<SymbolicExpression>) -> Result<SymbolicExpression> {
    match op {
        Operation::Add => apply_binary_numeric_op(args, |a, b| a + b, |a, b| a + b, "+"),
        Operation::Substract => apply_binary_numeric_op(args, |a, b| a - b, |a, b| a - b, "-"),
        Operation::Multiply => apply_binary_numeric_op(args, |a, b| a * b, |a, b| a * b, "*"),
        // Division always returns float
        Operation::Divide => args
            .into_iter()
            .reduce(|acc, elem| match (acc, elem) {
                (SymbolicExpression::Float(a), SymbolicExpression::Float(b)) => {
                    SymbolicExpression::Float(a / b)
                }
                (SymbolicExpression::Float(a), SymbolicExpression::Int(b)) => {
                    SymbolicExpression::Float(a / b as f64)
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Float(b)) => {
                    SymbolicExpression::Float(a as f64 / b)
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Int(b)) => {
                    SymbolicExpression::Float(a as f64 / b as f64)
                }
                _ => SymbolicExpression::Nil,
            })
            .ok_or_else(|| InterpreterError::ArgumentError("/ requires arguments".into())),
        Operation::Exp => match &args[0] {
            SymbolicExpression::Float(v) => Ok(SymbolicExpression::Float(v.exp())),
            SymbolicExpression::Int(v) => Ok(SymbolicExpression::Float((*v as f64).exp())),
            _ => Err(InterpreterError::ValueError("exp requires number".into())),
        },
        Operation::Pow => {
            let (first, second) = (&args[0], &args[1]);
            match (first, second) {
                (SymbolicExpression::Float(a), SymbolicExpression::Float(b)) => {
                    Ok(SymbolicExpression::Float(a.powf(*b)))
                }
                (SymbolicExpression::Float(a), SymbolicExpression::Int(b)) => {
                    Ok(SymbolicExpression::Float(a.powi(*b as i32)))
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Float(b)) => {
                    Ok(SymbolicExpression::Float((*a as f64).powf(*b)))
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Int(b)) => {
                    if *b < 0 {
                        Ok(SymbolicExpression::Float((*a as f64).powi(*b as i32)))
                    } else {
                        Ok(SymbolicExpression::Int(a.pow(*b as u32)))
                    }
                }
                _ => Err(InterpreterError::ValueError("pow requires numbers".into())),
            }
        }
        Operation::Cons => {
            let mut iter = args.into_iter();
            let head = Rc::new(iter.next().unwrap());
            let tail = Rc::new(iter.next().unwrap());
            Ok(SymbolicExpression::Cons(ConsCell { head, tail }))
        }
        Operation::List => {
            let result = args
                .into_iter()
                .rev()
                .fold(SymbolicExpression::Nil, |acc, elem| {
                    SymbolicExpression::Cons(ConsCell {
                        head: Rc::new(elem),
                        tail: Rc::new(acc),
                    })
                });
            Ok(result)
        }
        Operation::Car => match &args[0] {
            // Cloning SymbolicExpression from Rc: O(1) for Cons (just clones inner Rcs)
            SymbolicExpression::Cons(ConsCell { head, .. }) => Ok((**head).clone()),
            _ => Err(InterpreterError::ValueError("car on non-cons".into())),
        },
        Operation::Cdr => match &args[0] {
            // Cloning SymbolicExpression from Rc: O(1) for Cons (just clones inner Rcs)
            SymbolicExpression::Cons(ConsCell { tail, .. }) => Ok((**tail).clone()),
            _ => Err(InterpreterError::ValueError("cdr on non-cons".into())),
        },
        Operation::Eq => Ok(eval_comparison_operation(args, |a, b| a == b)),
        Operation::Smaller => Ok(eval_comparison_operation(args, |a, b| a < b)),
        Operation::SmallerOrEqual => Ok(eval_comparison_operation(args, |a, b| a <= b)),
        Operation::Greater => Ok(eval_comparison_operation(args, |a, b| a > b)),
        Operation::GreaterOrEqual => Ok(eval_comparison_operation(args, |a, b| a >= b)),
        // These are handled specially, shouldn't reach here
        Operation::If
        | Operation::Cond
        | Operation::Define
        | Operation::Set
        | Operation::Lambda
        | Operation::Let
        | Operation::Begin
        | Operation::Module
        | Operation::Quote => unreachable!("special form in apply_operation"),
    }
}

// ============================================================================
// Pure Special Form Handlers (no stack needed)
// ============================================================================

fn handle_quote(mut args: VecDeque<SymbolicExpression>) -> Result<Control> {
    match args.pop_front() {
        Some(expr) => Ok(Control::ApplyValue(expr)),
        None => Err(InterpreterError::ArgumentError("quote requires argument".into())),
    }
}

fn handle_lambda(env: &Env, mut args: VecDeque<SymbolicExpression>) -> Result<Control> {
    let params_expr = args.pop_front().unwrap();
    let parameters = parse_lambda_params(&params_expr)?;
    let body = args.pop_front().unwrap();
    let lambda_env = env.get_lambda_env();
    Ok(Control::ApplyValue(SymbolicExpression::Lambda {
        parameters,
        env: lambda_env,
        body: Box::new(body),
    }))
}

fn apply_if_predicate(
    env: Env,
    then_branch: SymbolicExpression,
    else_branch: SymbolicExpression,
    value: SymbolicExpression,
) -> Result<Control> {
    match value {
        SymbolicExpression::Bool(true) => Ok(Control::Eval { env, expr: then_branch }),
        SymbolicExpression::Bool(false) => Ok(Control::Eval { env, expr: else_branch }),
        _ => Err(InterpreterError::ValueError("if predicate must be boolean".into())),
    }
}

// ============================================================================
// EvalState Struct
// ============================================================================

struct EvalState {
    stack: Vec<Continuation>,
}

impl EvalState {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn push(&mut self, cont: Continuation) {
        self.stack.push(cont);
    }

    fn pop(&mut self) -> Option<Continuation> {
        self.stack.pop()
    }

    // ========================================================================
    // Special Form Handlers
    // ========================================================================

    fn handle_define(&mut self, env: Env, mut args: VecDeque<SymbolicExpression>) -> Result<Control> {
        let name_expr = args.pop_front().unwrap();
        let name = extract_symbol(&name_expr, "define requires symbol")?;
        let value_expr = args.pop_front().unwrap();
        self.push(Continuation::Define { env: env.clone(), name });
        Ok(Control::Eval { env, expr: value_expr })
    }

    fn handle_set(&mut self, env: Env, mut args: VecDeque<SymbolicExpression>) -> Result<Control> {
        let name_expr = args.pop_front().unwrap();
        let name = extract_symbol(&name_expr, "set! requires symbol")?;
        let value_expr = args.pop_front().unwrap();
        self.push(Continuation::Set { env: env.clone(), name });
        Ok(Control::Eval { env, expr: value_expr })
    }

    fn handle_if(&mut self, env: Env, mut args: VecDeque<SymbolicExpression>) -> Result<Control> {
        if args.len() < 3 {
            return Err(InterpreterError::ArgumentError("if requires 3 arguments".into()));
        }
        let predicate = args.pop_front().unwrap();
        let then_branch = args.pop_front().unwrap();
        let else_branch = args.pop_front().unwrap();
        self.push(Continuation::IfPredicate {
            env: env.clone(),
            then_branch,
            else_branch,
        });
        Ok(Control::Eval { env, expr: predicate })
    }

    fn handle_cond(&mut self, env: Env, args: VecDeque<SymbolicExpression>) -> Result<Control> {
        if args.is_empty() {
            return Err(InterpreterError::RuntimeError("cond requires clauses".into()));
        }
        let mut clauses: VecDeque<_> = parse_cond_clauses(args.into())?.into();
        let (pred, body) = clauses.pop_front().unwrap();
        self.push(Continuation::CondPredicate {
            env: env.clone(),
            current_body: body,
            remaining_clauses: clauses,
        });
        Ok(Control::Eval { env, expr: pred })
    }

    fn handle_let(&mut self, mut env: Env, mut args: VecDeque<SymbolicExpression>) -> Result<Control> {
        env.add_frame();
        let bindings_expr = args.pop_front().unwrap();
        let body = args.pop_front().unwrap();
        let mut bindings: VecDeque<_> = parse_let_bindings(bindings_expr)?.into();

        if bindings.is_empty() {
            return Ok(Control::Eval { env, expr: body });
        }

        let (name, val_expr) = bindings.pop_front().unwrap();
        self.push(Continuation::LetBindings {
            env: env.clone(),
            current_name: name,
            remaining_bindings: bindings,
            body,
        });
        Ok(Control::Eval { env, expr: val_expr })
    }

    fn handle_begin(&mut self, mut env: Env, mut args: VecDeque<SymbolicExpression>) -> Control {
        env.add_frame();
        if args.is_empty() {
            env.pop_frame();
            return Control::ApplyValue(SymbolicExpression::Nil);
        }
        if args.len() == 1 {
            return Control::Eval { env, expr: args.pop_front().unwrap() };
        }
        let first = args.pop_front().unwrap();
        self.push(Continuation::BeginExprs { env: env.clone(), remaining: args });
        Control::Eval { env, expr: first }
    }

    fn handle_module(&mut self, env: Env, mut args: VecDeque<SymbolicExpression>) -> Control {
        if args.is_empty() {
            return Control::ApplyValue(SymbolicExpression::Nil);
        }
        let first = args.pop_front().unwrap();
        self.push(Continuation::ModuleExprs { env: env.clone(), remaining: args });
        Control::Eval { env, expr: first }
    }

    fn handle_builtin_op(
        &mut self,
        env: Env,
        op: Operation,
        mut args: VecDeque<SymbolicExpression>,
    ) -> Result<Control> {
        if args.is_empty() {
            let result = apply_operation(op, vec![])?;
            return Ok(Control::ApplyValue(result));
        }
        let first = args.pop_front().unwrap();
        self.push(Continuation::OpArgs {
            env: env.clone(),
            op,
            evaluated: vec![],
            remaining: args,
        });
        Ok(Control::Eval { env, expr: first })
    }

    fn handle_application(
        &mut self,
        env: Env,
        func_expr: SymbolicExpression,
        args: VecDeque<SymbolicExpression>,
    ) -> Control {
        self.push(Continuation::ApplyFunc { env: env.clone(), args });
        Control::Eval { env, expr: func_expr }
    }

    // ========================================================================
    // Continuation Handlers
    // ========================================================================

    fn apply_op_args(
        &mut self,
        env: Env,
        op: Operation,
        mut evaluated: Vec<SymbolicExpression>,
        mut remaining: VecDeque<SymbolicExpression>,
        value: SymbolicExpression,
    ) -> Result<Control> {
        evaluated.push(value);
        if remaining.is_empty() {
            let result = apply_operation(op, evaluated)?;
            return Ok(Control::ApplyValue(result));
        }
        let next = remaining.pop_front().unwrap();
        self.push(Continuation::OpArgs { env: env.clone(), op, evaluated, remaining });
        Ok(Control::Eval { env, expr: next })
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_lambda_args(
        &mut self,
        caller_env: Env,
        mut lambda_env: Env,
        parameters: Vec<String>,
        body: SymbolicExpression,
        mut evaluated: Vec<SymbolicExpression>,
        mut remaining: VecDeque<SymbolicExpression>,
        value: SymbolicExpression,
    ) -> Control {
        evaluated.push(value);
        if remaining.is_empty() {
            lambda_env.add_frame();
            for (param, val) in parameters.iter().zip(evaluated) {
                lambda_env.define_symbol(param, val);
            }
            return Control::Eval { env: lambda_env, expr: body };
        }
        let next = remaining.pop_front().unwrap();
        self.push(Continuation::LambdaArgs {
            caller_env: caller_env.clone(),
            lambda_env,
            parameters,
            body,
            evaluated,
            remaining,
        });
        Control::Eval { env: caller_env, expr: next }
    }

    fn apply_func(
        &mut self,
        env: Env,
        mut args: VecDeque<SymbolicExpression>,
        func_value: SymbolicExpression,
    ) -> Result<Control> {
        match func_value {
            SymbolicExpression::Operation(op) => self.handle_builtin_op(env, op, args),
            SymbolicExpression::Lambda { parameters, env: lambda_env, body } => {
                if args.is_empty() {
                    let mut le = lambda_env;
                    le.add_frame();
                    return Ok(Control::Eval { env: le, expr: *body });
                }
                let first = args.pop_front().unwrap();
                self.push(Continuation::LambdaArgs {
                    caller_env: env.clone(),
                    lambda_env,
                    parameters,
                    body: *body,
                    evaluated: vec![],
                    remaining: args,
                });
                Ok(Control::Eval { env, expr: first })
            }
            _ => Err(InterpreterError::SyntaxError(func_value)),
        }
    }

    fn apply_let_bindings(
        &mut self,
        mut env: Env,
        current_name: String,
        mut remaining_bindings: VecDeque<(String, SymbolicExpression)>,
        body: SymbolicExpression,
        value: SymbolicExpression,
    ) -> Control {
        env.define_symbol(&current_name, value);
        if remaining_bindings.is_empty() {
            return Control::Eval { env, expr: body };
        }
        let (name, val_expr) = remaining_bindings.pop_front().unwrap();
        self.push(Continuation::LetBindings {
            env: env.clone(),
            current_name: name,
            remaining_bindings,
            body,
        });
        Control::Eval { env, expr: val_expr }
    }

    fn apply_begin_exprs(
        &mut self,
        env: Env,
        mut remaining: VecDeque<SymbolicExpression>,
        value: SymbolicExpression,
    ) -> Control {
        if remaining.is_empty() {
            return Control::ApplyValue(value);
        }
        if remaining.len() == 1 {
            return Control::Eval { env, expr: remaining.pop_front().unwrap() };
        }
        let next = remaining.pop_front().unwrap();
        self.push(Continuation::BeginExprs { env: env.clone(), remaining });
        Control::Eval { env, expr: next }
    }

    fn apply_module_exprs(&mut self, env: Env, mut remaining: VecDeque<SymbolicExpression>) -> Control {
        if remaining.is_empty() {
            return Control::ApplyValue(SymbolicExpression::Nil);
        }
        let next = remaining.pop_front().unwrap();
        self.push(Continuation::ModuleExprs { env: env.clone(), remaining });
        Control::Eval { env, expr: next }
    }

    fn apply_cond_predicate(
        &mut self,
        env: Env,
        current_body: SymbolicExpression,
        mut remaining_clauses: VecDeque<(SymbolicExpression, SymbolicExpression)>,
        value: SymbolicExpression,
    ) -> Result<Control> {
        match value {
            SymbolicExpression::Bool(true) => Ok(Control::Eval { env, expr: current_body }),
            SymbolicExpression::Bool(false) => {
                if remaining_clauses.is_empty() {
                    return Err(InterpreterError::RuntimeError("cond: all predicates false".into()));
                }
                let (pred, body) = remaining_clauses.pop_front().unwrap();
                self.push(Continuation::CondPredicate {
                    env: env.clone(),
                    current_body: body,
                    remaining_clauses,
                });
                Ok(Control::Eval { env, expr: pred })
            }
            _ => Err(InterpreterError::ValueError("cond predicate must be boolean".into())),
        }
    }

    // ========================================================================
    // Dispatch Functions
    // ========================================================================

    /// Dispatch evaluation of an expression
    fn eval_expression(&mut self, env: Env, mut exprs: VecDeque<SymbolicExpression>) -> Result<Control> {
        if exprs.is_empty() {
            return Err(InterpreterError::SyntaxError(SymbolicExpression::Nil));
        }

        let first = exprs.pop_front().unwrap();

        match first {
            SymbolicExpression::Operation(Operation::Quote) => handle_quote(exprs),
            SymbolicExpression::Operation(Operation::Define) => self.handle_define(env, exprs),
            SymbolicExpression::Operation(Operation::Set) => self.handle_set(env, exprs),
            SymbolicExpression::Operation(Operation::If) => self.handle_if(env, exprs),
            SymbolicExpression::Operation(Operation::Cond) => self.handle_cond(env, exprs),
            SymbolicExpression::Operation(Operation::Lambda) => handle_lambda(&env, exprs),
            SymbolicExpression::Operation(Operation::Let) => self.handle_let(env, exprs),
            SymbolicExpression::Operation(Operation::Begin) => Ok(self.handle_begin(env, exprs)),
            SymbolicExpression::Operation(Operation::Module) => Ok(self.handle_module(env, exprs)),
            SymbolicExpression::Operation(op) => self.handle_builtin_op(env, op, exprs),
            _ => Ok(self.handle_application(env, first, exprs)),
        }
    }

    /// Apply a continuation to a value
    fn apply_continuation(
        &mut self,
        cont: Continuation,
        value: SymbolicExpression,
    ) -> Result<Control> {
        match cont {
            Continuation::OpArgs { env, op, evaluated, remaining } => {
                self.apply_op_args(env, op, evaluated, remaining, value)
            }
            Continuation::LambdaArgs {
                caller_env, lambda_env, parameters, body, evaluated, remaining,
            } => Ok(self.apply_lambda_args(
                caller_env, lambda_env, parameters, body, evaluated, remaining, value,
            )),
            Continuation::ApplyFunc { env, args } => self.apply_func(env, args, value),
            Continuation::LetBindings { env, current_name, remaining_bindings, body } => {
                Ok(self.apply_let_bindings(env, current_name, remaining_bindings, body, value))
            }
            Continuation::BeginExprs { env, remaining } => {
                Ok(self.apply_begin_exprs(env, remaining, value))
            }
            Continuation::ModuleExprs { env, remaining } => {
                Ok(self.apply_module_exprs(env, remaining))
            }
            Continuation::Define { mut env, name } => {
                env.define_symbol(&name, value);
                Ok(Control::ApplyValue(SymbolicExpression::Nil))
            }
            Continuation::Set { mut env, name } => {
                env.set_symbol(&name, value)?;
                Ok(Control::ApplyValue(SymbolicExpression::Nil))
            }
            Continuation::IfPredicate { env, then_branch, else_branch } => {
                apply_if_predicate(env, then_branch, else_branch, value)
            }
            Continuation::CondPredicate { env, current_body, remaining_clauses } => {
                self.apply_cond_predicate(env, current_body, remaining_clauses, value)
            }
        }
    }
}

// ============================================================================
// Main Evaluation Loop
// ============================================================================

/// Main evaluation loop with explicit continuation stack
pub fn eval(env: &mut Env, expression: &SymbolicExpression) -> Result<SymbolicExpression> {
    let mut interp = EvalState::new();
    let mut control = Control::Eval {
        env: env.clone(),
        expr: expression.clone(),
    };

    loop {
        control = match control {
            Control::Eval { env, expr } => match expr {
                // Self-evaluating forms
                SymbolicExpression::Int(_)
                | SymbolicExpression::Float(_)
                | SymbolicExpression::Bool(_)
                | SymbolicExpression::Nil
                | SymbolicExpression::Str(_)
                | SymbolicExpression::Cons(_)
                | SymbolicExpression::Lambda { .. }
                | SymbolicExpression::Operation(_) => Control::ApplyValue(expr),

                // Symbol lookup
                SymbolicExpression::Symbol(name) => {
                    Control::ApplyValue(env.find_symbol(&name)?)
                }

                // Expression (function application or special form)
                SymbolicExpression::Expression(exprs) => interp.eval_expression(env, exprs.into())?,
            },

            Control::ApplyValue(value) => {
                let cont = match interp.pop() {
                    Some(c) => c,
                    None => return Ok(value),
                };
                interp.apply_continuation(cont, value)?
            }
        };
    }
}
