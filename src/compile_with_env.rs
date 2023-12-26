use core::panic;
use std::collections::HashMap;

use crate::asm::{Arg32, Arg64, BinArgs, FloatArg, FloatMem, Instr, MemRef, MovArgs, Reg, Reg32};
use crate::syntax::{FloatWrapper, ImmExp, Prim, SeqExp, SeqFunDecl};

pub type Space = i32;

pub static BOOL_MASK: u64 = 0x80_00_00_00_00_00_00_00;
pub static FLOAT_MASK: u64 = 0xFF_FF_FF_FF_E0_00_00_00;

pub static SNAKE_TRUE: u64 = 0xFF_FF_FF_FF_FF_FF_FF_FF;
pub static SNAKE_FALSE: u64 = 0x7F_FF_FF_FF_FF_FF_FF_FF;

type ErrorCode = u64;
static ARITH_ERROR: ErrorCode = 0;
static COMP_ERROR: ErrorCode = 1;
static OVERFLOW_ERROR: ErrorCode = 2;
static LOGIC_ERROR: ErrorCode = 3;
static IF_ERROR: ErrorCode = 4;
static DIVISION_ERROR: ErrorCode = 5;
static SQRT_ERROR: ErrorCode = 6;

pub enum RuntimeType {
    Num,
    Bool,
}

pub fn compile_with_env(
    e: &SeqExp<u32>,
    env: Vec<String>,
    space: i32,
    fun_to_env: &HashMap<String, Vec<String>>,
) -> Vec<Instr> {
    let mut instr = vec![];
    // ...
    match e {
        SeqExp::Imm(ie, _) => {
            immexp_instrs(ie, &env, &mut instr);
        }
        SeqExp::Let {
            var,
            bound_exp,
            body,
            ..
        } => let_instrs(var, bound_exp, body, &env, &mut instr, space, fun_to_env),
        SeqExp::If {
            cond,
            thn,
            els,
            ann,
        } => if_instrs(cond, thn, els, ann, &env, &mut instr, space, fun_to_env),
        SeqExp::Prim(op, exps, ann) => prim_instrs(op, exps, ann, &env, &mut instr, space),
        SeqExp::InternalTailCall(fun_name, args, _) => {
            incall_instr(fun_name, args, &env, &mut instr, fun_to_env)
        }
        SeqExp::ExternalCall {
            fun_name,
            args,
            is_tail,
            ..
        } => excall_instr(fun_name, args, is_tail, &env, &mut instr, space, fun_to_env),
        SeqExp::FunDefs { decls, body, ann } => {
            fundefs_instr(&decls, &body, &env, &mut instr, ann, space, fun_to_env);
        }
    }
    // ...
    instr
}

fn immexp_instrs(imme: &ImmExp, env: &Vec<String>, instr: &mut Vec<Instr>) {
    match imme {
        ImmExp::Num(n) => instr.push(Instr::Mov(MovArgs::ToReg(Reg::Rax, Arg64::Signed(n << 1)))),
        ImmExp::Bool(b) => {
            if *b {
                instr.push(Instr::Mov(MovArgs::ToReg(
                    Reg::Rax,
                    Arg64::Unsigned(SNAKE_TRUE),
                )))
            } else {
                instr.push(Instr::Mov(MovArgs::ToReg(
                    Reg::Rax,
                    Arg64::Unsigned(SNAKE_FALSE),
                )))
            }
        }
        ImmExp::Var(id) => instr.push(Instr::Mov(MovArgs::ToReg(
            Reg::Rax,
            Arg64::Mem(MemRef {
                reg: Reg::Rsp,
                offset: -8 * (env.iter().position(|x| x == id).unwrap() as i32 + 1),
            }),
        ))),
        ImmExp::Float(FloatWrapper(f)) => {
            instr.append(&mut st_constfloat(Reg::Rax, *f as f32));
        }
    }
}

fn let_instrs(
    var: &String,
    bound_exp: &Box<SeqExp<u32>>,
    body: &Box<SeqExp<u32>>,
    env: &Vec<String>,
    instr: &mut Vec<Instr>,
    space: i32,
    fun_to_env: &HashMap<String, Vec<String>>,
) {
    instr.append(&mut compile_with_env(
        bound_exp,
        env.clone(),
        space,
        fun_to_env,
    ));
    instr.push(Instr::Mov(MovArgs::ToMem(
        MemRef {
            reg: Reg::Rsp,
            offset: {
                match env.iter().position(|x| x == var) {
                    Some(n) => -8 * (n as i32 + 1),
                    None => -8 * (env.len() as i32 + 1),
                }
            },
        },
        Reg32::Reg(Reg::Rax),
    )));
    let mut new_env = env.clone();
    if !new_env.contains(var) {
        new_env.push(var.clone());
    }
    instr.append(&mut compile_with_env(body, new_env, space, fun_to_env));
}

