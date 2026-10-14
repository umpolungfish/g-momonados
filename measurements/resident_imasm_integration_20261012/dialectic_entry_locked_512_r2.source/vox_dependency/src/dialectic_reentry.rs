//! Dialectic re-entry: the operator imscribes its space, walks that lattice,
//! employs FOUR, and re-imscribes the transformed space.
//!
//! The `m` in imscription is load-bearing. IMSCRIB is the live read/write/execute
//! relation between the bulk object and its boundary. A descent consumes that
//! complete relation: the boundary reads the bulk N, executes the current IMASM
//! word, and writes the transformed lattice boundary carried by the next object.
//! The finite lattice span is part of that relation too; it is persisted and
//! consumed as a tape numeral rather than reconstructed or narrowed into host state.

use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::factor_extract::{FactorCarrier, Mark, Tape};
use crate::morphism_factor::{add, cmp, divmod, isqrt, mul, sub, tape_u64, trim, zero};
use crate::producer_provenance::{
    SUPPORT_DEEP_ARM, SUPPORT_EXTENDED_FERMAT, SUPPORT_PARITY, SUPPORT_PRIMALITY,
    SUPPORT_PRODUCT_BOUNDARY, SUPPORT_SHORT_FRONTIER,
};
use crate::provenance_envelope::{restored_support, LaneSupport, RestoredSupport};
use crate::router_marks::{GStep, M_FIX, M_T};
use crate::trace_word::encode_trace;
use crate::vox::{verdict, EVALF, EVALT, IMSCRIB, TANCH, VINIT};

/// An unresolved imscription deliberately leaves its factor fork open: FOUR=B.
pub const SHORT_FRONTIER_WORD: &str = "⊢⊙∈≻⊤≺⊥⊡⊣";
/// The continuation is a new whole imscription, not the old word plus a cursor.
pub const EXTENDED_FERMAT_WORD: &str = "⊢⊙∈≻⊤⊞≺⊥⊡⊣";
/// The first change of lattice: the boundary is now Lehman's multiplier k.
pub const LEHMAN_WORD: &str = "⊢⊙∈≻⋈⊤⊥⊡⊣";

/// A route-around on the short frontier: ∈ immediately rejoins at ∋, so no
/// work occurs inside the fork and FOUR=N while the lattice identity remains
/// present in the succeeding grammar.
pub const SHORT_FRONTIER_N_WORD: &str = "⊢⊙∈∋≻⊤≺⊥⊡⊣";
/// The same no-new-distinction route-around on the extended Fermat lattice.
pub const EXTENDED_FERMAT_N_WORD: &str = "⊢⊙∈∋≻⊤⊞≺⊥⊡⊣";
/// The same no-new-distinction route-around on the Lehman lattice.
pub const LEHMAN_N_WORD: &str = "⊢⊙∈∋≻⋈⊤⊥⊡⊣";

const SHORT_FRONTIER_CLOSED_WORD: &str = "⊢⊙∈≻⊤≺⊥∋⊡⊣";
const EXTENDED_FERMAT_CLOSED_WORD: &str = "⊢⊙∈≻⊤⊞≺⊥∋⊡⊣";
const LEHMAN_CLOSED_WORD: &str = "⊢⊙∈≻⋈⊤⊥∋⊡⊣";
const SUPPORT_BITS: usize = 6;
const RWX_BITS: usize = 3;

/// The word semantics define finite imscribed regions, but the region size is
/// carried by the object itself as a tape numeral.
pub const SHORT_FRONTIER_SPAN: u64 = 64;
pub const EXTENDED_FERMAT_SPAN: u64 = 4096 - SHORT_FRONTIER_SPAN;
pub const LEHMAN_LOCAL_SPAN: u64 = 64;

const BASE_SUPPORT: LaneSupport = SUPPORT_PARITY | SUPPORT_PRIMALITY;

/// The lattice action decoded from the current IMASM word.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImscriptionLattice {
    ShortFrontier,
    ExtendedFermat,
    Lehman,
}

