use crate::syntax::{Exp, Prim, SurfProg, SurfFunDecl, FloatWrapper};

use std::rc::Rc;

#[derive(Debug, Clone)]
enum SnakeVal {
    Num(i64), // should fit into 63 bits though
    Bool(bool),
    Float(f64),
    Closure(usize), // index into the closure arena
}

// A reference-counted linked list/the functional programmer's List
#[derive(Debug, Clone)]
enum List<T> {
    Empty,
    Cons(T, Rc<List<T>>),
}

type Env = Rc<List<(String, SnakeVal)>>;

fn push_local(env: &Env, name: String, v: SnakeVal) -> Env {
    Rc::new(List::Cons((name, v), env.clone()))
}

#[derive(Debug, Clone)]
struct Closure<'exp, Ann> {
    exp: &'exp Exp<Ann>,
    env: Env,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpErr {
    ExpectedNum {
        who: String,
        got: String,
        msg: String,
    },
    ExpectedBool {
        who: String,
        got: String,
        msg: String,
    },
    ExpectedFun {
        who: String,
        got: String,
        msg: String,
    },
    Overflow {
        msg: String,
    },
    Write {
        msg: String,
    },
    ArityErr {
        expected_arity: usize,
        num_provided: usize,
    },
}

type Interp<T> = Result<T, InterpErr>;

use std::fmt;
use std::fmt::Display;

impl Display for SnakeVal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SnakeVal::Num(n) => write!(f, "{}", n),
            SnakeVal::Bool(b) => write!(f, "{}", b),
            SnakeVal::Float(n) => write!(f, "{}", n),
            SnakeVal::Closure { .. } => write!(f, "closure"),
        }
    }
}

impl Display for InterpErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InterpErr::ExpectedNum { who, got: v, msg } => {
                write!(f, "{} expected a number, but got {} in {}", who, v, msg)
            }
            InterpErr::ExpectedBool { who, got: v, msg } => {
                write!(f, "{} expected a boolean, but got {} in {}", who, v, msg)
            }
            InterpErr::ExpectedFun { who, got: v, msg } => {
                write!(f, "{} expected a function, but got {} in {}", who, v, msg)
            }
            InterpErr::Overflow { msg } => write!(f, "Operation {} overflowed", msg),
            InterpErr::Write { msg } => write!(f, "I/O Error when printing: {}", msg),
            InterpErr::ArityErr {
                expected_arity,
                num_provided,
            } => {
                write!(
                    f,
                    "Function expecting {} arguments called with {} arguments",
                    expected_arity, num_provided
                )
            }
        }
    }
}

fn get<'l, T>(stk: &'l List<(String, T)>, x: &str) -> Option<&'l T> {
    match stk {
        List::Empty => None,
        List::Cons((y, n), stk) => {
            if x == *y {
                Some(n)
            } else {
                get(stk, x)
            }
        }
    }
}

fn bool(v: SnakeVal, who: &str, msg: &str) -> Interp<bool> {
    match v {
        SnakeVal::Bool(b) => Ok(b),
        _ => Err(InterpErr::ExpectedBool {
            who: String::from(who),
            got: v.to_string(),
            msg: String::from(msg),
        }),
    }
}

fn num(v: SnakeVal, who: &str, msg: &str) -> Interp<i64> {
    match v {
        SnakeVal::Num(n) => Ok(n),
        _ => Err(InterpErr::ExpectedNum {
            who: String::from(who),
            got: v.to_string(),
            msg: String::from(msg),
        }),
    }
}