fn if_instrs(
    cond: &ImmExp,
    thn: &Box<SeqExp<u32>>,
    els: &Box<SeqExp<u32>>,
    ann: &u32,
    env: &Vec<String>,
    instr: &mut Vec<Instr>,
    space: i32,
    fun_to_env: &HashMap<String, Vec<String>>,
) {
    immexp_instrs(cond, &env, instr);
    instr.append(&mut check_bool(Reg::Rax, IF_ERROR, true));
    instr.push(Instr::Mov(MovArgs::ToReg(
        Reg::R8,
        Arg64::Unsigned(SNAKE_FALSE),
    )));

    instr.push(Instr::Cmp(BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R8))));
    instr.push(Instr::Je(format!("else_{:?}", ann)));
    instr.append(&mut compile_with_env(thn, env.clone(), space, fun_to_env));
    instr.push(Instr::Jmp(format!("end_{:?}", ann)));
    instr.push(Instr::Label(format!("else_{:?}", ann)));
    instr.append(&mut compile_with_env(els, env.clone(), space, fun_to_env));
    instr.push(Instr::Label(format!("end_{:?}", ann)));
}

fn logic_prim(op: &Prim, exps: &Vec<ImmExp>, env: &Vec<String>, instr: &mut Vec<Instr>) {
    if exps.len() == 1 {
        immexp_instrs(&exps[0], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, LOGIC_ERROR, true));
    } else if exps.len() == 2 {
        immexp_instrs(&exps[1], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, LOGIC_ERROR, true));
        instr.push(Instr::Mov(MovArgs::ToReg(Reg::R8, Arg64::Reg(Reg::Rax))));
        immexp_instrs(&exps[0], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, LOGIC_ERROR, true));
    }
    match *op {
        Prim::Not => {
            instr.push(Instr::Mov(MovArgs::ToReg(
                Reg::R8,
                Arg64::Unsigned(BOOL_MASK),
            )));
            instr.push(Instr::Xor(BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R8))));
        }
        Prim::And | Prim::Or => {
            instr.push(if *op == Prim::And {
                Instr::And(BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R8)))
            } else {
                Instr::Or(BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R8)))
            });
        }

        _ => panic!("logic Prim here"),
    }
}

fn other_prim(
    op: &Prim,
    exps: &Vec<ImmExp>,
    env: &Vec<String>,
    instr: &mut Vec<Instr>,
    ann: &u32,
    space: i32,
) {
    match *op {
        Prim::Print => {
            immexp_instrs(&exps[0], &env, instr);
            instr.extend(vec![
                Instr::Mov(MovArgs::ToReg(Reg::Rdi, Arg64::Reg(Reg::Rax))),
                Instr::Sub(BinArgs::ToReg(Reg::Rsp, Arg32::Signed(8 * space))),
                Instr::Call("print_snake_val".to_string()),
                Instr::Add(BinArgs::ToReg(Reg::Rsp, Arg32::Signed(8 * space))),
            ]);
        }
        Prim::IsNum => {
            instr.push(Instr::Mov(MovArgs::ToReg(Reg::R9, Arg64::Unsigned(1))));
            instr.push(Instr::And(BinArgs::ToReg(Reg::R9, Arg32::Reg(Reg::Rax))));
            instr.push(Instr::Cmp(BinArgs::ToReg(Reg::R9, Arg32::Unsigned(0))));
            instr.push(Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(SNAKE_TRUE),
            )));
            instr.push(Instr::Jz(format!("isnum_done_{}", ann)));
            instr.push(Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(SNAKE_FALSE),
            )));
            instr.push(Instr::Label(format!("isnum_done_{}", ann)));
        }
        Prim::IsBool => {
            instr.push(Instr::Mov(MovArgs::ToReg(Reg::R9, Arg64::Unsigned(3))));
            instr.push(Instr::And(BinArgs::ToReg(Reg::R9, Arg32::Reg(Reg::Rax))));
            instr.push(Instr::Cmp(BinArgs::ToReg(Reg::R9, Arg32::Unsigned(3))));
            instr.push(Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(SNAKE_TRUE),
            )));
            instr.push(Instr::Jz(format!("isbool_done_{}", ann)));
            instr.push(Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(SNAKE_FALSE),
            )));
            instr.push(Instr::Label(format!("isbool_done_{}", ann)));
        }
        Prim::IsFloat => {
            instr.push(Instr::Mov(MovArgs::ToReg(Reg::R9, Arg64::Unsigned(3))));
            instr.push(Instr::And(BinArgs::ToReg(Reg::R9, Arg32::Reg(Reg::Rax))));
            instr.push(Instr::Cmp(BinArgs::ToReg(Reg::R9, Arg32::Unsigned(1))));
            instr.push(Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(SNAKE_TRUE),
            )));
            instr.push(Instr::Jz(format!("isfloat_done_{}", ann)));
            instr.push(Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(SNAKE_FALSE),
            )));
            instr.push(Instr::Label(format!("isfloat_done_{}", ann)));
        }
        _ => panic!("other prim here"),
    }
}