impl ImscriptionLattice {
    fn open_word(self) -> &'static str {
        match self {
            Self::ShortFrontier => SHORT_FRONTIER_WORD,
            Self::ExtendedFermat => EXTENDED_FERMAT_WORD,
            Self::Lehman => LEHMAN_WORD,
        }
    }

    fn neutral_word(self) -> &'static str {
        match self {
            Self::ShortFrontier => SHORT_FRONTIER_N_WORD,
            Self::ExtendedFermat => EXTENDED_FERMAT_N_WORD,
            Self::Lehman => LEHMAN_N_WORD,
        }
    }

    fn closed_word(self) -> &'static str {
        match self {
            Self::ShortFrontier => SHORT_FRONTIER_CLOSED_WORD,
            Self::ExtendedFermat => EXTENDED_FERMAT_CLOSED_WORD,
            Self::Lehman => LEHMAN_CLOSED_WORD,
        }
    }

    fn span(self) -> u64 {
        match self {
            Self::ShortFrontier => SHORT_FRONTIER_SPAN,
            Self::ExtendedFermat => EXTENDED_FERMAT_SPAN,
            Self::Lehman => LEHMAN_LOCAL_SPAN,
        }
    }

    fn unresolved_support(self) -> LaneSupport {
        match self {
            Self::ShortFrontier => BASE_SUPPORT,
            Self::ExtendedFermat => BASE_SUPPORT | SUPPORT_SHORT_FRONTIER,
            Self::Lehman => BASE_SUPPORT | SUPPORT_SHORT_FRONTIER | SUPPORT_EXTENDED_FERMAT,
        }
    }

    fn terminal_support(self) -> LaneSupport {
        self.support_after_t(self.unresolved_support())
    }

    fn next_after_b(self) -> Self {
        match self {
            Self::ShortFrontier => Self::ExtendedFermat,
            Self::ExtendedFermat => Self::Lehman,
            Self::Lehman => Self::Lehman,
        }
    }

    fn support_after_b(self, support: LaneSupport) -> LaneSupport {
        match self {
            Self::ShortFrontier => support | SUPPORT_SHORT_FRONTIER,
            Self::ExtendedFermat => support | SUPPORT_EXTENDED_FERMAT,
            Self::Lehman => support,
        }
    }

    fn support_after_t(self, support: LaneSupport) -> LaneSupport {
        match self {
            Self::ShortFrontier => {
                support | SUPPORT_SHORT_FRONTIER | SUPPORT_PRODUCT_BOUNDARY
            }
            Self::ExtendedFermat => {
                support | SUPPORT_EXTENDED_FERMAT | SUPPORT_PRODUCT_BOUNDARY
            }
            Self::Lehman => support | SUPPORT_DEEP_ARM | SUPPORT_PRODUCT_BOUNDARY,
        }
    }
}

/// Semantic execution decoded from one valid dialectic IMASM word. The grammar
/// state is structural: there is no independent FOUR mark or closed boolean that
/// can disagree with the executable form. Malformed/incomplete grammar is FOUR=F
/// at the judgment layer and therefore is not an `ImasmExecution` value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImasmExecution {
    B { lattice: ImscriptionLattice },
    T { lattice: ImscriptionLattice },
    N { lattice: ImscriptionLattice },
}

impl ImasmExecution {
    pub fn lattice(self) -> ImscriptionLattice {
        match self {
            Self::B { lattice } | Self::T { lattice } | Self::N { lattice } => lattice,
        }
    }

    pub fn four(self) -> Mark {
        match self {
            Self::B { .. } => 'B',
            Self::T { .. } => 'T',
            Self::N { .. } => 'N',
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::T { .. })
    }

    pub fn is_neutral(self) -> bool {
        matches!(self, Self::N { .. })
    }
}

/// A factor relation exposed by a lattice walk that FOUR judges `T`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DialecticWitness {
    pub p: Tape,
    pub q: Tape,
    pub lattice_cell: Tape,
}

/// Capabilities of a live imscription relation. The bit-set says which actions
/// are enabled; the endpoints below say what those actions currently relate.
pub const IM_READ: u8 = 1 << 0;
pub const IM_WRITE: u8 = 1 << 1;
pub const IM_EXEC: u8 = 1 << 2;
pub const IM_RWX: u8 = IM_READ | IM_WRITE | IM_EXEC;

/// The load-bearing r/w/x relation of IMSCRIB.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ImscriptionRwx {
    pub rights: u8,
    pub read_bulk: Tape,
    pub write_boundary: Tape,
    pub execute_span: Tape,
    pub execute_word: Vec<Mark>,
}

impl ImscriptionRwx {
    fn live(
        read_bulk: &[Mark],
        write_boundary: &[Mark],
        execute_span: &[Mark],
        execute_word: &[Mark],
    ) -> Self {
        Self {
            rights: IM_RWX,
            read_bulk: trim(read_bulk.to_vec()),
            write_boundary: trim(write_boundary.to_vec()),
            execute_span: trim(execute_span.to_vec()),
            execute_word: execute_word.to_vec(),
        }
    }

    pub fn can_read(&self) -> bool {
        self.rights & IM_READ != 0
    }

    pub fn can_write(&self) -> bool {
        self.rights & IM_WRITE != 0
    }

    pub fn can_execute(&self) -> bool {
        self.rights & IM_EXEC != 0
    }
}

/// The live imscription relation. Boundary, span and executable word are owned
/// only here, by the r/w/x endpoints; there is no surrounding mirror copy.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Imscription {
    pub rwx: ImscriptionRwx,
}

impl Imscription {
    fn active(n: &[Mark], boundary: Tape, span: Tape, word: &[Mark]) -> Self {
        Self {
            rwx: ImscriptionRwx::live(n, &boundary, &span, word),
        }
    }

    pub fn boundary(&self) -> &Tape {
        &self.rwx.write_boundary
    }

    pub fn span(&self) -> &Tape {
        &self.rwx.execute_span
    }

    pub fn word(&self) -> &[Mark] {
        &self.rwx.execute_word
    }

    pub fn relation_is_live_for(&self, n: &[Mark]) -> bool {
        self.rwx.rights == IM_RWX
            && self.rwx.can_read()
            && self.rwx.can_write()
            && self.rwx.can_execute()
            && self.rwx.read_bulk == trim(n.to_vec())
            && !self.rwx.write_boundary.is_empty()
            && !self.rwx.execute_span.is_empty()
            && self
                .rwx
                .write_boundary
                .iter()
                .chain(self.rwx.execute_span.iter())
                .all(|&mark| mark == EVALT || mark == EVALF)
            && !self.rwx.execute_word.is_empty()
    }
}

