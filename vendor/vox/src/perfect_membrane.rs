//! perfect_membrane.rs — the membrane as matched circuitry, not as search.
//!
//! A membrane closes when mu∘delta = id: split the object, transform it, fuse it
//! back, and recover what you started with. The sieve membranes reach that closure
//! by collecting relations until a dependency appears. A PERFECT membrane reaches
//! it by construction: every delta (a fork, FSPLIT ∈) is wired to its own mu (a
//! fuse, FFUSE ∋), so split-then-fuse is the identity by the shape of the circuit.
//!
//! The tower has a depth n >= 2 (two is the base Frobenius cell, one split wired to
//! one fuse) and no maximum. Each level forks down and, through CLINK bridges,
//! reconnects to every deeper level and back, the all-to-all lattice, so no level
//! sits on a single path. The closure auditor `vox::verdict` reads this: matched
//! forks with work in their interior return T, an unmatched fork would return B and
//! an over-fuse F. A perfect membrane returns T at every depth with zero surplus.

use crate::morphism_factor::{cmp, dec_of, mul, trim};
use crate::vox::{
    verdict, AFWD, AREV, CLINK, ENGAGR, EVALF, EVALT, FFUSE, FSPLIT, IFIX, IMSCRIB, TANCH, VINIT,
};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

type Word = Vec<char>;

// ---- executable delta / mu over the bit-tape ----
// The tape is little-endian bits: index i is bit i, EVALF = 1, EVALT = 0.

/// delta: deinterlace the bits into two lanes. Lane 0 takes the even-index bits,
/// lane 1 the odd. This is the split the tower forks on.
fn delta(v: &[char]) -> (Word, Word) {
    let mut a = Word::new();
    let mut b = Word::new();
    for (i, &bit) in v.iter().enumerate() {
        if i % 2 == 0 {
            a.push(bit);
        } else {
            b.push(bit);
        }
    }
    (trim(a), trim(b))
}

/// mu: interlace the two lanes back, the exact inverse of delta. Bit 2j comes from
/// lane 0, bit 2j+1 from lane 1. mu(delta(v)) = v on the nose.
fn mu(a: &[char], b: &[char]) -> Word {
    let n = a.len().max(b.len());
    let mut out = Word::new();
    for j in 0..n {
        out.push(*a.get(j).unwrap_or(&EVALT));
        out.push(*b.get(j).unwrap_or(&EVALT));
    }
    trim(out)
}

/// Run a value through a depth-n perfect membrane and report each stage: the split
/// lanes at every level, the transform carried at the core (here the product of the
/// two deepest lanes, a real computed quantity), and the fuse back up, with the
/// mu∘delta = id recovery checked against the input at every level.
pub fn run(value: &[char], depth: usize) -> String {
    let v = trim(value.to_vec());
    let (recovered, _product, trace) = transit(&v, depth);
    let closed = cmp(&recovered, &v) == core::cmp::Ordering::Equal;
    format!(
        "value in: {}\n{}value out: {}\n  mu∘delta = id : {}\n",
        dec_of(&v),
        trace,
        dec_of(&recovered),
        if closed { "CLOSED (identity recovered)" } else { "LEAK" }
    )
}

/// The transformed object at the core: advance, engage the paradox, imscribe. This
/// is the "transformed" in "mu∘delta = id over a transformed object" — without a
/// real transform the closure is trivial.
fn core() -> Word {
    vec![AFWD, EVALT, AREV, EVALF, ENGAGR, IMSCRIB]
}

/// Depth-n perfect membrane word: VINIT, then n forks each bridged to the deeper
/// levels (delta with its all-to-all cross-link), the transformed core, then n
/// fuses (mu), latched with IFIX and closed with TANCH. Forks and fuses are equal
/// in number, so the auditor sees no surplus and the round trip is the identity.
pub fn perfect_membrane(depth: usize) -> Word {
    let n = depth.max(2);
    let mut w = vec![VINIT];
    for _ in 0..n {
        w.push(FSPLIT); // delta: fork this level
        w.push(CLINK); // bridge it to the deeper levels (the lateral link)
    }
    w.extend(core());
    for _ in 0..n {
        w.push(FFUSE); // mu: fuse the matching level back
    }
    w.push(IFIX);
    w.push(TANCH);
    w
}

/// Read the closure of a depth-n perfect membrane with the instrument: its verdict
/// mark, and the fork/fuse surplus (zero when every delta has its mu).
pub fn report(depth: usize) -> (char, i32, Word) {
    let w = perfect_membrane(depth);
    let of = crate::vox::open_forks(&w);
    (verdict(&w), of.surplus, w)
}

// ---- the operculum: a membrane is a vessel with one puncture ----
//
// A membrane's entry wound is its exit wound: the frame that opens the split is
// the frame that closes the fuse, one opening, not an in-port and an out-port. So
// you do not pass a value through it. You open the operculum, drop the value into
// the sealed interior, close it, run the circuit while it is shut, then open the
// same operculum and lift the result out. The lifecycle below enforces that: you
// cannot run unsealed, cannot deposit once sealed, cannot extract before the run.

/// State of the operculum, the membrane's single lid.
#[derive(PartialEq)]
pub enum Lid {
    Open,
    Sealed,
    Spent,
}

