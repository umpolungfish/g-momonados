//! Local socket service for source-bound ququart factor execution.
//! Usage: anyon_generator <unix-socket-path> <prepared-operator-dir>.
//! Requests are status or factor. Factor requests execute the complete
//! contracted-Fibonacci Fourier operator and folded modular work register.
#[path = "ququart_support/mod.rs"]
#[allow(dead_code)]
mod support;

use g_momonados::anyon_ququart::QuquartDigit;
use g_momonados::ququart_factor::{QuquartFactorExecutor, QuquartPhaseDevice};
use g_momonados::ququart_folded_work::QuquartFoldedWorkDevice;
use g_momonados::ququart_reentry_clock::RuntimeClock;
use num_bigint::BigUint;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

#[path = "../../../p4rakernel/bridge/client.rs"]
#[allow(dead_code)]
mod lean_kernel;

const BACKEND: &str = "contracted_fibonacci_ququart";

fn y_reentry_orientation_word(prepared: &Value, y_fixed_point: u8, digits: usize)
    -> Result<Vec<(bool, bool)>, String>
{
    let clock = RuntimeClock::from_record(prepared.get("phase_reentry_clock")
        .ok_or("physical operator lacks the certified phase re-entry clock")?)?;
    let read_states = |key: &str| -> Result<Vec<u8>, String> {
        prepared.get(key).and_then(Value::as_array).ok_or_else(|| format!("missing {key}"))?
            .iter().map(|state| state.as_u64().filter(|state| *state < 16)
                .map(|state| state as u8).ok_or_else(|| format!("invalid state in {key}")))
            .collect()
    };
    let source_states = read_states("phase_source_states")?;
    let clock_states = read_states("phase_clock_states")?;
    if source_states.is_empty() || source_states.len() != clock_states.len() {
        return Err("phase source and re-entry clock words differ in length".into());
    }
    let mut base = Vec::with_capacity(digits);
    for (&source_state, &clock_state) in source_states.iter().zip(&clock_states) {
        if clock.step(source_state) != clock_state {
            return Err("phase coordinate differs from its retained re-entry step".into());
        }
        let pair = (clock_state & 2 != 0, clock_state & 4 != 0);
        base.extend(core::iter::repeat(pair).take(4));
    }
    if base.len() < digits {
        let extra = digits - base.len();
        if extra > base.len() { return Err("phase schedule exceeds the re-entry motif prefix".into()); }
        let prefix = base[..extra].to_vec();
        base.extend(prefix);
    }
    if base.len() != digits { return Err("re-entry motif word differs from the phase schedule".into()); }
    let recorded = prepared.get("phase_orientation_pairs").and_then(Value::as_array)
        .ok_or("physical operator lacks its source-bound orientation word")?;
    let recorded: Vec<(bool, bool)> = recorded.iter().map(|pair| {
        let pair = pair.as_array().ok_or("phase orientation pair must be an array")?;
        if pair.len() != 2 { return Err("phase orientation pair must contain two flags".into()); }
        Ok((pair[0].as_bool().ok_or("phase orientation flag must be Boolean")?,
            pair[1].as_bool().ok_or("phase orientation flag must be Boolean")?))
    }).collect::<Result<_, String>>()?;
    if recorded != base { return Err("orientation word differs from re-entered motif coordinates".into()); }
    let y_forward = y_fixed_point & 2 != 0;
    let y_return = y_fixed_point & 4 != 0;
    Ok(base.into_iter().map(|(forward, inverse)|
        (forward ^ y_forward, inverse ^ y_return)).collect())
}

fn word(value: &BigUint) -> String {
    let natural = g_momonados::godel_calculus::Nat::from_bits_le(
        (0..value.bits()).map(|bit| value.bit(bit)).collect());
    g_momonados::godel_calculus::encode_cell_binary(&natural)
}

fn send(stream: &mut UnixStream, value: Value) -> Result<(), String> {
    writeln!(stream, "{value}").map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())
}