/// FOUR itself is the judgment type. Each state owns exactly the payload that
/// state permits; impossible cross-state payload combinations are unrepresentable.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DialecticJudgment {
    /// Closing relation: terminal imscription + terminal support + factor witness.
    T {
        imscription: Imscription,
        support: LaneSupport,
        witness: DialecticWitness,
    },
    /// Productive contradiction: complete succeeding imscription + succeeding support.
    B {
        imscription: Imscription,
        support: LaneSupport,
    },
    /// No new distinction: the complete whole-object relation remains unchanged.
    N {
        imscription: Imscription,
        support: LaneSupport,
    },
    /// Malformed/structurally incomplete relation: no valid result exists.
    F,
}

impl DialecticJudgment {
    pub fn four(&self) -> Mark {
        match self {
            Self::T { .. } => 'T',
            Self::B { .. } => 'B',
            Self::N { .. } => 'N',
            Self::F => 'F',
        }
    }

    pub fn resulting_imscription(&self) -> Option<&Imscription> {
        match self {
            Self::T { imscription, .. }
            | Self::B { imscription, .. }
            | Self::N { imscription, .. } => Some(imscription),
            Self::F => None,
        }
    }

    pub fn resulting_support(&self) -> Option<LaneSupport> {
        match self {
            Self::T { support, .. }
            | Self::B { support, .. }
            | Self::N { support, .. } => Some(*support),
            Self::F => None,
        }
    }

    pub fn witness(&self) -> Option<&DialecticWitness> {
        match self {
            Self::T { witness, .. } => Some(witness),
            Self::B { .. } | Self::N { .. } | Self::F => None,
        }
    }
}

/// The arithmetic lattice itself has exactly the two exposures it can actually
/// produce. N and F arise at the imscription/grammar judgment layer, not as
/// fabricated arithmetic outcomes.
#[derive(Clone, PartialEq, Eq, Debug)]
enum LatticeExposure {
    T {
        write_boundary: Tape,
        witness: DialecticWitness,
    },
    B {
        write_boundary: Tape,
    },
}

/// Decode one complete dialectic IMASM grammar. B leaves the fork open, T
/// closes it after work, and N closes an empty fork before the lattice kernel:
/// the latter is the explicit route-around/no-new-distinction form. FOUR=F is
/// intentionally not a decodable execution; judgment catches it as malformed.
pub fn decode_imasm_execution(word: &[Mark]) -> Result<ImasmExecution, String> {
    if word.len() < 6
        || word.first().copied() != Some(VINIT)
        || word.get(1).copied() != Some(IMSCRIB)
        || word.get(word.len() - 2).copied() != Some('⊡')
        || word.last().copied() != Some(TANCH)
    {
        return Err(String::from("malformed dialectic IMASM execution framing"));
    }

    let four = verdict(word);
    let body = &word[2..word.len() - 2];
    let kernel = match four {
        'B' => {
            if body.get(0).copied() != Some('∈') || body.get(1).copied() != Some('≻') {
                return Err(String::from("malformed FOUR=B dialectic IMASM grammar"));
            }
            &body[2..]
        }
        'T' => {
            if body.get(0).copied() != Some('∈')
                || body.get(1).copied() != Some('≻')
                || body.last().copied() != Some('∋')
            {
                return Err(String::from("malformed FOUR=T dialectic IMASM grammar"));
            }
            &body[2..body.len() - 1]
        }
        'N' => {
            if body.get(0).copied() != Some('∈')
                || body.get(1).copied() != Some('∋')
                || body.get(2).copied() != Some('≻')
            {
                return Err(String::from("unknown FOUR=N dialectic route-around grammar"));
            }
            &body[3..]
        }
        'F' => {
            return Err(String::from(
                "FOUR=F dialectic IMASM grammar is malformed or structurally incomplete",
            ));
        }
        _ => return Err(String::from("unknown FOUR verdict")),
    };

    let lattice = match kernel {
        ['⊤', '≺', '⊥'] => ImscriptionLattice::ShortFrontier,
        ['⊤', '⊞', '≺', '⊥'] => ImscriptionLattice::ExtendedFermat,
        ['⋈', '⊤', '⊥'] => ImscriptionLattice::Lehman,
        _ => return Err(String::from("unknown dialectic IMASM lattice kernel")),
    };

    Ok(match four {
        'B' => ImasmExecution::B { lattice },
        'T' => ImasmExecution::T { lattice },
        'N' => ImasmExecution::N { lattice },
        _ => unreachable!("FOUR=F and unknown verdicts returned above"),
    })
}

/// One complete, restartable operator-space object.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DialecticObject {
    pub n: Tape,
    pub support: LaneSupport,
    pub imscription: Imscription,
}

impl DialecticObject {
    pub fn word(&self) -> &[Mark] {
        self.imscription.word()
    }

    pub fn boundary(&self) -> &Tape {
        self.imscription.boundary()
    }

    pub fn span(&self) -> &Tape {
        self.imscription.span()
    }
}

/// A terminal contraction handed to the passive factor-carrier layer.
#[derive(Clone, PartialEq, Debug)]
pub struct DialecticClosure {
    pub carrier: FactorCarrier,
    pub support: LaneSupport,
    pub imscription: Imscription,
    pub lattice_cell: Tape,
    pub lehman_multiplier: Option<Tape>,
}

impl DialecticClosure {
    pub fn word(&self) -> &[Mark] {
        self.imscription.word()
    }

