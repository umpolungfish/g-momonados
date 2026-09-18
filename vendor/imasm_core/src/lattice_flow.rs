//! Lattice cycling and weight flow over an IMASM word, in the kernel.
//!
//! A word is a ring and ROTAT is the cyclic shift, so every rotation is the
//! same object. The verdict and the topology hold across the whole orbit; the
//! FINAL REGISTER does not. That makes the phase the only handle on where a
//! word comes to rest, and `cycle_report` prints the map from cut to landing
//! register so the handle can be read rather than guessed.
//!
//! `weight_report` answers the other half. The trilattice machine holds each
//! open fork as a set and closes it with a union, so a finished walk knows
//! WHICH base values were touched and nothing else: not how many times, not by
//! which arm, not whether a value reached the end or was destroyed and restored
//! on the way. This walks the same rules while counting, so the movement is
//! visible. Weight banked in a frame survives a clear that empties the
//! register; weight left in the open does not.
//!
//! The lift of OR to weights is MAX, not sum. Adding would count each deposit
//! twice, once landing in the register and again when its frame closed; under
//! max the fuse RESTORES what a clear destroyed and leaves the rest alone, and
//! at weights zero and one the accounting reduces to the set semantics exactly.
//!
//! Two movements carry no weight at all and are reported because they are
//! otherwise invisible in a final register:
//!
//!   SEED   AFWD and IMSCRIB put T into an empty register directly, so a walk
//!          can land in T having carried nothing
//!   INERT  after IFIX every token but IFIX and IMSCRIB is a no-op, so a word
//!          can be almost entirely inert


/// The reports were written straight to the kernel console with `sprintln!`,
/// which is why the host `ask` binary could not use them. They build a String
/// now and the kernel prints it; `sw!` is the one-line shim that made the move
/// mechanical rather than a rewrite.
macro_rules! sw {
    ($o:expr) => {{ $o.push('\n'); }};
    ($o:expr, $($t:tt)*) => {{
        use core::fmt::Write as _;
        let _ = write!($o, $($t)*);
        $o.push('\n');
    }};
}

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use crate::imasm16_3::{parse_glyph_word, run_word_register, tri_ancestral_verdict, Token16_3};