fn print_snake_val<'e, W>(w: &mut W, v: SnakeVal, _h: &Heap) -> Interp<SnakeVal>
where
    W: std::io::Write,
{
    fn fixup_err(e: std::io::Error) -> InterpErr {
        InterpErr::Write { msg: e.to_string() }
    }
    fn print_loop<'e, W>(
        w: &mut W,
        v: &SnakeVal,
    ) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        match v {
            SnakeVal::Num(n) => write!(w, "{}", n)?,
            SnakeVal::Bool(b) => write!(w, "{}", b)?,
            SnakeVal::Float(n) => write!(w, "{}", n)?,
            SnakeVal::Closure { .. } => {
                write!(w, "<closure>")?;
            }
        }
        Ok(())
    }

    print_loop(w, &v).map_err(fixup_err)?;
    writeln!(w).map_err(fixup_err)?;
    Ok(v)
}

fn equal_snake_val(v1: &SnakeVal, v2: &SnakeVal, _h: &Heap) -> bool {
    fn eq_loop(
        v1: &SnakeVal,
        v2: &SnakeVal,
    ) -> bool {
        match (v1, v2) {
            (SnakeVal::Bool(b1), SnakeVal::Bool(b2)) => b1 == b2,
            (SnakeVal::Num(n1), SnakeVal::Num(n2)) => n1 == n2,
            _ => false,
        }
    }
    eq_loop(v1, v2)
}

fn interpret_prim1<W>(p: &Prim, w: &mut W, v: SnakeVal, h: &Heap) -> Interp<SnakeVal>
where
    W: std::io::Write,
{
    match p {
        Prim::Add1 => snake_arith(v, SnakeVal::Num(1), |n1, n2| n1.overflowing_add(n2), "add1"),
        Prim::Sub1 => snake_arith(v, SnakeVal::Num(1), |n1, n2| n1.overflowing_sub(n2), "sub1"),
        Prim::Not => Ok(SnakeVal::Bool(!bool(v, "logic", "!")?)),
        Prim::Print => print_snake_val(w, v, h),
        Prim::IsBool => match v {
            SnakeVal::Bool(_) => Ok(SnakeVal::Bool(true)),
            _ => Ok(SnakeVal::Bool(false)),
        },
        Prim::IsNum => match v {
            SnakeVal::Num(_) => Ok(SnakeVal::Bool(true)),
            _ => Ok(SnakeVal::Bool(false)),
        },
        _ => unreachable!(),
    }
}

static MAX_INT: i64 = 2i64.pow(62) - 1;
static MIN_INT: i64 = -(2i64.pow(62));
fn out_of_bounds(n: i64) -> bool {
    n > MAX_INT || n < MIN_INT
}

fn snake_arith<F>(v1: SnakeVal, v2: SnakeVal, arith: F, op: &str) -> Interp<SnakeVal>
where
    F: Fn(i64, i64) -> (i64, bool),
{
    let n1 = num(v1, "arithmetic", op)?;
    let n2 = num(v2, "arithmetic", op)?;
    let (n3, overflow) = arith(n1, n2);
    if overflow || out_of_bounds(n3) {
        Err(InterpErr::Overflow {
            msg: format!("{} {} {} = {}", n1, op, n2, n3),
        })
    } else {
        Ok(SnakeVal::Num(n3))
    }
}

fn snake_log<F>(v1: SnakeVal, v2: SnakeVal, log: F, op: &str) -> Interp<SnakeVal>
where
    F: Fn(bool, bool) -> bool,
{
    Ok(SnakeVal::Bool(log(
        bool(v1, "logic", op)?,
        bool(v2, "logic", op)?,
    )))
}

fn snake_cmp<F>(v1: SnakeVal, v2: SnakeVal, cmp: F, op: &str) -> Interp<SnakeVal>
where
    F: Fn(i64, i64) -> bool,
{
    Ok(SnakeVal::Bool(cmp(
        num(v1, "comparison", op)?,
        num(v2, "comparison", op)?,
    )))
}

