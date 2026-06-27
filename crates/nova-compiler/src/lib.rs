pub mod error;

pub use error::CompileError;

use nova_parser::{parse, Program};
use nova_typechecker::{type_check, Env};
use std::path::Path;

/// The result of a full compilation run.
pub struct CompileResult {
    pub program: Program,
    pub errors: Vec<CompileError>,
    /// Compiled artifact path (`None` if errors prevented codegen).
    pub output: Option<std::path::PathBuf>,
}

impl CompileResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Compile a Nova source file to a native binary.
///
/// Pipeline:
///   Source → Lexer → Parser → Type Checker → MIR → LLVM IR → Binary
///
/// Steps 4 and 5 (MIR lowering and LLVM codegen) are stubs — they will
/// be implemented in `nova-codegen` once the type checker is complete.
pub fn compile(source_path: &Path, output_path: &Path) -> CompileResult {
    // ── 1. Read source ────────────────────────────────────────────────────────
    let source = match std::fs::read_to_string(source_path) {
        Ok(s) => s,
        Err(e) => {
            return CompileResult {
                program: Program {
                    items: vec![],
                    source_file: source_path.display().to_string(),
                },
                errors: vec![CompileError::Io(e.to_string())],
                output: None,
            };
        }
    };

    let filename = source_path.display().to_string();

    // ── 2. Parse ──────────────────────────────────────────────────────────────
    let (program, parse_errors) = parse(&source, &filename);

    let mut errors: Vec<CompileError> = parse_errors.into_iter().map(CompileError::Parse).collect();

    if !errors.is_empty() {
        return CompileResult {
            program,
            errors,
            output: None,
        };
    }

    // ── 3. Type check ─────────────────────────────────────────────────────────
    let mut env = Env::with_builtins();
    let tc_result = type_check(&program, &mut env);

    if !tc_result.is_ok() {
        // Render rich diagnostics to stderr.
        tc_result.render_errors(&filename, &source);

        errors.extend(tc_result.errors.into_iter().map(CompileError::Type));
        return CompileResult {
            program,
            errors,
            output: None,
        };
    }

    // ── 4. MIR Lowering (TODO: nova-mir crate) ────────────────────────────────
    // ── 5. LLVM Codegen (TODO: nova-codegen crate via inkwell) ───────────────

    CompileResult {
        program,
        errors: vec![],
        output: Some(output_path.to_path_buf()),
    }
}