fn arith_prim(op: &Prim, exps: &Vec<ImmExp>, ann: &u32, env: &Vec<String>, instr: &mut Vec<Instr>) {
    let offset = -8 * (env.len() + 1) as i32;
    if exps.len() == 1 {
        immexp_instrs(&exps[0], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, ARITH_ERROR, false));
    } else if exps.len() == 2 {
        immexp_instrs(&exps[1], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, ARITH_ERROR, false));
        instr.push(Instr::Mov(MovArgs::ToReg(Reg::R8, Arg64::Reg(Reg::Rax))));
        immexp_instrs(&exps[0], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, ARITH_ERROR, false));
    }
    match *op {
        Prim::Add1 => {
            let mut instr1 = vec![Instr::Add(BinArgs::ToReg(Reg::Rax, Arg32::Signed(1 << 1)))];
            instr1.append(&mut check_overflow());
            let instr2 = vec![Instr::Fld1, Instr::Faddp(FloatArg::Blank)];

            unaryop(&instr1, &instr2, ann, offset, instr);
        }
        Prim::Sub1 => {
            let mut instr1 = vec![Instr::Sub(BinArgs::ToReg(Reg::Rax, Arg32::Signed(1 << 1)))];
            instr1.append(&mut check_overflow());
            let instr2 = vec![Instr::Fld1, Instr::Fsubp(FloatArg::Blank)];

            unaryop(&instr1, &instr2, ann, offset, instr);
        }
        Prim::Add => {
            let mut instr1 = vec![Instr::Add(BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R8)))];
            instr1.append(&mut check_overflow());
            let mut instr2 = vec![Instr::Faddp(FloatArg::Blank)];
            instr2.append(&mut check_float_overflow(ann, offset));
            instr2.append(&mut st_float_to_reg(Reg::Rax, offset));

            binop(&instr1, &instr2, ann, offset, instr);
        }
        Prim::Sub => {
            let mut instr1 = vec![Instr::Sub(BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R8)))];
            instr1.append(&mut check_overflow());
            let mut instr2 = vec![Instr::Fsubp(FloatArg::Blank)];
            instr2.append(&mut check_float_overflow(ann, offset));
            instr2.append(&mut st_float_to_reg(Reg::Rax, offset));

            binop(&instr1, &instr2, ann, offset, instr);
        }
        Prim::Mul => {
            let mut instr1 = vec![
                Instr::Sar(BinArgs::ToReg(Reg::R8, Arg32::Unsigned(1))),
                Instr::Sar(BinArgs::ToReg(Reg::Rax, Arg32::Unsigned(1))),
                Instr::IMul(BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R8))),
            ];
            instr1.append(&mut check_overflow());
            instr1.push(Instr::Shl(BinArgs::ToReg(Reg::Rax, Arg32::Unsigned(1))));
            let mut instr2 = vec![Instr::Fmulp(FloatArg::Blank)];
            instr2.append(&mut check_float_overflow(ann, offset));
            instr2.append(&mut st_float_to_reg(Reg::Rax, offset));

            binop(&instr1, &instr2, ann, offset, instr);
        }
        Prim::Div => {
            let mut instr1 = vec![];
            instr1.append(&mut ld_num_from_reg(Reg::Rax, offset));
            instr1.append(&mut ld_num_from_reg(Reg::R8, offset));
            instr1.append(&mut check_division_zero_num(Reg::R8, ann));
            instr1.push(Instr::Fdivp(FloatArg::Blank));
            instr1.append(&mut st_float_to_reg(Reg::Rax, offset));

            let mut instr2 = check_division_zero_float(ann, offset);
            instr2.push(Instr::Fdivp(FloatArg::Blank));
            instr2.append(&mut check_float_overflow(ann, offset));
            instr2.append(&mut st_float_to_reg(Reg::Rax, offset));

            binop(&instr1, &instr2, ann, offset, instr);
        }
        Prim::FloorDiv => {
            let mut instr1 = ld_num_from_reg(Reg::Rax, offset);
            instr1.append(&mut ld_num_from_reg(Reg::R8, offset));
            instr1.append(&mut check_division_zero_num(Reg::R8, ann));
            instr1.push(Instr::Fdivp(FloatArg::Blank));
            instr1.append(&mut st_floornum_to_reg(Reg::Rax, offset));

            let mut instr2 = check_division_zero_float(ann, offset);
            instr2.push(Instr::Fdivp(FloatArg::Blank));
            instr2.append(&mut check_float_overflow(ann, offset));
            instr2.append(&mut st_floornum_to_reg(Reg::Rax, offset));

            binop(&instr1, &instr2, ann, offset, instr);
        }

        Prim::Cos => {
            let mut instr1 = vec![];
            instr1.append(&mut ld_num_from_reg(Reg::Rax, offset));
            instr1.push(Instr::Fcos);
            instr1.append(&mut st_float_to_reg(Reg::Rax, offset));
            let instr2 = vec![Instr::Fcos];

            unaryop(&instr1, &instr2, ann, offset, instr);
        }
        Prim::Sqrt => {
            let mut instr1 = vec![];
            instr1.append(&mut check_sqrt_num(Reg::Rax, ann));
            instr1.append(&mut ld_num_from_reg(Reg::Rax, offset));
            instr1.push(Instr::Fsqrt);
            instr1.append(&mut st_float_to_reg(Reg::Rax, offset));
            let mut instr2 = check_sqrt_float(ann, offset);
            instr2.push(Instr::Fsqrt);

            unaryop(&instr1, &instr2, ann, offset, instr);
        }
        _ => panic!("arith prim here"),
    }
}

