//! Stream generated Fibonacci exchanges to an anyon generator and return its
//! control-fusion readout to the recycled phase membrane.
//!
//! The transport is newline-delimited JSON. Exchanges are buffered and sent
//! as they are generated. A fusion request flushes the braid stream and blocks
//! for the anyon generator's measured fusion channel.

use crate::anyon_braid_cnot::FibonacciAnyonDevice;
use alloc::string::String;
use g_momonados::recycled_carrier::WorkPreparation;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

pub struct FibonacciGenerator<R: Read, W: Write> {
    reader: BufReader<R>,
    writer: BufWriter<W>,
    active: bool,
    phase_bits: usize,
    measured_bits: usize,
    shot_id: u64,
}

impl<R: Read, W: Write> FibonacciGenerator<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
            active: false,
            phase_bits: 0,
            measured_bits: 0,
            shot_id: 0,
        }
    }

    fn send(&mut self, message: &serde_json::Value, flush: bool) -> Result<(), String> {
        self.writer
            .write_all(message.to_string().as_bytes())
            .map_err(|error| format!("anyon generator request failed: {error}"))?;
        self.writer
            .write_all(b"\n")
            .map_err(|error| format!("anyon generator stream failed: {error}"))?;
        if flush {
            self.writer
                .flush()
                .map_err(|error| format!("anyon generator flush failed: {error}"))?;
        }
        Ok(())
    }

    fn response(&mut self) -> Result<serde_json::Value, String> {
        let mut line = String::new();
        let read = self
            .reader
            .read_line(&mut line)
            .map_err(|error| format!("anyon generator readout failed: {error}"))?;
        if read == 0 {
            return Err("anyon generator closed before replying".into());
        }
        let response: serde_json::Value = serde_json::from_str(&line)
            .map_err(|error| format!("anyon generator returned invalid JSON: {error}"))?;
        if let Some(error) = response.get("error").and_then(serde_json::Value::as_str) {
            return Err(format!("anyon generator error: {error}"));
        }
        Ok(response)
    }

    fn require_ack(&mut self, stage: &str) -> Result<(), String> {
        let response = self.response()?;
        self.require_shot(&response)?;
        if response.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
            Ok(())
        } else {
            Err(format!("anyon generator did not acknowledge {stage}"))
        }
    }

    fn require_shot(&self, response: &serde_json::Value) -> Result<(), String> {
        if response.get("shot_id").and_then(serde_json::Value::as_u64) != Some(self.shot_id) {
            return Err("anyon generator reply does not belong to the active shot".into());
        }
        Ok(())
    }
}

impl FibonacciGenerator<std::os::unix::net::UnixStream, std::os::unix::net::UnixStream> {
    pub fn connect(path: &str) -> Result<Self, String> {
        let stream = std::os::unix::net::UnixStream::connect(path).map_err(|error| {
            format!("cannot connect to Fibonacci anyon generator at {path}: {error}")
        })?;
        let reader = stream
            .try_clone()
            .map_err(|error| format!("cannot open anyon generator read channel: {error}"))?;
        Ok(Self::new(reader, stream))
    }