/// A depth-n perfect membrane as a loadable vessel.
pub struct Operculum {
    depth: usize,
    lid: Lid,
    payload: Option<Word>,
    recovered: Option<Word>,
    product: Option<Word>,
}

impl Operculum {
    /// Open a fresh operculum at the given depth, lid up, interior empty.
    pub fn open(depth: usize) -> Self {
        Operculum {
            depth: depth.max(2),
            lid: Lid::Open,
            payload: None,
            recovered: None,
            product: None,
        }
    }

    /// Drop a value into the interior through the open lid.
    pub fn deposit(&mut self, value: &[char]) -> Result<(), &'static str> {
        if self.lid != Lid::Open {
            return Err("operculum is not open");
        }
        self.payload = Some(trim(value.to_vec()));
        Ok(())
    }

    /// Seal the lid. Nothing enters or leaves while it is shut.
    pub fn seal(&mut self) -> Result<(), &'static str> {
        if self.lid != Lid::Open {
            return Err("operculum is not open");
        }
        if self.payload.is_none() {
            return Err("nothing deposited to seal");
        }
        self.lid = Lid::Sealed;
        Ok(())
    }

    /// Run the closed circuit: split the payload down the tower, transform at the
    /// core, fuse it back. Split-then-fuse is the identity, so the interior holds
    /// the recovered value and the transform's product. Only runs while sealed.
    pub fn run(&mut self) -> Result<(), &'static str> {
        if self.lid != Lid::Sealed {
            return Err("operculum must be sealed before it runs");
        }
        let v = self.payload.as_ref().unwrap();
        let (recovered, product, _trace) = transit(v, self.depth);
        self.recovered = Some(recovered);
        self.product = Some(product);
        Ok(())
    }

    /// Open the lid again and lift out what the run left: the recovered value (the
    /// entry, returned through the one puncture) and the core transform's product.
    pub fn extract(&mut self) -> Result<(Word, Word), &'static str> {
        if self.lid != Lid::Sealed || self.recovered.is_none() {
            return Err("nothing to extract; seal and run first");
        }
        self.lid = Lid::Spent;
        Ok((
            self.recovered.take().unwrap(),
            self.product.take().unwrap(),
        ))
    }
}

/// The transit through a sealed depth-n membrane: fork the value down, transform at
/// the core, fuse back. Returns the recovered value (equal to the input by
/// mu∘delta = id), the core transform's product, and a stage trace.
fn transit(value: &[char], depth: usize) -> (Word, Word, String) {
    let n = depth.max(2);
    let v = trim(value.to_vec());
    let mut trace = String::new();
    let mut cur = v.clone();
    let mut siblings: Vec<Word> = Vec::new();
    for level in 1..=n {
        let (a, b) = delta(&cur);
        trace.push_str(&format!("  delta L{level}: lane0={} lane1={}\n", dec_of(&a), dec_of(&b)));
        siblings.push(b);
        cur = a;
    }
    let product = mul(&cur, siblings.last().unwrap());
    trace.push_str(&format!("  core transform: lane product = {}\n", dec_of(&product)));
    for level in (1..=n).rev() {
        let b = siblings.pop().unwrap();
        cur = mu(&cur, &b);
        trace.push_str(&format!("  mu    L{level}: recombined = {}\n", dec_of(&cur)));
    }
    (cur, product, trace)
}

/// Walk the operculum lifecycle on a value and narrate each step, for the CLI.
pub fn operculum_demo(value: &[char], depth: usize) -> String {
    let mut out = String::new();
    let mut op = Operculum::open(depth);
    out.push_str(&format!("open operculum (depth {depth})\n"));
    op.deposit(value).unwrap();
    out.push_str(&format!("deposit: {}\n", dec_of(&trim(value.to_vec()))));
    op.seal().unwrap();
    out.push_str("seal\n");
    op.run().unwrap();
    out.push_str("run (sealed)\n");
    let (recovered, product) = op.extract().unwrap();
    let closed = cmp(&recovered, &trim(value.to_vec())) == core::cmp::Ordering::Equal;
    out.push_str(&format!(
        "open operculum, extract: value {}  transform {}\n  entry wound = exit wound : {}\n",
        dec_of(&recovered),
        dec_of(&product),
        if closed { "CLOSED (recovered through the one puncture)" } else { "LEAK" }
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphism_factor::decimal_to_tape;

    #[test]
    fn operculum_recovers_through_one_puncture() {
        let v = decimal_to_tape("1234567890123456789").unwrap();
        let mut op = Operculum::open(4);
        assert!(op.run().is_err(), "cannot run before sealing");
        op.deposit(&v).unwrap();
        assert!(op.extract().is_err(), "cannot extract before running");
        op.seal().unwrap();
        assert!(op.deposit(&v).is_err(), "cannot deposit once sealed");
        op.run().unwrap();
        let (recovered, _product) = op.extract().unwrap();
        assert_eq!(trim(recovered), trim(v), "the entry must return as the exit");
    }

    #[test]
    fn closes_with_identity_at_every_depth() {
        // mu∘delta = id holds by construction: T verdict, zero surplus, 2..=32.
        for n in 2..=32 {
            let (v, surplus, _) = report(n);
            assert_eq!(surplus, 0, "depth {n}: forks and fuses must match");
            assert_eq!(v, 'T', "depth {n}: perfect membrane must close with work");
        }
    }
}
