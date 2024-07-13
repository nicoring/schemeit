use crate::{
    env::Env,
    error::{InterpreterError, Result},
    parse::{Operation, SymbolicExpression},
};

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

fn eval_operation<'a>(
    env: &mut Env,
    operation: Operation,
    expression_iter: &mut impl DoubleEndedIterator<Item = &'a SymbolicExpression>,
) -> Result<SymbolicExpression> {
    let mut eval_w_env = |expression| eval(env, expression);

    match operation {
        Operation::Add => expression_iter
            .map(eval_w_env)
            .reduce(|acc, elem| match (acc?, elem?) {
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Float(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value + elem_value))
                }
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Int(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value + elem_value as f64))
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Float(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value as f64 + elem_value))
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Int(elem_value)) => {
                    Ok(SymbolicExpression::Int(acc_value + elem_value))
                }
                _ => Err(InterpreterError::ValueError("wrong type for +".into())),
            })
            .unwrap(),
        Operation::Substract => expression_iter
            .map(eval_w_env)
            .reduce(|acc, elem| match (acc?, elem?) {
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Float(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value - elem_value))
                }
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Int(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value - elem_value as f64))
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Float(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value as f64 - elem_value))
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Int(elem_value)) => {
                    Ok(SymbolicExpression::Int(acc_value - elem_value))
                }
                _ => Err(InterpreterError::ValueError("wrong type for -".into())),
            })
            .unwrap(),
        Operation::Multiply => expression_iter
            .map(eval_w_env)
            .reduce(|acc, elem| match (acc?, elem?) {
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Float(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value * elem_value))
                }
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Int(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value * elem_value as f64))
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Float(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value as f64 * elem_value))
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Int(elem_value)) => {
                    Ok(SymbolicExpression::Int(acc_value * elem_value))
                }
                _ => Err(InterpreterError::ValueError("wrong type for *".into())),
            })
            .unwrap(),
        Operation::Divide => expression_iter
            .map(eval_w_env)
            .reduce(|acc, elem| match (acc?, elem?) {
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Float(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value / elem_value))
                }
                (SymbolicExpression::Float(acc_value), SymbolicExpression::Int(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value / elem_value as f64))
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Float(elem_value)) => {
                    Ok(SymbolicExpression::Float(acc_value as f64 / elem_value))
                }
                (SymbolicExpression::Int(acc_value), SymbolicExpression::Int(elem_value)) => Ok(
                    SymbolicExpression::Float(acc_value as f64 / elem_value as f64),
                ),
                _ => Err(InterpreterError::ValueError("wrong types for /".into())),
            })
            .unwrap(),
        Operation::Exp => match expression_iter.map(eval_w_env).next().unwrap()? {
            SymbolicExpression::Float(value) => Ok(SymbolicExpression::Float(value.exp())),
            SymbolicExpression::Int(value) => Ok(SymbolicExpression::Float((value as f64).exp())),
            value => Err(InterpreterError::RuntimeError(
                format!("exp on {}", value).to_string(),
            )),
        },
        Operation::Pow => {
            let mut evaluated_arguments = expression_iter.map(eval_w_env);
            let value_first = evaluated_arguments.next().unwrap()?;
            let value_second = evaluated_arguments.next().unwrap()?;
            match (value_first, value_second) {
                (SymbolicExpression::Float(first), SymbolicExpression::Float(second)) => {
                    Ok(SymbolicExpression::Float(first.powf(second)))
                }
                (SymbolicExpression::Float(first), SymbolicExpression::Int(second)) => {
                    Ok(SymbolicExpression::Float(first.powi(second as i32)))
                }
                (SymbolicExpression::Int(first), SymbolicExpression::Float(second)) => {
                    Ok(SymbolicExpression::Float((first as f64).powf(second)))
                }
                (SymbolicExpression::Int(first), SymbolicExpression::Int(second)) => {
                    if second < 0 {
                        Ok(SymbolicExpression::Float(
                            (first as f64).powi(second as i32),
                        ))
                    } else {
                        Ok(SymbolicExpression::Int(first.pow(second as u32)))
                    }
                }
                _ => Err(InterpreterError::ValueError("wrong types for pow".into())),
            }
        }
        Operation::Begin => {
            env.add_frame();
            let result = expression_iter
                .map(|el| eval(env, el))
                .try_fold(SymbolicExpression::Nil, |_, res| res);
            env.pop_frame();
            result
        }
        Operation::Module => {
            expression_iter.try_for_each(|el| eval_w_env(el).map(|_| ()))?;
            Ok(SymbolicExpression::Nil)
        }
        Operation::Cons => {
            let mut args = expression_iter.map(eval_w_env);
            let head = Box::new(args.next().unwrap()?);
            let tail = Box::new(args.next().unwrap()?);
            Ok(SymbolicExpression::Cons { head, tail })
        }
        Operation::List => {
            expression_iter
                .map(eval_w_env)
                .try_rfold(SymbolicExpression::Nil, |acc, elem| {
                    Ok(SymbolicExpression::Cons {
                        head: Box::new(elem?),
                        tail: Box::new(acc),
                    })
                })
        }
        Operation::Car => match expression_iter.map(eval_w_env).next().unwrap()? {
            SymbolicExpression::Cons { head, .. } => Ok(*head),
            _ => panic!("car on non cons type"),
        },
        Operation::Cdr => match expression_iter.map(eval_w_env).next().unwrap()? {
            SymbolicExpression::Cons { tail, .. } => Ok(*tail),
            _ => panic!("car on non cons type"),
        },
        Operation::Eq => Ok(eval_comparison_operation(
            expression_iter
                .map(eval_w_env)
                .collect::<Result<Vec<SymbolicExpression>>>()?,
            |left, right| left == right,
        )),
        Operation::Smaller => Ok(eval_comparison_operation(
            expression_iter
                .map(eval_w_env)
                .collect::<Result<Vec<SymbolicExpression>>>()?,
            |left, right| left < right,
        )),
        Operation::SmallerOrEqual => Ok(eval_comparison_operation(
            expression_iter
                .map(eval_w_env)
                .collect::<Result<Vec<SymbolicExpression>>>()?,
            |left, right| left <= right,
        )),
        Operation::Greater => Ok(eval_comparison_operation(
            expression_iter
                .map(eval_w_env)
                .collect::<Result<Vec<SymbolicExpression>>>()?,
            |left, right| left > right,
        )),
        Operation::GreaterOrEqual => Ok(eval_comparison_operation(
            expression_iter
                .map(eval_w_env)
                .collect::<Result<Vec<SymbolicExpression>>>()?,
            |left, right| left >= right,
        )),
        Operation::If => {
            let predicate = eval_w_env(expression_iter.next().unwrap())?;
            match predicate {
                SymbolicExpression::Bool(true) => eval_w_env(expression_iter.next().unwrap()),
                SymbolicExpression::Bool(false) => eval_w_env(expression_iter.nth(1).unwrap()),
                _ => Err(InterpreterError::ValueError(
                    "predicate must evaluate to boolean".into(),
                )),
            }
        }
        Operation::Cond => expression_iter
            .find_map(|expression| match expression {
                SymbolicExpression::Expression(values) => {
                    let predicate = eval_w_env(&values[0]);
                    match predicate {
                        Ok(SymbolicExpression::Bool(true)) => Some(eval_w_env(&values[1])),
                        Ok(SymbolicExpression::Bool(false)) => None,
                        err if err.is_err() => Some(err),
                        _ => Some(Err(InterpreterError::ValueError(
                            "predicate must evaluate to boolean".into(),
                        ))),
                    }
                }
                _ => Some(Err(InterpreterError::ArgumentError(
                    "invalid argument to cond".into(),
                ))),
            })
            .unwrap_or(Err(InterpreterError::RuntimeError(
                "cond all predicate false".into(),
            ))),
        Operation::Quote => expression_iter
            .next()
            .ok_or(InterpreterError::ArgumentError("missing arguments".into()))
            .cloned(),
        Operation::Define => {
            let name = match expression_iter.next() {
                Some(SymbolicExpression::Symbol(value)) => value,
                _ => {
                    return Err(InterpreterError::ArgumentError(
                        "first argument to define has to be symbol".into(),
                    ))
                }
            };
            let value_exp = expression_iter
                .next()
                .ok_or(InterpreterError::ArgumentError(
                    "empty arguments for define".into(),
                ))?;
            let value = eval_w_env(value_exp)?;
            env.define_symbol(name, value);
            Ok(SymbolicExpression::Nil)
        }
        Operation::Set => {
            let name = match expression_iter.next() {
                Some(SymbolicExpression::Symbol(value)) => value,
                _ => {
                    return Err(InterpreterError::ArgumentError(
                        "first argument to set! has to be symbol".into(),
                    ))
                }
            };
            let value_exp = expression_iter
                .next()
                .ok_or(InterpreterError::ArgumentError(
                    "empty arguments for set!".into(),
                ))?;
            let value = eval_w_env(value_exp)?;
            env.set_symbol(name, value)?;
            Ok(SymbolicExpression::Nil)
        }
        Operation::Lambda => {
            let parameters = match expression_iter.next().unwrap() {
                SymbolicExpression::Expression(values) => values.iter().map(|each| match each {
                    SymbolicExpression::Symbol(name) => name.to_owned(),
                    _ => panic!("non symbol arg in lambda {}", each),
                }),
                _ => panic!("invalid arg list for lambda"),
            }
            .collect();

            let body: Box<SymbolicExpression> = Box::new(expression_iter.next().unwrap().clone());
            let lambda_env = env.get_lambda_env();
            Ok(SymbolicExpression::Lambda {
                parameters,
                env: lambda_env,
                body,
            })
        }
        Operation::Let => {
            // example: (let ((a 5) (b (+ 5 1))) (+ a b))
            env.add_frame();
            if let Some(SymbolicExpression::Expression(expression)) = expression_iter.next() {
                expression.iter().try_for_each(|each| {
                    match each {
                        SymbolicExpression::Expression(sub_expression) => {
                            let mut sub_iter = sub_expression.iter();
                            if let Some(SymbolicExpression::Symbol(name)) = sub_iter.next() {
                                let exp = sub_iter.next().unwrap();
                                let value = eval(env, exp)?;
                                env.define_symbol(name, value);
                            } else {
                                panic!("invalid args for let")
                            }
                        }
                        _ => panic!("invalid args for let"),
                    };
                    Result::Ok(())
                })?
            } else {
                panic!("invalid args for let")
            }
            let result = eval(env, expression_iter.next().unwrap());
            env.pop_frame();
            result
        }
    }
}

