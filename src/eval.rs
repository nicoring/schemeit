use crate::{
    env::Env,
    error::{InterpreterError, Result},
    parse::{ConsCell, Operation, SymbolicExpression},
};

/// A continuation frame - represents pending work waiting for a value
#[derive(Clone)]
enum Continuation {
    /// Evaluating arguments for an operation
    OpArgs {
        env: Env,
        op: Operation,
        evaluated: Vec<SymbolicExpression>,
        remaining: Vec<SymbolicExpression>,
    },
    /// Evaluating arguments for a lambda call
    LambdaArgs {
        caller_env: Env,
        lambda_env: Env,
        parameters: Vec<String>,
        body: SymbolicExpression,
        evaluated: Vec<SymbolicExpression>,
        remaining: Vec<SymbolicExpression>,
    },
    /// Evaluating the function position of an application
    ApplyFunc {
        env: Env,
        args: Vec<SymbolicExpression>,
    },
    /// Evaluating let bindings
    LetBindings {
        env: Env,
        current_name: String,
        remaining_bindings: Vec<(String, SymbolicExpression)>,
        body: SymbolicExpression,
    },
    /// Evaluating expressions in begin (non-tail)
    BeginExprs {
        env: Env,
        remaining: Vec<SymbolicExpression>,
    },
    /// Evaluating expressions in module
    ModuleExprs {
        env: Env,
        remaining: Vec<SymbolicExpression>,
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
        remaining_clauses: Vec<(SymbolicExpression, SymbolicExpression)>,
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
        Operation::Add => args
            .into_iter()
            .reduce(|acc, elem| match (acc, elem) {
                (SymbolicExpression::Float(a), SymbolicExpression::Float(b)) => {
                    SymbolicExpression::Float(a + b)
                }
                (SymbolicExpression::Float(a), SymbolicExpression::Int(b)) => {
                    SymbolicExpression::Float(a + b as f64)
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Float(b)) => {
                    SymbolicExpression::Float(a as f64 + b)
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Int(b)) => {
                    SymbolicExpression::Int(a + b)
                }
                _ => SymbolicExpression::Nil, // Error case, will be caught
            })
            .ok_or_else(|| InterpreterError::ArgumentError("+ requires arguments".into())),
        Operation::Substract => args
            .into_iter()
            .reduce(|acc, elem| match (acc, elem) {
                (SymbolicExpression::Float(a), SymbolicExpression::Float(b)) => {
                    SymbolicExpression::Float(a - b)
                }
                (SymbolicExpression::Float(a), SymbolicExpression::Int(b)) => {
                    SymbolicExpression::Float(a - b as f64)
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Float(b)) => {
                    SymbolicExpression::Float(a as f64 - b)
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Int(b)) => {
                    SymbolicExpression::Int(a - b)
                }
                _ => SymbolicExpression::Nil,
            })
            .ok_or_else(|| InterpreterError::ArgumentError("- requires arguments".into())),
        Operation::Multiply => args
            .into_iter()
            .reduce(|acc, elem| match (acc, elem) {
                (SymbolicExpression::Float(a), SymbolicExpression::Float(b)) => {
                    SymbolicExpression::Float(a * b)
                }
                (SymbolicExpression::Float(a), SymbolicExpression::Int(b)) => {
                    SymbolicExpression::Float(a * b as f64)
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Float(b)) => {
                    SymbolicExpression::Float(a as f64 * b)
                }
                (SymbolicExpression::Int(a), SymbolicExpression::Int(b)) => {
                    SymbolicExpression::Int(a * b)
                }
                _ => SymbolicExpression::Nil,
            })
            .ok_or_else(|| InterpreterError::ArgumentError("* requires arguments".into())),
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
            let head = Box::new(args[0].clone());
            let tail = Box::new(args[1].clone());
            Ok(SymbolicExpression::Cons(ConsCell { head, tail }))
        }
        Operation::List => {
            let result = args
                .into_iter()
                .rev()
                .fold(SymbolicExpression::Nil, |acc, elem| {
                    SymbolicExpression::Cons(ConsCell {
                        head: Box::new(elem),
                        tail: Box::new(acc),
                    })
                });
            Ok(result)
        }
        Operation::Car => match &args[0] {
            SymbolicExpression::Cons(ConsCell { ref head, .. }) => Ok((**head).clone()),
            _ => Err(InterpreterError::ValueError("car on non-cons".into())),
        },
        Operation::Cdr => match &args[0] {
            SymbolicExpression::Cons(ConsCell { ref tail, .. }) => Ok((**tail).clone()),
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

/// Main evaluation loop with explicit continuation stack
pub fn eval(env: &mut Env, expression: &SymbolicExpression) -> Result<SymbolicExpression> {
    let mut stack: Vec<Continuation> = Vec::new();
    let mut control = Control::Eval {
        env: env.clone(),
        expr: expression.clone(),
    };

    loop {
        control = match control {
            Control::Eval { mut env, expr } => match expr {
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
                    let value = env.find_symbol(&name)?;
                    Control::ApplyValue(value)
                }

                // Expression (function application or special form)
                SymbolicExpression::Expression(exprs) => {
                    if exprs.is_empty() {
                        return Err(InterpreterError::SyntaxError(SymbolicExpression::Nil));
                    }

                    let first = &exprs[0];
                    let args: Vec<_> = exprs[1..].to_vec();

                    // Check for special forms
                    match first {
                        SymbolicExpression::Operation(Operation::Quote) => {
                            if args.is_empty() {
                                return Err(InterpreterError::ArgumentError(
                                    "quote requires argument".into(),
                                ));
                            }
                            Control::ApplyValue(args[0].clone())
                        }
                        SymbolicExpression::Operation(Operation::Define) => {
                            let name = match &args[0] {
                                SymbolicExpression::Symbol(s) => s.clone(),
                                _ => {
                                    return Err(InterpreterError::ArgumentError(
                                        "define requires symbol".into(),
                                    ))
                                }
                            };
                            stack.push(Continuation::Define {
                                env: env.clone(),
                                name,
                            });
                            Control::Eval {
                                env,
                                expr: args[1].clone(),
                            }
                        }
                        SymbolicExpression::Operation(Operation::Set) => {
                            let name = match &args[0] {
                                SymbolicExpression::Symbol(s) => s.clone(),
                                _ => {
                                    return Err(InterpreterError::ArgumentError(
                                        "set! requires symbol".into(),
                                    ))
                                }
                            };
                            stack.push(Continuation::Set {
                                env: env.clone(),
                                name,
                            });
                            Control::Eval {
                                env,
                                expr: args[1].clone(),
                            }
                        }
                        SymbolicExpression::Operation(Operation::If) => {
                            if args.len() < 3 {
                                return Err(InterpreterError::ArgumentError(
                                    "if requires 3 arguments".into(),
                                ));
                            }
                            stack.push(Continuation::IfPredicate {
                                env: env.clone(),
                                then_branch: args[1].clone(),
                                else_branch: args[2].clone(),
                            });
                            Control::Eval {
                                env,
                                expr: args[0].clone(),
                            }
                        }
                        SymbolicExpression::Operation(Operation::Cond) => {
                            if args.is_empty() {
                                return Err(InterpreterError::RuntimeError(
                                    "cond requires clauses".into(),
                                ));
                            }
                            // Parse all clauses
                            let mut clauses = Vec::new();
                            for arg in &args {
                                match arg {
                                    SymbolicExpression::Expression(clause) if clause.len() >= 2 => {
                                        clauses.push((clause[0].clone(), clause[1].clone()));
                                    }
                                    _ => {
                                        return Err(InterpreterError::ArgumentError(
                                            "invalid cond clause".into(),
                                        ))
                                    }
                                }
                            }
                            let (pred, body) = clauses.remove(0);
                            stack.push(Continuation::CondPredicate {
                                env: env.clone(),
                                current_body: body,
                                remaining_clauses: clauses,
                            });
                            Control::Eval { env, expr: pred }
                        }
                        SymbolicExpression::Operation(Operation::Lambda) => {
                            let parameters = match &args[0] {
                                SymbolicExpression::Expression(params) => params
                                    .iter()
                                    .map(|p| match p {
                                        SymbolicExpression::Symbol(s) => Ok(s.clone()),
                                        _ => Err(InterpreterError::ArgumentError(
                                            "lambda param must be symbol".into(),
                                        )),
                                    })
                                    .collect::<Result<Vec<_>>>()?,
                                _ => {
                                    return Err(InterpreterError::ArgumentError(
                                        "lambda requires parameter list".into(),
                                    ))
                                }
                            };
                            let body = args[1].clone();
                            let lambda_env = env.get_lambda_env();
                            Control::ApplyValue(SymbolicExpression::Lambda {
                                parameters,
                                env: lambda_env,
                                body: Box::new(body),
                            })
                        }
                        SymbolicExpression::Operation(Operation::Let) => {
                            // (let ((a 1) (b 2)) body)
                            env.add_frame();
                            let bindings = match &args[0] {
                                SymbolicExpression::Expression(bs) => bs
                                    .iter()
                                    .map(|b| match b {
                                        SymbolicExpression::Expression(pair) if pair.len() >= 2 => {
                                            match &pair[0] {
                                                SymbolicExpression::Symbol(name) => {
                                                    Ok((name.clone(), pair[1].clone()))
                                                }
                                                _ => Err(InterpreterError::ArgumentError(
                                                    "let binding name must be symbol".into(),
                                                )),
                                            }
                                        }
                                        _ => Err(InterpreterError::ArgumentError(
                                            "invalid let binding".into(),
                                        )),
                                    })
                                    .collect::<Result<Vec<_>>>()?,
                                _ => {
                                    return Err(InterpreterError::ArgumentError(
                                        "let requires bindings".into(),
                                    ))
                                }
                            };
                            let body = args[1].clone();

                            if bindings.is_empty() {
                                // No bindings, just evaluate body
                                Control::Eval { env, expr: body }
                            } else {
                                let mut bindings = bindings;
                                let (name, val_expr) = bindings.remove(0);
                                stack.push(Continuation::LetBindings {
                                    env: env.clone(),
                                    current_name: name,
                                    remaining_bindings: bindings,
                                    body,
                                });
                                Control::Eval { env, expr: val_expr }
                            }
                        }
                        SymbolicExpression::Operation(Operation::Begin) => {
                            env.add_frame();
                            if args.is_empty() {
                                env.pop_frame();
                                Control::ApplyValue(SymbolicExpression::Nil)
                            } else if args.len() == 1 {
                                // Single expression - tail position
                                Control::Eval {
                                    env,
                                    expr: args[0].clone(),
                                }
                            } else {
                                let mut exprs = args;
                                let first = exprs.remove(0);
                                stack.push(Continuation::BeginExprs {
                                    env: env.clone(),
                                    remaining: exprs,
                                });
                                Control::Eval { env, expr: first }
                            }
                        }
                        SymbolicExpression::Operation(Operation::Module) => {
                            if args.is_empty() {
                                Control::ApplyValue(SymbolicExpression::Nil)
                            } else {
                                let mut exprs = args;
                                let first = exprs.remove(0);
                                stack.push(Continuation::ModuleExprs {
                                    env: env.clone(),
                                    remaining: exprs,
                                });
                                Control::Eval { env, expr: first }
                            }
                        }
                        SymbolicExpression::Operation(op) => {
                            // Regular operation - evaluate all arguments
                            if args.is_empty() {
                                let result = apply_operation(*op, vec![])?;
                                Control::ApplyValue(result)
                            } else {
                                let mut remaining = args;
                                let first = remaining.remove(0);
                                stack.push(Continuation::OpArgs {
                                    env: env.clone(),
                                    op: *op,
                                    evaluated: vec![],
                                    remaining,
                                });
                                Control::Eval { env, expr: first }
                            }
                        }
                        // Not a special form - evaluate function position first
                        _ => {
                            stack.push(Continuation::ApplyFunc {
                                env: env.clone(),
                                args,
                            });
                            Control::Eval {
                                env,
                                expr: first.clone(),
                            }
                        }
                    }
                }
            },

            Control::ApplyValue(value) => {
                if stack.is_empty() {
                    return Ok(value);
                }

                let cont = stack.pop().unwrap();
                match cont {
                    Continuation::OpArgs {
                        env,
                        op,
                        mut evaluated,
                        mut remaining,
                    } => {
                        evaluated.push(value);
                        if remaining.is_empty() {
                            let result = apply_operation(op, evaluated)?;
                            Control::ApplyValue(result)
                        } else {
                            let next = remaining.remove(0);
                            stack.push(Continuation::OpArgs {
                                env: env.clone(),
                                op,
                                evaluated,
                                remaining,
                            });
                            Control::Eval { env, expr: next }
                        }
                    }

                    Continuation::LambdaArgs {
                        caller_env,
                        mut lambda_env,
                        parameters,
                        body,
                        mut evaluated,
                        mut remaining,
                    } => {
                        evaluated.push(value);
                        if remaining.is_empty() {
                            // All args evaluated, bind and evaluate body
                            lambda_env.add_frame();
                            for (param, val) in parameters.iter().zip(evaluated) {
                                lambda_env.define_symbol(param, val);
                            }
                            // Body is in tail position - no continuation pushed
                            Control::Eval {
                                env: lambda_env,
                                expr: body,
                            }
                        } else {
                            let next = remaining.remove(0);
                            stack.push(Continuation::LambdaArgs {
                                caller_env: caller_env.clone(),
                                lambda_env,
                                parameters,
                                body,
                                evaluated,
                                remaining,
                            });
                            Control::Eval {
                                env: caller_env,
                                expr: next,
                            }
                        }
                    }

                    Continuation::ApplyFunc { env, args } => {
                        // We now have the function value
                        match value {
                            SymbolicExpression::Operation(op) => {
                                // Built-in operation
                                if args.is_empty() {
                                    let result = apply_operation(op, vec![])?;
                                    Control::ApplyValue(result)
                                } else {
                                    let mut remaining = args;
                                    let first = remaining.remove(0);
                                    stack.push(Continuation::OpArgs {
                                        env: env.clone(),
                                        op,
                                        evaluated: vec![],
                                        remaining,
                                    });
                                    Control::Eval { env, expr: first }
                                }
                            }
                            SymbolicExpression::Lambda {
                                parameters,
                                env: lambda_env,
                                body,
                            } => {
                                if args.is_empty() {
                                    // No args - evaluate body directly
                                    let mut le = lambda_env;
                                    le.add_frame();
                                    Control::Eval {
                                        env: le,
                                        expr: (*body).clone(),
                                    }
                                } else {
                                    let mut remaining = args;
                                    let first = remaining.remove(0);
                                    stack.push(Continuation::LambdaArgs {
                                        caller_env: env.clone(),
                                        lambda_env,
                                        parameters,
                                        body: (*body).clone(),
                                        evaluated: vec![],
                                        remaining,
                                    });
                                    Control::Eval { env, expr: first }
                                }
                            }
                            _ => {
                                return Err(InterpreterError::SyntaxError(value));
                            }
                        }
                    }

                    Continuation::LetBindings {
                        mut env,
                        current_name,
                        mut remaining_bindings,
                        body,
                    } => {
                        env.define_symbol(&current_name, value);
                        if remaining_bindings.is_empty() {
                            // All bindings done, evaluate body (tail position)
                            Control::Eval { env, expr: body }
                        } else {
                            let (name, val_expr) = remaining_bindings.remove(0);
                            stack.push(Continuation::LetBindings {
                                env: env.clone(),
                                current_name: name,
                                remaining_bindings,
                                body,
                            });
                            Control::Eval {
                                env,
                                expr: val_expr,
                            }
                        }
                    }

                    Continuation::BeginExprs { env, mut remaining } => {
                        // Discard value, continue with remaining
                        if remaining.is_empty() {
                            // Shouldn't happen - last expr doesn't push BeginExprs
                            Control::ApplyValue(value)
                        } else if remaining.len() == 1 {
                            // Last expression - tail position
                            Control::Eval {
                                env,
                                expr: remaining.remove(0),
                            }
                        } else {
                            let next = remaining.remove(0);
                            stack.push(Continuation::BeginExprs {
                                env: env.clone(),
                                remaining,
                            });
                            Control::Eval { env, expr: next }
                        }
                    }

                    Continuation::ModuleExprs { env, mut remaining } => {
                        if remaining.is_empty() {
                            Control::ApplyValue(SymbolicExpression::Nil)
                        } else {
                            let next = remaining.remove(0);
                            stack.push(Continuation::ModuleExprs {
                                env: env.clone(),
                                remaining,
                            });
                            Control::Eval { env, expr: next }
                        }
                    }

                    Continuation::Define { mut env, name } => {
                        env.define_symbol(&name, value);
                        Control::ApplyValue(SymbolicExpression::Nil)
                    }

                    Continuation::Set { mut env, name } => {
                        env.set_symbol(&name, value)?;
                        Control::ApplyValue(SymbolicExpression::Nil)
                    }

                    Continuation::IfPredicate {
                        env,
                        then_branch,
                        else_branch,
                    } => match value {
                        SymbolicExpression::Bool(true) => Control::Eval {
                            env,
                            expr: then_branch,
                        },
                        SymbolicExpression::Bool(false) => Control::Eval {
                            env,
                            expr: else_branch,
                        },
                        _ => Err(InterpreterError::ValueError(
                            "if predicate must be boolean".into(),
                        ))?,
                    },

                    Continuation::CondPredicate {
                        env,
                        current_body,
                        mut remaining_clauses,
                    } => match value {
                        SymbolicExpression::Bool(true) => {
                            // Evaluate body (tail position)
                            Control::Eval {
                                env,
                                expr: current_body,
                            }
                        }
                        SymbolicExpression::Bool(false) => {
                            if remaining_clauses.is_empty() {
                                return Err(InterpreterError::RuntimeError(
                                    "cond: all predicates false".into(),
                                ));
                            }
                            let (pred, body) = remaining_clauses.remove(0);
                            stack.push(Continuation::CondPredicate {
                                env: env.clone(),
                                current_body: body,
                                remaining_clauses,
                            });
                            Control::Eval { env, expr: pred }
                        }
                        _ => Err(InterpreterError::ValueError(
                            "cond predicate must be boolean".into(),
                        ))?,
                    },
                }
            }
        };
    }
}