    pub fn boundary(&self) -> &Tape {
        self.imscription.boundary()
    }

    pub fn span(&self) -> &Tape {
        self.imscription.span()
    }
}

/// Whole-object execution result. FOUR remains structurally visible after the
/// judgment is consumed: T owns a closure, B owns the succeeding whole object,
/// N owns the exact unchanged whole object, and F owns no invalid payload.
#[derive(Clone, PartialEq, Debug)]
pub enum Descent {
    T(DialecticClosure),
    B(DialecticObject),
    N(DialecticObject),
    F,
}

impl Descent {
    pub fn four(&self) -> Mark {
        match self {
            Self::T(_) => 'T',
            Self::B(_) => 'B',
            Self::N(_) => 'N',
            Self::F => 'F',
        }
    }
}

impl DialecticObject {
    pub fn new(n: Tape) -> Result<Self, String> {
        let n = trim(n);
        let boundary = fermat_origin(&n);
        let span = tape_u64(SHORT_FRONTIER_SPAN);
        let word: Vec<Mark> = SHORT_FRONTIER_WORD.chars().collect();
        let imscription = Imscription::active(&n, boundary, span, &word);
        let object = Self {
            n,
            support: BASE_SUPPORT,
            imscription,
        };
        object.validate()?;
        Ok(object)
    }

    fn validate_relation_shape(&self) -> Result<(), String> {
        if cmp(&self.n, &tape_u64(1)) != Ordering::Greater {
            return Err(String::from("dialectic object requires N > 1"));
        }
        if !self.imscription.relation_is_live_for(&self.n) {
            return Err(String::from(
                "dialectic imscription r/w/x relation is not live for this whole object",
            ));
        }
        Ok(())
    }

    fn validate_execution_binding(&self, execution: ImasmExecution) -> Result<(), String> {
        let lattice = execution.lattice();
        let expected_support = match execution {
            ImasmExecution::B { .. } | ImasmExecution::N { .. } => lattice.unresolved_support(),
            ImasmExecution::T { .. } => lattice.terminal_support(),
        };
        if self.support != expected_support {
            return Err(String::from("dialectic imscription/support mismatch"));
        }

        let expected_span = tape_u64(lattice.span());
        if cmp(self.span(), &expected_span) != Ordering::Equal {
            return Err(String::from(
                "dialectic imscription span does not match the current IMASM lattice",
            ));
        }

        match lattice {
            ImscriptionLattice::ShortFrontier | ImscriptionLattice::ExtendedFermat => {
                let origin = fermat_origin(&self.n);
                let start = if lattice == ImscriptionLattice::ShortFrontier {
                    origin
                } else {
                    add(&origin, &tape_u64(SHORT_FRONTIER_SPAN))
                };
                if matches!(execution, ImasmExecution::T { .. }) {
                    let end = add(&start, &expected_span);
                    if cmp(self.boundary(), &start) == Ordering::Less
                        || cmp(self.boundary(), &end) != Ordering::Less
                    {
                        return Err(String::from(
                            "terminal dialectic Fermat boundary lies outside its imscribed span",
                        ));
                    }
                } else if cmp(self.boundary(), &start) != Ordering::Equal {
                    return Err(String::from(
                        "dialectic imscription boundary does not match the IMASM-selected Fermat space",
                    ));
                }
            }
            ImscriptionLattice::Lehman => {
                if cmp(self.boundary(), &tape_u64(1)) == Ordering::Less {
                    return Err(String::from(
                        "dialectic Lehman boundary must be a positive imscribed multiplier",
                    ));
                }
            }
        }

        Ok(())
    }

    /// A persisted current object may be productive B or genuine no-change N.
    /// Terminal T is a closure, while malformed F is never admitted as a valid
    /// restart object.
    pub fn validate(&self) -> Result<(), String> {
        self.validate_relation_shape()?;
        let execution = decode_imasm_execution(self.word())?;
        if execution.is_terminal() {
            return Err(String::from(
                "terminal dialectic imscription is a closure, not a current restart object",
            ));
        }
        self.validate_execution_binding(execution)
    }

    /// Re-imscribe the same current lattice as a genuine FOUR=N route-around.
    /// Boundary, span, support, bulk and r/w/x endpoints remain unchanged; only
    /// X employs the neutral grammar for that same lattice. Re-entering N again
    /// returns this exact whole object.
    pub fn route_around(mut self) -> Result<Self, String> {
        self.validate()?;
        let execution = decode_imasm_execution(self.word())?;
        let boundary = self.boundary().clone();
        let span = self.span().clone();
        let word: Vec<Mark> = execution.lattice().neutral_word().chars().collect();
        self.imscription = Imscription::active(&self.n, boundary, span, &word);
        self.validate()?;
        Ok(self)
    }

    /// Judge the current relation through FOUR. This boundary is semantically
    /// total: every complete in-memory relation yields exactly T, B, N, or F.
    /// Rust errors remain confined to persistence/codec APIs, not FOUR judgment.
    pub fn judge_current_imscription(&self) -> DialecticJudgment {
        self.judge_with_kernel(None)
    }