fn eval_lambda<'a>(
    env: &mut Env,
    lambda_env: &mut Env,
    parameters: &[String],
    body: &SymbolicExpression,
    expression_iter: &mut impl DoubleEndedIterator<Item = &'a SymbolicExpression>,
) -> Result<SymbolicExpression> {
    lambda_env.add_frame();
    parameters
        .iter()
        .zip(expression_iter)
        .try_for_each(|(param, expression)| {
            eval(env, expression).map(|value| lambda_env.define_symbol(param, value))
        })?;

    let result = eval(lambda_env, body);
    lambda_env.pop_frame();
    result
}

fn eval_expression(env: &mut Env, expression: &[SymbolicExpression]) -> Result<SymbolicExpression> {
    let mut expression_iter = expression.iter();

    let first_expression = eval(env, expression_iter.next().unwrap())?;

    match first_expression {
        SymbolicExpression::Operation(operation) => {
            eval_operation(env, operation, &mut expression_iter)
        }
        SymbolicExpression::Lambda {
            parameters,
            env: mut lambda_env,
            body,
        } => eval_lambda(
            env,
            &mut lambda_env,
            &parameters,
            &body,
            &mut expression_iter,
        ),
        _ => Err(InterpreterError::SyntaxError(first_expression)),
    }
}

pub fn eval(env: &mut Env, expression: &SymbolicExpression) -> Result<SymbolicExpression> {
    match expression {
        SymbolicExpression::Symbol(name) => env.find_symbol(name),
        SymbolicExpression::Expression(expression) => eval_expression(env, expression),
        value => Ok(value.clone()),
    }
}
