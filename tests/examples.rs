use snake::runner;

macro_rules! mk_test {
    ($test_name:ident, $file_name:expr, $expected_output:expr) => {
        #[test]
        fn $test_name() -> std::io::Result<()> {
            test_example_file($file_name, $expected_output)
        }
    };
}

macro_rules! mk_fail_test {
    ($test_name:ident, $file_name:expr, $expected_output:expr) => {
        #[test]
        fn $test_name() -> std::io::Result<()> {
            test_example_fail($file_name, $expected_output)
        }
    };
}

fn test_two_floats(str1: &str, str2: &str)  {
    match str1.parse::<f64>() {
        Ok(value1) => match str2.parse::<f64>() {
            Ok(value2) => {
                assert!(((value1 - value2).abs() as f32) <= f32::EPSILON);
                
            }
            Err(_) => {assert!(str1 == str2)},
        },
        Err(_) => {assert!(str1 == str2)},
    }
}

fn test_two_nums(str1: &str, str2: &str)  {
    match str1.parse::<u64>() {
        Ok(value1) => match str2.parse::<u64>() {
            Ok(value2) => assert!(value1 == value2),
            Err(_) => {},
        },
        Err(_) => {},
    }
}

// IMPLEMENTATION
fn test_example_file(f: &str, expected_str: &str) -> std::io::Result<()> {
    use std::path::Path;
    let p_name = format!("examples/{}", f);
    let path = Path::new(&p_name);

    // Test the compiler
    let tmp_dir = tempfile::TempDir::new()?;
    let mut w = Vec::new();
    match runner::compile_and_run_file(&path, tmp_dir.path(), &mut w) {
        Ok(()) => {
            let stdout = std::str::from_utf8(&w).unwrap();
            let str1 = stdout.trim();
            let str2 = expected_str;
            println!("Expected {}, got {}", str2, str1);
            test_two_nums(str1, str2) ;
            test_two_floats(str1, str2) ;
            
            
        }
        Err(e) => {
            assert!(false, "Expected {}, got an error: {}", expected_str, e)
        }
    }

    Ok(())
}

fn test_example_fail(f: &str, includes: &str) -> std::io::Result<()> {
    use std::path::Path;

    // Test the compiler
    let tmp_dir = tempfile::TempDir::new()?;
    let mut w_run = Vec::new();
    match runner::compile_and_run_file(
        &Path::new(&format!("examples/{}", f)),
        tmp_dir.path(),
        &mut w_run,
    ) {
        Ok(()) => {
            let stdout = std::str::from_utf8(&w_run).unwrap();
            assert!(false, "Expected a failure but got: {}", stdout.trim())
        }
        Err(e) => {
            let msg = format!("{}", e);
            assert!(
                msg.contains(includes),
                "Expected error message to include the string \"{}\" but got the error: {}",
                includes,
                msg
            )
        }
    }

    Ok(())
}

/* Diamondback tests */
mk_fail_test!(
    addbool,
    "../examples/addbool.cobra",
    "arithmetic expected a number"
);
mk_test!(basic_print, "../examples/basic_print.cobra", "1\n1");
mk_fail_test!(
    dulicate,
    "../examples/duplicate.adder",
    "Variable y defined twice"
);
mk_test!(foo, "../examples/foo.diamondback", "6");
mk_test!(mul, "../examples/mul.cobra", "20\n20");
mk_fail_test!(overflow, "../examples/overflow.cobra", "overflow");
mk_fail_test!(wrong_if, "../examples/wrong_if.boa", "if expected a bool");
mk_test!(simple_let0, "../examples/simple_let0.adder", "false");
mk_test!(simple_let1, "../examples/simple_let1.adder", "1");
mk_test!(simple_let2, "../examples/simple_let2.adder", "13");
mk_test!(simple_let4, "../examples/simple_let4.adder", "11");
mk_fail_test!(unbound, "../examples/unbound.adder", "Unbound variable b");
mk_test!(recursion, "../examples/recursion.diamondback", "true");
mk_test!(isnum1, "../examples/test_isnum1.diamondback", "true");
mk_test!(isbool, "../examples/test_isbool.diamondback", "true");

/* Garter tests */
mk_test!(simple_float, "../examples/simple_float.garter", "3.6");
mk_test!(func_arg_float, "../examples/func_arg_float.garter", "2.1");
mk_fail_test!(float_overflow1, "../examples/float_overflow1.garter", "overflow");
mk_fail_test!(float_overflow2, "../examples/float_overflow2.garter", "overflow");

mk_test!(float_underflow1, "../examples/float_underflow1.garter", "0.0");
mk_test!(float_logic1, "../examples/float_logic1.garter", "true");
mk_test!(simple_float_minus, "../examples/simple_float_minus.garter", "-0.6");
mk_test!(cos_and_sqrt, "../examples/cos_and_sqrt.garter", "11.937541");
mk_fail_test!(wrong_sqrt, "../examples/wrong_sqrt.garter", "sqrt expected a non-negative value");
mk_fail_test!(division_by_zero, "../examples/division_by_zero.garter", "division by zero");
mk_test!(add, "../examples/add.garter", "2.5");
mk_fail_test!(addbool2, "../examples/addbool.garter", "arithmetic expected a number or float");
mk_test!(basic_print2, "../examples/basic_print.garter", "10.5\n10.5");
mk_test!(basic, "../examples/basic.garter", "true");
mk_fail_test!(divide0, "../examples/divide0.garter", "division by zero");
mk_fail_test!(divide00, "../examples/divide00.garter", "division by zero");
mk_fail_test!(divide000, "../examples/divide000.garter", "division by zero");
mk_fail_test!(divide0000, "../examples/divide0000.garter", "division by zero");
mk_test!(bool, "../examples/bool.garter", "false");
mk_test!(divide, "../examples/divide.garter", "0.75");
mk_fail_test!(dulicate2, "../examples/duplicate.garter", "Variable y defined twice");
mk_test!(equal, "../examples/equal.garter", "true");
mk_test!(foo2, "../examples/foo.garter", "7.88");
mk_fail_test!(overflow1, "../examples/overflow.garter", "overflow");
mk_fail_test!(overflow2, "../examples/overflow2.garter", "overflow");
mk_test!(recursion2, "../examples/recursion.garter", "true");
mk_test!(underflow, "../examples/underflow.garter", "0.0");
mk_test!(underflow2, "../examples/underflow2.garter", "0.0");
mk_test!(isfloat1, "../examples/test_isfloat1.garter", "true");
mk_test!(isfloat2, "../examples/test_isfloat2.garter", "false");
mk_test!(isboolnum, "../examples/test_isboolnum.garter", "false");
mk_test!(floor_division, "../examples/floor_division.garter", "3");
mk_test!(test_float_eq, "../examples/test_float_eq.garter", "true");
mk_test!(test_float_neq, "../examples/test_float_neq.garter", "false");
mk_fail_test!(arith_error, "../examples/arith_error.garter", "arithmetic expected a number or float");
mk_fail_test!(wrong_if1, "../examples/logic_error.garter", "if expected a bool");
mk_fail_test!(comp_error, "../examples/comp_error.garter", "comparison expected a number or float");
mk_test!(func, "../examples/func.garter", "430");
mk_test!(func2, "../examples/func2.garter", "14.4742565"); 