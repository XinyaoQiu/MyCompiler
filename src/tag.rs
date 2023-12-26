use crate::syntax::{Exp, FunDecl, ImmExp, SeqExp, SeqFunDecl, SurfProg};
use std::collections::HashMap;

pub fn tag_exp<Span>(
    e: &SurfProg<Span>,
    counter: &mut u32,
    fun_map: &mut HashMap<String, String>,
    is_uniquify: bool,
) -> Exp<u32> {
    let result = match e {
        Exp::Num(n, _) => Exp::Num(*n, *counter),
        Exp::Bool(b, _) => Exp::Bool(*b, *counter),
        Exp::Float(f, _) => {
            Exp::Float(f.clone(), *counter)
        }
        Exp::Var(x, _) => {
            if is_uniquify {
                match fun_map.get(x) {
                    Some(value) => Exp::Var(value.clone(), *counter),
                    None => Exp::Var(x.clone(), *counter),
                }
            } else {
                Exp::Var(x.clone(), *counter)
            }
        }
        Exp::Prim(p, exps, _) => {
            let new_exps = exps
                .iter()
                .map(|e| Box::new(tag_exp(e, counter, fun_map, is_uniquify)))
                .collect();
            Exp::Prim(*p, new_exps, *counter)
        }
        Exp::Let { bindings, body, .. } => {
            let mut new_bindings = vec![];
            for (x, e) in bindings {
                let new_exp = tag_exp(e, counter, fun_map, is_uniquify);
                let new_x = format!("{}_{}", x, *counter);
                fun_map.insert(x.clone(), new_x.clone());
                new_bindings.push((if is_uniquify { new_x } else { x.clone() }, new_exp));
            }
            Exp::Let {
                bindings: new_bindings,
                body: Box::new(tag_exp(body, counter, fun_map, is_uniquify)),
                ann: *counter,
            }
        }
        Exp::If { cond, thn, els, .. } => Exp::If {
            cond: Box::new(tag_exp(cond, counter, fun_map, is_uniquify)),
            thn: Box::new(tag_exp(thn, counter, fun_map, is_uniquify)),
            els: Box::new(tag_exp(els, counter, fun_map, is_uniquify)),
            ann: *counter,
        },
        Exp::FunDefs { decls, body, .. } => {
            let mut new_decls = vec![];
            let mut new_names = vec![];
            for decl in decls {
                let new_name = format!("fun_{}_{}", decl.name, *counter);
                new_names.push(new_name.clone());
                *counter += 1;
                fun_map.insert(decl.name.clone(), new_name.clone());
            }
            for i in 0..decls.len() {
                let new_name = new_names[i].clone();
                let mut new_fun_map = fun_map.clone();
                let mut new_parameters = vec![];
                for para in &decls[i].parameters {
                    let new_para = format!("{}_{}", para.clone(), *counter);
                    *counter += 1;
                    new_fun_map.insert(para.clone(), new_para.clone());
                    new_parameters.push(new_para);
                }
                let new_decl = FunDecl {
                    name: if is_uniquify {
                        new_name
                    } else {
                        decls[i].name.clone()
                    },
                    parameters: if is_uniquify {
                        new_parameters
                    } else {
                        decls[i].parameters.clone()
                    },
                    body: tag_exp(&decls[i].body, counter, &mut new_fun_map, is_uniquify),
                    ann: *counter,
                };
                new_decls.push(new_decl);
                *counter += 1
            }
            Exp::FunDefs {
                decls: new_decls,
                body: Box::new(tag_exp(body, counter, fun_map, is_uniquify)),
                ann: *counter,
            }
        }
        Exp::Call(fun_name, args, _) => {
            let new_args = args
                .iter()
                .map(|e| tag_exp(e, counter, fun_map, is_uniquify))
                .collect();
            if is_uniquify {
                match fun_map.get(fun_name) {
                    Some(value) => Exp::Call(value.clone(), new_args, *counter),
                    None => panic!("wrong in call is_uniquify"),
                }
            } else {
                Exp::Call(fun_name.clone(), new_args, *counter)
            }
        }
        Exp::InternalTailCall(fun_name, args, _) => {
            let new_args = args
                .iter()
                .map(|e| tag_exp(e, counter, fun_map, is_uniquify))
                .collect();
            if is_uniquify {
                // match fun_map.get(fun_name) {
                //     Some(value) => Exp::InternalTailCall(value.clone(), new_args, *counter),
                //     None => panic!("uniquify wrong"),
                // }
                panic!("uniquify cannot meet Internal")
            } else {
                Exp::InternalTailCall(fun_name.clone(), new_args, *counter)
            }
        }
        Exp::ExternalCall {
            fun_name,
            args,
            is_tail,
            ..
        } => {
            let new_args = args
                .iter()
                .map(|e| tag_exp(e, counter, fun_map, is_uniquify))
                .collect();
            if is_uniquify {
                // match fun_map.get(fun_name) {
                //     Some(value) => Exp::ExternalCall {
                //         fun_name: value.clone(),
                //         args: new_args,
                //         is_tail: *is_tail,
                //         ann: *counter,
                //     },
                //     None => Exp::ExternalCall {
                //         fun_name: fun_name.clone(),
                //         args: new_args,
                //         is_tail: *is_tail,
                //         ann: *counter,
                //     },
                // }
                panic!("uniquify cannot meet External")
            } else {
                Exp::ExternalCall {
                    fun_name: fun_name.clone(),
                    args: new_args,
                    is_tail: *is_tail,
                    ann: *counter,
                }
            }
        }
    };
    *counter += 1;
    result
}

