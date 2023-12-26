use std::collections::HashMap;

use crate::{
    syntax::{Exp, FunDecl, ImmExp, Prim, SeqExp},
    tag::tag_exp,
};

pub fn seq_exp(e: &Exp<u32>) -> SeqExp<()> {
    match e {
        Exp::Var(name, _) => SeqExp::Imm(ImmExp::Var(name.clone()), ()),
        Exp::Num(num, _) => SeqExp::Imm(ImmExp::Num(*num), ()),
        Exp::Bool(b, _) => SeqExp::Imm(ImmExp::Bool(*b), ()),
        Exp::Float(s, _) => SeqExp::Imm(ImmExp::Float(s.clone()), ()),
        Exp::Prim(op, exps, tag) => match op {
            Prim::Add
            | Prim::Sub
            | Prim::Mul
            | Prim::And
            | Prim::Or
            | Prim::Lt
            | Prim::Gt
            | Prim::Le
            | Prim::Ge
            | Prim::Eq
            | Prim::Div
            | Prim::FloorDiv
            | Prim::Neq => {
                let (s1, s2) = (seq_exp(&exps[0]), seq_exp(&exps[1]));
                let (n1, n2) = (format!("#prim2_1_{}", tag), format!("#prim2_2_{}", tag));
                SeqExp::Let {
                    var: n1.clone(),
                    bound_exp: Box::new(s1),
                    body: Box::new(SeqExp::Let {
                        var: n2.clone(),
                        bound_exp: Box::new(s2),
                        body: Box::new(SeqExp::Prim(
                            *op,
                            vec![ImmExp::Var(n1), ImmExp::Var(n2)],
                            (),
                        )),
                        ann: (),
                    }),
                    ann: (),
                }
            }
            Prim::Add1
            | Prim::Sub1
            | Prim::IsBool
            | Prim::IsFloat
            | Prim::IsNum
            | Prim::Sqrt
            | Prim::Cos
            | Prim::Not
            | Prim::Print => {
                let n1 = format!("#prim1_1_{}", tag);
                let s1 = seq_exp(&exps[0]);
                SeqExp::Let {
                    var: n1.clone(),
                    bound_exp: Box::new(s1),
                    body: Box::new(SeqExp::Prim(*op, vec![ImmExp::Var(n1)], ())),
                    ann: (),
                }
            }
        },
        Exp::Let { bindings, body, .. } => {
            let mut seq = Box::new(seq_exp(body));
            for (x, bound_exp) in bindings.iter().rev() {
                seq = Box::new(SeqExp::Let {
                    var: x.clone(),
                    bound_exp: Box::new(seq_exp(bound_exp)),
                    body: seq,
                    ann: (),
                });
            }
            *seq
        }
        Exp::If {
            cond,
            thn,
            els,
            ann,
        } => {
            let name = format!("{}{:?}", "if_", ann);
            let bound_exp = Box::new(seq_exp(cond));
            let body = Box::new(SeqExp::If {
                cond: ImmExp::Var(name.clone()),
                thn: Box::new(seq_exp(thn)),
                els: Box::new(seq_exp(els)),
                ann: (),
            });

            SeqExp::Let {
                var: name,
                bound_exp,
                body,
                ann: (),
            }
        }
        Exp::FunDefs { decls, body, .. } => {
            let mut new_decls = vec![];
            for decl in decls {
                new_decls.push(FunDecl {
                    name: decl.name.clone(),
                    parameters: decl.parameters.clone(),
                    body: seq_exp(&decl.body),
                    ann: (),
                });
            }
            SeqExp::FunDefs {
                decls: new_decls,
                body: Box::new(seq_exp(body)),
                ann: (),
            }
        }
        Exp::InternalTailCall(fun_name, args, ann) => {
            if args.is_empty() {
                return SeqExp::InternalTailCall(fun_name.clone(), Vec::new(), ());
            }
            let mut new_names = vec![];
            for i in 0..args.len() {
                new_names.push(format!("#arg_{}_{}", ann, i));
            }
            let mut new_args = vec![];
            for n in &new_names {
                new_args.push(ImmExp::Var(n.clone()));
            }
            let mut seq = Box::new(SeqExp::InternalTailCall(fun_name.clone(), new_args, ()));
            for i in (0..args.len()).rev() {
                seq = Box::new(SeqExp::Let {
                    var: new_names[i].clone(),
                    bound_exp: Box::new(seq_exp(&args[i])),
                    body: seq,
                    ann: (),
                });
            }
            *seq
        }
        Exp::ExternalCall {
            fun_name,
            args,
            is_tail,
            ann,
        } => {
            if args.is_empty() {
                return SeqExp::ExternalCall {
                    fun_name: fun_name.clone(),
                    args: Vec::new(),
                    is_tail: *is_tail,
                    ann: (),
                };
            }
            let mut new_names = vec![];
            for i in 0..args.len() {
                new_names.push(format!("#arg_{}_{}", ann, i));
            }
            let mut new_args = vec![];
            for n in &new_names {
                new_args.push(ImmExp::Var(n.clone()));
            }
            let mut seq = Box::new(SeqExp::ExternalCall {
                fun_name: fun_name.clone(),
                args: new_args,
                is_tail: *is_tail,
                ann: (),
            });
            for i in (0..args.len()).rev() {
                seq = Box::new(SeqExp::Let {
                    var: new_names[i].clone(),
                    bound_exp: Box::new(seq_exp(&args[i])),
                    body: seq,
                    ann: (),
                });
            }
            *seq
        }
        _ => panic!("seq_exp() cannot"),
    }
}

pub fn seq_decl(decls: &[FunDecl<Exp<()>, ()>]) -> Vec<FunDecl<SeqExp<()>, ()>> {
    let mut new_decls = vec![];
    for decl in decls {
        new_decls.push(FunDecl {
            name: decl.name.clone(),
            parameters: decl.parameters.clone(),
            body: seq_exp(&tag_exp(&decl.body, &mut 0, &mut HashMap::new(), false)),
            ann: (),
        });
    }
    new_decls
}
