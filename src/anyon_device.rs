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
}

impl<R: Read, W: Write> FibonacciGenerator<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
            active: false,
            phase_bits: 0,
            measured_bits: 0,
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
        if response.get("ok").and_then(serde_json::Value::as_bool) == Some(true) {
            Ok(())
        } else {
            Err(format!("anyon generator did not acknowledge {stage}"))
        }
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
        self.send(
            &serde_json::json!({
                "op": "begin",
                "protocol": "g-momonados/fibonacci-anyons-v1",
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
            &serde_json::json!({"op": "exchange", "generator": generator}),
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
        self.send(&serde_json::json!({"op": "measure_control_fusion"}), true)?;
        let response = self.response()?;
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
        self.send(&serde_json::json!({"op": "finish"}), true)?;
        self.require_ack("shot completion")?;
        self.active = false;
        Ok(())
    }

    fn abort(&mut self) {
        if self.active {
            let _ = self.send(&serde_json::json!({"op": "abort"}), true);
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
        let mut replies = String::from("{\"ok\":true}\n");
        for _ in 0..256 {
            replies.push_str("{\"fusion_bit\":1}\n");
        }
        replies.push_str("{\"ok\":true}\n");
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
        assert!(emitted.contains("\"protocol\":\"g-momonados/fibonacci-anyons-v1\""));
        assert!(emitted.contains("\"format\":\"little_endian_bits\""));
        assert!(emitted.contains("\"work_preparation\":\"uniform_residues\""));
        assert!(emitted.contains("\"generator\":1"));
        assert!(emitted.contains("\"op\":\"measure_control_fusion\""));
        assert!(emitted.contains("\"op\":\"finish\""));
    }
}
