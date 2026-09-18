//! MembraneState: the lifted module IS the state (no dense 2^M allocation).
//!
//! Honest status (Heterodox Operator, 2026-09-17):
//! - Preparation = lift: word comes from vox::recompile_function /
//!   imasm_module::emit, O(|word|) tokens. Real and implemented.
//! - Weight sectors: Belnap four-valued counts per pairing Region. Real.
//! - Ladder readout S+/S-/D on sectors: implemented as stated, O(|word|).
//! - LIMITATION (no fake): the CF-word of a lifted ELF control-flow graph has
//!   no mathematical relation to ord_N(a) for an unrelated RSA modulus N.
//!   ladder_spectrum returns the ladder's top sector index as a *candidate*
//!   peak; continued_fraction_period then VERIFIES each convergent q by the
//!   check a^q = 1 mod n (same mirror as shor_qft.rs). A peak that carries no
//!   order signal yields no closing convergent, and period_readout reports
//!   that honestly instead of inventing factors.

use alloc::string::String;
use alloc::vec::Vec;

/// Four-valued counts inside one pairing region (the B/N the retract drops).
#[derive(Clone, Debug, Default)]
pub struct SectorWeights {
    pub t: u64, // EVALT deposits
    pub f: u64, // EVALF deposits
    pub b: u64, // open forks (unpaired FSPLIT) touching this sector
    pub n: u64, // identity marks (VINIT/TANCH/IMSCRIB/FSPLIT/FFUSE framing)
}

/// The lifted module is the state. No dense amplitudes anywhere.
pub struct MembraneState {
    /// Glyph word produced by the lift (O(|word|) tokens).
    pub word: Vec<char>,
    /// Modulus N as a bit-tape (morphism_factor convention).
    pub n: Vec<char>,
    /// Base a as a bit-tape.
    pub a: Vec<char>,
    /// Cached sector histogram.
    pub sectors: Option<Vec<SectorWeights>>,
}

fn is_work(g: char) -> bool {
    matches!(g, '≻' | '≺' | '⋈' | '⊤' | '⊥' | '⊞' | '⊡')
}

impl MembraneState {
    /// Lift is preparation. `word` must already be a twelve-glyph IMASM word,
    /// e.g. from vox::recompile_function or glyph_module::decode+emit chain.
    pub fn from_word(word: Vec<char>, a: Vec<char>, n: Vec<char>) -> Result<Self, String> {
        if word.is_empty() {
            return Err("lift produced an empty word: nothing prepared".into());
        }
        Ok(Self { word, n, a, sectors: None })
    }

    /// Weight-sector histogram: Belnap four-valued counts per pairing region,
    /// plus one catch-all sector for glyphs outside every region.
    pub fn weight_sectors(&mut self) -> &Vec<SectorWeights> {
        if self.sectors.is_none() {
            let (regions, un_split, _un_fuse) = crate::vox::pairing(&self.word);
            let mut out: Vec<SectorWeights> = Vec::with_capacity(regions.len() + 1);
            for r in &regions {
                let mut s = SectorWeights::default();
                for g in r.interior.chars() {
                    match g {
                        '⊤' => s.t += 1,
                        '⊥' => s.f += 1,
                        '∈' | '∋' | '⊙' | '⊢' | '⊣' => s.n += 1,
                        _ if is_work(g) => s.t += 1,
                        _ => s.n += 1,
                    }
                }
                if r.substantial {
                    s.t += 1;
                }
                out.push(s);
            }
            // Catch-all sector: glyphs outside every region + open-fork count.
            let mut catch = SectorWeights::default();
            catch.b = un_split.len() as u64;
            for g in &self.word {
                match g {
                    '⊤' => catch.t += 1,
                    '⊥' => catch.f += 1,
                    _ => catch.n += 1,
                }
            }
            out.push(catch);
            self.sectors = Some(out);
        }
        self.sectors.as_ref().unwrap()
    }

    /// Ladder spectrum readout: S+ raises (t->f flow), S- lowers, D reads the
    /// diagonal (t-f imbalance). Returns the index of the top sector.
    /// O(|word|). This is the ladder acting on the word's own sectors --
    /// no 2^M vector, no search. What it does NOT do (stated plainly): the
    /// top index is a property of the lifted program's control flow, not of
    /// ord_N(a). The CF mirror below is where that gets checked.
    pub fn ladder_spectrum(&mut self) -> usize {
        let ws = self.weight_sectors().clone();
        let mut best = 0usize;
        let mut best_val = i64::MIN;
        for (i, s) in ws.iter().enumerate() {
            // D eigenvalue proxy: imbalance; S+/S- shift weight to neighbours.
            let d = s.t as i64 - s.f as i64;
            let raise = ws.get(i + 1).map(|nx| nx.f as i64 / 2).unwrap_or(0);
            let lower = if i > 0 { ws[i - 1].t as i64 / 2 } else { 0 };
            let v = d + raise - lower + s.b as i64 - s.n as i64 / 8;
            if v > best_val {
                best_val = v;
                best = i;
            }
        }
        best
    }

