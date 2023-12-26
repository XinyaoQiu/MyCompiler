use crate::syntax::{Exp, FunDecl};
use core::panic;
use std::collections::{HashMap, HashSet};

pub fn lambda_lift_helper<Ann>(
    e: &Exp<Ann>,
    env: &mut Vec<String>,
    lifted: &mut Vec<FunDecl<Exp<()>, ()>>,
    should_lift: &HashSet<String>,
    fun_to_decl: &mut HashMap<String, FunDecl<Exp<()>, ()>>,
    is_tail: bool,
    fun_to_env: &mut HashMap<String, Vec<String>>,
) -> Exp<()> {
    match e {
        Exp::Num(n, _) => Exp::Num(*n, ()),
        Exp::Bool(b, _) => Exp::Bool(*b, ()),
        Exp::Float(f, _) => Exp::Float(f.clone(), ()),
        Exp::Var(s, _) => Exp::Var(s.clone(), ()),
        Exp::Prim(p, exps, _) => {
            let mut new_exps = vec![];
            for exp in exps {
                new_exps.push(Box::new(lambda_lift_helper(
                    exp,
                    &mut env.clone(),
                    lifted,
                    should_lift,
                    fun_to_decl,
                    false,
                    fun_to_env,
                )));
            }
            Exp::Prim(p.clone(), new_exps, ())
        }
        Exp::Let { bindings, body, .. } => {
            let mut new_bindings = vec![];
            for (a, b) in bindings {
                new_bindings.push((
                    a.clone(),
                    lambda_lift_helper(
                        b,
                        &mut env.clone(),
                        lifted,
                        should_lift,
                        fun_to_decl,
                        false,
                        fun_to_env,
                    ),
                ));
                env.push(a.clone());
            }
            Exp::Let {
                bindings: new_bindings,
                body: Box::new(lambda_lift_helper(
                    body,
                    &mut env.clone(),
                    lifted,
                    should_lift,
                    fun_to_decl,
                    is_tail,
                    fun_to_env,
                )),
                ann: (),
            }
        }
        Exp::If { cond, thn, els, .. } => Exp::If {
            cond: Box::new(lambda_lift_helper(
                cond,
                &mut env.clone(),
                lifted,
                should_lift,
                fun_to_decl,
                false,
                fun_to_env,
            )),
            thn: Box::new(lambda_lift_helper(
                thn,
                &mut env.clone(),
                lifted,
                should_lift,
                fun_to_decl,
                is_tail,
                fun_to_env,
            )),
            els: Box::new(lambda_lift_helper(
                els,
                &mut env.clone(),
                lifted,
                should_lift,
                fun_to_decl,
                is_tail,
                fun_to_env,
            )),
            ann: (),
        },
        Exp::FunDefs { decls, body, .. } => {
            let mut new_bodies = vec![];
            let mut new_envs = vec![];
            for decl in decls {
                fun_to_env.insert(decl.name.clone(), env.clone());
                let mut new_env = vec![];
                for key in env.iter() {
                    if new_env.contains(key) {
                        panic!("wrong in lambda_lift, line 96");
                    }
                    new_env.push(key.clone());
                }
                new_env.extend(decl.parameters.clone());
                let new_body = lambda_lift_helper(
                    &decl.body,
                    &mut new_env.clone(),
                    lifted,
                    should_lift,
                    fun_to_decl,
                    true,
                    fun_to_env,
                );
                let new_decl = FunDecl {
                    name: decl.name.clone(),
                    parameters: decl.parameters.clone(),
                    body: new_body.clone(),
                    ann: (),
                };
                fun_to_decl.insert(decl.name.clone(), new_decl.clone());
                new_bodies.push(new_body.clone());
                new_envs.push(new_env.clone());
            }

            let mut new_decls = vec![];
            for i in 0..decls.len() {
                if should_lift.contains(&decls[i].name) {
                    let new_body =
                        copy_def(&new_bodies[i], fun_to_decl, fun_to_env, &mut env.clone());
                    lifted.push(FunDecl {
                        name: decls[i].name.clone(),
                        parameters: new_envs[i].clone(),
                        body: new_body.clone(),
                        ann: (),
                    });
                } else {
                    new_decls.push(FunDecl {
                        name: decls[i].name.clone(),
                        parameters: decls[i].parameters.clone(),
                        body: new_bodies[i].clone(),
                        ann: (),
                    });
                }
            }
            let new_body = lambda_lift_helper(
                body,
                &mut env.clone(),
                lifted,
                should_lift,
                fun_to_decl,
                true,
                fun_to_env,
            );
            if new_decls.is_empty() {
                return new_body;
            }
            Exp::FunDefs {
                decls: new_decls,
                body: Box::new(new_body),
                ann: (),
            }
        }
        Exp::Call(fun_name, args, _) => {
            let mut new_args = vec![];
            for arg in args {
                new_args.push(lambda_lift_helper(
                    arg,
                    env,
                    lifted,
                    should_lift,
                    fun_to_decl,
                    false,
                    fun_to_env,
                ))
            }
            if is_tail && !should_lift.contains(fun_name) {
                return Exp::InternalTailCall(fun_name.clone(), new_args, ());
            }
            Exp::ExternalCall {
                fun_name: fun_name.clone(),
                args: new_args,
                is_tail,
                ann: (),
            }
        }
        _ => panic!("lambda_lift get wrong Exp"),
    }
}

