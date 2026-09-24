use imasm_core::imasm16_3::{leq_c, leq_i, Reg16_3};
use vox_core::{godel_analyzer, godel_calculus};

fn support_pattern_state(bits_le: &str, frame_width: usize) -> Reg16_3 {
    let symbols: Vec<u8> = bits_le
        .as_bytes()
        .chunks(frame_width)
        .map(|group| {
            group.iter().enumerate().fold(0u8, |state, (bit, value)| {
                state | (u8::from(*value == b'1') << bit)
            })
        })
        .collect();
    let periodic = (1..=symbols.len() / 2).any(|period| {
        (period..symbols.len()).all(|index| symbols[index] == symbols[index - period])
    });
    Reg16_3 {
        small_t: periodic,
        small_f: !periodic,
        ..Reg16_3::default()
    }
}

/// Codec assertions are constructive T/F support. A periodic/nonperiodic
/// frame signature occupies the kernel's informational t/f support and never
/// asserts a factorization.
fn codec_support_state(assertions: &godel_analyzer::CodecAssertions) -> Reg16_3 {
    let checks = [
        assertions.support,
        assertions.binary_reverse,
        assertions.word_glyph_map,
        assertions.roundtrip,
        assertions.bitlength,
    ];
    Reg16_3 {
        big_t: checks.iter().any(|check| *check),
        big_f: checks.iter().any(|check| !*check),
        ..Reg16_3::default()
    }
}

fn analyze_with_kernel(args: &[&str]) -> Result<String, String> {
    let raw = args
        .get(1)
        .ok_or_else(|| "godel analyze <natural-number|cell-binary-word>".to_string())?;
    let value = godel_analyzer::parse_input(raw)?;
    let analysis = godel_analyzer::analyze(&value, None)?;
    let codec = codec_support_state(&analysis.assertions);
    // Every frame regroups the same support stream. Keep each adjacent kernel
    // order read, including false relations, instead of collapsing the sweep.
    let support_pattern = support_pattern_state(&analysis.bits_le, 2);
    let joint = codec.union(support_pattern);
    let factor_bound = if analysis.divisor_bound.is_some() {
        "supplied"
    } else {
        "N"
    };
    let mut report = godel_analyzer::render(&analysis);
    use core::fmt::Write;
    writeln!(
        report,
        "kernel.object                  cell-binary representation and finite support pattern\
         \nkernel.register.codec          {}\
         \nkernel.frame-sweep widths=2..8 preserved-bits={}\
         \nkernel.register.pattern       {}\
         \nkernel.register.joint         {}\
         \nkernel.truth-support           {}\
         \nkernel.falsity-support         {}\
         \nkernel.factor-bound-support    {}",
        codec.four_name(),
        analysis.bits_le.len(),
        support_pattern.name(),
        joint.name(),
        codec.big_t,
        codec.big_f,
        factor_bound,
    )
    .expect("append kernel readout");
    for width in 2..=8 {
        let pattern = support_pattern_state(&analysis.bits_le, width);
        let state = codec.union(pattern);
        writeln!(
            report,
            "kernel.frame width={} groups={} state={} prev≤i={} prev≤c={}",
            width,
            analysis.bits_le.len().div_ceil(width),
            state.name(),
            if width == 2 {
                "seed"
            } else {
                let previous = codec.union(support_pattern_state(&analysis.bits_le, width - 1));
                if leq_i(previous, state) {
                    "true"
                } else {
                    "false"
                }
            },
            if width == 2 {
                "seed"
            } else {
                let previous = codec.union(support_pattern_state(&analysis.bits_le, width - 1));
                if leq_c(previous, state) {
                    "true"
                } else {
                    "false"
                }
            },
        )
        .expect("append kernel frame state");
    }
    Ok(report)
}

fn dispatch(args: &[&str]) -> Result<String, String> {
    match args.first().copied().unwrap_or("help") {
        "analyze" => analyze_with_kernel(args),
        "lte2" => godel_analyzer::command(args),
        "selftest" | "verify" => {
            let mut out = godel_calculus::selftest_report()?;
            out.push_str(&godel_analyzer::selftest_report()?);
            Ok(out)
        }
        "help" | "-h" | "--help" => Ok(format!(
            "{}{}",
            godel_calculus::help(),
            godel_analyzer::help_addendum()
        )),
        _ => godel_calculus::command(args),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    match dispatch(&refs) {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprint!("{error}");
            if !error.ends_with('\n') {
                eprintln!();
            }
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzer_holds_codec_and_pattern_supports_in_the_kernel_register() {
        let twenty_one = analyze_with_kernel(&["analyze", "21"]).unwrap();
        assert!(twenty_one.contains("kernel.register.codec          T\n"));
        assert!(twenty_one.contains("kernel.register.pattern       t\n"));
        assert!(twenty_one.contains("kernel.frame-sweep widths=2..8 preserved-bits=5\n"));
        assert!(twenty_one
            .lines()
            .any(|line| line.starts_with("kernel.register.joint") && line.ends_with("Tt")));
        assert!(twenty_one.contains("kernel.truth-support           true\n"));
        assert!(twenty_one.contains("kernel.falsity-support         false\n"));
        assert!(twenty_one.contains("kernel.factor-bound-support    N\n"));
        assert!(
            twenty_one.contains("kernel.frame width=2 groups=3 state=Tt prev≤i=seed prev≤c=seed\n")
        );
        assert!(twenty_one
            .contains("kernel.frame width=3 groups=2 state=Tf prev≤i=false prev≤c=false\n"));

        let nines = analyze_with_kernel(&["analyze", "999999"]).unwrap();
        assert!(nines.contains("kernel.truth-support           true\n"));
        assert!(nines.contains("kernel.falsity-support         false\n"));
        assert!(nines.contains("kernel.factor-bound-support    N\n"));
        assert!(nines.contains("kernel.frame-sweep widths=2..8 preserved-bits=20\n"));
        assert!(nines.contains("kernel.frame width=2 groups=10 state=Tf prev≤i=seed prev≤c=seed\n"));
    }

    #[test]
    fn codec_assertion_conflict_is_held_as_b() {
        let mixed = godel_analyzer::CodecAssertions {
            support: true,
            binary_reverse: false,
            word_glyph_map: true,
            roundtrip: false,
            bitlength: true,
        };
        assert_eq!(codec_support_state(&mixed).four_name(), "B");
    }

    #[test]
    fn analyzer_preserves_cell_binary_input_route() {
        let report = analyze_with_kernel(&["analyze", "⊢≻⋈∈⊤∋≻⋈∈⊥∋≻⋈∈⊤∋≻⋈∈⊥∋⊙⊡⊣"]).unwrap();
        assert!(report.contains("value                      10\n"));
        assert!(report.contains("kernel.register.codec          T\n"));
        assert!(report.contains("kernel.frame-sweep widths=2..8 preserved-bits=4\n"));
    }
}
