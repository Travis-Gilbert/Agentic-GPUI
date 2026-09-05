//! Check a DTCG token document against the theme law.
//!
//!   theme_check <tokens.json>
//!
//! The document is an argument rather than an embedded asset, because the law
//! lives in AGPUI and the documents live in the products that own them. This
//! is what SPEC-AGPUI-HOME-1.0 H7 means by the check still running on
//! generated roles from the Theorem tree: same binary, product's file.

use std::path::PathBuf;
use std::process::ExitCode;

use agpui_theme::{TokenSet, PROSE_CAPTURES};

fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: theme_check <tokens.json>");
        return ExitCode::FAILURE;
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };

    // Parsing is most of the check. `from_dtcg_str` refuses an authored
    // neutral hex, a dangling alias and a missing law decision, so a document
    // that parses has already cleared those three.
    let tokens = match TokenSet::from_dtcg_str(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            eprintln!("{}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let mut failures = Vec::new();
    let law = tokens.neutral_law();

    for path in [
        "color.cream.25",
        "color.cream.50",
        "color.cream.100",
        "color.cream.200",
        "color.cream.300",
        "color.cream.400",
        "color.cream.700",
        "color.cream.900",
        "color.ink.primary",
        "color.ink.muted",
        "color.ink.faint",
    ] {
        let Some(sample) = tokens.neutral_sample(path) else {
            failures.push(format!("{path}: not generated"));
            continue;
        };
        let relative = sample.chroma / sample.lightness;
        if relative > law.max_relative_chroma + 1e-7 {
            failures.push(format!(
                "{path}: relative chroma {relative} exceeds {}",
                law.max_relative_chroma
            ));
        }
    }

    for capture in PROSE_CAPTURES {
        if tokens.prose_highlight_style(capture).is_none() {
            failures.push(format!("prose capture {capture}: unresolved"));
        }
    }

    if failures.is_empty() {
        println!("ok {} ({} generated roles)", path.display(), 11);
        ExitCode::SUCCESS
    } else {
        for failure in &failures {
            eprintln!("FAIL {failure}");
        }
        ExitCode::FAILURE
    }
}