fn comp_prim(op: &Prim, exps: &Vec<ImmExp>, ann: &u32, env: &Vec<String>, instr: &mut Vec<Instr>) {
    let offset = -8 * (env.len() + 1) as i32;
    if exps.len() == 1 {
        immexp_instrs(&exps[0], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, COMP_ERROR, false));
    } else if exps.len() == 2 {
        immexp_instrs(&exps[1], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, COMP_ERROR, false));
        instr.push(Instr::Mov(MovArgs::ToReg(Reg::R8, Arg64::Reg(Reg::Rax))));
        immexp_instrs(&exps[0], &env, instr);
        instr.append(&mut check_bool(Reg::Rax, COMP_ERROR, false));
    }
    match *op {
        Prim::Lt | Prim::Gt | Prim::Le | Prim::Ge | Prim::Eq | Prim::Neq => {
            let mut instr1 = vec![];
            instr1.push(Instr::Cmp(BinArgs::ToReg(Reg::Rax, Arg32::Reg(Reg::R8))));
            instr1.push(Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(SNAKE_TRUE),
            )));
            instr1.push(match op {
                Prim::Ge => Instr::Jge(format!("greater_equal_{}", ann)),
                Prim::Eq => Instr::Je(format!("equal_{}", ann)),
                Prim::Neq => Instr::Jne(format!("not_equal_{}", ann)),
                Prim::Lt => Instr::Jl(format!("less_than_{}", ann)),
                Prim::Gt => Instr::Jg(format!("greater_than_{}", ann)),
                Prim::Le => Instr::Jle(format!("less_equal_{}", ann)),
                _ => panic!("invalid logic comparison"),
            });
            instr1.push(Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(SNAKE_FALSE),
            )));
            instr1.push(Instr::Label(format!(
                "{}_{}",
                match op {
                    Prim::Lt => "less_than",
                    Prim::Gt => "greater_than",
                    Prim::Le => "less_equal",
                    Prim::Ge => "greater_equal",
                    Prim::Eq => "equal",
                    Prim::Neq => "not_equal",
                    _ => panic!("invalid logic comparison"),
                },
                ann,
            )));

            // r8 > rax, so rax < r8
            let larger = vec![Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(match *op {
                    Prim::Ge => SNAKE_FALSE,
                    Prim::Eq => SNAKE_FALSE,
                    Prim::Neq => SNAKE_TRUE,
                    Prim::Lt => SNAKE_TRUE,
                    Prim::Gt => SNAKE_FALSE,
                    Prim::Le => SNAKE_TRUE,
                    _ => panic!("invalid logic comparison"),
                }),
            ))];

            // r8 < rax, so rax > r8
            let smaller = vec![Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(match *op {
                    Prim::Ge => SNAKE_TRUE,
                    Prim::Eq => SNAKE_FALSE,
                    Prim::Neq => SNAKE_TRUE,
                    Prim::Lt => SNAKE_FALSE,
                    Prim::Gt => SNAKE_TRUE,
                    Prim::Le => SNAKE_FALSE,
                    _ => panic!("invalid logic comparison"),
                }),
            ))];

            // r8 == rax, so rax == r8
            let equal = vec![Instr::Mov(MovArgs::ToReg(
                Reg::Rax,
                Arg64::Unsigned(match *op {
                    Prim::Ge => SNAKE_FALSE,
                    Prim::Eq => SNAKE_TRUE,
                    Prim::Neq => SNAKE_FALSE,
                    Prim::Lt => SNAKE_FALSE,
                    Prim::Gt => SNAKE_FALSE,
                    Prim::Le => SNAKE_FALSE,
                    _ => panic!("invalid logic comparison"),
                }),
            ))];

            let instr2 = cmp_floats(
                (&larger, format!("logic_larger_{}", ann)),
                (&smaller, format!("logic_smaller_{}", ann)),
                (&equal, format!("logic_equal_{}", ann)),
                format!("cmp_done_{}", ann),
                2,
            );

            binop(&instr1, &instr2, ann, offset, instr);
        }
        _ => panic!("comp prim here"),
    }
}

