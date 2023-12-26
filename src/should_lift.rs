use crate::syntax::Exp;
use std::collections::HashSet;

pub fn should_lift_helper<Ann>(p: &Exp<Ann>, hashset: &mut HashSet<String>, is_tail: bool) {
    match p {
        Exp::Prim(_, exps, _) => {
            for exp in exps {
                should_lift_helper(exp, hashset, false);
            }
        }
        Exp::Let { bindings, body, .. } => {
            for (_, b) in bindings {
                should_lift_helper(b, hashset, false);
            }
            should_lift_helper(body, hashset, is_tail);
        }
        Exp::If { cond, thn, els, .. } => {
            should_lift_helper(cond, hashset, false);
            should_lift_helper(thn, hashset, is_tail);
            should_lift_helper(els, hashset, is_tail);
        }
        Exp::FunDefs { decls, body, .. } => {
            for decl in decls {
                should_lift_helper(&decl.body, hashset, is_tail);
            }
            should_lift_helper(body, hashset, true);
        }
        Exp::Call(fun_name, args, _) => {
            if !is_tail {
                hashset.insert(fun_name.clone());
            }
            for arg in args {
                should_lift_helper(arg, hashset, false);
            }
        }
        Exp::Num(_, _) | Exp::Bool(_, _) | Exp::Var(_, _) | Exp::Float(_, _) => (),
        _ => panic!("should_lift wrong"),
    }
}
