use std::env;
use std::fs;
use std::path::PathBuf;

use theorem_design_core::TokenSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: emit_css <output.css>")?;
    fs::write(output, TokenSet::builtin().emit_css())?;
    Ok(())
}