fn prim_instrs(
    op: &Prim,
    exps: &Vec<ImmExp>,
    ann: &u32,
    env: &Vec<String>,
    instr: &mut Vec<Instr>,
    space: i32,
) {
    match *op {
        Prim::Add1
        | Prim::Sub1
        | Prim::Add
        | Prim::Sub
        | Prim::Mul
        | Prim::Div
        | Prim::FloorDiv
        | Prim::Cos
        | Prim::Sqrt => arith_prim(op, exps, ann, env, instr),
        Prim::Lt | Prim::Gt | Prim::Le | Prim::Ge | Prim::Eq | Prim::Neq => {
            comp_prim(op, exps, ann, env, instr)
        }
        Prim::And | Prim::Or | Prim::Not => logic_prim(op, exps, env, instr),
        Prim::Print | Prim::IsBool | Prim::IsNum | Prim::IsFloat => {
            other_prim(op, exps, env, instr, ann, space)
        }
    }
}

pub fn check_division_zero_num(reg: Reg, ann: &u32) -> Vec<Instr> {
    let mut instr = vec![];
    instr.push(Instr::Mov(MovArgs::ToReg(Reg::R9, Arg64::Unsigned(0))));
    instr.push(Instr::Cmp(BinArgs::ToReg(Reg::R9, Arg32::Reg(reg))));
    instr.push(Instr::Jne(format!("not_zero_{}", ann)));
    instr.push(Instr::Mov(MovArgs::ToReg(
        Reg::Rdi,
        Arg64::Unsigned(DIVISION_ERROR),
    )));
    instr.push(Instr::Jmp("snake_error".to_string()));
    instr.push(Instr::Label(format!("not_zero_{}", ann)));
    instr
}

pub fn check_sqrt_num(reg: Reg, ann: &u32) -> Vec<Instr> {
    let mut instr = vec![];
    instr.push(Instr::Mov(MovArgs::ToReg(Reg::R9, Arg64::Unsigned(0))));
    instr.push(Instr::Cmp(BinArgs::ToReg(Reg::R9, Arg32::Reg(reg))));
    instr.push(Instr::Jle(format!("not_lt_zero_{}", ann)));
    instr.push(Instr::Mov(MovArgs::ToReg(
        Reg::Rdi,
        Arg64::Unsigned(SQRT_ERROR),
    )));
    instr.push(Instr::Jmp("snake_error".to_string()));
    instr.push(Instr::Label(format!("not_lt_zero_{}", ann)));
    instr
}

pub fn check_division_zero_float(ann: &u32, offset: i32) -> Vec<Instr> {
    let mut instr = vec![];
    instr.append(&mut ld_constfloat(0.0, offset));
    let equal = vec![
        Instr::Mov(MovArgs::ToReg(Reg::Rdi, Arg64::Unsigned(DIVISION_ERROR))),
        Instr::Jmp("snake_error".to_string()),
    ];
    let larger = vec![];
    let smaller = vec![];
    instr.append(&mut cmp_floats(
        (&larger, format!("check_div_larger_{}", ann)),
        (&smaller, format!("check_div_smaller_{}", ann)),
        (&equal, format!("check_div_equal_{}", ann)),
        format!("cmp_done_{}", ann),
        1,
    ));
    instr
}

pub fn check_sqrt_float(ann: &u32, offset: i32) -> Vec<Instr> {
    let mut instr = vec![];
    instr.append(&mut ld_constfloat(0.0, offset));
    let larger = vec![
        Instr::Mov(MovArgs::ToReg(Reg::Rdi, Arg64::Unsigned(DIVISION_ERROR))),
        Instr::Jmp("snake_error".to_string()),
    ];
    let equal = vec![];
    let smaller = vec![];
    instr.append(&mut cmp_floats(
        (&larger, format!("check_sqrt_larger_{}", ann)),
        (&smaller, format!("check_sqrt_smaller_{}", ann)),
        (&equal, format!("check_sqrt_equal_{}", ann)),
        format!("cmp_done_{}", ann),
        1,
    ));
    instr
}

pub fn check_overflow() -> Vec<Instr> {
    vec![
        Instr::Mov(MovArgs::ToReg(Reg::Rdi, Arg64::Unsigned(OVERFLOW_ERROR))),
        Instr::Jo("snake_error".to_string()),
    ]
}

pub fn unaryop(
    instr1: &Vec<Instr>,
    instr2: &Vec<Instr>,
    ann: &u32,
    offset: i32,
    instr: &mut Vec<Instr>,
) {
    // check number of float
    let label = format!("when_float_{}", ann);
    let done = format!("done_{}", ann);
    instr.append(&mut if_float(Reg::Rax, &label));

    // when number
    instr.append(&mut instr1.clone());

    instr.push(Instr::Jmp(done.clone()));

    //when float
    instr.push(Instr::Label(label));
    instr.append(&mut ld_float_from_reg(Reg::Rax, offset));
    instr.append(&mut instr2.clone());
    instr.append(&mut check_float_overflow(ann, offset));
    instr.append(&mut st_float_to_reg(Reg::Rax, offset));

    // done
    instr.push(Instr::Label(done));
}