fn copy_def(
    e: &Exp<()>,
    fun_to_decl: &mut HashMap<String, FunDecl<Exp<()>, ()>>,
    fun_to_env: &mut HashMap<String, Vec<String>>,
    env: &mut Vec<String>,
) -> Exp<()> {
    match e {
        Exp::Let { bindings, body, .. } => {
            for (a, _) in bindings {
                env.push(a.clone());
            }
            Exp::Let {
                bindings: bindings.clone(),
                body: Box::new(copy_def(body, fun_to_decl, fun_to_env, &mut env.clone())),
                ann: (),
            }
        }
        Exp::If { cond, thn, els, .. } => Exp::If {
            cond: cond.clone(),
            thn: Box::new(copy_def(thn, fun_to_decl, fun_to_env, &mut env.clone())),
            els: Box::new(copy_def(els, fun_to_decl, fun_to_env, &mut env.clone())),
            ann: (),
        },
        Exp::FunDefs { decls, body, .. } => {
            let mut new_decls = vec![];
            for decl in decls {
                let new_body = copy_def(&decl.body, fun_to_decl, fun_to_env, &mut env.clone());
                let new_decl: FunDecl<Exp<()>, ()> = FunDecl {
                    name: decl.name.clone(),
                    parameters: decl.parameters.clone(),
                    body: new_body,
                    ann: (),
                };
                new_decls.push(new_decl);
            }
            Exp::FunDefs {
                decls: new_decls,
                body: Box::new(copy_def(body, fun_to_decl, fun_to_env, &mut env.clone())),
                ann: (),
            }
        }
        Exp::InternalTailCall(fun_name, args, _) => match fun_to_decl.get(fun_name) {
            Some(value) => {
                if !fun_to_env.contains_key(&format!("{}_copy", value.name.clone())) {
                    let new_name = format!("{}_copy", value.name.clone());
                    let new_decl = FunDecl {
                        name: new_name.clone(),
                        parameters: value.parameters.clone(),
                        body: value.body.clone(),
                        ann: (),
                    };
                    match fun_to_env.get(fun_name) {
                        Some(value) => {
                            fun_to_env.insert(new_name.clone(), value.clone());
                        }
                        None => panic!("copy cannot find in fun_to_env"),
                    }
                    Exp::FunDefs {
                        decls: vec![new_decl],
                        body: Box::new(Exp::InternalTailCall(new_name.clone(), args.clone(), ())),
                        ann: (),
                    }
                } else {
                    Exp::InternalTailCall(format!("{}_copy", value.name.clone()), args.clone(), ())
                }
            }
            None => panic!("copy_def didn't find any fun name"),
        },
        Exp::Call(_, _, _) => panic!("copy_def get wrong Exp type"),
        _ => e.clone(),
    }
}