    /// Ask the local source-only factor service to execute a complete
    /// contracted-Fibonacci phase shot and return a product-closed pair.
    pub fn factor_source(
        &mut self,
        source: &num_bigint::BigUint,
        base: &num_bigint::BigUint,
        max_shots: u32,
    ) -> Result<Option<SocketFactorReadout>, String> {
        if source.bits() < 200 {
            return Err("socket factor execution requires a source of at least 200 bits".into());
        }
        self.send(
            &serde_json::json!({"op":"factor","source":source.to_str_radix(10),
                "base":base.to_str_radix(10),"max_shots":max_shots}),
            true,
        )?;
        let mut started = false;
        let mut last_shot = 0u64;
        loop {
            let response = self.response()?;
            if response.get("source").and_then(serde_json::Value::as_str)
                .is_some_and(|value| value != source.to_str_radix(10))
            {
                return Err("socket factor event belongs to a different source".into());
            }
            match response.get("event").and_then(serde_json::Value::as_str) {
                Some("started") => {
                    if response.get("backend").and_then(serde_json::Value::as_str)
                        != Some("contracted_fibonacci_ququart")
                    {
                        return Err("socket selected an unexpected phase backend".into());
                    }
                    started = true;
                    eprintln!("anyon_factor_started bits={}", source.bits());
                }
                Some("shot_started") => {
                    last_shot = response.get("shot").and_then(serde_json::Value::as_u64)
                        .ok_or("factor service omitted shot number")?;
                    eprintln!("anyon_factor_shot_started shot={last_shot}");
                }
                Some("phase_measured") => {
                    let completed = response["phase_digits_completed"].as_u64()
                        .ok_or("factor service omitted phase progress")?;
                    let total = response["phase_digits_total"].as_u64()
                        .ok_or("factor service omitted phase width")?;
                    if completed % 16 == 0 || completed == total {
                        eprintln!("anyon_factor_phase shot={last_shot} digits={completed}/{total}");
                    }
                }
                Some("shot_completed") => {
                    if response["phase_digits_completed"].as_u64()
                        != response["phase_digits_total"].as_u64()
                    {
                        return Err("factor service closed an incomplete phase stack".into());
                    }
                    eprintln!("anyon_factor_shot_closed shot={last_shot}");
                }
                Some("retrying_shot") => {
                    eprintln!("anyon_factor_retry_after_nonclosing_shot");
                }
                Some("nonclosing_budget") => return Ok(None),
                Some("factored") => {
                    if !started || response.get("product_closed").and_then(serde_json::Value::as_bool) != Some(true) {
                        return Err("factor service did not close its source product".into());
                    }
                    let p = response["p"].as_str().ok_or("factor service omitted p")?
                        .parse::<num_bigint::BigUint>().map_err(|_| "invalid p")?;
                    let q = response["q"].as_str().ok_or("factor service omitted q")?
                        .parse::<num_bigint::BigUint>().map_err(|_| "invalid q")?;
                    if p <= num_bigint::BigUint::from(1u8)
                        || q <= num_bigint::BigUint::from(1u8) || &p * &q != *source
                    {
                        return Err("socket factor pair failed exact source multiplication".into());
                    }
                    let order = response["order"].as_str().ok_or("factor service omitted order")?
                        .parse::<num_bigint::BigUint>().map_err(|_| "invalid order")?;
                    return Ok(Some(SocketFactorReadout { p, q, order, shots:last_shot }));
                }
                Some(event) => return Err(format!("unexpected factor service event: {event}")),
                None => return Err("factor service response omitted event type".into()),
            }
        }
    }
}

pub struct SocketFactorReadout {
    pub p: num_bigint::BigUint,
    pub q: num_bigint::BigUint,
    pub order: num_bigint::BigUint,
    pub shots: u64,
}

impl<R: Read, W: Write> FibonacciAnyonDevice for FibonacciGenerator<R, W> {
    fn begin(
        &mut self,
        source: &[char],
        base: &[char],
        logical_qubits: usize,
        phase_bits: usize,
        work_preparation: WorkPreparation,
    ) -> Result<(), String> {
        if self.active {
            return Err("anyon generator already has an active shot".into());
        }
        if phase_bits == 0 {
            return Err("anyon shot requires a positive phase width".into());
        }
        let bits = |tape: &[char]| -> Result<String, String> {
            tape.iter()
                .map(|cell| match *cell {
                    vox_core::vox::EVALF => Ok('1'),
                    vox_core::vox::EVALT => Ok('0'),
                    _ => Err("anyon generator source is not a binary numeral".into()),
                })
                .collect()
        };
        let source = bits(source)?;
        let base = bits(base)?;
        self.shot_id = self
            .shot_id
            .checked_add(1)
            .ok_or("anyon shot identifier overflow")?;
        self.send(
            &serde_json::json!({
                "op": "begin",
                "protocol": "g-momonados/fibonacci-anyons-v2",
                "shot_id": self.shot_id,
                "format": "little_endian_bits",
                "source": source,
                "base": base,
                "logical_qubits": logical_qubits,
                "phase_bits": phase_bits,
                "work_preparation": match work_preparation {
                    WorkPreparation::UniformResidues => "uniform_residues",
                },
            }),
            true,
        )?;
        self.active = true;
        self.phase_bits = phase_bits;
        self.measured_bits = 0;
        self.require_ack("shot preparation")
    }