pub fn binop(
    instr1: &Vec<Instr>,
    instr2: &Vec<Instr>,
    ann: &u32,
    offset: i32,
    instr: &mut Vec<Instr>,
) {
    // check number or float
    let first_float_label = format!("first_float_{}", ann);
    let second_float_label = format!("second_float_{}", ann);
    let handle_floats = format!("handle_float_{}", ann);
    let done = format!("done_{}", ann);
    instr.append(&mut if_float(Reg::Rax, &first_float_label));
    instr.append(&mut if_float(Reg::R8, &second_float_label));

    // when both numbers
    instr.append(&mut instr1.clone());
    instr.push(Instr::Jmp(done.clone()));

    // when first is float, check second
    instr.push(Instr::Label(first_float_label));
    let both_floats_label = format!("both_floats_{}", ann);
    instr.append(&mut if_float(Reg::R8, &both_floats_label));

    // when first is float and second is not
    instr.append(&mut ld_float_from_reg(Reg::Rax, offset));
    instr.append(&mut ld_num_from_reg(Reg::R8, offset));
    instr.push(Instr::Jmp(handle_floats.clone()));

    // when both are floats
    instr.push(Instr::Label(both_floats_label));
    instr.append(&mut ld_float_from_reg(Reg::Rax, offset));
    instr.append(&mut ld_float_from_reg(Reg::R8, offset));
    instr.push(Instr::Jmp(handle_floats.clone()));

    // when first is not but second is float
    instr.push(Instr::Label(second_float_label));
    instr.append(&mut ld_num_from_reg(Reg::Rax, offset));
    instr.append(&mut ld_float_from_reg(Reg::R8, offset));

    // handle_floats
    instr.push(Instr::Label(handle_floats.clone()));
    instr.append(&mut instr2.clone());

    // done
    instr.push(Instr::Label(done));
}

fn check_bool(reg: Reg, error_code: ErrorCode, is_or_not: bool) -> Vec<Instr> {
    vec![
        // only check 2 bits, 11 then bool
        Instr::Mov(MovArgs::ToReg(Reg::R9, Arg64::Unsigned(3))),
        Instr::And(BinArgs::ToReg(Reg::R9, Arg32::Reg(reg))),
        Instr::Cmp(BinArgs::ToReg(Reg::R9, Arg32::Unsigned(3))),
        Instr::Mov(MovArgs::ToReg(Reg::Rdi, Arg64::Unsigned(error_code))),
        if is_or_not {
            Instr::Jne("snake_error".to_string())
        } else {
            Instr::Je("snake_error".to_string())
        },
    ]
}

fn if_float(reg: Reg, label: &String) -> Vec<Instr> {
    vec![
        // check 1 bit, 0 then num and 1 float
        Instr::Mov(MovArgs::ToReg(Reg::R9, Arg64::Unsigned(1))),
        Instr::And(BinArgs::ToReg(Reg::R9, Arg32::Reg(reg))),
        Instr::Cmp(BinArgs::ToReg(Reg::R9, Arg32::Unsigned(1))),
        Instr::Je(label.clone()),
    ]
}

fn ld_float_from_reg(reg: Reg, offset: i32) -> Vec<Instr> {
    vec![
        Instr::Sub(BinArgs::ToReg(reg, Arg32::Unsigned(1))),
        Instr::Mov(MovArgs::ToMem(
            MemRef {
                reg: Reg::Rsp,
                offset: offset,
            },
            Reg32::Reg(reg),
        )),
        Instr::Fld(FloatMem::RegMem(MemRef {
            reg: Reg::Rsp,
            offset: offset,
        })),
    ]
}

fn ld_num_from_reg(reg: Reg, offset: i32) -> Vec<Instr> {
    vec![
        Instr::Shr(BinArgs::ToReg(reg, Arg32::Unsigned(1))),
        Instr::Mov(MovArgs::ToMem(
            MemRef {
                reg: Reg::Rsp,
                offset: offset,
            },
            Reg32::Reg(reg),
        )),
        Instr::Fild(FloatMem::RegMem(MemRef {
            reg: Reg::Rsp,
            offset: offset,
        })),
    ]
}

fn st_constfloat(reg: Reg, f: f32) -> Vec<Instr> {
    vec![
        Instr::Mov(MovArgs::ToReg(reg, Arg64::Unsigned((f as f64).to_bits()))),
        Instr::Add(BinArgs::ToReg(reg, Arg32::Unsigned(1))),
    ]
}

fn ld_constfloat(f: f32, offset: i32) -> Vec<Instr> {
    vec![
        Instr::Mov(MovArgs::ToReg(
            Reg::R9,
            Arg64::Unsigned((f as f64).to_bits()),
        )),
        Instr::Mov(MovArgs::ToMem(
            MemRef {
                reg: Reg::Rsp,
                offset: offset,
            },
            Reg32::Reg(Reg::R9),
        )),
        Instr::Fld(FloatMem::RegMem(MemRef {
            reg: Reg::Rsp,
            offset: offset,
        })),
    ]
}