struct ProgressDevice {
    inner: QuquartFoldedWorkDevice,
    stream: UnixStream,
    source: String,
    digits: usize,
    expected: usize,
    shots: u64,
}
impl QuquartPhaseDevice for ProgressDevice {
    fn begin(&mut self, n: &BigUint, base: &BigUint, digits: usize) -> Result<(), String> {
        self.inner.begin(n, base, digits)?;
        self.digits = 0;
        self.expected = digits;
        self.shots += 1;
        send(&mut self.stream, json!({"event":"shot_started", "source":self.source,
            "shot":self.shots, "phase_digits_total":digits}))
    }
    fn fourier(&mut self, inverse: bool) -> Result<(), String> {
        self.inner.fourier(inverse)
    }
    fn controlled_multiply(&mut self, multiplier: &BigUint) -> Result<(), String> {
        self.inner.controlled_multiply(multiplier)
    }
    fn feedback(&mut self, numerator: &BigUint, digits: usize) -> Result<(), String> {
        self.inner.feedback(numerator, digits)
    }
    fn measure_phase_digit(&mut self) -> Result<QuquartDigit, String> {
        let digit = self.inner.measure_phase_digit()?;
        self.digits += 1;
        send(&mut self.stream, json!({"event":"phase_measured", "source":self.source,
            "shot":self.shots, "phase_digits_completed":self.digits,
            "phase_digits_total":self.expected, "digit":digit as u8}))?;
        Ok(digit)
    }
    fn reset_control(&mut self, digit: QuquartDigit) -> Result<(), String> {
        self.inner.reset_control(digit)
    }
    fn finish(&mut self) -> Result<(), String> {
        self.inner.finish()?;
        send(&mut self.stream, json!({"event":"shot_completed", "source":self.source,
            "shot":self.shots, "phase_digits_completed":self.digits,
            "phase_digits_total":self.expected}))
    }
    fn abort(&mut self) { self.inner.abort(); }
}

fn factor(stream: &mut UnixStream, request: &Value, directory: &Path) -> Result<(), String> {
    let raw = request["source"].as_str().ok_or("factor requires a decimal source string")?;
    let source = raw.parse::<BigUint>().map_err(|_| "invalid source integer")?;
    let base = request["base"].as_str().unwrap_or("2").parse::<BigUint>()
        .map_err(|_| "invalid base integer")?;
    let shot_limit = request["max_shots"].as_u64().ok_or("factor requires max_shots")?;
    if source.bits() < 200 || base < BigUint::from(2u8) || shot_limit == 0 {
        return Err("factor requires a source of at least 200 bits, base >= 2, and positive shot budget".into());
    }
    let operator = directory.join(format!(
        "rsa_physical_ququart_radix16_{}_input.json", source.bits()));
    let prepared: Value = serde_json::from_str(
        &std::fs::read_to_string(&operator).map_err(|e| format!("{}: {e}", operator.display()))?
    ).map_err(|e| e.to_string())?;
    if prepared.get("prepared_operator").is_none() {
        return Err("factor requires a contracted physical operator".into());
    }
    let y_fixed_point = lean_kernel::ensure_default()
        .map_err(|error| format!("Lean Y/re-entry admission: {error}"))?;
    let (bound_source, matrix, metrics) = support::contract(&prepared)?;
    if bound_source != source {
        return Err("physical operator belongs to a different source".into());
    }
    let mut prepared = prepared;
    let schedule = g_momonados::ququart_factor::QuquartPowerSchedule::prepare(&source, &base)?;
    let orientations = y_reentry_orientation_word(&prepared, y_fixed_point, schedule.powers().len())?;
    prepared["base_word"] = json!(word(&base));
    prepared["seed_word"] = json!(word(&BigUint::from(1729u32)));
    prepared["prepared_operator"]["phase_digits_word"] =
        json!(word(&BigUint::from(schedule.powers().len())));
    prepared["prepared_operator"]["controlled_power_words"] = json!(
        schedule.powers().iter().map(word).collect::<Vec<_>>());
    // Emit each nested multiplier when its phase slot executes. Expanding the
    // complete source schedule into a retained pool costs more memory than the
    // measured path and delays the first phase digit.
    let radix = prepared["radix_word"].as_str().ok_or("physical operator lacks the work radix")?;
    let inner = QuquartFoldedWorkDevice::new_interleaved_radix(
        source.clone(), matrix, 1729, radix)?;
    send(stream, json!({"event":"started", "source":source.to_string(),
        "source_bits":source.bits(), "backend":BACKEND,
        "physical_exchanges":metrics.exchanges, "producer_pid":std::process::id(),
        "max_shots":shot_limit}))?;
    let device = ProgressDevice {
        inner, stream:stream.try_clone().map_err(|e| e.to_string())?,
        source:source.to_string(), digits:0, expected:0, shots:0,
    };
    let mut executor = QuquartFactorExecutor::new(device);
    for _ in 0..shot_limit {
        let shot = executor.shot_with_orientations(&source, &base, Some(&orientations))?;
        send(stream, json!({"event":"phase_readout", "source":source.to_string(),
            "shot":executor.device().shots, "digits":orientations.len(),
            "numerator":shot.phase.numerator().to_string(),
            "denominator":shot.phase.denominator()?.to_string()}))?;
        if let Some(closure) = shot.closure {
            let (p, q) = closure.factors();
            if p <= &BigUint::from(1u8) || q <= &BigUint::from(1u8) || p * q != source {
                return Err("resident pair did not reconstruct the source".into());
            }
            return send(stream, json!({"event":"factored", "source":source.to_string(),
                "p":p.to_string(), "q":q.to_string(), "order":closure.order().to_string(),
                "backend":BACKEND, "product_closed":true,
                "source_word":closure.source_word(),
                "factor_words":closure.factor_words()}));
        }
        send(stream, json!({"event":"retrying_shot", "source":source.to_string()}))?;
    }
    send(stream, json!({"event":"nonclosing_budget", "source":source.to_string(),
        "max_shots":shot_limit}))
}