    fn generate_exchange(&mut self, generator: i32) -> Result<(), String> {
        if !self.active {
            return Err("anyon exchange requested outside an active shot".into());
        }
        if generator == 0 {
            return Err("zero is not a Fibonacci exchange generator".into());
        }
        self.send(
            &serde_json::json!({"op": "exchange", "generator": generator, "shot_id": self.shot_id}),
            false,
        )
    }

    fn measure_control_fusion(&mut self) -> Result<bool, String> {
        if !self.active {
            return Err("fusion readout requested outside an active shot".into());
        }
        if self.measured_bits >= self.phase_bits {
            return Err("fusion readout exceeds the prepared phase width".into());
        }
        self.send(
            &serde_json::json!({
                "op": "measure_control_fusion", "shot_id": self.shot_id,
                "phase_index": self.measured_bits
            }),
            true,
        )?;
        let response = self.response()?;
        self.require_shot(&response)?;
        if response
            .get("phase_index")
            .and_then(serde_json::Value::as_u64)
            != Some(self.measured_bits as u64)
        {
            return Err("anyon generator fusion reply has the wrong phase index".into());
        }
        let bit = response
            .get("fusion_bit")
            .and_then(serde_json::Value::as_u64)
            .and_then(|bit| match bit {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            })
            .ok_or_else(|| {
                "anyon generator fusion readout must be the measured bit 0 or 1".to_string()
            })?;
        self.measured_bits += 1;
        Ok(bit)
    }

    fn finish(&mut self) -> Result<(), String> {
        if !self.active {
            return Err("anyon shot finished without preparation".into());
        }
        if self.measured_bits != self.phase_bits {
            return Err(format!(
                "anyon shot has {} fusion bits; prepared phase width is {}",
                self.measured_bits, self.phase_bits
            ));
        }
        self.send(
            &serde_json::json!({"op": "finish", "shot_id": self.shot_id}),
            true,
        )?;
        self.require_ack("shot completion")?;
        self.active = false;
        Ok(())
    }