    fn judge_with_kernel(&self, mut kernel: Option<&mut ResidentScan>) -> DialecticJudgment {
        if self.validate_relation_shape().is_err() {
            return DialecticJudgment::F;
        }
        let execution = match decode_imasm_execution(self.word()) {
            Ok(execution) => execution,
            Err(_) => return DialecticJudgment::F,
        };
        if self.validate_execution_binding(execution).is_err() {
            return DialecticJudgment::F;
        }

        if execution.is_neutral() {
            return DialecticJudgment::N {
                imscription: self.imscription.clone(),
                support: self.support,
            };
        }

        let lattice = execution.lattice();
        let exposure = if let Some(kernel) = kernel.as_deref_mut() {
            resident_lattice(&self.n, self.boundary(), self.span(), lattice, kernel)
        } else {
        let exposure = match lattice {
            ImscriptionLattice::ShortFrontier => fermat_lattice_from(
                &self.n,
                self.boundary(),
                &tape_u64(0),
                self.span(),
            ),
            ImscriptionLattice::ExtendedFermat => fermat_lattice_from(
                &self.n,
                self.boundary(),
                &tape_u64(SHORT_FRONTIER_SPAN),
                self.span(),
            ),
            ImscriptionLattice::Lehman => {
                lehman_multiplier(&self.n, self.boundary(), self.span())
            }
        };
        exposure
        };

        match (execution, exposure) {
            (
                ImasmExecution::B { lattice },
                LatticeExposure::T {
                    write_boundary,
                    witness,
                },
            ) => {
                let word: Vec<Mark> = lattice.closed_word().chars().collect();
                let imscription = Imscription::active(
                    &self.n,
                    write_boundary,
                    self.span().clone(),
                    &word,
                );
                let terminal = match decode_imasm_execution(imscription.word()) {
                    Ok(terminal) => terminal,
                    Err(error) => {
                        if let Some(kernel) = kernel.as_deref_mut() {
                            kernel.failure = Some(format!("terminal decode: {error}; word={}", imscription.word().iter().collect::<String>()));
                        }
                        return DialecticJudgment::F;
                    }
                };
                if terminal != (ImasmExecution::T { lattice }) {
                    if let Some(kernel) = kernel.as_deref_mut() { kernel.failure = Some(format!("terminal execution mismatch: {terminal:?}")); }
                    return DialecticJudgment::F;
                }
                DialecticJudgment::T {
                    imscription,
                    support: lattice.support_after_t(self.support),
                    witness,
                }
            }
            (ImasmExecution::B { lattice }, LatticeExposure::B { write_boundary }) => {
                let next_lattice = lattice.next_after_b();
                let next_boundary = if lattice == ImscriptionLattice::ExtendedFermat {
                    tape_u64(1)
                } else {
                    write_boundary
                };
                let next_word: Vec<Mark> = next_lattice.open_word().chars().collect();
                let imscription = Imscription::active(
                    &self.n,
                    next_boundary,
                    tape_u64(next_lattice.span()),
                    &next_word,
                );
                let next = DialecticObject {
                    n: self.n.clone(),
                    support: lattice.support_after_b(self.support),
                    imscription: imscription.clone(),
                };
                if next.validate().is_err() {
                    return DialecticJudgment::F;
                }
                DialecticJudgment::B {
                    imscription,
                    support: next.support,
                }
            }
            (
                ImasmExecution::T { .. },
                LatticeExposure::T {
                    witness,
                    ..
                },
            ) => DialecticJudgment::T {
                imscription: self.imscription.clone(),
                support: self.support,
                witness,
            },
            (ImasmExecution::T { .. }, LatticeExposure::B { .. }) => DialecticJudgment::F,
            (ImasmExecution::N { .. }, _) => DialecticJudgment::N {
                imscription: self.imscription.clone(),
                support: self.support,
            },
        }
    }

    pub fn envelope(&self) -> RestoredSupport {
        restored_support(&[self.support])
    }

    /// Marks-only persistence of the complete current operator-space relation:
    /// `⊢ ⊙ ∈N∋ ∈rights[3]∋ ∈read-N∋ ∈write-boundary∋
    ///    ∈exec-span∋ ∈len(exec-word)∋ exec-word ∈support[6]∋ ⊣`
    pub fn encode(&self) -> Vec<Mark> {
        let mut out = Vec::with_capacity(
            self.n.len()
                + self.imscription.rwx.read_bulk.len()
                + self.imscription.rwx.write_boundary.len()
                + self.imscription.rwx.execute_span.len()
                + self.imscription.rwx.execute_word.len()
                + RWX_BITS
                + SUPPORT_BITS
                + 18,
        );
        out.push(VINIT);
        out.push(IMSCRIB);
        push_tape(&mut out, &self.n);
        push_mask(&mut out, self.imscription.rwx.rights as u32, RWX_BITS);
        push_tape(&mut out, &self.imscription.rwx.read_bulk);
        push_tape(&mut out, &self.imscription.rwx.write_boundary);
        push_tape(&mut out, &self.imscription.rwx.execute_span);
        push_blob(&mut out, &self.imscription.rwx.execute_word);
        push_mask(&mut out, self.support, SUPPORT_BITS);
        out.push(TANCH);
        out
    }