/// Bring a word onto the current alphabet before the core reads it.
///
/// The retired spellings are still in every stored word and in anything copied
/// out of an older report, and the tensor forms turn up where a fork and a fuse
/// were written as ⊗ and ⊕. Translating is what lets those load; dropping them
/// would read the word as shorter than it is and change its verdict.
pub fn normalize(word: &str) -> String {
    let mut out = String::new();
    for c in word.chars() {
        match c {
            '◇' | '⊗' => out.push('∈'),
            '●' | '⊕' => out.push('∋'),
            '=' | '═' => out.push('⋈'),
            '+' => out.push('⊤'),
            '×' => out.push('⊥'),
            '¬' => out.push('⊡'),
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}

fn render(steps: &[Token16_3]) -> String {
    let mut s = String::new();
    for t in steps { s.push(t.glyph()); }
    s
}

/// The landing register at every cut of the word's rotation orbit, in order
/// k=0..period. Separated from `cycle_report` the same way `banked_walk` sits
/// apart from `banked_report`: a caller that needs the verdict, not the
/// paragraph, walks the orbit once and reads the list.
pub fn cycle_landings(word: &str) -> Option<Vec<String>> {
    let steps = parse_glyph_word(&normalize(word));
    let n = steps.len();
    if n == 0 { return None; }
    let mut landings = Vec::with_capacity(n);
    for k in 0..n {
        let mut rot: Vec<Token16_3> = Vec::with_capacity(n);
        for i in 0..n { rot.push(steps[(i + k) % n]); }
        landings.push(run_word_register(&rot));
    }
    Some(landings)
}

/// The tri-ancestral close condition (`imasm16_3::tri_ancestral_verdict`) on
/// the word AS A LOOP — the Grammar's own existing T/N/B/F verdict, not a
/// reduction invented on top of the register content. Pairing is cyclic
/// (every FSPLIT3 sought against every FFUSE3 around the ring, not linearly),
/// so this already reads the whole word in one call; it is not one cut
/// among many the way `cycle_landings`' entries are.
///
///   T — every fork pairs with a fuse around the cycle, and real work (a
///       WORK opcode) ran inside at least one paired region.
///   N — paired, but no work ran anywhere — void, verifies nothing.
///   B — a fork has no fuse to pair with around the cycle — left open.
///   F — a fuse has no fork to pair with — ill-typed.
pub fn tri_ancestral_word_verdict(word: &str) -> Option<char> {
    let steps = parse_glyph_word(&normalize(word));
    if steps.is_empty() { return None; }
    Some(tri_ancestral_verdict(&steps).0)
}

pub fn cycle_report(word: &str) -> String {
    let mut out = String::new();
    let steps = parse_glyph_word(&normalize(word));
    let n = steps.len();
    if n == 0 {
        sw!(out, "  no IMASM glyphs in that word");
        return out;
    }
    sw!(out, "word   : {}   period {}", render(&steps), n);
    sw!(out, "   {:>3}  {:<6} {:<8} word", "k", "final", "verdict");

    let mut finals: Vec<(String, usize)> = Vec::new();
    for k in 0..n {
        let mut rot: Vec<Token16_3> = Vec::with_capacity(n);
        for i in 0..n { rot.push(steps[(i + k) % n]); }
        let reg = run_word_register(&rot);
        let (v, _) = tri_ancestral_verdict(&rot);
        sw!(out, "   {:>3}  {:<6} {:<8} {}", k, reg, v, render(&rot));
        finals.push((reg, k));
    }

    // The map the whole thing exists for: which cut lands you where.
    sw!(out, "");
    sw!(out, "  landing register by cut:");
    let mut seen: Vec<String> = Vec::new();
    for (reg, _) in finals.iter() {
        if !seen.iter().any(|s| s == reg) { seen.push(reg.clone()); }
    }
    for reg in seen.iter() {
        let mut ks = String::new();
        for (r, k) in finals.iter() {
            if r == reg {
                if !ks.is_empty() { ks.push_str(", "); }
                ks.push_str(&format!("{}", k));
            }
        }
        sw!(out, "    {:<6} at k = {}", reg, ks);
    }
    let distinct = seen.len();
    if distinct == 1 {
        sw!(out, "  final register is INVARIANT under ROTAT here");
    } else {
        sw!(out, "  final register is PHASE-BEARING: {} distinct landings", distinct);
    }
    out
}


/// FRAMES — which readings survive rotation and which are artifacts of the cut.
///
/// A word is a ring. We evaluate ONE cut of it and take what we see there for a
/// property of the word, but every rotation is the same object and every frame is
/// equally available to be read. So a reported quantity is one of two things, and
/// the report must say which:
///
///   INVARIANT   true of the word — it reads the same from every frame
///   FRAME-BOUND it reads the cut, not the word; quoting it without the frame is
///               quoting an accident of where the ring was opened
///
/// This walks the whole orbit and partitions the readings. Anything landing in the
/// second column is still real — it is just a fact about a frame, and the frame has
/// to travel with it.
pub fn frames_report(word: &str) -> String {
    let mut out = String::new();
    let steps = parse_glyph_word(&normalize(word));
    let n = steps.len();
    if n == 0 {
        sw!(out, "  no IMASM glyphs in that word");
        return out;
    }
    sw!(out, "word   : {}   period {}   ({} frames, all equally readable)", render(&steps), n, n);

    let mut verdicts: Vec<char> = Vec::new();
    let mut finals: Vec<String> = Vec::new();
    let mut firsts: Vec<char> = Vec::new();
    for k in 0..n {
        let mut rot: Vec<Token16_3> = Vec::with_capacity(n);
        for i in 0..n { rot.push(steps[(i + k) % n]); }
        let (v, _) = tri_ancestral_verdict(&rot);
        verdicts.push(v);
        finals.push(run_word_register(&rot));
        firsts.push(rot[0].glyph());
    }
    let uniq_v = {
        let mut u: Vec<char> = Vec::new();
        for v in verdicts.iter() { if !u.contains(v) { u.push(*v); } }
        u
    };
    let uniq_f = {
        let mut u: Vec<String> = Vec::new();
        for f in finals.iter() { if !u.iter().any(|x| x == f) { u.push(f.clone()); } }
        u
    };
    let uniq_first = {
        let mut u: Vec<char> = Vec::new();
        for c in firsts.iter() { if !u.contains(c) { u.push(*c); } }
        u
    };

    sw!(out, "");
    sw!(out, "  INVARIANT — true of the word, readable from any frame:");
    sw!(out, "    period                {}", n);
    sw!(out, "    glyph multiset        {} distinct over {} positions", {
        let mut u: Vec<char> = Vec::new();
        for t in steps.iter() { let g = t.glyph(); if !u.contains(&g) { u.push(g); } }
        u.len()
    }, n);
    if uniq_v.len() == 1 {
        sw!(out, "    verdict               {}", uniq_v[0]);
    }
    sw!(out, "    ring transitions      counted with the closing edge, so rotation cannot move them");

    sw!(out, "");
    sw!(out, "  FRAME-BOUND — reads the cut, and must be quoted with it:");
    if uniq_v.len() > 1 {
        sw!(out, "    verdict               {} distinct over the orbit — NOT a property of the word", uniq_v.len());
    }
    sw!(out, "    final register        {} distinct landing(s)", uniq_f.len());
    for reg in uniq_f.iter() {
        let mut ks = String::new();
        for (k, f) in finals.iter().enumerate() {
            if f == reg {
                if !ks.is_empty() { ks.push_str(", "); }
                ks.push_str(&format!("{}", k));
            }
        }
        sw!(out, "      {:<6} at k = {}", reg, ks);
    }
    sw!(out, "    opening glyph         {} distinct — every frame opens on a different mark", uniq_first.len());
    sw!(out, "    absolute position     any row/tier/parity reading; one rotation moves every value");

    sw!(out, "");
    if uniq_f.len() == 1 {
        sw!(out, "  This word is FRAME-FREE in its landing: every cut comes to rest in the same");
        sw!(out, "  register, so the evaluated frame carries no privilege over the others.");
    } else {
        sw!(out, "  This word is PHASE-BEARING: {} of its {} frames disagree about where it rests.", uniq_f.len(), n);
        sw!(out, "  The frame you evaluated is one of them and is not the word's own answer.");
    }
    out
}

/// Opcode-to-opcode transitions counted ON THE RING.
///
/// A word is a cycle and ROTAT is the cyclic shift, so a word of length n has n
/// transitions, not n-1. The one a linear read drops is the wrap from the last
/// opcode back to the first, and in IMASM that is overwhelmingly TANCH -> VINIT:
/// the anchor returning to the source. Across k programs a linear read loses
/// exactly k such edges, and a table built without them can show a rule as
/// universal that the closing edges break.
pub fn transitions_report(word: &str) -> String {
    let mut out = String::new();
    let steps = parse_glyph_word(&normalize(word));
    let n = steps.len();
    if n == 0 { sw!(out, "  no IMASM glyphs in that word"); return out; }
    sw!(out, "word   : {}   length {}", render(&steps), n);
    sw!(out, "  ring transitions   : {}", n);
    sw!(out, "  linear would give  : {}   (drops the closing edge)", n - 1);
    sw!(out, "  closing edge       : {} -> {}",
              steps[n - 1].glyph(), steps[0].glyph());
    sw!(out, "");
    // count them, most frequent first, without allocating a map
    let mut seen: Vec<((char, char), u32)> = Vec::new();
    for i in 0..n {
        let key = (steps[i].glyph(), steps[(i + 1) % n].glyph());
        if let Some(e) = seen.iter_mut().find(|e| e.0 == key) { e.1 += 1; }
        else { seen.push((key, 1)); }
    }
    seen.sort_by(|a, b| b.1.cmp(&a.1));
    sw!(out, "  transitions:");
    for ((a, b), c) in seen.iter() {
        sw!(out, "    {} -> {}   {}", a, b, c);
    }
    sw!(out, "");
    sw!(out, "  Anything read from ABSOLUTE position on a ring measures the cut,");
    sw!(out, "  not the word: matrix rows, tetraktys tiers, odd against even.");
    sw!(out, "  One rotation moves every value into a different row.");
    out
}

/// Was anything counted, then cleared with nothing banked behind it?
///
/// AREV empties the register and leaves open frames alone, so a result fused
/// back to depth zero is exposed to the next reversal, while the same result
/// held one level up survives it. A program that establishes something, then
/// reverses, then bounds must open the region that will HOLD the result before
/// the region that COMPUTES it, and close them in that order.
/// What the banking walk found. Separated from the printing so a caller can
/// test a word instead of reading about it -- `insert` sweeps hundreds of
/// candidates and needs the verdict, not the paragraph. One walk, two callers.
pub struct Banked {
    pub exposed: Vec<(usize, char, u32)>,
    pub live_clears: u32,
    pub deposits: u32,
    pub inert: u32,
    pub reg: [u32; 4],
}

impl Banked {
    /// Held across every clear that actually fired. Vacuous words are not OK:
    /// nothing was ever at risk, so nothing was held.
    pub fn holds(&self) -> bool { self.exposed.is_empty() && self.live_clears > 0 }
    pub fn vacuous(&self) -> bool { self.exposed.is_empty() && self.live_clears == 0 }
}

pub fn banked_walk(word: &str) -> Option<Banked> {
    let steps = parse_glyph_word(&normalize(word));
    if steps.is_empty() { return None; }
    let mut reg = [0u32; 4];
    let mut frames: Vec<[u32; 4]> = Vec::new();
    let mut fixed = false;
    let mut exposed: Vec<(usize, char, u32)> = Vec::new();
    let mut live_clears = 0u32;
    let mut inert = 0u32;
    let mut deposits = 0u32;

    for (i, t) in steps.iter().enumerate() {
        if fixed && !matches!(t, Token16_3::Ifix | Token16_3::Imscrib) { inert += 1; continue; }
        match t {
            Token16_3::Fsplit3 => frames.push([0; 4]),
            Token16_3::Ffuse3 => {
                if let Some(closed) = frames.pop() {
                    for j in 0..4 {
                        if closed[j] > reg[j] { reg[j] = closed[j]; }
                        if let Some(o) = frames.last_mut() {
                            if closed[j] > o[j] { o[j] = closed[j]; }
                        }
                    }
                }
            }
            Token16_3::Arev | Token16_3::Vinit => {
                let lost: u32 = reg.iter().sum();
                let banked: u32 = frames.iter().map(|f| f.iter().sum::<u32>()).sum();
                if lost > 0 {
                    live_clears += 1;
                    if banked == 0 { exposed.push((i + 1, t.glyph(), lost)); }
                }
                reg = [0; 4];
                if matches!(t, Token16_3::Vinit) { frames.clear(); }
            }
            Token16_3::Ifix => fixed = true,
            _ => {
                let touched: &[usize] = match t {
                    Token16_3::Evalt => &[0],
                    Token16_3::Evalf => &[1],
                    Token16_3::Evali => &[2, 3],
                    _ => &[],
                };
                if !touched.is_empty() { deposits += 1; }
                for &j in touched {
                    reg[j] += 1;
                    if let Some(f) = frames.last_mut() { f[j] += 1; }
                }
            }
        }
    }

    Some(Banked { exposed, live_clears, deposits, inert, reg })
}

pub fn banked_report(word: &str) -> String {
    let mut out = String::new();
    let steps = parse_glyph_word(&normalize(word));
    let b = match banked_walk(word) {
        Some(b) => b,
        None => { sw!(out, "  no IMASM glyphs in that word"); return out; }
    };
    let (exposed, live_clears, deposits, inert, reg) =
        (b.exposed, b.live_clears, b.deposits, b.inert, b.reg);

    sw!(out, "word   : {}", render(&steps));
    if exposed.is_empty() && live_clears == 0 {
        // Passing because nothing was ever at risk is not the same as passing
        // because the frame held.
        sw!(out, "  VACUOUS — no clear ever fired against a live register");
        sw!(out, "    {} deposit(s), {} step(s) inert after a fixation", deposits, inert);
    } else if exposed.is_empty() {
        sw!(out, "  OK — weight survived {} live clear(s) by being banked", live_clears);
        // The second, independent fact. Banking asks whether a frame was open
        // at the clear; surplus asks where the splits fell between deposits of
        // the same value. A word can bank correctly and still lose a count.
        let mut surplus = 0u32;
        for j in 0..4 { if reg[j] > 1 { surplus += reg[j] - 1; } }
        if surplus > 0 {
            sw!(out, "    up to {} unit(s) of repeat deposit may be flattened by a", surplus);
            sw!(out, "    fold between sibling regions: the fold keeps the larger,");
            sw!(out, "    not the sum. Deposits in ONE region keep both.");
        }
    } else {
        let total: u32 = exposed.iter().map(|e| e.2).sum();
        sw!(out, "  {} unit(s) cleared with nothing banked behind them:", total);
        for (step, g, w) in exposed.iter() {
            sw!(out, "    step {} {} cleared {} with nothing behind it", step, g, w);
        }
        sw!(out, "  open the region that HOLDS the result before the region that");
        sw!(out, "  COMPUTES it, and close them in that order.");
    }
    out
}

/// A candidate must hold on THREE different instruments, not two.
/// `banked_walk` asks whether weight cleared in the open. `tri_ancestral_word_verdict`
/// asks whether the fork/fuse pairing closes over real work. Neither of
/// those asks whether the wiring itself is legal -- `check::word_verdict`'s
/// `Graph::validate()` does, and a candidate can pass the first two while
/// failing it. Caught live on gpu_native_build_of_momonados's own repaired
/// word: inserting `⊢` (VINIT, zero in-arity by contract) between `⊞` and
/// `∋` fixed the fork/fuse count for tri-ancestral, but the chain gives that
/// node an in-edge from `⊞` regardless, which `validate()` flags as
/// "in-degree 1 > arity_in 0" -- a real grammar violation the first two
/// instruments cannot see, because neither of them asks about arity.
/// `insert_report`/`repair_count` used to accept that candidate as holding.
/// A closure carrying `⊞` legitimately reads B on `check` (paradox held,
/// not a clean T) -- that is not a failure here, only F (validate() non-empty)
/// disqualifies a candidate.
pub fn candidate_holds(cand: &str) -> bool {
    match banked_walk(cand) {
        Some(b) if b.holds() => {}
        _ => return false,
    }
    if tri_ancestral_word_verdict(cand) != Some('T') {
        return false;
    }
    let tokens: Option<Vec<crate::classic::Token>> = cand
        .chars()
        .map(|c| {
            let mut buf = [0u8; 4];
            crate::classic::Token::parse(c.encode_utf8(&mut buf))
        })
        .collect();
    match tokens {
        Some(toks) => {
            let pairs = crate::check::match_pairs(&toks);
            let g = crate::check::from_sequence(&toks, &pairs);
            g.validate().is_empty()
        }
        None => false,
    }
}

/// Every single-glyph insertion that turns an exposed word into one that
/// both holds (banked) and still closes (tri-ancestral T). A word that
/// loses weight is not usually rewritten; it is repaired, and the repair
/// is almost always one glyph in the right place. Rather than reason
/// about which, this walks all twelve glyphs at every position and reports
/// the ones that pass both checks. The search is small -- twelve times
/// length-plus-one -- and exact, so there is nothing to infer.
pub fn insert_report(word: &str) -> String {
    let mut out = String::new();
    let steps = parse_glyph_word(&normalize(word));
    if steps.is_empty() { sw!(out, "  no IMASM glyphs in that word"); return out; }
    let base = render(&steps);
    let n = steps.len();

    sw!(out, "word   : {}   length {}", base, n);
    match banked_walk(&base) {
        Some(b) if b.holds()   => { sw!(out, "  already holds — nothing to repair"); return out; }
        Some(b) if b.vacuous() => sw!(out, "  vacuous: no clear ever fired, so nothing is at risk"),
        Some(b) => {
            let lost: u32 = b.exposed.iter().map(|e| e.2).sum();
            sw!(out, "  exposed: {} unit(s) cleared with nothing banked", lost);
        }
        None => return out,
    }

    let glyphs = ['⊢', '⊙', '∈', '∋', '⊤', '⊥', '≻', '≺', '⋈', '⊞', '⊡', '⊣'];
    let chars: Vec<char> = base.chars().collect();

    // Distinct words, not distinct sites. Inserting a glyph beside an identical
    // one yields the same word from two positions, and counting both would
    // report a repair twice and overstate how many ways out there are.
    let mut seen: Vec<String> = Vec::new();
    let mut tried = 0u32;

    sw!(out, "  insertions that hold:");
    for pos in 0..=n {
        for g in glyphs.iter() {
            let mut cand = String::new();
            for (k, c) in chars.iter().enumerate() {
                if k == pos { cand.push(*g); }
                cand.push(*c);
            }
            if pos == n { cand.push(*g); }
            if seen.iter().any(|w| w == &cand) { continue; }
            tried += 1;
            if candidate_holds(&cand) {
                sw!(out, "    {} at {:>2}   {}", g, pos, cand);
                seen.push(cand);
            }
        }
    }
    let found = seen.len();
    if found == 0 {
        sw!(out, "    none — no single glyph repairs this word without breaking tri-ancestral closure");
    } else {
        sw!(out, "  {} distinct word(s) hold (banked AND tri-ancestral T), of {} tried", found, tried);
    }
    out
}

/// How many distinct one-glyph insertions make `base` hold, without printing.
pub fn repair_count(base: &str) -> usize {
    let glyphs = ['⊢', '⊙', '∈', '∋', '⊤', '⊥', '≻', '≺', '⋈', '⊞', '⊡', '⊣'];
    let chars: Vec<char> = base.chars().collect();
    let n = chars.len();
    let mut seen: Vec<String> = Vec::new();
    for pos in 0..=n {
        for g in glyphs.iter() {
            let mut cand = String::new();
            for (k, c) in chars.iter().enumerate() {
                if k == pos { cand.push(*g); }
                cand.push(*c);
            }
            if pos == n { cand.push(*g); }
            if seen.iter().any(|w| w == &cand) { continue; }
            if candidate_holds(&cand) { seen.push(cand); }
        }
    }
    seen.len()
}


/// Count what the union throws away.
pub fn weight_report(word: &str) -> String {
    let mut out = String::new();
    let steps = parse_glyph_word(&normalize(word));
    if steps.is_empty() {
        sw!(out, "  no IMASM glyphs in that word");
        return out;
    }
    sw!(out, "word   : {}", render(&steps));

    // Base values are indexed T, F, t, f throughout.
    const NAMES: [&str; 4] = ["T", "F", "t", "f"];
    let mut reg = [0u32; 4];
    let mut frames: Vec<[u32; 4]> = Vec::new();
    let (mut deposits, mut cleared, mut restored, mut seeded, mut inert) =
        (0u32, 0u32, 0u32, 0u32, 0u32);
    let mut fixed = false;
    let mut nonempty = false;

    sw!(out, "  movement:");
    for (i, t) in steps.iter().enumerate() {
        let step = i + 1;
        let g = t.glyph();

        // The machine returns early once IFIX has fired: everything but IFIX
        // and IMSCRIB is inert. Counting a movement without the same guard
        // reports clears and fuses that never happened.
        if fixed && !matches!(t, Token16_3::Ifix | Token16_3::Imscrib) {
            inert += 1;
            continue;
        }

        match t {
            Token16_3::Fsplit3 => {
                frames.push([0; 4]);
                sw!(out, "   {:>3} {}  open frame at depth {}", step, g, frames.len());
            }
            Token16_3::Ffuse3 => {
                if let Some(closed) = frames.pop() {
                    let mut got = 0u32;
                    for j in 0..4 {
                        if closed[j] > reg[j] { got += closed[j] - reg[j]; reg[j] = closed[j]; }
                        if let Some(outer) = frames.last_mut() {
                            if closed[j] > outer[j] { outer[j] = closed[j]; }
                        }
                    }
                    restored += got;
                    nonempty = reg.iter().any(|&w| w > 0);
                    sw!(out, "   {:>3} {}  fuse restores {}", step, g, got);
                }
            }
            Token16_3::Arev | Token16_3::Vinit => {
                let lost: u32 = reg.iter().sum();
                let banked: u32 = frames.iter().map(|f| f.iter().sum::<u32>()).sum();
                cleared += lost;
                reg = [0; 4];
                if matches!(t, Token16_3::Vinit) { frames.clear(); }
                nonempty = false;
                sw!(out, "   {:>3} {}  CLEAR loses {}   ({} banked in frames)",
                          step, g, lost, banked);
            }
            Token16_3::Afwd | Token16_3::Imscrib => {
                if !nonempty {
                    seeded += 1;
                    nonempty = true;
                    sw!(out, "   {:>3} {}  SEED T into an empty register, no weight", step, g);
                }
            }
            Token16_3::Ifix => { fixed = true; }
            _ => {
                // The evaluators are the only depositors: EVALT touches T,
                // EVALF touches F, EVALI touches t and f together, which is
                // why the constructive pair is never seen split.
                let touched: &[usize] = match t {
                    Token16_3::Evalt => &[0],
                    Token16_3::Evalf => &[1],
                    Token16_3::Evali => &[2, 3],
                    _ => &[],
                };
                if !touched.is_empty() {
                    let mut names = String::new();
                    for &j in touched {
                        reg[j] += 1;
                        if let Some(f) = frames.last_mut() { f[j] += 1; }
                        if !names.is_empty() { names.push('+'); }
                        names.push_str(NAMES[j]);
                    }
                    deposits += 1;
                    nonempty = true;
                    sw!(out, "   {:>3} {}  deposit {}   into depth {}",
                              step, g, names, frames.len());
                }
            }
        }
    }

    let mut surv = String::new();
    for j in 0..4 {
        if reg[j] > 0 {
            if !surv.is_empty() { surv.push_str(", "); }
            surv.push_str(&format!("{}×{}", NAMES[j], reg[j]));
        }
    }
    let stranded: u32 = frames.iter().map(|f| f.iter().sum::<u32>()).sum();
    sw!(out, "");
    sw!(out, "  final    : {}", run_word_register(&steps));
    sw!(out, "  surviving: {}", if surv.is_empty() { "none" } else { &surv });
    sw!(out, "  deposits {}  cleared {}  restored {}  seeded {}  inert {}",
              deposits, cleared, restored, seeded, inert);
    if stranded > 0 {
        sw!(out, "  stranded in frames never fused: {}", stranded);
    }
    out
}