fn interpret_prim2(p: &Prim, v1: SnakeVal, v2: SnakeVal, heap: &Heap) -> Interp<SnakeVal>
where
{
    match p {
        Prim::Add => snake_arith(v1, v2, |n1, n2| n1.overflowing_add(n2), "+"),
        Prim::Sub => snake_arith(v1, v2, |n1, n2| n1.overflowing_sub(n2), "-"),
        Prim::Mul => snake_arith(v1, v2, |n1, n2| n1.overflowing_mul(n2), "*"),

        Prim::And => snake_log(v1, v2, |b1, b2| b1 && b2, "&&"),
        Prim::Or => snake_log(v1, v2, |b1, b2| b1 || b2, "||"),

        Prim::Lt => snake_cmp(v1, v2, |n1, n2| n1 < n2, "<"),
        Prim::Le => snake_cmp(v1, v2, |n1, n2| n1 <= n2, "<="),
        Prim::Gt => snake_cmp(v1, v2, |n1, n2| n1 > n2, ">"),
        Prim::Ge => snake_cmp(v1, v2, |n1, n2| n1 >= n2, ">="),

        Prim::Eq => Ok(SnakeVal::Bool(equal_snake_val(&v1, &v2, heap))),
        Prim::Neq => Ok(SnakeVal::Bool(!equal_snake_val(&v1, &v2, heap))),
        _ => unreachable!(),
    }
}

impl<A> std::iter::FromIterator<A> for List<A> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = A>,
    {
        let mut l = List::Empty;
        for t in iter {
            l = List::Cons(t, Rc::new(l));
        }
        l
    }
}

// Abstract machine
enum Machine<'exp, Ann> {
    Descending {
        e: &'exp Exp<Ann>,
        stk: Stack<'exp, Ann>,
        env: Env,
    },
    Returning {
        v: SnakeVal,
        stk: Stack<'exp, Ann>,
    },
}

enum Stack<'exp, Ann> {
    Done,
    Prim1(Prim, Box<Stack<'exp, Ann>>),
    Prim2L(Prim, Closure<'exp, Ann>, Box<Stack<'exp, Ann>>),
    Prim2R(Prim, SnakeVal, Box<Stack<'exp, Ann>>),
    If {
        thn: &'exp Exp<Ann>,
        els: &'exp Exp<Ann>,
        env: Env,
        stk: Box<Stack<'exp, Ann>>,
    },
    Let {
        var: &'exp str,
        env: Env,
        bindings: Vec<&'exp (String, Exp<Ann>)>,
        body: &'exp Exp<Ann>,
        stk: Box<Stack<'exp, Ann>>,
    },
    CallArgs {
        fun: usize, // the closure
        evaled_args: Vec<SnakeVal>,
        env: Env,
        remaining_args: Vec<&'exp Exp<Ann>>,
        stk: Box<Stack<'exp, Ann>>,
    },
}

// No heap for now!
type Heap = ();
type Funs<'e, Ann> = Vec<(Env, &'e SurfFunDecl<Ann>)>;
struct State<'e, Ann> {
    funs: Funs<'e, Ann>,
    heap: Heap,
}

impl<'e, Ann> State<'e, Ann> {
    fn new() -> Self {
        State {
            funs: vec![],
            heap: (),
        }
    }
}

