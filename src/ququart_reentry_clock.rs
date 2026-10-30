//! Retained feedback in runtime SIC labels, checked by retained_runtime_bits.
//! Packed input is state + 16 * retained. The same leaf is lifted by Vox.
#[no_mangle]
#[inline(never)]
pub extern "C" fn ququart_reentry_step(packed: u64) -> u64 {
    let state = packed & 15;
    let retained = (packed >> 4) & 15;
    (state & 6) | ((state & 1) << 3) | ((state & 8) >> 3) | retained
}

#[cfg(feature = "hosted")]
pub struct RuntimeClock {
    retained: u8,
    entry: u64,
    period: u64,
    ticks: [u8; 16],
}

#[cfg(feature = "hosted")]
impl RuntimeClock {
    /// Check the exported table and primitive orbit before accepting a clock.
    pub fn from_record(record: &serde_json::Value) -> Result<Self, String> {
        let field = |key: &str| record[key].as_u64().ok_or_else(|| format!("clock requires {key}"));
        let retained = field("retained")?;
        let seed = field("seed")?;
        let entry = field("entry")?;
        let period = field("period")?;
        if retained >= 16 || seed >= 16 || period == 0 || entry >= 16 || period > 16 - entry {
            return Err("clock bounds differ from native carrier".into());
        }
        let read_array = |key: &str| -> Result<[u8; 16], String> {
            record[key].as_array().ok_or_else(|| format!("missing clock {key}"))?
                .iter().map(|v| v.as_u64().filter(|n| *n < 16).map(|n| n as u8)
                    .ok_or_else(|| format!("invalid clock {key} state")))
                .collect::<Result<Vec<_>, _>>()?.try_into().map_err(|_| format!("clock {key} must contain sixteen states"))
        };
        let table = read_array("table")?;
        let ticks = read_array("ticks")?;
        for state in 0..16 {
            if table[state] as u64 != ququart_reentry_step(state as u64 | (retained << 4)) {
                return Err("clock table differs from compiled retained update".into());
            }
        }
        let mut state = seed as u8;
        let mut seen = [false; 16];
        for tick in 0..entry + period {
            if ticks[tick as usize] != state || seen[state as usize] {
                return Err("clock transient or primitive period differs".into());
            }
            seen[state as usize] = true;
            state = table[state as usize];
        }
        if state != ticks[entry as usize] { return Err("clock does not close".into()); }
        for tick in 0..16u64 {
            let phase = if tick < entry { tick } else { entry + (tick - entry) % period };
            if ticks[tick as usize] != ticks[phase as usize] {
                return Err("cached clock differs after first repeat".into());
            }
        }
        Ok(Self { retained: retained as u8, entry, period, ticks })
    }
    pub fn read(&self, tick: u64) -> u8 {
        let phase = if tick < self.entry { tick } else { self.entry + (tick - self.entry) % self.period };
        self.ticks[phase as usize]
    }
    pub fn step(&self, state: u8) -> u8 {
        ququart_reentry_step(state as u64 | ((self.retained as u64) << 4)) as u8
    }
    pub fn retained(&self) -> u8 { self.retained }
    pub fn entry(&self) -> u64 { self.entry }
    pub fn period(&self) -> u64 { self.period }
}
