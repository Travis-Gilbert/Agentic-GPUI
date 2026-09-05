use std::env;
use std::path::PathBuf;

use theorem_design_core::{bake_grain_png, TokenSet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: bake_grain <output.png>")?;
    let receipt = bake_grain_png(output, TokenSet::builtin().grain())?;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