    pub fn decode(encoded: &[Mark]) -> Result<Self, String> {
        if encoded.len() < 4
            || encoded[0] != VINIT
            || encoded[1] != IMSCRIB
            || *encoded.last().unwrap() != TANCH
        {
            return Err(String::from("malformed dialectic object framing"));
        }
        let end = encoded.len() - 1;
        let mut i = 2usize;
        let n = read_tape(encoded, &mut i)?;
        let rights = read_mask(encoded, &mut i, RWX_BITS)? as u8;
        let read_bulk = read_tape(encoded, &mut i)?;
        let write_boundary = read_tape(encoded, &mut i)?;
        let execute_span = read_tape(encoded, &mut i)?;
        let execute_word = read_blob(encoded, &mut i, end)?;
        let support = read_mask(encoded, &mut i, SUPPORT_BITS)?;
        if i != end {
            return Err(String::from("trailing marks after dialectic imscription relation"));
        }
        let object = Self {
            n,
            support,
            imscription: Imscription {
                rwx: ImscriptionRwx {
                    rights,
                    read_bulk,
                    write_boundary,
                    execute_span,
                    execute_word,
                },
            },
        };
        object.validate()?;
        Ok(object)
    }

    /// Consume the whole object without erasing FOUR into control-flow or errors.
    /// The execution result itself is T/B/N/F; persistence and codecs are the
    /// only layers that use Rust `Result` for malformed serialized input.
    pub fn descend(self) -> Descent {
        let judgment = self.judge_current_imscription();
        self.consume_judgment(judgment, None)
    }

    pub fn descend_with_kernel(self, kernel: &mut ResidentScan) -> Descent {
        let judgment = self.judge_with_kernel(Some(kernel));
        self.consume_judgment(judgment, Some(&mut kernel.failure))
    }

    fn consume_judgment(self, judgment: DialecticJudgment, failure: Option<&mut Option<String>>) -> Descent {
        match judgment {
            DialecticJudgment::T {
                imscription,
                support,
                witness,
            } => {
                let lattice = match decode_imasm_execution(imscription.word()) {
                    Ok(ImasmExecution::T { lattice }) => lattice,
                    _ => return Descent::F,
                };
                let lehman_multiplier = if lattice == ImscriptionLattice::Lehman {
                    Some(imscription.boundary().clone())
                } else {
                    None
                };
                close(
                    self.n,
                    witness.p,
                    witness.q,
                    support,
                    imscription,
                    witness.lattice_cell,
                    lehman_multiplier,
                    failure,
                )
            }
            DialecticJudgment::B {
                imscription,
                support,
            } => {
                let next = DialecticObject {
                    n: self.n,
                    support,
                    imscription,
                };
                if next.validate().is_err() {
                    Descent::F
                } else {
                    Descent::B(next)
                }
            }
            DialecticJudgment::N {
                imscription,
                support,
            } => {
                let unchanged = DialecticObject {
                    n: self.n,
                    support,
                    imscription,
                };
                if unchanged.validate().is_err() {
                    Descent::F
                } else {
                    Descent::N(unchanged)
                }
            }
            DialecticJudgment::F => Descent::F,
        }
    }
}

fn close(
    n: Tape,
    p: Tape,
    q: Tape,
    support: LaneSupport,
    imscription: Imscription,
    lattice_cell: Tape,
    lehman_multiplier: Option<Tape>,
    failure: Option<&mut Option<String>>,
) -> Descent {
    if !matches!(
        decode_imasm_execution(imscription.word()),
        Ok(ImasmExecution::T { .. })
    ) {
        if let Some(failure) = failure { *failure = Some("closure terminal decode".into()); }
        return Descent::F;
    }
    if !imscription.relation_is_live_for(&n) {
        if let Some(failure) = failure { *failure = Some("closure source relation".into()); }
        return Descent::F;
    }
    let word = imscription.word().to_vec();
    let trace = encode_trace(&[GStep {
        repr: '⋈',
        judgment: M_T,
        recognised: M_T,
        next: M_FIX,
        applied_word: word,
    }]);
    let carrier = match FactorCarrier::new(n, p, q, trace) {
        Ok(carrier) => carrier,
        Err(error) => {
            if let Some(failure) = failure { *failure = Some(error); }
            return Descent::F;
        }
    };
    Descent::T(DialecticClosure {
        carrier,
        support,
        imscription,
        lattice_cell,
        lehman_multiplier,
    })
}

fn fermat_origin(n: &[Mark]) -> Tape {
    let mut a = isqrt(n);
    if cmp(&mul(&a, &a), n) == Ordering::Less {
        a = add(&a, &tape_u64(1));
    }
    a
}

/// The compiled Lean low-word support and temporal orientation act on the
/// residual of the same source/boundary relation used by dialectic descent.
#[derive(Clone, Debug)]
pub struct ResidentScan {
    pub square_support: [bool; 256],
    pub reverse: bool,
    pub cells: u64,
    pub square_tests: u64,
    pub rejected_cells: u64,
    pub residual_advances: u64,
    pub last_frame: Option<ResidualFrame>,
    pub failure: Option<String>,
}

/// Full-width measured state retained across the next Y re-entry.
#[derive(Clone, Debug)]
pub struct ResidualFrame {
    pub source: Tape,
    pub multiplier: Tape,
    pub a: Tape,
    pub residual: Tape,
    pub lattice_cell: Tape,
    pub lattice: ImscriptionLattice,
}

impl ResidentScan {
    pub fn new(square_support: [bool; 256]) -> Self {
        Self { square_support, reverse: false, cells: 0, square_tests: 0,
            rejected_cells: 0, residual_advances: 0, last_frame: None, failure: None }
    }
}