pub fn tag_seq<Span>(e: &SeqExp<Span>, counter: &mut u32) -> SeqExp<u32> {
    let result = match e {
        SeqExp::Imm(ie, _) => match ie {
            ImmExp::Num(n) => SeqExp::Imm(ImmExp::Num(*n), *counter),
            ImmExp::Var(x) => SeqExp::Imm(ImmExp::Var(x.clone()), *counter),
            ImmExp::Bool(b) => SeqExp::Imm(ImmExp::Bool(*b), *counter),
            ImmExp::Float(f) => SeqExp::Imm(ImmExp::Float(f.clone()), *counter),
        },
        SeqExp::Prim(p, ies, _) => SeqExp::Prim(*p, ies.clone(), *counter),
        SeqExp::If { cond, thn, els, .. } => SeqExp::If {
            cond: cond.clone(),
            thn: Box::new(tag_seq(thn, counter)),
            els: Box::new(tag_seq(els, counter)),
            ann: *counter,
        },
        SeqExp::Let {
            var,
            bound_exp,
            body,
            ..
        } => SeqExp::Let {
            var: var.clone(),
            bound_exp: Box::new(tag_seq(bound_exp, counter)),
            body: Box::new(tag_seq(body, counter)),
            ann: *counter,
        },
        SeqExp::FunDefs {
            decls,
            body,
            ann: _,
        } => {
            let mut new_decls = vec![];
            for decl in decls {
                new_decls.push(FunDecl {
                    name: decl.name.clone(),
                    parameters: decl.parameters.clone(),
                    body: tag_seq(&decl.body, counter),
                    ann: *counter,
                });
                *counter += 1;
            }
            SeqExp::FunDefs {
                decls: new_decls,
                body: Box::new(tag_seq(body, counter)),
                ann: *counter,
            }
        }
        SeqExp::InternalTailCall(fun_name, args, _) => {
            SeqExp::InternalTailCall(fun_name.clone(), args.clone(), *counter)
        }
        SeqExp::ExternalCall {
            fun_name,
            args,
            is_tail,
            ..
        } => SeqExp::ExternalCall {
            fun_name: fun_name.clone(),
            args: args.clone(),
            is_tail: *is_tail,
            ann: *counter,
        },
    };
    *counter += 1;
    result
}

pub fn tag_funs(funs: &Vec<SeqFunDecl<()>>, counter: &mut u32) -> Vec<SeqFunDecl<u32>> {
    let mut new_funs = vec![];
    for fun in funs {
        new_funs.push(FunDecl {
            name: fun.name.clone(),
            parameters: fun.parameters.clone(),
            body: tag_seq(&fun.body, counter),
            ann: *counter,
        });
        *counter += 1;
    }
    new_funs
}
