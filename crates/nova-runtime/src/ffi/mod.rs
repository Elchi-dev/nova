// Nova Foreign Function Interface
//
// Allows importing C (and later C++, Rust) libraries directly:
//
//   import foreign("raylib.h", lang: "c")
//
// The compiler parses the C header, generates Nova-compatible
// bindings, and links the library at compile time. No manual
// wrapper code needed.

/// Placeholder — FFI implementation coming soon
pub fn load_foreign_header(_path: &str, _lang: &str) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
