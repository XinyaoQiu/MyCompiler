use crate::asm::instrs_to_string;
use crate::asm::Instr;
use crate::compile_with_env::compile_with_env;
use crate::lambda_lift::lambda_lift_helper;
use crate::seq_exp::{seq_decl, seq_exp};
use crate::should_lift::should_lift_helper;
use crate::syntax::FloatWrapper;
use crate::syntax::{Exp, SeqExp, SeqProg, SurfFunDecl, SurfProg};
use crate::tag::tag_funs;
use crate::tag::{tag_exp, tag_seq};

use std::collections::{HashMap, HashSet};

use crate::check::check_exp;

#[derive(Debug, PartialEq, Eq)]
pub enum CompileErr<Span> {
    UnboundVariable {
        unbound: String,
        location: Span,
    },
    UndefinedFunction {
        undefined: String,
        location: Span,
    },
    // The Span here is the Span of the let-expression that has the two duplicated bindings
    DuplicateBinding {
        duplicated_name: String,
        location: Span,
    },

    NumOverflow {
        num: i64,
        location: Span,
    },

    FloatOverflow {
        num: FloatWrapper,
        location: Span,
    },

    DuplicateFunName {
        duplicated_name: String,
        location: Span, // the location of the 2nd function
    },

    DuplicateArgName {
        duplicated_name: String,
        location: Span,
    },

    FunctionUsedAsValue {
        function_name: String,
        location: Span,
    },

    ValueUsedAsFunction {
        variable_name: String,
        location: Span,
    },

    FunctionCalledWrongArity {
        function_name: String,
        correct_arity: usize,
        arity_used: usize,
        location: Span, // location of the function *call*
    },
}

pub fn check_prog<Span>(p: &SurfProg<Span>) -> Result<(), CompileErr<Span>>
where
    Span: Clone,
{
    let mut env = HashSet::new();
    let mut func = HashMap::new();
    check_exp(p, &mut env, &mut func)
}

// Identify which functions should be lifted to the top level
fn should_lift<Ann>(p: &Exp<Ann>) -> HashSet<String> {
    let mut hashset: HashSet<String> = HashSet::new();
    should_lift_helper(p, &mut hashset, true);
    hashset
}


// Lift some functions to global definitions
fn lambda_lift<Ann>(p: &Exp<Ann>) -> (Vec<SurfFunDecl<()>>, Exp<()>, HashMap<String, Vec<String>>) {
    let mut env = Vec::new();
    let mut lifted = vec![];
    let should_lift = should_lift(p);
    let mut fun_to_decl = HashMap::new();
    let mut fun_to_env = HashMap::new();
    let body = lambda_lift_helper(
        p,
        &mut env,
        &mut lifted,
        &should_lift,
        &mut fun_to_decl,
        true,
        &mut fun_to_env,
    );
    (lifted, body, fun_to_env)
}

fn seq_prog(decls: &[SurfFunDecl<()>], p: &Exp<()>) -> SeqProg<()> {
    SeqProg {
        funs: seq_decl(decls),
        main: seq_exp(&tag_exp(p, &mut 0, &mut HashMap::new(),  false)),
        ann: (),
    }
}

fn space_needed<Ann>(e: &SeqExp<Ann>) -> i32 {
    match e {
        SeqExp::Let {
            bound_exp, body, ..
        } => std::cmp::max(1 + space_needed(body), space_needed(bound_exp)),
        SeqExp::If { thn, els, .. } => std::cmp::max(space_needed(thn), space_needed(els)),
        SeqExp::FunDefs { decls, body, .. } => {
            let mut max = 0;
            for decl in decls {
                max = std::cmp::max(max, space_needed(&decl.body) + decl.parameters.len() as i32);
            }
            std::cmp::max(space_needed(body), max)
        }
        _ => 0,
    }
}

fn compile_to_instrs(
    seq_prog: &SeqProg<()>,
    fun_to_env: &HashMap<String, Vec<String>>,
         
) -> (Vec<Instr>, Vec<Instr>) {
    let mut counter: u32 = 1;
    let funs = tag_funs(&seq_prog.funs, &mut counter);
    let main = tag_seq(&seq_prog.main, &mut counter);

    // handle funs
    let mut space = 0;
    for fun in &funs {
        space = std::cmp::max(space, space_needed(&fun.body) + fun.parameters.len() as i32);
    }
    let space = std::cmp::max(space, space_needed(&main));
    let space = if space % 2 == 0 { space + 1 } else { space };
    let mut funs_instrs = vec![];
    for decl in &funs {
        funs_instrs.push(Instr::Label(decl.name.clone()));
        funs_instrs.append(&mut compile_with_env(
            &decl.body,
            decl.parameters.clone(),
            space,
            &fun_to_env,
        ));
        funs_instrs.push(Instr::Ret);
    }
    funs_instrs.push(Instr::Label(format!("funend_{}", 0)));

    // handle main
    let mut main_instrs = compile_with_env(&main, vec![], space, &fun_to_env);

    main_instrs.push(Instr::Ret);
    (funs_instrs, main_instrs)
}

pub fn compile_to_string<Span>(p: &SurfProg<Span>) -> Result<String, CompileErr<Span>>
where
    Span: Clone,
{
    check_prog(p)?;
    let uniquified = tag_exp(p, &mut 0, &mut HashMap::new(), true);
    let (lifted, exp, fun_to_env) = lambda_lift(&uniquified);
    let seq_prog = seq_prog(&lifted, &exp);
    let (fun_instrs, main_instrs) = compile_to_instrs(&seq_prog, &fun_to_env);

    Ok(format!(
        "\
        section .text
        global start_here
        extern print_snake_val
        extern snake_error
{}        start_here:
        push r15
        mov r15, rdi
        call main
        pop r15
        ret
main:
{}
        ",
        instrs_to_string(&fun_instrs),
        instrs_to_string(&main_instrs)
    ))
}
