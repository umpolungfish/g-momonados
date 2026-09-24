use imasm_core::imasm16_3::{leq_c, leq_i, Reg16_3};
use vox_core::{godel_analyzer, godel_calculus};

fn support_pattern_state(bits_le: &str, frame_width: usize) -> Reg16_3 {
    let symbols = frame_codes(bits_le, frame_width);
    let periodic = (1..=symbols.len() / 2).any(|period| {
        (period..symbols.len()).all(|index| symbols[index] == symbols[index - period])
    });
    Reg16_3 {
        small_t: periodic,
        small_f: !periodic,
        ..Reg16_3::default()
    }
}

fn frame_codes(bits_le: &str, frame_width: usize) -> Vec<u8> {
    bits_le
        .as_bytes()
        .chunks(frame_width)
        .map(|group| {
            group.iter().enumerate().fold(0u8, |state, (bit, value)| {
                state | (u8::from(*value == b'1') << bit)
            })
        })
        .collect()
}

fn frame_symbols(bits_le: &str, frame_width: usize) -> String {
    let mut stream = String::from("[");
    for (group_index, group) in bits_le.as_bytes().chunks(frame_width).enumerate() {
        if group_index != 0 {
            stream.push('|');
        }
        for bit in group {
            stream.push_str(if *bit == b'1' { "⊥" } else { "⊤" });
        }
    }
    stream.push(']');
    stream
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

fn sieve_support_state(sieve: &godel_calculus::PrimeSieveRead) -> Reg16_3 {
    if sieve.divisor.is_some() {
        Reg16_3 {
            small_t: true,
            ..Reg16_3::default()
        }
    } else if sieve.lower_bound.is_some() {
        Reg16_3 {
            small_f: true,
            ..Reg16_3::default()
        }
    } else {
        Reg16_3::default()
    }
}

fn analyze_with_kernel(args: &[&str]) -> Result<String, String> {
    let raw = args.get(1).ok_or_else(|| {
        "godel analyze <natural-number|cell-binary-word> [sieve-window=8]".to_string()
    })?;
    let value = godel_analyzer::parse_input(raw)?;
    let sieve_width = args
        .get(2)
        .map(|width| width.parse::<usize>())
        .transpose()
        .map_err(|_| "sieve window must be an integer of at least 2".to_string())?
        .unwrap_or(8);
    let sieve = godel_calculus::prime_sieve_read(&value, sieve_width)?;
    let analysis = godel_analyzer::analyze_with_sieve(&value, None, Some(sieve.clone()))?;
    let codec = codec_support_state(&analysis.assertions);
    // Every frame regroups the same support stream. Keep each adjacent kernel
    // order read, including false relations, instead of collapsing the sweep.
    let support_pattern = support_pattern_state(&analysis.bits_le, 2);
    let sieve_state = sieve_support_state(&sieve);
    let joint = codec.union(support_pattern).union(sieve_state);
    let factor_bound = sieve_state.name();
    let mut report = godel_analyzer::render(&analysis);
    use core::fmt::Write;
    writeln!(
        report,
        "kernel.object                  cell-binary representation and finite support pattern\
         \nkernel.register.codec          {}\
         \nkernel.frame-sweep widths=2..8 preserved-bits={}\
         \nkernel.register.sieve          {}\
         \nkernel.sieve.aperture           2^{}={} tested-primes={}\
         \nkernel.register.pattern       {}\
         \nkernel.register.joint         {}\
         \nkernel.truth-support           {}\
         \nkernel.falsity-support         {}\
         \nkernel.factor-bound-support    {}",
        codec.four_name(),
        analysis.bits_le.len(),
        sieve_state.name(),
        sieve.aperture_width,
        sieve.aperture,
        sieve.tested_primes,
        support_pattern.name(),
        joint.name(),
        joint.big_t || joint.small_t,
        joint.big_f || joint.small_f,
        factor_bound,
    )
    .expect("append kernel readout");
    for width in 2..=8 {
        let pattern = support_pattern_state(&analysis.bits_le, width);
        let state = codec.union(pattern).union(sieve_state);
        writeln!(
            report,
            "kernel.frame width={} groups={} symbols={} state={} prev≤i={} prev≤c={}",
            width,
            analysis.bits_le.len().div_ceil(width),
            frame_symbols(&analysis.bits_le, width),
            state.name(),
            if width == 2 {
                "seed"
            } else {
                let previous = codec
                    .union(support_pattern_state(&analysis.bits_le, width - 1))
                    .union(sieve_state);
                if leq_i(previous, state) {
                    "true"
                } else {
                    "false"
                }
            },
            if width == 2 {
                "seed"
            } else {
                let previous = codec
                    .union(support_pattern_state(&analysis.bits_le, width - 1))
                    .union(sieve_state);
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
        assert!(twenty_one.contains("kernel.register.sieve          t\n"));
        assert!(twenty_one.contains("kernel.sieve.aperture           2^8=256 tested-primes=1\n"));
        assert!(twenty_one.contains("kernel.frame-sweep widths=2..8 preserved-bits=5\n"));
        assert!(twenty_one
            .lines()
            .any(|line| line.starts_with("kernel.register.joint") && line.ends_with("Tt")));
        assert!(twenty_one.contains("kernel.truth-support           true\n"));
        assert!(twenty_one.contains("kernel.falsity-support         false\n"));
        assert!(twenty_one.contains("kernel.factor-bound-support    t\n"));
        assert!(twenty_one.contains(
            "kernel.frame width=2 groups=3 symbols=[⊥⊤|⊥⊤|⊥] state=Tt prev≤i=seed prev≤c=seed\n"
        ));
        assert!(twenty_one.contains(
            "kernel.frame width=3 groups=2 symbols=[⊥⊤⊥|⊤⊥] state=Ttf prev≤i=true prev≤c=false\n"
        ));

        let nines = analyze_with_kernel(&["analyze", "999999"]).unwrap();
        assert!(nines.contains("kernel.truth-support           true\n"));
        assert!(nines.contains("kernel.falsity-support         true\n"));
        assert!(nines.contains("kernel.factor-bound-support    t\n"));
        assert!(nines.contains("kernel.frame-sweep widths=2..8 preserved-bits=20\n"));
        assert!(nines.contains("kernel.frame width=2 groups=10 symbols=[⊥⊥|⊥⊥|⊥⊥|⊤⊤|⊤⊥|⊤⊤|⊤⊤|⊥⊤|⊥⊥|⊥⊥] state=Ttf prev≤i=seed prev≤c=seed\n"));
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

    #[test]
    fn rsa_challenge_frames_preserve_each_joint_symbol_and_kernel_face() {
        let cases = [
            (
                "1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139",
                330,
                165,
            ),
            (
                "35794234179725868774991807832568455403003778024228226193532908190484670252364677411513516111204504060317568667",
                364,
                182,
            ),
            (
                "227010481295437363334259960947493668895875336466084780038173258247009162675779735389791151574049166747880487470296548479",
                397,
                199,
            ),
        ];

        for (value, bitlength, width_two_groups) in cases {
            let parsed_value = godel_analyzer::parse_input(value).unwrap();
            let sieve = godel_calculus::prime_sieve_read(&parsed_value, 8).unwrap();
            let analysis =
                godel_analyzer::analyze_with_sieve(&parsed_value, None, Some(sieve.clone()))
                    .unwrap();
            assert_eq!(analysis.bits_le.len(), bitlength);
            assert!(analysis.assertions.all());
            for sieve_width in [8, 10, 12, 14, 16, 21] {
                let sieve = godel_calculus::prime_sieve_read(&parsed_value, sieve_width).unwrap();
                assert!(sieve.divisor.is_none());
                assert_eq!(
                    sieve.lower_bound,
                    Some(godel_calculus::Nat::from_bits_le(
                        (0..=sieve_width).map(|bit| bit == sieve_width).collect(),
                    ))
                );
                assert!(!sieve.tested_primes.is_zero());
                assert_eq!(sieve_support_state(&sieve).name(), "f");
            }
            for width in 2..=8 {
                let symbols = frame_symbols(&analysis.bits_le, width);
                let reconstructed = symbols
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .replace('|', "")
                    .replace("⊥", "1")
                    .replace("⊤", "0");
                assert_eq!(reconstructed, analysis.bits_le);
                assert_eq!(
                    frame_codes(&analysis.bits_le, width).len(),
                    bitlength.div_ceil(width)
                );
                assert!(
                    support_pattern_state(&analysis.bits_le, width).small_f,
                    "RSA frame width {width} retains its nonperiodic support"
                );
                if width > 2 {
                    let previous = codec_support_state(&analysis.assertions)
                        .union(support_pattern_state(&analysis.bits_le, width - 1))
                        .union(sieve_support_state(&sieve));
                    let current = codec_support_state(&analysis.assertions)
                        .union(support_pattern_state(&analysis.bits_le, width))
                        .union(sieve_support_state(&sieve));
                    assert!(leq_i(previous, current));
                    assert!(leq_c(previous, current));
                }
            }
            assert_eq!(frame_codes(&analysis.bits_le, 2).len(), width_two_groups);
            let report = analyze_with_kernel(&["analyze", value]).unwrap();
            assert!(report.contains("kernel.register.codec          T\n"));
            assert!(report.contains("kernel.register.sieve          f\n"));
            assert!(report.contains("kernel.register.joint         Tf\n"));
            assert!(report.contains("kernel.truth-support           true\n"));
            assert!(report.contains("kernel.falsity-support         true\n"));
            assert!(report.contains("negative.factor-bound      >256\n"));
            assert!(report.contains("kernel.factor-bound-support    f\n"));
            assert!(report.contains("prev≤i=true prev≤c=true\n"));
        }
    }
}
