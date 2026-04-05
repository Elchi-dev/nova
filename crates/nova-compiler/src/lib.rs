pub mod error;

pub use error::CompileError;

use nova_parser::{parse, Program};
use std::path::Path;

/// The result of a full compilation run.
pub struct CompileResult {
    pub program: Program,
    pub errors: Vec<CompileError>,
    /// Compiled artifact path (None if errors prevented codegen)
    pub output: Option<std::path::PathBuf>,
}

/// Compile a Nova source file to a native binary.
///
/// Pipeline:
///   Source → Lexer → Parser → Type Checker → MIR → LLVM IR → Binary
///
/// Each stage is a separate crate. This function orchestrates them.
pub fn compile(source_path: &Path, output_path: &Path) -> CompileResult {
    // ── 1. Read source ────────────────────────────────────────────────────────
    let source = match std::fs::read_to_string(source_path) {
        Ok(s) => s,
        Err(e) => {
            return CompileResult {
                program: Program { items: vec![], source_file: source_path.display().to_string() },
                errors: vec![CompileError::Io(e.to_string())],
                output: None,
            };
        }
    };

    let filename = source_path.display().to_string();

    // ── 2. Parse ──────────────────────────────────────────────────────────────
    let (program, parse_errors) = parse(&source, &filename);

    let errors: Vec<CompileError> = parse_errors
        .into_iter()
        .map(CompileError::Parse)
        .collect();

    if !errors.is_empty() {
        return CompileResult { program, errors, output: None };
    }

    // ── 3. Type Check  (TODO: nova-typechecker crate) ─────────────────────────
    // ── 4. MIR Lowering (TODO: nova-mir crate) ────────────────────────────────
    // ── 5. LLVM Codegen (TODO: nova-codegen crate via inkwell) ───────────────

    CompileResult {
        program,
        errors: vec![],
        output: Some(output_path.to_path_buf()),
    }
}