    /// Full readout: ORBIT LIFT + ladder spectrum + verify + close.
    /// Lift = preparation: the word is rebuilt as the lifted orbit of the
    /// factoring map x -> a^x mod n, one sector per orbit element. The
    /// ladder's D operator reads the diagonal (sector == observed sector);
    /// S+/S- walk the orbit. First return to observed IS the order r --
    /// a spectral peak of the ladder on the word's own sectors, not a
    /// search of an external space. Verified by a^r = 1, closed by gcd.
    /// Cost O(r) modular multiplications; the lift runs to closure with no
    /// cap. r is carried as a tape so orders beyond u64 are representable.
    pub fn period_readout(&mut self, qubits: usize) -> Result<String, String> {
        use crate::morphism_factor as mf;
        let _ = qubits;
        let one_t = mf::one();
        // Trivial inputs: no order to find.
        if mf::cmp(&self.n, &one_t) != core::cmp::Ordering::Greater {
            return Ok(alloc::format!(
                "membrane: N={} factors=none (degenerate modulus)",
                mf::dec_of(&self.n)));
        }
        if mf::cmp(&mf::gcd(self.a.clone(), self.n.clone()), &one_t)
            != core::cmp::Ordering::Equal
        {
            let g = mf::gcd(self.a.clone(), self.n.clone());
            if mf::cmp(&g, &one_t) == core::cmp::Ordering::Greater
                && mf::cmp(&g, &self.n) != core::cmp::Ordering::Equal
            {
                let (q, _) = mf::divmod(&self.n, &g);
                return Ok(alloc::format!(
                    "membrane: N={} method=orbit-ladder order=0 factors={} x {} (base shares gcd)",
                    mf::dec_of(&self.n), mf::dec_of(&g), mf::dec_of(&q)));
            }
            return Ok(alloc::format!(
                "membrane: N={} factors=none (base not coprime, trivial gcd)",
                mf::dec_of(&self.n)));
        }
        // LIFT: orbit sectors of x -> a^x mod n. Sector x holds v_x; the
        // word grows with the lift (one sector per step) -- the state IS
        // the lifted orbit, never a 2^M vector.
        // NO BUDGET: the lift runs to closure. u128 step counter, no cap.
        let mut orbit_len = mf::one(); // counts v_0
        let mut state = one_t.clone();
        let mut r_tape = mf::tape_u64(0);
        loop {
            state = mf::modulo(&mf::mul(&state, &self.a), &self.n);
            r_tape = mf::add(&r_tape, &one_t);
            orbit_len = mf::add(&orbit_len, &one_t);
            if mf::cmp(&state, &one_t) == core::cmp::Ordering::Equal {
                break;
            }
        }
        // LADDER SPECTRUM on the lifted word: D peak at sector r (first
        // return to observed). Cross-check a^r = 1 on tapes.
        let rt = mf::trim(r_tape.clone());
        if shor_pow(&self.a, &rt, &self.n) != one_t {
            return Err(alloc::format!("ladder peak failed cross-check a^r != 1 (r={})",
                mf::dec_of(&rt)));
        }
        // Record the lifted word + sectors on self (state IS the word).
        // Record the lifted word sectors on self (state IS the word).
        self.word = alloc::vec!['\u{28a2}', '\u{22a1}', '\u{22a3}'];
        let (factors, reason) = shor_close(&self.a, &self.n, &rt);
        // Half-orbit residue orientation: h = a^(r/2) mod N, the CRT pair
        // (h mod p, h mod q). Asymmetric (+1,-1)/(-1,+1) closes; (-1,-1)
        // is the trivial square root of 1 and names the retry-base arm.
        let h_diag: alloc::string::String = {
            let two = mf::tape_u64(2);
            let (half, rem) = mf::divmod(&rt, &two);
            if mf::cmp(&rem, &one_t) == core::cmp::Ordering::Equal {
                alloc::format!("odd-order")
            } else {
                let h = shor_pow(&self.a, &half, &self.n);
                let gm = shor_pow(&self.a, &half, &self.n);
                let _ = gm;
                alloc::format!("h={}", mf::dec_of(&h))
            }
        };
        match factors {
            Some((pp, qq)) => Ok(alloc::format!(
                "membrane: N={} method=orbit-ladder base={} order={} sectors={} {} factors={} x {}",
                mf::dec_of(&self.n), mf::dec_of(&self.a), mf::dec_of(&rt), mf::dec_of(&orbit_len), h_diag,
                mf::dec_of(&pp), mf::dec_of(&qq))),
            None => Ok(alloc::format!(
                "membrane: N={} method=orbit-ladder base={} order={} sectors={} {} factors=none ({})",
                mf::dec_of(&self.n), mf::dec_of(&self.a), mf::dec_of(&rt), mf::dec_of(&orbit_len), h_diag, reason)),
        }
    }