fn resident_lattice(n: &[Mark], boundary: &[Mark], span: &[Mark],
    lattice: ImscriptionLattice, kernel: &mut ResidentScan) -> LatticeExposure {
    let one = tape_u64(1);
    let two = tape_u64(2);
    let lehman = lattice == ImscriptionLattice::Lehman;
    let source = if lehman {
        if let Some(previous) = &kernel.last_frame {
            if previous.lattice == ImscriptionLattice::Lehman
                && add(&previous.multiplier, &one) == boundary {
                // Re-entry consumes the previous source relation, advancing
                // 4*k*N to 4*(k+1)*N within the same retained frame.
                add(&previous.source, &mul(&tape_u64(4), n))
            } else { mul(&tape_u64(4), &mul(boundary, n)) }
        } else { mul(&tape_u64(4), &mul(boundary, n)) }
    }
        else { n.to_vec() };
    let origin = if lehman { fermat_origin(&source) } else { boundary.to_vec() };
    let end = add(&origin, span);
    let mut a = if kernel.reverse { sub(&end, &one) } else { origin };
    let mut cell = if lattice == ImscriptionLattice::ExtendedFermat {
        tape_u64(SHORT_FRONTIER_SPAN)
    } else { tape_u64(0) };
    if kernel.reverse { cell = add(&cell, &sub(span, &one)); }
    // Preparation computes the complete residual once. Every following cell
    // updates the full-width carry relation proven by Lean residual_advance.
    let mut residual = sub(&mul(&a, &a), &source);
    let mut remaining = span.to_vec();
    while !zero(&remaining) {
        kernel.cells += 1;
        let residue = residual.iter().take(8).enumerate().fold(0usize,
            |r, (bit, mark)| r | (usize::from(*mark == EVALF) << bit));
        if kernel.square_support[residue] {
            kernel.square_tests += 1;
            let b = isqrt(&residual);
            if mul(&b, &b) == residual {
                let pair = if lehman {
                    factor_from_gcd(n, &add(&a, &b))
                        .or_else(|| factor_from_gcd(n, &sub(&a, &b)))
                } else {
                    let p = sub(&a, &b);
                    if cmp(&p, &one) == Ordering::Greater && cmp(&p, n) == Ordering::Less {
                        let (q, remainder) = divmod(n, &p);
                        if zero(&remainder) && cmp(&q, &one) == Ordering::Greater {
                            Some((p, q))
                        } else { None }
                    } else { None }
                };
                if let Some((p, q)) = pair {
                    kernel.last_frame = Some(ResidualFrame { source, multiplier: boundary.to_vec(),
                        a: a.clone(), residual, lattice_cell: cell.clone(), lattice });
                    return LatticeExposure::T {
                        write_boundary: if lehman { boundary.to_vec() } else { a },
                        witness: DialecticWitness { p, q, lattice_cell: cell },
                    };
                }
            }
        } else { kernel.rejected_cells += 1; }
        remaining = sub(&remaining, &one);
        if zero(&remaining) { break; }
        if kernel.reverse {
            residual = sub(&residual, &sub(&mul(&two, &a), &one));
            a = sub(&a, &one);
            cell = sub(&cell, &one);
        } else {
            residual = add(&residual, &add(&mul(&two, &a), &one));
            a = add(&a, &one);
            cell = add(&cell, &one);
        }
        kernel.residual_advances += 1;
    }
    kernel.last_frame = Some(ResidualFrame { source, multiplier: boundary.to_vec(),
        a, residual, lattice_cell: cell, lattice });
    LatticeExposure::B { write_boundary: if lehman { add(boundary, &one) } else { end } }
}

fn fermat_lattice_from(
    n: &[Mark],
    start_boundary: &[Mark],
    base_cell: &[Mark],
    span: &[Mark],
) -> LatticeExposure {
    let one = tape_u64(1);
    let mut a = trim(start_boundary.to_vec());
    let mut cell = trim(base_cell.to_vec());
    let mut remaining = trim(span.to_vec());

    while !zero(&remaining) {
        let a2 = mul(&a, &a);
        if cmp(&a2, n) != Ordering::Less {
            let b2 = sub(&a2, n);
            let b = isqrt(&b2);
            if mul(&b, &b) == b2 {
                let p = sub(&a, &b);
                if cmp(&p, &one) == Ordering::Greater && cmp(&p, n) == Ordering::Less {
                    let (q, remainder) = divmod(n, &p);
                    if zero(&remainder) && cmp(&q, &one) == Ordering::Greater {
                        return LatticeExposure::T {
                            write_boundary: a,
                            witness: DialecticWitness {
                                p,
                                q,
                                lattice_cell: cell,
                            },
                        };
                    }
                }
            }
        }
        a = add(&a, &one);
        cell = add(&cell, &one);
        remaining = sub(&remaining, &one);
    }

    LatticeExposure::B { write_boundary: a }
}

