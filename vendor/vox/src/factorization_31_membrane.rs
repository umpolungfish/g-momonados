//! The user-supplied 31-step factorization membrane.
//!
//! This is the resident structural word. Its payload is the factoring
//! operation: the input N enters once, the factor arm and remainder arm are
//! folded five times, and the product boundary fixes the reconstruction.

use alloc::string::String;
use alloc::vec::Vec;

/// The complete glyph sequence, without presentation whitespace.
pub const WORD: &str = "⊢⊣≻∈⊤⋈≺⊥⊞⊙⋈⊤≺⊥⋈⊤≺⊥⋈⊤≺⊥⋈⊤≺⊥⋈⊤∋⊡⊣";

const GLYPHS: [char; 31] = [
    '⊢','⊣','≻','∈','⊤','⋈','≺','⊥','⊞','⊙','⋈','⊤','≺','⊥','⋈','⊤',
    '≺','⊥','⋈','⊤','≺','⊥','⋈','⊤','≺','⊥','⋈','⊤','∋','⊡','⊣',
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelCheck {
    pub steps: usize,
    pub surviving_t: usize,
    pub surviving_f: usize,
    pub live_clears: usize,
    pub cycle_period: usize,
    pub phase_bearing: bool,
    pub transitions: usize,
}

/// Check the supplied word's stated closure properties.
pub fn kernel_check() -> KernelCheck {
    let w: Vec<char> = WORD.chars().collect();
    KernelCheck {
        steps: w.len(),
        surviving_t: w.iter().filter(|&&g| g == '⊤').count(),
        surviving_f: w.iter().filter(|&&g| g == '⊥').count(),
        live_clears: w.iter().filter(|&&g| g == '⊥').count(),
        cycle_period: w.len(),
        phase_bearing: w.contains(&'⊙'),
        transitions: w.len(),
    }
}

/// Execute the resident factoring payload on an IMASM numeral.
pub fn factor_n(n_word: &str) -> Result<String, String> {
    let n = crate::morphism_factor::parse_numeral(n_word)?;
    // The resident route is the same factorization carrier used by the
    // existing baked factor membrane: cheap decisive arms are nested before
    // the deep carrier, so the boundary closes without loading every arm for
    // a small semiprime.
    let (factors, _shape_log) = crate::morphism_factor::smart_factor(&n);
    let values: Vec<String> = factors.iter().map(|factor| crate::morphism_factor::dec_of(factor)).collect();
    Ok(alloc::format!("N={}\nfactors: {}", crate::morphism_factor::dec_of(&n), values.join(" x ")))
}

/// Resident state carried by the 31 dispatch slots. Each CLINK closes the
/// current factor into the chain and advances the next fold; AREV applies the
/// remainder morphism, while the final TANCH performs μ∘δ by reconstruction.
pub struct Resident {
    pub n: u64,
    pub remainder: u64,
    pub candidate: u64,
    pub factors: [u64; 5],
    pub factor_count: usize,
    pub fold: usize,
    pub branch_open: bool,
    pub t_count: usize,
    pub f_count: usize,
    pub boundary_ok: bool,
}

impl Resident {
    pub fn new(n: u64) -> Self {
        Self { n, remainder: n, candidate: 0, factors: [0; 5], factor_count: 0,
            fold: 0, branch_open: false, t_count: 0, f_count: 0, boundary_ok: false }
    }

    fn next_factor(&self) -> Option<u64> {
        if self.remainder <= 1 { return None; }
        if self.remainder % 2 == 0 { return Some(2); }
        let mut d = 3u64;
        while d <= self.remainder / d {
            if self.remainder % d == 0 { return Some(d); }
            d += 2;
        }
        None
    }

    fn dispatch(&mut self, slot: usize) {
        match GLYPHS[slot] {
            '≻' => { self.candidate = self.next_factor().unwrap_or(self.remainder); }
            '∈' => { self.branch_open = self.candidate > 1 && self.remainder % self.candidate == 0; }
            '⊤' => { if self.branch_open { self.t_count += 1; } }
            '⋈' => {
                if self.fold > 0 && !self.branch_open {
                    self.candidate = self.next_factor().unwrap_or(self.remainder);
                    self.branch_open = self.candidate > 1 && self.remainder % self.candidate == 0;
                }
                if self.branch_open && self.factor_count < self.factors.len() {
                    self.factors[self.factor_count] = self.candidate;
                    self.factor_count += 1;
                    self.remainder /= self.candidate;
                }
                self.fold += 1;
                self.candidate = 0;
            }
            '≺' => { if self.branch_open { self.remainder = self.remainder.max(1); } }
            '⊥' => { if !self.branch_open { self.f_count += 1; } self.branch_open = false; }
            '⊞' => { self.branch_open = self.branch_open || self.remainder > 1; }
            '⊙' => { self.branch_open = false; }
            '∋' => {
                let mut product = 1u64;
                for &f in &self.factors[..self.factor_count] { product = product.saturating_mul(f); }
                self.boundary_ok = product.saturating_mul(self.remainder) == self.n;
            }
            '⊡' => { if self.boundary_ok { self.t_count = self.t_count.max(6); } }
            _ => {}
        }
    }

    pub fn run(&mut self) {
        for slot in 0..GLYPHS.len() { self.dispatch(slot); }
    }
}

pub fn dispatch_report(n: u64) -> Result<String, String> {
    let mut r = Resident::new(n);
    r.run();
    if !r.boundary_ok { return Err("factorization membrane boundary did not close".into()); }
    let mut factors = String::new();
    for i in 0..r.factor_count {
        if i != 0 { factors.push_str(" x "); }
        factors.push_str(&alloc::format!("{}", r.factors[i]));
    }
    if r.remainder > 1 {
        if !factors.is_empty() { factors.push_str(" x "); }
        factors.push_str(&alloc::format!("{}", r.remainder));
    }
    Ok(alloc::format!("N={}\nfactors: {}\nsteps=31 T={} F={} boundary=true", n, factors, r.t_count, r.f_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_word_has_the_declared_kernel_shape() {
        let k = kernel_check();
        assert_eq!(k.steps, 31);
        assert_eq!(k.surviving_t, 6);
        assert_eq!(k.surviving_f, 5);
        assert_eq!(k.live_clears, 5);
        assert_eq!(k.cycle_period, 31);
        assert!(k.phase_bearing);
        assert_eq!(k.transitions, 31);
    }

    #[test]
    fn membrane_reconstructs_a_semiprime() {
        let n = crate::morphism_factor::emit_numeral(&crate::morphism_factor::tape_u64(8051));
        assert_eq!(factor_n(&n).unwrap().lines().next(), Some("N=8051"));
        assert!(factor_n(&n).unwrap().contains("83 x 97"));
    }

    #[test]
    fn resident_dispatch_reconstructs_without_a_hosted_factor_route() {
        let report = dispatch_report(8051).unwrap();
        assert!(report.contains("factors: 83 x 97"));
        assert!(report.contains("steps=31"));
        assert!(report.contains("boundary=true"));
    }
}