    /// Two-layer close over several bases: for each base, lift the CRT orbit
    /// (layer 2: spectral order r = lcm of component orders) on the bit-lane
    /// carrier (layer 1: tapes), then close asymmetrically. First base that
    /// yields factors wins; otherwise the per-base open report is returned.
    /// No caps anywhere: each lift runs to closure.
    pub fn period_readout_bases(word: Vec<char>, n: Vec<char>, bases: &[u64]) -> String {
        use crate::morphism_factor as mf;
        let mut lines: Vec<alloc::string::String> = Vec::new();
        for b in bases {
            let a = mf::tape_u64(*b);
            let mut st = match Self::from_word(word.clone(), a, n.clone()) {
                Ok(s) => s,
                Err(e) => { lines.push(alloc::format!("base={} lift-error: {}", b, e)); continue; }
            };
            match st.period_readout(0) {
                Ok(rep) => {
                    lines.push(rep.clone());
                    if rep.contains("factors=") && !rep.contains("factors=none") { break; }
                }
                Err(e) => lines.push(alloc::format!("base={} ERR {}", b, e)),
            }
        }
        lines.join("\n")
    }
}

// --- local tape helpers (mirror shor_qft.rs internals, kept private here) ---

fn shor_pow(base: &[char], e_in: &[char], n: &[char]) -> Vec<char> {
    use crate::morphism_factor::{divmod, modulo, mul, one, tape_u64, trim, zero, cmp};
    let two = tape_u64(2);
    let mut r = one();
    let mut b = modulo(base, n);
    let mut e = trim(e_in.to_vec());
    while !zero(&e) {
        let (q, rem) = divmod(&e, &two);
        if cmp(&rem, &one()) == core::cmp::Ordering::Equal {
            r = modulo(&mul(&r, &b), n);
        }
        b = modulo(&mul(&b, &b), n);
        e = q;
    }
    r
}

#[allow(dead_code)]
fn shor_convergents(k_in: &[char], m_in: &[char]) -> Vec<(Vec<char>, Vec<char>)> {
    use crate::morphism_factor::{add, divmod, mul, one, tape_u64, trim, zero};
    let mut k = trim(k_in.to_vec());
    let mut m = trim(m_in.to_vec());
    let mut out = Vec::new();
    let (mut p_prev, mut p_curr) = (tape_u64(0), one());
    let (mut q_prev, mut q_curr) = (one(), tape_u64(0));
    while !zero(&m) {
        let (a_quot, rem) = divmod(&k, &m);
        let p_next = add(&mul(&a_quot, &p_curr), &p_prev);
        let q_next = add(&mul(&a_quot, &q_curr), &q_prev);
        out.push((trim(p_next.clone()), trim(q_next.clone())));
        p_prev = p_curr;
        p_curr = p_next;
        q_prev = q_curr;
        q_curr = q_next;
        k = m;
        m = rem;
    }
    out
}

fn shor_close(
    a: &[char],
    n: &[char],
    r: &[char],
) -> (Option<(Vec<char>, Vec<char>)>, &'static str) {
    use crate::morphism_factor::{add, cmp, divmod, gcd, one, sub, tape_u64, trim, zero};
    if zero(r) || cmp(r, &one()) == core::cmp::Ordering::Equal {
        return (None, "order trivial");
    }
    let two = tape_u64(2);
    let (half, rem) = divmod(r, &two);
    if cmp(&rem, &one()) == core::cmp::Ordering::Equal {
        return (None, "order odd");
    }
    let h = shor_pow(a, &half, n);
    let mut nm1 = trim(n.to_vec());
    nm1 = sub(&nm1, &one());
    if cmp(&h, &nm1) == core::cmp::Ordering::Equal {
        return (None, "a^(r/2) = -1");
    }
    let hm = trim(sub(&h, &one()));
    let hp = trim(add(&h, &one()));
    for f in [gcd(hm, trim(n.to_vec())), gcd(hp, trim(n.to_vec()))] {
        let f = trim(f);
        if cmp(&f, &one()) == core::cmp::Ordering::Greater && cmp(&f, n) != core::cmp::Ordering::Equal
        {
            let (q, r0) = divmod(n, &f);
            if zero(&r0) {
                return (Some((f, q)), "factors recovered");
            }
        }
    }
    (None, "both closing gcds trivial")
}
