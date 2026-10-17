//! Resident SIXTEEN_3 operations in Lean's membership order [N,T,F,B].
//! Definitions: p4rakernel/src/Lean/Sixteen3.lean. This carrier is distinct
//! from the IMASM constructive register's [T,F,t,f] coordinate convention.
use crate::word_tape::WordTape;
use vox_core::dialectic_lean::ResidentY;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sixteen3(u8);

#[derive(Clone, Copy, Debug)]
pub enum Order {
    Information,
    Truth,
    Falsity,
}

impl Sixteen3 {
    pub fn from_mask(mask: u8) -> Result<Self, String> {
        if mask < 16 {
            Ok(Self(mask))
        } else {
            Err("SIXTEEN_3 mask exceeds 15".into())
        }
    }
    pub fn mask(self) -> u8 {
        self.0
    }
    pub fn asserts_true(self) -> bool {
        self.0 & 10 != 0
    }
    pub fn asserts_false(self) -> bool {
        self.0 & 12 != 0
    }
    pub fn join(self, other: Self, order: Order) -> Self {
        let positive = match order {
            Order::Information => 15,
            Order::Truth => 10,
            Order::Falsity => 12,
        };
        Self(((self.0 | other.0) & positive) | ((self.0 & other.0) & (15 ^ positive)))
    }
    pub fn meet(self, other: Self, order: Order) -> Self {
        let positive = match order {
            Order::Information => 15,
            Order::Truth => 10,
            Order::Falsity => 12,
        };
        Self(((self.0 & other.0) & positive) | ((self.0 | other.0) & (15 ^ positive)))
    }
    pub fn le(self, other: Self, order: Order) -> bool {
        self.join(other, order) == other
    }
}

/// Verify monotonicity, then obtain both information-order fixed-point bounds.
/// Enumeration is over the complete finite carrier, with no execution deadline.
pub fn fixed_bounds(table: &[Sixteen3; 16]) -> Result<(Sixteen3, Sixteen3), String> {
    for x in 0..16 {
        for y in 0..16 {
            if Sixteen3(x).le(Sixteen3(y), Order::Information)
                && !table[x as usize].le(table[y as usize], Order::Information)
            {
                return Err("SIXTEEN_3 table is not information-monotone".into());
            }
        }
    }
    let iterate = |mut point: Sixteen3| loop {
        let next = table[point.0 as usize];
        if next == point {
            return point;
        }
        point = next;
    };
    Ok((iterate(Sixteen3(0)), iterate(Sixteen3(15))))
}

/// Execute the retained Curry Y reduction and check its constant-six carrier.
pub fn admit() -> Result<Sixteen3, String> {
    let value = Sixteen3(6);
    let mut y = ResidentY::from_certified_program(ResidentY::certified_program())?;
    let mut observed = None;
    if !y.advance(|| {
        observed = Some(value);
        Ok(true)
    })? || observed != Some(value)
    {
        return Err("resident Y did not return its carried evidence".into());
    }
    let (lower, upper) = fixed_bounds(&[value; 16])?;
    if lower != value || upper != value || y.calls != 1 || y.contractions == 0 {
        return Err("resident recursive carrier did not close".into());
    }
    Ok(value)
}

/// Exact product evidence, retaining the native contradictory B membership.
/// A rejected product has falsity evidence rather than a claimed closure.
pub fn product_evidence(
    source: &WordTape,
    left: &WordTape,
    right: &WordTape,
) -> Result<Sixteen3, String> {
    let one = WordTape::one();
    let valid = left > &one && right > &one && left.mul(right) == *source;
    let evidence = Sixteen3(if valid { 8 } else { 4 });
    let table = core::array::from_fn(|i| Sixteen3(i as u8).join(evidence, Order::Information));
    let (lower, _) = fixed_bounds(&table)?;
    if lower != evidence {
        return Err("product evidence lost its information boundary".into());
    }
    Ok(evidence)
}

/// Clock and continuation share one resident owner through open and closed returns.
pub struct ResidentClock {
    clock: crate::ququart_reentry_clock::RuntimeClock,
    state: u8,
    ticks: u64,
    forward_calls: u64,
    reverse_calls: u64,
}
impl ResidentClock {
    pub fn tick(&mut self) -> Result<bool, String> {
        let next_tick = self.ticks.checked_add(1).ok_or("resident clock overflow")?;
        let next = self.clock.step(self.state);
        if next != self.clock.read(next_tick) {
            return Err("resident clock differs from its retained orbit".into());
        }
        let reverse = next & 1 != 0;
        let calls = if reverse {
            &mut self.reverse_calls
        } else {
            &mut self.forward_calls
        };
        *calls = calls.checked_add(1).ok_or("resident clock call overflow")?;
        self.state = next;
        self.ticks = next_tick;
        Ok(reverse)
    }
}

pub struct ResidentContinuation {
    y: ResidentY,
    clock: ResidentClock,
    record: serde_json::Value,
}
impl ResidentContinuation {
    pub fn from_record(record: &serde_json::Value) -> Result<Self, String> {
        let clock_record = record.get("clock").ok_or("missing retained clock")?;
        let clock = crate::ququart_reentry_clock::RuntimeClock::from_record(clock_record)?;
        let seed = clock_record["seed"].as_u64().ok_or("missing clock seed")? as u8;
        let program = match record.get("Y_program") {
            Some(value) => value
                .as_str()
                .ok_or("Y_program must be a retained program string")?,
            None => ResidentY::certified_program(),
        };
        // Named historical records carry a complete clock and the same Y
        // continuation. Recognize their ancestry without spawning a process.
        if record
            .get("y_combinator_func")
            .is_some_and(|v| v != "resident_y_combinator")
            || record
                .get("clock_func")
                .is_some_and(|v| v != "resident_clock")
        {
            return Err("unknown resident continuation entry".into());
        }
        let y = ResidentY::from_certified_program(program)?;
        let mut retained = record.clone();
        retained["Y_program"] = program.into();
        retained["execution"] = "resident-native".into();
        Ok(Self {
            y,
            record: retained,
            clock: ResidentClock {
                clock,
                state: seed,
                ticks: 0,
                forward_calls: 0,
                reverse_calls: 0,
            },
        })
    }
    pub fn advance(
        &mut self,
        mut work: impl FnMut(&mut ResidentClock) -> Result<bool, String>,
    ) -> Result<bool, String> {
        self.y.advance(|| work(&mut self.clock))
    }
    pub fn record(&self) -> &serde_json::Value {
        &self.record
    }
    pub fn operations(&self) -> (u64, u64, u64, u64, u64) {
        (
            self.y.contractions,
            self.y.calls,
            self.clock.ticks,
            self.clock.forward_calls,
            self.clock.reverse_calls,
        )
    }
}
