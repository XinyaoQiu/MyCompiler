#[repr(C)]
#[derive(PartialEq, Eq, Copy, Clone)]
struct SnakeVal(u64);

static TAG_MASK: u64 = 0x00_00_00_00_00_00_00_01;
static SNAKE_TRU: SnakeVal = SnakeVal(0xFF_FF_FF_FF_FF_FF_FF_FF);
static SNAKE_FLS: SnakeVal = SnakeVal(0x7F_FF_FF_FF_FF_FF_FF_FF);

type ErrorCode = u64;
static ARITH_ERROR: ErrorCode = 0;
static COMP_ERROR: ErrorCode = 1;
static OVERFLOW_ERROR: ErrorCode = 2;
static LOGIC_ERROR: ErrorCode = 3;
static IF_ERROR: ErrorCode = 4;
static DIVISION_ERROR: ErrorCode = 5;
static SQRT_ERROR: ErrorCode = 6;

#[link(name = "compiled_code", kind = "static")]
extern "sysv64" {

    // The \x01 here is an undocumented feature of LLVM that ensures
    // it does not add an underscore in front of the name.
    #[link_name = "\x01start_here"]
    fn start_here() -> SnakeVal;
}

// reinterprets the bytes of an unsigned number to a signed number
fn unsigned_to_signed(x: u64) -> i64 {
    i64::from_le_bytes(x.to_le_bytes())
}

fn u64_to_u8_array(value: u64) -> [u8; 8] {
    let byte0 = ((value >> 56) & 0xFF) as u8;
    let byte1 = ((value >> 48) & 0xFF) as u8;
    let byte2 = ((value >> 40) & 0xFF) as u8;
    let byte3 = ((value >> 32) & 0xFF) as u8;
    let byte4 = ((value >> 24) & 0xFF) as u8;
    let byte5 = ((value >> 16) & 0xFF) as u8;
    let byte6 = ((value >> 8) & 0xFF) as u8;
    let byte7 = (value & 0xFF) as u8;

    [byte0, byte1, byte2, byte3, byte4, byte5, byte6, byte7]
}

fn sprint_snake_val(x: SnakeVal) -> String {
    if x.0 & TAG_MASK == 0 {
        // it's a number
        format!("{}", unsigned_to_signed(x.0) >> 1)
    } else if x == SNAKE_TRU {
        String::from("true")
    } else if x == SNAKE_FLS {
        String::from("false")
    } else if x.0 & 3 == 1 {
        // it's a float
        format!("{}", f64::from_be_bytes(u64_to_u8_array(x.0 - 1)) as f32)
    } else {
        format!("error: cannot print {}", x.0)
    }
}

#[export_name = "\x01print_snake_val"]
extern "sysv64" fn print_snake_val(v: SnakeVal) -> SnakeVal {
    println!("{}", sprint_snake_val(v));
    v
}

/* Implement the following error function. You are free to change the
 * input and output types as needed for your design.
 *
**/
#[export_name = "\x01snake_error"]
extern "sysv64" fn snake_error(err_code: u64, v: SnakeVal) {
    if err_code == ARITH_ERROR {
        eprintln!(
            "arithmetic expected a number or float, but got {}",
            sprint_snake_val(v)
        );
    } else if err_code == COMP_ERROR {
        eprintln!(
            "comparison expected a number or float, but got {}",
            sprint_snake_val(v)
        );
    } else if err_code == OVERFLOW_ERROR {
        eprintln!("overflow");
    } else if err_code == LOGIC_ERROR {
        eprintln!("logic expected a boolean, but got {}", sprint_snake_val(v));
    } else if err_code == IF_ERROR {
        eprintln!("if expected a boolean, but got {}", sprint_snake_val(v));
    } else if err_code == DIVISION_ERROR {
        eprintln!("division by zero");
    } else if err_code == SQRT_ERROR {
        eprintln!("sqrt expected a non-negative value");
    }
    
    else {
        eprintln!(
            "I apologize to you, dear user. I made a bug. Here's a snake value: {}",
            sprint_snake_val(v)
        );
    }
    std::process::exit(1);
}

fn main() {
    let output = unsafe { start_here() };
    println!("{}", sprint_snake_val(output));
}
