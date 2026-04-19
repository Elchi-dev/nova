pub mod env;
pub mod eval;
pub mod value;

use crate::ast::Program;
use eval::Interpreter;
use value::RuntimeError;

/// Execute a Nova program, returning captured output lines.
pub fn run(program: &Program) -> Result<Vec<String>, RuntimeError> {
    let mut interp = Interpreter::new();
    interp.execute(program)?;
    Ok(interp.output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use crate::parser;

    fn run_source(source: &str) -> Result<Vec<String>, String> {
        let tokens = lexer::tokenize(source).map_err(|e| e.to_string())?;
        let program = parser::parse(tokens).map_err(|e| e.to_string())?;
        run(&program).map_err(|e| e.to_string())
    }

    fn run_ok(source: &str) -> Vec<String> {
        run_source(source).expect("program should succeed")
    }

    // ── Basic output ─────────────────────────────────────────

    #[test]
    fn test_hello_world() {
        let output = run_ok("print(\"Hello, World!\")");
        assert_eq!(output, vec!["Hello, World!"]);
    }

    #[test]
    fn test_print_integer() {
        let output = run_ok("print(42)");
        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn test_print_multiple() {
        let output = run_ok("print(\"a\")\nprint(\"b\")\nprint(\"c\")");
        assert_eq!(output, vec!["a", "b", "c"]);
    }

    // ── Variables ────────────────────────────────────────────

    #[test]
    fn test_let_binding() {
        let output = run_ok("let x: int = 42\nprint(x)");
        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn test_mutable_variable() {
        let output = run_ok("let mut x: int = 1\nx = 2\nprint(x)");
        assert_eq!(output, vec!["2"]);
    }

    #[test]
    fn test_immutable_fails() {
        let result = run_source("let x: int = 1\nx = 2");
        assert!(result.is_err());
    }

    // ── Arithmetic ───────────────────────────────────────────

    #[test]
    fn test_arithmetic() {
        let output = run_ok("print(2 + 3)\nprint(10 - 4)\nprint(3 * 7)\nprint(10 // 3)");
        assert_eq!(output, vec!["5", "6", "21", "3"]);
    }

    #[test]
    fn test_float_arithmetic() {
        let output = run_ok("print(3.14 + 2.86)");
        assert_eq!(output, vec!["6.0"]);
    }

    #[test]
    fn test_string_concat() {
        let output = run_ok("print(\"hello\" + \" \" + \"world\")");
        assert_eq!(output, vec!["hello world"]);
    }

    #[test]
    fn test_power() {
        let output = run_ok("print(2 ** 10)");
        assert_eq!(output, vec!["1024"]);
    }

    // ── Comparisons & Logic ──────────────────────────────────

    #[test]
    fn test_comparison() {
        let output = run_ok("print(1 < 2)\nprint(5 > 3)\nprint(2 == 2)\nprint(1 != 2)");
        assert_eq!(output, vec!["true", "true", "true", "true"]);
    }

    #[test]
    fn test_logic() {
        let output = run_ok("print(true and false)\nprint(true or false)\nprint(not true)");
        assert_eq!(output, vec!["false", "true", "false"]);
    }

    // ── If/Elif/Else ─────────────────────────────────────────

    #[test]
    fn test_if_true() {
        let output = run_ok("if true:\n    print(\"yes\")");
        assert_eq!(output, vec!["yes"]);
    }

    #[test]
    fn test_if_else() {
        let output = run_ok("if false:\n    print(\"no\")\nelse:\n    print(\"yes\")");
        assert_eq!(output, vec!["yes"]);
    }

    #[test]
    fn test_elif() {
        let output = run_ok(
            "let x: int = 2\nif x == 1:\n    print(\"one\")\nelif x == 2:\n    print(\"two\")\nelse:\n    print(\"other\")",
        );
        assert_eq!(output, vec!["two"]);
    }

    // ── Loops ────────────────────────────────────────────────

    #[test]
    fn test_while_loop() {
        let output = run_ok("let mut i: int = 0\nwhile i < 3:\n    print(i)\n    i = i + 1");
        assert_eq!(output, vec!["0", "1", "2"]);
    }

    #[test]
    fn test_for_loop() {
        let output = run_ok("for x in [10, 20, 30]:\n    print(x)");
        assert_eq!(output, vec!["10", "20", "30"]);
    }

    #[test]
    fn test_break() {
        let output = run_ok(
            "let mut i: int = 0\nwhile true:\n    if i == 3:\n        break\n    print(i)\n    i = i + 1",
        );
        assert_eq!(output, vec!["0", "1", "2"]);
    }

    #[test]
    fn test_for_with_range() {
        let output = run_ok("for i in range(4):\n    print(i)");
        assert_eq!(output, vec!["0", "1", "2", "3"]);
    }

    // ── Functions ────────────────────────────────────────────

    #[test]
    fn test_function_def_and_call() {
        let output = run_ok(
            "fn greet(name: str) -> str:\n    return \"Hello, \" + name\nprint(greet(\"Nova\"))",
        );
        assert_eq!(output, vec!["Hello, Nova"]);
    }

    #[test]
    fn test_recursive_function() {
        let output = run_ok(
            "fn fib(n: int) -> int:\n    if n <= 1:\n        return n\n    return fib(n - 1) + fib(n - 2)\nprint(fib(10))",
        );
        assert_eq!(output, vec!["55"]);
    }

    #[test]
    fn test_main_function() {
        let output = run_ok("fn main():\n    print(\"from main\")");
        assert_eq!(output, vec!["from main"]);
    }

    // ── Lists ────────────────────────────────────────────────

    #[test]
    fn test_list_creation() {
        let output = run_ok("let items = [1, 2, 3]\nprint(items)");
        assert_eq!(output, vec!["[1, 2, 3]"]);
    }

    #[test]
    fn test_list_index() {
        let output = run_ok("let items = [10, 20, 30]\nprint(items[1])");
        assert_eq!(output, vec!["20"]);
    }

    #[test]
    fn test_list_len() {
        let output = run_ok("print(len([1, 2, 3, 4]))");
        assert_eq!(output, vec!["4"]);
    }

    // ── Pipes ────────────────────────────────────────────────

    #[test]
    fn test_pipe_sort() {
        let output = run_ok("let result = [3, 1, 2] |> sort\nprint(result)");
        assert_eq!(output, vec!["[1, 2, 3]"]);
    }

    #[test]
    fn test_pipe_sum() {
        let output = run_ok("let result = [1, 2, 3, 4] |> sum\nprint(result)");
        assert_eq!(output, vec!["10"]);
    }

    #[test]
    fn test_pipe_filter() {
        let output = run_ok("let result = [1, 2, 3, 4, 5] |> filter(x => x > 3)\nprint(result)");
        assert_eq!(output, vec!["[4, 5]"]);
    }

    #[test]
    fn test_pipe_map() {
        let output = run_ok("let result = [1, 2, 3] |> map(x => x * 10)\nprint(result)");
        assert_eq!(output, vec!["[10, 20, 30]"]);
    }

    #[test]
    fn test_pipe_chain() {
        let output =
            run_ok("let result = [5, 3, 1, 4, 2] |> filter(x => x > 2) |> sort\nprint(result)");
        assert_eq!(output, vec!["[3, 4, 5]"]);
    }

    // ── Structs ──────────────────────────────────────────────

    #[test]
    fn test_struct_creation() {
        let output = run_ok(
            "struct Point:\n    let x: int = 0\n    let y: int = 0\nlet p = Point { x: 3, y: 7 }\nprint(p.x)\nprint(p.y)",
        );
        assert_eq!(output, vec!["3", "7"]);
    }

    // ── String methods ───────────────────────────────────────

    #[test]
    fn test_string_methods() {
        let output =
            run_ok("print(\"hello\".upper())\nprint(\"WORLD\".lower())\nprint(\"  hi  \".trim())");
        assert_eq!(output, vec!["HELLO", "world", "hi"]);
    }

    #[test]
    fn test_string_contains() {
        let output = run_ok("print(\"hello world\".contains(\"world\"))");
        assert_eq!(output, vec!["true"]);
    }

    // ── Lambda ───────────────────────────────────────────────

    #[test]
    fn test_lambda() {
        let output = run_ok("let double = x => x * 2\nprint(double(5))");
        assert_eq!(output, vec!["10"]);
    }

    // ── Built-in functions ───────────────────────────────────

    #[test]
    fn test_abs() {
        let output = run_ok("print(abs(-42))");
        assert_eq!(output, vec!["42"]);
    }

    #[test]
    fn test_min_max() {
        let output = run_ok("print(min(3, 7))\nprint(max(3, 7))");
        assert_eq!(output, vec!["3", "7"]);
    }

    // ── Error handling ───────────────────────────────────────

    #[test]
    fn test_division_by_zero() {
        let result = run_source("print(1 // 0)");
        assert!(result.is_err());
    }

    #[test]
    fn test_index_out_of_bounds() {
        let result = run_source("let items = [1, 2]\nprint(items[5])");
        assert!(result.is_err());
    }
}
