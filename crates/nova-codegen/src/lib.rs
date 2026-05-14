pub mod codegen;
pub mod error;

pub use codegen::Codegen;
pub use error::{CodegenError, CodegenResult};

// Re-export inkwell so dependents don't need it as a direct dep
pub use inkwell;