fn handle(mut stream: UnixStream, directory: PathBuf) -> Result<(), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 { return Ok(()); }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                send(&mut stream, json!({"error":error.to_string()}))?;
                continue;
            }
        };
        let result = match request["op"].as_str() {
            Some("status") => {
                let live = &g_momonados::ququart_folded_work::VOX_QUQUART_COUNTERS;
                let counters: Vec<u64> = live.iter().map(|c| c.load(std::sync::atomic::Ordering::Relaxed)).collect();
                send(&mut stream, json!({"ok":true,
                    "protocol":"g-momonados/fibonacci-anyons-v2", "backend":BACKEND,
                    "factor_execution_available":true, "fusion_readout_available":false,
                    "live":{"source_bits":counters[0],"phase_digits_total":counters[1],
                        "phase_digits_completed":counters[2],"nested_operations":counters[3],
                        "retained_nodes":counters[4],"peak_nodes":counters[5],
                        "shots_started":counters[6],"shots_completed":counters[7]},
                    "operations":["status","factor"]}))
            }
            Some("factor") => factor(&mut stream, &request, &directory),
            _ => Err("supported operations: status, factor; raw exchanges need a fusion device".into()),
        };
        if let Err(error) = result {
            send(&mut stream, json!({"event":"error", "source":request["source"], "error":error}))?;
        }
    }
}

fn main() {
    #[cfg(feature = "hosted")]
    vox_core::ordering::cli::intercept();
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: anyon_generator <unix-socket-path> <prepared-operator-dir>");
        std::process::exit(2);
    }
    let directory = PathBuf::from(&args[2]);
    let listener = UnixListener::bind(&args[1]).unwrap_or_else(|error| {
        eprintln!("bind {}: {error}", args[1]);
        std::process::exit(1)
    });
    eprintln!("anyon_generator listening on {} backend={BACKEND}", args[1]);
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                let directory = directory.clone();
                if let Err(error) = std::thread::Builder::new()
                    .name("anyon-factor".into()).stack_size(64 * 1024 * 1024)
                    .spawn(move || {
                        if let Err(error) = handle(stream, directory) { eprintln!("anyon socket: {error}"); }
                    })
                { eprintln!("anyon socket thread: {error}"); }
            }
            Err(error) => eprintln!("anyon socket accept: {error}"),
        }
    }
}
