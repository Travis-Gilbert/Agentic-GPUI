//! Build gate for the theme law, over a product's own documents.
//!
//!   theme_check <tokens.json>
//!
//! The document is an argument rather than an embedded asset, because the law
//! lives in AGPUI and the documents live in the products that own them. This
//! is what SPEC-AGPUI-HOME-1.0 H7 means by the check still running on
//! generated roles from the Theorem tree: same binary, product's file.
//!
//! Two halves. The token document is checked for what a DTCG file can get
//! wrong - a missing theme input, a contrast out of range, a dangling or
//! cyclic alias, an unresolved prose capture. The theme document is checked
//! for what a palette can get wrong: text that does not clear the APCA floor,
//! a high-lightness neutral that carries colour, a contrast control that moves
//! nothing, and status hues too close to tell apart. The palette for the
//! second half comes from the document itself: a DTCG file authors exactly the
//! three theme inputs and everything else derives, so the file names its own
//! palette and the gate never has to be told which one to assume.

use std::path::PathBuf;
use std::process::ExitCode;

use agpui_theme::{apca_contrast, CieLch, ThemeInput, TokenSet, PROSE_CAPTURES};

const MIN_TEXT_APCA: f32 = 38.0;

/// The narrowest two status hues may sit, in degrees.
const MIN_HUE_SEPARATION: f64 = 24.0;

fn main() -> ExitCode {
    let Some(tokens_path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: theme_check <tokens.json>");
        return ExitCode::FAILURE;
    };

    let mut failures = Vec::new();

    let source = match std::fs::read_to_string(&tokens_path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {}: {error}", tokens_path.display());
            return ExitCode::FAILURE;
        }
    };

    // Parsing is most of the token check. `from_dtcg_str` refuses a missing
    // theme input, a contrast outside 0..=100, and a dangling or cyclic alias,
    // so a document that parses has already cleared those.
    let tokens = match TokenSet::from_dtcg_str(&source) {
        Ok(tokens) => tokens,
        Err(error) => {
            eprintln!("{}: {error}", tokens_path.display());
            return ExitCode::FAILURE;
        }
    };

    check_prose_captures(&tokens, &mut failures);

    let input = tokens.theme_input();

    check_apca(input, &mut failures);
    check_neutral_chroma(input, &mut failures);
    check_contrast_sensitivity(input, &mut failures);
    check_hue_bands(input, &mut failures);

    if failures.is_empty() {
        println!("ok {}", tokens_path.display());
        ExitCode::SUCCESS
    } else {
        for failure in &failures {
            eprintln!("FAIL {failure}");
        }
        ExitCode::FAILURE
    }
}

/// Every capture the prose highlighter can emit resolves to a style.
fn check_prose_captures(tokens: &TokenSet, failures: &mut Vec<String>) {
    for capture in PROSE_CAPTURES {
        if tokens.prose_highlight_style(capture).is_none() {
            failures.push(format!("prose capture {capture}: unresolved"));
        }
    }
}

/// Text clears the APCA floor against the ground it is set on.
fn check_apca(input: ThemeInput, failures: &mut Vec<String>) {
    let theme = input.derive();
    for (foreground, background) in [
        ("labelTitle", "bgBase"),
        ("labelBase", "bgBase"),
        ("errorForeground", "errorBase"),
    ] {
        let contrast = apca_contrast(
            theme.lch(foreground).expect("registered foreground role"),
            theme.lch(background).expect("registered background role"),
        )
        .abs();
        if contrast < MIN_TEXT_APCA {
            failures.push(format!(
                "APCA {foreground}/{background} was {contrast:.2}, below {MIN_TEXT_APCA}"
            ));
        }
    }
}

/// A near-white neutral reads as beige once it carries chroma.
fn check_neutral_chroma(input: ThemeInput, failures: &mut Vec<String>) {
    let theme = input.derive();
    for role in [
        "bgBase",
        "bgBaseHover",
        "bgSub",
        "bgSubHover",
        "bgShade",
        "bgShadeHover",
        "bgBorderFaint",
    ] {
        let color = theme.lch(role).expect("registered neutral role");
        if color.l > 80.0 && color.c > 0.8 {
            failures.push(format!(
                "high-lightness neutral {role} carries CIE chroma {:.3}",
                color.c
            ));
        }
    }
}

/// The contrast control moves every role it claims to.
///
/// Swept between two fixed ends rather than from the document's own contrast,
/// so a product that ships at either end still proves the control works.
fn check_contrast_sensitivity(input: ThemeInput, failures: &mut Vec<String>) {
    let low = ThemeInput {
        contrast: 30.0,
        ..input
    }
    .derive();
    let high = ThemeInput {
        contrast: 70.0,
        ..input
    }
    .derive();
    for role in [
        "bgBaseHover",
        "bgSub",
        "bgShade",
        "bgBorder",
        "labelTitle",
        "labelBase",
        "labelMuted",
        "controlSecondaryHover",
        "controlTertiaryHover",
    ] {
        if low.color(role) == high.color(role) {
            failures.push(format!("contrast 30 -> 70 did not move {role}"));
        }
    }
}

/// Accent and the three status hues stay far enough apart to be told apart.
fn check_hue_bands(input: ThemeInput, failures: &mut Vec<String>) {
    let theme = input.derive();
    let bands = [
        ("accent", input.accent),
        ("error", theme.lch("errorBase").expect("error role")),
        ("success", theme.lch("successBase").expect("success role")),
        ("agent", theme.lch("agentBase").expect("agent role")),
    ];
    for (index, (left_name, left)) in bands.iter().enumerate() {
        for (right_name, right) in bands.iter().skip(index + 1) {
            let distance = hue_distance(*left, *right);
            if distance < MIN_HUE_SEPARATION {
                failures.push(format!(
                    "hue bands {left_name}/{right_name} are only {distance:.1} degrees apart"
                ));
            }
        }
    }
}

fn hue_distance(left: CieLch, right: CieLch) -> f64 {
    let direct = (left.h - right.h).abs();
    direct.min(360.0 - direct)
}