fn lehman_multiplier(n: &[Mark], k: &[Mark], span: &[Mark]) -> LatticeExposure {
    let one = tape_u64(1);
    let four_kn = mul(&tape_u64(4), &mul(k, n));
    let mut a = isqrt(&four_kn);
    let mut cell = tape_u64(0);
    let mut remaining = trim(span.to_vec());
    if cmp(&mul(&a, &a), &four_kn) == Ordering::Less {
        a = add(&a, &one);
    }

    while !zero(&remaining) {
        let a2 = mul(&a, &a);
        if cmp(&a2, &four_kn) != Ordering::Less {
            let b2 = sub(&a2, &four_kn);
            let b = isqrt(&b2);
            if mul(&b, &b) == b2 {
                let plus = add(&a, &b);
                if let Some((p, q)) = factor_from_gcd(n, &plus) {
                    return LatticeExposure::T {
                        write_boundary: trim(k.to_vec()),
                        witness: DialecticWitness {
                            p,
                            q,
                            lattice_cell: cell,
                        },
                    };
                }
                if cmp(&a, &b) != Ordering::Less {
                    let minus = sub(&a, &b);
                    if let Some((p, q)) = factor_from_gcd(n, &minus) {
                        return LatticeExposure::T {
                            write_boundary: trim(k.to_vec()),
                            witness: DialecticWitness {
                                p,
                                q,
                                lattice_cell: cell,
                            },
                        };
                    }
                }
            }
        }
        a = add(&a, &one);
        cell = add(&cell, &one);
        remaining = sub(&remaining, &one);
    }

    LatticeExposure::B {
        write_boundary: add(k, &one),
    }
}

fn factor_from_gcd(n: &[Mark], x: &[Mark]) -> Option<(Tape, Tape)> {
    let one = tape_u64(1);
    let p = gcd_tape(x, n);
    if cmp(&p, &one) != Ordering::Greater || cmp(&p, n) != Ordering::Less {
        return None;
    }
    let (q, remainder) = divmod(n, &p);
    if !zero(&remainder) || cmp(&q, &one) != Ordering::Greater {
        return None;
    }
    Some((p, q))
}

fn gcd_tape(a: &[Mark], b: &[Mark]) -> Tape {
    let mut x = trim(a.to_vec());
    let mut y = trim(b.to_vec());
    while !zero(&y) {
        let (_, r) = divmod(&x, &y);
        x = y;
        y = r;
    }
    x
}

// Host-sized conversions below are confined to in-memory blob framing. They do
// not mediate N, boundary, span, lattice-cell, multiplier, or r/w/x semantics.
fn usize_to_tape(mut value: usize) -> Tape {
    if value == 0 {
        return vec![EVALT];
    }
    let mut out = Vec::new();
    while value != 0 {
        out.push(if value & 1 == 1 { EVALF } else { EVALT });
        value >>= 1;
    }
    out
}

fn tape_to_usize(tape: &[Mark]) -> Option<usize> {
    if tape.is_empty() {
        return None;
    }
    let mut value = 0usize;
    for (bit, &mark) in tape.iter().enumerate() {
        match mark {
            EVALT => {}
            EVALF => {
                if bit >= usize::BITS as usize {
                    return None;
                }
                value |= 1usize.checked_shl(bit as u32)?;
            }
            _ => return None,
        }
    }
    Some(value)
}

fn push_tape(out: &mut Vec<Mark>, tape: &[Mark]) {
    out.push('∈');
    out.extend_from_slice(tape);
    out.push('∋');
}

fn push_mask(out: &mut Vec<Mark>, mask: u32, bits: usize) {
    out.push('∈');
    for bit in 0..bits {
        out.push(if (mask >> bit) & 1 == 1 { EVALF } else { EVALT });
    }
    out.push('∋');
}

fn push_blob(out: &mut Vec<Mark>, blob: &[Mark]) {
    push_tape(out, &usize_to_tape(blob.len()));
    out.extend_from_slice(blob);
}

fn read_blob(encoded: &[Mark], i: &mut usize, end: usize) -> Result<Vec<Mark>, String> {
    let len_tape = read_tape(encoded, i)?;
    let len = tape_to_usize(&len_tape)
        .ok_or_else(|| String::from("dialectic blob length overflows host address space"))?;
    let blob_end = i
        .checked_add(len)
        .ok_or_else(|| String::from("dialectic blob length overflow"))?;
    if blob_end > end {
        return Err(String::from("truncated dialectic blob"));
    }
    let blob = encoded[*i..blob_end].to_vec();
    *i = blob_end;
    Ok(blob)
}

fn read_tape(encoded: &[Mark], i: &mut usize) -> Result<Tape, String> {
    if encoded.get(*i).copied() != Some('∈') {
        return Err(String::from("missing dialectic numeral field"));
    }
    *i += 1;
    let mut tape = Vec::new();
    loop {
        let mark = *encoded
            .get(*i)
            .ok_or_else(|| String::from("truncated dialectic numeral field"))?;
        *i += 1;
        if mark == '∋' {
            break;
        }
        if mark != EVALT && mark != EVALF {
            return Err(String::from("dialectic numeral contains a non-numeral mark"));
        }
        tape.push(mark);
    }
    if tape.is_empty() {
        return Err(String::from("dialectic numeral is empty"));
    }
    Ok(trim(tape))
}

fn read_mask(encoded: &[Mark], i: &mut usize, bits: usize) -> Result<u32, String> {
    if encoded.get(*i).copied() != Some('∈') {
        return Err(String::from("missing dialectic imscription mask field"));
    }
    *i += 1;
    let mut mask = 0u32;
    for bit in 0..bits {
        match encoded.get(*i).copied() {
            Some(EVALT) => {}
            Some(EVALF) => mask |= 1u32 << bit,
            _ => return Err(String::from("malformed dialectic imscription mask")),
        }
        *i += 1;
    }
    if encoded.get(*i).copied() != Some('∋') {
        return Err(String::from("unterminated dialectic imscription mask"));
    }
    *i += 1;
    Ok(mask)
}