fn st_float_to_reg(reg: Reg, offset: i32) -> Vec<Instr> {
    vec![
        Instr::Fstp(FloatMem::RegMem(MemRef {
            reg: Reg::Rsp,
            offset: offset,
        })),
        Instr::Mov(MovArgs::ToReg(
            reg,
            Arg64::Mem(MemRef {
                reg: Reg::Rsp,
                offset: offset,
            }),
        )),
        // make sure the last two bits are 01
        Instr::Mov(MovArgs::ToReg(Reg::R9, Arg64::Unsigned(FLOAT_MASK))),
        Instr::Add(BinArgs::ToReg(reg, Arg32::Unsigned(0x10000000))),
        Instr::And(BinArgs::ToReg(reg, Arg32::Reg(Reg::R9))),
        Instr::Add(BinArgs::ToReg(reg, Arg32::Unsigned(1))),
    ]
}

fn st_floornum_to_reg(reg: Reg, offset: i32) -> Vec<Instr> {
    let mut instr = ld_constfloat(0.5, offset);
    instr.append(&mut vec![
        Instr::Fsubp(FloatArg::Blank),
        Instr::Fistp(FloatMem::RegMem(MemRef {
            reg: Reg::Rsp,
            offset: offset,
        })),
        Instr::Mov(MovArgs::ToReg(
            reg,
            Arg64::Mem(MemRef {
                reg: Reg::Rsp,
                offset: offset,
            }),
        )),
        // make sure the last bit is 0
        Instr::Shl(BinArgs::ToReg(reg, Arg32::Unsigned(1))),
    ]);
    instr
}

fn cmp_floats(
    larger: (&Vec<Instr>, String),
    smaller: (&Vec<Instr>, String),
    equal: (&Vec<Instr>, String),
    done: String,
    pop: u32,
) -> Vec<Instr> {
    let mut instr = vec![
        if pop == 2 {
            Instr::Fcompp(FloatArg::Blank)
        } else if pop == 1 {
            Instr::Fcomp(FloatArg::Blank)
        } else if pop == 0 {
            Instr::Fcom(FloatArg::Blank)
        } else {
            panic!("pop wrong")
        },
        Instr::Fstsw(FloatArg::Reg(Reg::Ax)),
        Instr::And(BinArgs::ToReg(Reg::Ax, Arg32::Unsigned(0x4100))),
        Instr::Cmp(BinArgs::ToReg(Reg::Ax, Arg32::Unsigned(0x4000))),
        Instr::Je(equal.1.clone()),
        Instr::Cmp(BinArgs::ToReg(Reg::Ax, Arg32::Unsigned(0x0100))),
        Instr::Je(smaller.1.clone()),
    ];
    instr.append(&mut larger.0.clone());
    instr.push(Instr::Jmp(done.clone()));
    instr.push(Instr::Label(equal.1.clone()));
    instr.append(&mut equal.0.clone());
    instr.push(Instr::Jmp(done.clone()));
    instr.push(Instr::Label(smaller.1.clone()));
    instr.append(&mut smaller.0.clone());
    instr.push(Instr::Label(done.clone()));
    instr
}

fn check_float_overflow(ann: &u32, offset: i32) -> Vec<Instr> {
    let mut instr = vec![];

    instr.append(&mut ld_constfloat(f32::MAX, offset));
    let smaller = vec![
        Instr::Mov(MovArgs::ToReg(Reg::Rdi, Arg64::Unsigned(OVERFLOW_ERROR))),
        Instr::Jmp("snake_error".to_string()),
    ];
    let larger = vec![];
    let equal = vec![];
    instr.append(&mut cmp_floats(
        (&larger, format!("checkfloatover1_larger_{}", ann)),
        (&smaller, format!("checkfloatover1_smaller_{}", ann)),
        (&equal, format!("checkfloatover1_equal_{}", ann)),
        format!("overflow1_done_{}", ann),
        1,
    ));

    instr.append(&mut ld_constfloat(f32::MIN, offset));
    let larger = vec![
        Instr::Mov(MovArgs::ToReg(Reg::Rdi, Arg64::Unsigned(OVERFLOW_ERROR))),
        Instr::Jo("snake_error".to_string()),
    ];
    let smaller = vec![];
    let equal = vec![];
    instr.append(&mut cmp_floats(
        (&larger, format!("checkfloatover2_larger_{}", ann)),
        (&smaller, format!("checkfloatover2_smaller_{}", ann)),
        (&equal, format!("checkfloatover2_equal_{}", ann)),
        format!("overflow2_done_{}", ann),
        1,
    ));

    instr
}