    fn abort(&mut self) {
        if self.active {
            let _ = self.send(
                &serde_json::json!({"op": "abort", "shot_id": self.shot_id}),
                true,
            );
            self.active = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;
    use std::io::Cursor;

    #[test]
    fn phase_tags_bind_readouts_at_all_required_source_widths() {
        let manifest = include_str!("../measurements/anyon-extractor-width-controls.tsv");
        let mut widths = Vec::new();
        for line in manifest.lines().skip(1) {
            let fields: Vec<_> = line.split('\t').collect();
            let width: usize = fields[1].parse().unwrap();
            let source = BigUint::parse_bytes(fields[2].as_bytes(), 10).unwrap();
            assert_eq!(source.bits(), width as u64);
            let source_bits: Vec<_> = (0..width)
                .map(|bit| {
                    if source.bit(bit as u64) {
                        vox_core::vox::EVALF
                    } else {
                        vox_core::vox::EVALT
                    }
                })
                .collect();
            let base = [vox_core::vox::EVALT, vox_core::vox::EVALF];
            let count = 2 * width + 8;
            // Explicit transport fixtures. No phase probability or factor is
            // inferred from these replies.
            let mut replies = String::from("{\"ok\":true,\"shot_id\":1}\n");
            for index in 0..count {
                replies.push_str(&format!(
                    "{{\"shot_id\":1,\"phase_index\":{index},\"fusion_bit\":{}}}\n",
                    index % 2
                ));
            }
            replies.push_str("{\"ok\":true,\"shot_id\":1}\n");
            let mut device = FibonacciGenerator::new(Cursor::new(replies.into_bytes()), Vec::new());
            device
                .begin(
                    &source_bits,
                    &base,
                    3 * width + 4,
                    count,
                    WorkPreparation::UniformResidues,
                )
                .unwrap();
            assert!(device.finish().is_err());
            for index in 0..count {
                assert_eq!(device.measure_control_fusion().unwrap(), index % 2 == 1);
            }
            assert!(device.measure_control_fusion().is_err());
            device.finish().unwrap();
            for invalid in [
                "{\"shot_id\":0,\"phase_index\":0,\"fusion_bit\":1}",
                "{\"shot_id\":2,\"phase_index\":0,\"fusion_bit\":1}",
                "{\"shot_id\":1,\"phase_index\":1,\"fusion_bit\":1}",
                "{\"fusion_bit\":1}",
            ] {
                let replies = format!("{{\"ok\":true,\"shot_id\":1}}\n{invalid}\n");
                let mut device =
                    FibonacciGenerator::new(Cursor::new(replies.into_bytes()), Vec::new());
                device
                    .begin(
                        &source_bits,
                        &base,
                        3 * width + 4,
                        count,
                        WorkPreparation::UniformResidues,
                    )
                    .unwrap();
                assert!(device.measure_control_fusion().is_err());
                assert_eq!(device.measured_bits, 0);
                assert!(device.finish().is_err());
                device.abort();
            }
            widths.push(width);
            println!("{width}-bit source: {count} tagged transport replies accepted; stale, reordered, and untagged replies rejected");
        }
        assert_eq!(widths, vec![128, 256, 512, 1024, 2048]);
    }

    #[test]
    fn exchange_stream_transports_fusion_readout_for_128_bit_semiprime() {
        let source = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        let source_bits: Vec<char> = (0u64..128)
            .map(|bit| {
                if source.bit(bit) {
                    vox_core::vox::EVALF
                } else {
                    vox_core::vox::EVALT
                }
            })
            .collect();
        let base_bits: Vec<char> = [vox_core::vox::EVALT, vox_core::vox::EVALF]
            .into_iter()
            .collect();
        let mut replies = String::from("{\"ok\":true,\"shot_id\":1}\n");
        for index in 0..256 {
            replies.push_str(&format!(
                "{{\"fusion_bit\":1,\"shot_id\":1,\"phase_index\":{index}}}\n"
            ));
        }
        replies.push_str("{\"ok\":true,\"shot_id\":1}\n");
        let mut device = FibonacciGenerator::new(Cursor::new(replies.into_bytes()), Vec::new());
        device
            .begin(
                &source_bits,
                &base_bits,
                388,
                256,
                WorkPreparation::UniformResidues,
            )
            .unwrap();
        device.generate_exchange(1).unwrap();
        assert!(device.finish().is_err());
        for _ in 0..256 {
            assert!(device.measure_control_fusion().unwrap());
        }
        assert!(device.measure_control_fusion().is_err());
        device.finish().unwrap();

        let emitted = String::from_utf8(device.writer.into_inner().unwrap()).unwrap();
        assert!(emitted.contains("\"op\":\"begin\""));
        assert!(emitted.contains("\"protocol\":\"g-momonados/fibonacci-anyons-v2\""));
        assert!(emitted.contains("\"format\":\"little_endian_bits\""));
        assert!(emitted.contains("\"work_preparation\":\"uniform_residues\""));
        assert!(emitted.contains("\"generator\":1"));
        assert!(emitted.contains("\"op\":\"measure_control_fusion\""));
        assert!(emitted.contains("\"op\":\"finish\""));
    }
}
