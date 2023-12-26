use crate::compile::CompileErr;
use crate::syntax::{Exp, FloatWrapper, SurfProg};
use std::collections::{HashMap, HashSet};

static MAX_SNAKE_INT: i64 = i64::MAX >> 1;
static MIN_SNAKE_INT: i64 = i64::MIN >> 1;

pub fn check_exp<Span>(
    p: &SurfProg<Span>,
    env: &mut HashSet<String>,
    fun_to_paralen: &mut HashMap<String, usize>,
) -> Result<(), CompileErr<Span>>
where
    Span: Clone,
{
    match p {
        Exp::Num(n, ann) => {
            if *n <= MAX_SNAKE_INT && *n >= MIN_SNAKE_INT {
                Ok(())
            } else {
                Err(CompileErr::NumOverflow {
                    num: *n,
                    location: ann.clone(),
                })
            }
        }
        Exp::Bool(_, _) => {
            Ok(())
        }
        Exp::Float(FloatWrapper(f), ann) => {
            if *f <= f32::MAX as f64 && *f >= f32::MIN as f64 {
                Ok(())
            } else {
                Err(CompileErr::FloatOverflow {
                    num: FloatWrapper(*f),
                    location: ann.clone(),
                })
            }
        }
        Exp::Var(s, ann) => {
            if !env.contains(s) {
                if fun_to_paralen.contains_key(s) {
                    Err(CompileErr::FunctionUsedAsValue {
                        function_name: s.clone(),
                        location: ann.clone(),
                    })
                } else {
                    Err(CompileErr::UnboundVariable {
                        unbound: s.clone(),
                        location: ann.clone(),
                    })
                }
            } else {
                Ok(())
            }
        }
        Exp::Prim(_, exps, _) => {
            for e in exps {
                match check_exp(e, &mut env.clone(), &mut fun_to_paralen.clone()) {
                    Ok(()) => {}
                    Err(e) => return Err(e),
                }
            }
            Ok(())
        }
        Exp::Let {
            bindings,
            body,
            ann,
        } => {
            let mut uniq_names = HashSet::new();
            for (a, b) in bindings {
                if !uniq_names.insert(a.clone()) {
                    return Err(CompileErr::DuplicateBinding {
                        duplicated_name: a.clone(),
                        location: ann.clone(),
                    });
                }

                match check_exp(b, &mut env.clone(), &mut fun_to_paralen.clone()) {
                    Ok(()) => {}
                    Err(err) => return Err(err),
                }
                env.insert(a.clone());
            }
            check_exp(body, &mut env.clone(), &mut fun_to_paralen.clone())
        }
        Exp::If { cond, thn, els, .. } => {
            match check_exp(cond, &mut env.clone(), &mut fun_to_paralen.clone()) {
                Ok(()) => {}
                Err(err) => return Err(err),
            }
            match check_exp(thn, &mut env.clone(), &mut fun_to_paralen.clone()) {
                Ok(()) => {}
                Err(err) => return Err(err),
            }
            match check_exp(els, &mut env.clone(), &mut fun_to_paralen.clone()) {
                Ok(()) => {}
                Err(err) => return Err(err),
            }
            Ok(())
        }
        Exp::FunDefs { decls, body, ann } => {
            let mut uniq_fun_names = HashSet::new();
            for decl in decls {
                if fun_to_paralen.contains_key(&decl.name)
                    || !uniq_fun_names.insert(decl.name.clone())
                {
                    return Err(CompileErr::DuplicateFunName {
                        duplicated_name: decl.name.clone(),
                        location: ann.clone(),
                    });
                }
                fun_to_paralen.insert(decl.name.clone(), decl.parameters.len());
            }
            for decl in decls {
                // if !new_env.contains(&decl.name) {
                //     new_func.insert(decl.name.clone(), decl.parameters.len());
                // }
                let mut uniq_para_names = HashSet::new();
                for para_name in &decl.parameters {
                    if !uniq_para_names.insert(para_name.clone()) {
                        return Err(CompileErr::DuplicateArgName {
                            duplicated_name: para_name.clone(),
                            location: ann.clone(),
                        });
                    }
                    env.insert(para_name.clone());
                }
                match check_exp(&decl.body, &mut env.clone(), &mut fun_to_paralen.clone()) {
                    Ok(()) => {}
                    Err(err) => return Err(err),
                }
            }
            check_exp(body, &mut env.clone(), &mut fun_to_paralen.clone())
        }
        Exp::Call(fun_name, args, ann)
        | Exp::InternalTailCall(fun_name, args, ann)
        | Exp::ExternalCall {
            fun_name,
            args,
            ann,
            ..
        } => {
            match fun_to_paralen.get(fun_name) {
                Some(value) => {
                    if args.len() != *value {
                        return Err(CompileErr::FunctionCalledWrongArity {
                            function_name: fun_name.clone(),
                            correct_arity: *value,
                            arity_used: args.len(),
                            location: ann.clone(),
                        });
                    } else {
                        for arg in args {
                            match check_exp(arg, &mut env.clone(), &mut fun_to_paralen.clone()) {
                                Ok(()) => {}
                                Err(err) => return Err(err),
                            }
                        }
                        Ok(())
                    }
                }
                None => {
                    if env.contains(fun_name) {
                        return Err(CompileErr::ValueUsedAsFunction {
                            variable_name: fun_name.clone(),
                            location: ann.clone(),
                        });
                    } else {
                        return Err(CompileErr::UndefinedFunction {
                            undefined: fun_name.clone(),
                            location: ann.clone(),
                        });
                    }
                }
            }
        }
    }
}