fn excall_instr(
    fun_name: &String,
    args: &Vec<ImmExp>,
    is_tail: &bool,
    env: &Vec<String>,
    instr: &mut Vec<Instr>,
    space: i32,
    fun_to_env: &HashMap<String, Vec<String>>,
) {
    if *is_tail {
        incall_instr(fun_name, args, env, instr, fun_to_env);
    } else {
        match fun_to_env.get(fun_name) {
            Some(e) => {
                for i in 0..e.len() {
                    instr.push(Instr::Mov(MovArgs::ToReg(
                        Reg::Rax,
                        Arg64::Mem(MemRef {
                            reg: Reg::Rsp,
                            offset: -8 * (env.iter().position(|x| *x == *e[i]).unwrap() as i32 + 1),
                        }),
                    )));
                    instr.push(Instr::Mov(MovArgs::ToMem(
                        MemRef {
                            reg: Reg::Rsp,
                            offset: -8 * (space + 2 + i as i32),
                        },
                        Reg32::Reg(Reg::Rax),
                    )))
                }
                for i in 0..args.len() {
                    let s = match &args[i] {
                        ImmExp::Var(value) => value,
                        _ => panic!("wrong in args"),
                    };
                    instr.push(Instr::Mov(MovArgs::ToReg(
                        Reg::Rax,
                        Arg64::Mem(MemRef {
                            reg: Reg::Rsp,
                            offset: -8 * (env.iter().position(|x| *x == *s).unwrap() as i32 + 1),
                        }),
                    )));
                    instr.push(Instr::Mov(MovArgs::ToMem(
                        MemRef {
                            reg: Reg::Rsp,
                            offset: -8 * (space + 2 + i as i32 + e.len() as i32),
                        },
                        Reg32::Reg(Reg::Rax),
                    )))
                }
                instr.push(Instr::Sub(BinArgs::ToReg(
                    Reg::Rsp,
                    Arg32::Signed(8 * space),
                )));
                instr.push(Instr::Call(fun_name.clone()));
                instr.push(Instr::Add(BinArgs::ToReg(
                    Reg::Rsp,
                    Arg32::Signed(8 * space),
                )));
            }
            None => {
                panic!("compile_with_env, line 371");
            }
        }
    }
}

fn incall_instr(
    fun_name: &String,
    args: &Vec<ImmExp>,
    env: &Vec<String>,
    instr: &mut Vec<Instr>,
    fun_to_env: &HashMap<String, Vec<String>>,
) {
    match fun_to_env.get(fun_name) {
        Some(e) => {
            for i in 0..e.len() {
                instr.push(Instr::Mov(MovArgs::ToReg(
                    Reg::Rax,
                    Arg64::Mem(MemRef {
                        reg: Reg::Rsp,
                        offset: -8 * (env.iter().position(|x| *x == *e[i]).unwrap() as i32 + 1),
                    }),
                )));
                instr.push(Instr::Mov(MovArgs::ToMem(
                    MemRef {
                        reg: Reg::Rsp,
                        offset: -8 * (1 + i as i32),
                    },
                    Reg32::Reg(Reg::Rax),
                )))
            }
            for i in 0..args.len() {
                let s = match &args[i] {
                    ImmExp::Var(value) => value,
                    _ => panic!("wrong in args"),
                };
                instr.push(Instr::Mov(MovArgs::ToReg(
                    Reg::Rax,
                    Arg64::Mem(MemRef {
                        reg: Reg::Rsp,
                        offset: -8 * (env.iter().position(|x| *x == *s).unwrap() as i32 + 1),
                    }),
                )));
                instr.push(Instr::Mov(MovArgs::ToMem(
                    MemRef {
                        reg: Reg::Rsp,
                        offset: -8 * (e.len() + i + 1) as i32,
                    },
                    Reg32::Reg(Reg::Rax),
                )))
            }
            instr.push(Instr::Jmp(fun_name.clone()));
        }
        None => {
            panic!("wrong in fun_to_env")
        }
    }
}

fn fundefs_instr(
    decls: &Vec<SeqFunDecl<u32>>,
    body: &Box<SeqExp<u32>>,
    env: &Vec<String>,
    instr: &mut Vec<Instr>,
    ann: &u32,
    space: i32,
    fun_to_env: &HashMap<String, Vec<String>>,
) {
    instr.push(Instr::Jmp(format!("funend_{}", ann)));
    for decl in decls {
        instr.push(Instr::Label(decl.name.clone()));
        match fun_to_env.get(&decl.name) {
            Some(value) => {
                let mut new_env = value.clone();
                for parameter in &decl.parameters {
                    if !new_env.contains(&parameter) {
                        new_env.push(parameter.clone());
                    }
                }
                instr.append(&mut compile_with_env(
                    &decl.body, new_env, space, fun_to_env,
                ));
                instr.push(Instr::Ret);
            }
            None => panic!("wrong"),
        }
    }
    instr.push(Instr::Label(format!("funend_{}", ann)));

    instr.append(&mut compile_with_env(&body, env.clone(), space, fun_to_env));
}