/*
 *  Abstract machine-style interpreter.
 *
 *  Defunctionalizes the kontinuation of the direct-style interpreter
 *  so that we don't blow the Rust stack/rely on Rust TCE.
 *
*/
fn machine<'exp, Ann, W>(e: &'exp Exp<Ann>, buf: &mut W, store: &mut State<'exp, Ann>) -> Interp<()>
where
    W: std::io::Write,
    Ann: Clone,
{
    fn call<'exp, Ann>(
        fun: usize,
        args: Vec<SnakeVal>,
        stk: Stack<'exp, Ann>,
        funs: &Funs<'exp, Ann>,
    ) -> Interp<Machine<'exp, Ann>>
    where
        Ann: Clone,
    {
        let (orig_env, d) = &funs[fun];
	let mut env = orig_env.clone();

        if args.len() != d.parameters.len() {
            return Err(InterpErr::ArityErr {
                expected_arity: d.parameters.len(),
                num_provided: args.len(),
            });
        }
	// environment for the body should consist of the captured env
	// extended with the new parameters
	for (v, x) in args.iter().zip(d.parameters.iter()) {
	    env = push_local(&env, x.to_string(), v.clone())
	}
	Ok(Machine::Descending {
            e: &d.body,
            env,
            stk,
        })
    }

    fn mangle_fun_name(s: &str) -> String {
	format!("{}#fun", s)
    }
    
    // Allocate closures for a mutually recursive sequence of function
    // declarations, returning an environment where the functions'
    // names (mangled to avoid clashing with value variables) are
    // associated to the address of their closure.
    fn alloc_funs<'e, Ann>(decls: &'e [SurfFunDecl<Ann>], mut env: Env, funs: &mut Funs<'e, Ann>) -> Env {
	// Each of the closures captures the same environment: the
	// current environment extended with all of their names
	// i.e., the env we return.
	let i = funs.len();
	for (j, d) in decls.iter().enumerate() {
	    env = push_local(&env, mangle_fun_name(&d.name), SnakeVal::Closure(i + j));
	}
	for d in decls.iter() {
	    funs.push((env.clone(), &d));
	}
	env
    }

    let mut machine = Machine::Descending {
        e,
        stk: Stack::Done,
        env: Rc::new(List::Empty),
    };
    loop {
        match machine {
            Machine::Descending { e, stk, env } => match e {
                Exp::InternalTailCall(..) | Exp::ExternalCall { .. } => {
                    panic!("Should never happen: interpreter called with internal compiler forms")
                }
                Exp::Num(n, _) => {
                    machine = Machine::Returning {
                        v: SnakeVal::Num(*n),
                        stk,
                    }
                }
                Exp::Float(FloatWrapper(f), _) => {
                    machine = Machine::Returning {
                        v: SnakeVal::Float(*f),
                        stk,
                    }
                }
                Exp::Bool(b, _) => {
                    machine = Machine::Returning {
                        v: SnakeVal::Bool(*b),
                        stk,
                    }
                }
                Exp::Var(x, _) => {
                    let v = get(&*env, x).expect("Unbound variable in interpreter! You should catch this in the check function!");
                    machine = Machine::Returning { v: v.clone(), stk }
                }

                Exp::Prim(op, es, _) => {
                    match op {
                        Prim::Add1 | Prim::Sub1 | Prim::Not |
                        Prim::Print | Prim::IsBool |
                        Prim::IsNum | Prim::IsFloat | Prim::Sqrt | Prim::Cos => {
                            let e = &es[0];
                            machine = Machine::Descending {
                                e,
                                stk: Stack::Prim1(*op, Box::new(stk)),
                                env,
                            };
                        },
                        Prim::Add | Prim::Sub | Prim::Mul |
                        Prim::And | Prim::Or | Prim::Lt |
                        Prim::Gt | Prim::Le | Prim::Ge |
                        Prim::Eq | Prim::Neq | Prim::Div | Prim::FloorDiv => {
                            let e1 = &es[0];
                            let e2 = &es[1];
                            machine = Machine::Descending {
                                e: e1,
                                stk: Stack::Prim2L(
                                    *op,
                                    Closure {
                                        exp: e2,
                                        env: env.clone(),
                                    },
                                    Box::new(stk),
                                ),
                                env,
                            };
                        }
                    }
                }
                Exp::Let { bindings, body, .. } => {
                    let mut rbindings: Vec<&(String, Exp<Ann>)> = bindings.iter().rev().collect();
                    match rbindings.pop() {
                        None => {
                            machine = Machine::Descending { e: body, stk, env };
                        }
                        Some((var, e)) => {
                            machine = Machine::Descending {
                                e,
                                stk: Stack::Let {
                                    var,
                                    env: env.clone(),
                                    bindings: rbindings,
                                    body,
                                    stk: Box::new(stk),
                                },
                                env,
                            };
                        }
                    }
                }
                Exp::If { cond, thn, els, .. } => {
                    machine = Machine::Descending {
                        e: cond,
                        stk: Stack::If {
                            thn,
                            els,
                            env: env.clone(),
                            stk: Box::new(stk),
                        },
                        env,
                    }
                }
                Exp::Call(fun, args, _) => {
		    let ix = match get(&*env, &mangle_fun_name(fun)) {
			Some(SnakeVal::Closure(ix)) => *ix,
			_ => panic!("bug in interpreter?"),
		    };
                    let mut remaining_args: Vec<&Exp<_>> = args.iter().collect();
                    remaining_args.reverse();
                    match remaining_args.pop() {
                        None => {
                            machine =
                                call(ix, Vec::new(), stk, &store.funs)?;
                        }
                        Some(e) => {
                            machine = Machine::Descending {
                                e: &e,
                                env: env.clone(),
                                stk: Stack::CallArgs {
                                    fun: ix,
                                    evaled_args: Vec::new(),
                                    env,
                                    remaining_args,
                                    stk: Box::new(stk),
                                },
                            }
                        }
                    }
                }
		Exp::FunDefs { decls, body, .. } => {
		    let env = alloc_funs(&decls, env, &mut store.funs);
		    machine = Machine::Descending {
			e: &body,
			env: env.clone(),
			stk			
		    }
		    
		}
            },
            Machine::Returning { v, stk } => match stk {
                Stack::Done => {
                    print_snake_val(buf, v, &store.heap)?;
                    return Ok(());
                }
                Stack::Prim1(op, stk) => {
                    let v = interpret_prim1(&op, buf, v, &store.heap)?;
                    machine = Machine::Returning { v, stk: *stk }
                }
                Stack::Prim2L(op, r, stk) => {
                    machine = Machine::Descending {
                        e: r.exp,
                        env: r.env,
                        stk: Stack::Prim2R(op, v, stk),
                    };
                }
                Stack::Prim2R(op, vl, stk) => {
                    let v = interpret_prim2(&op, vl, v, &store.heap)?;
                    machine = Machine::Returning { v, stk: *stk };
                }
                Stack::Let {
                    var,
                    mut env,
                    mut bindings,
                    body,
                    stk,
                } => {
                    env = push_local(&env, var.to_string(), v);
                    machine = match bindings.pop() {
                        None => Machine::Descending {
                            e: body,
                            env,
                            stk: *stk,
                        },
                        Some((var, e)) => Machine::Descending {
                            e,
                            stk: Stack::Let {
                                var,
                                env: env.clone(),
                                bindings,
                                body,
                                stk,
                            },
                            env,
                        },
                    }
                }

                Stack::If { thn, els, env, stk } => {
                    let e = if bool(v, "if", "if")? { thn } else { els };
                    machine = Machine::Descending { e, env, stk: *stk }
                }
                Stack::CallArgs {
                    fun: fun_v,
                    mut evaled_args,
                    env,
                    mut remaining_args,
                    stk,
                } => {
                    evaled_args.push(v);
                    match remaining_args.pop() {
                        None => {
                            machine = call(
                                fun_v,
                                evaled_args,
                                *stk,
                                &store.funs,
                            )?;
                        }
                        Some(e) => {
                            machine = Machine::Descending {
                                e,
                                env: env.clone(),
                                stk: Stack::CallArgs {
                                    fun: fun_v,
                                    evaled_args,
                                    env,
                                    remaining_args,
                                    stk,
                                },
                            }
                        }
                    }
                }
            },
        }
    }
}

// Runs the reference interpreter.
pub fn exp<Ann, W>(e: &Exp<Ann>, w: &mut W) -> Interp<()>
where
    Ann: Clone,
    W: std::io::Write,
{
    machine(e, w, &mut State::new())
}

pub fn prog<Ann, W>(p: &SurfProg<Ann>, w: &mut W) -> Interp<()>
where
    W: std::io::Write,
    Ann: Clone,
{
    machine(&p, w, &mut State::new())
}
