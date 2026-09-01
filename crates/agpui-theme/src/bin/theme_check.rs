//! Build gate for generated theme invariants and shell metric authority.

use std::{fs, path::PathBuf, process::ExitCode};

use theorem_design_core::{apca_contrast, CieLch, ThemeInput};

const MIN_TEXT_APCA: f32 = 38.0;

fn main() -> ExitCode {
    let mut failures = Vec::new();
    check_apca(&mut failures);
    check_neutral_chroma(&mut failures);
    check_contrast_sensitivity(&mut failures);
    check_hue_bands(&mut failures);
    check_shell_metrics(&mut failures);

    if failures.is_empty() {
        println!("theme-check: generated roles and shell metrics passed");
        ExitCode::SUCCESS
    } else {
        for failure in failures {
            eprintln!("theme-check: {failure}");
        }
        ExitCode::FAILURE
    }
}

fn check_apca(failures: &mut Vec<String>) {
    let theme = ThemeInput::theorem_default().derive();
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

fn check_neutral_chroma(failures: &mut Vec<String>) {
    let theme = ThemeInput::theorem_default().derive();
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

fn check_contrast_sensitivity(failures: &mut Vec<String>) {
    let low = ThemeInput::theorem_default().derive();
    let high = ThemeInput {
        contrast: 70.0,
        ..ThemeInput::theorem_default()
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

fn check_hue_bands(failures: &mut Vec<String>) {
    let theme = ThemeInput::theorem_default().derive();
    let bands = [
        ("accent", ThemeInput::theorem_default().accent),
        ("error", theme.lch("errorBase").expect("error role")),
        ("success", theme.lch("successBase").expect("success role")),
        ("agent", theme.lch("agentBase").expect("agent role")),
    ];
    for (index, (left_name, left)) in bands.iter().enumerate() {
        for (right_name, right) in bands.iter().skip(index + 1) {
            let distance = hue_distance(*left, *right);
            if distance < 24.0 {
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

fn check_shell_metrics(failures: &mut Vec<String>) {
    let shell = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gpui/chat/src/shell.rs");
    let source = match fs::read_to_string(&shell) {
        Ok(source) => source,
        Err(error) => {
            failures.push(format!("cannot read {}: {error}", shell.display()));
            return;
        }
    };
    for (line_index, line) in source.lines().enumerate() {
        if let Some(offset) = line.find("px(") {
            let argument = line[offset + 3..].trim_start();
            if argument
                .starts_with(|character: char| character.is_ascii_digit() || character == '-')
            {
                failures.push(format!(
                    "{}:{} contains a numeric px literal",
                    shell.display(),
                    line_index + 1
                ));
            }
        }
    }
    if !source.contains("theorem_design_core::METRICS") {
        failures.push("GPUI workspace shell does not consume shared METRICS".to_owned());
    }
}
