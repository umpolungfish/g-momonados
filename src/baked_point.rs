//! Typed Cargo image of the complete source-bound point membrane.
use crate::baked_resident::{BakedNode,BakedSic,BakedWordComplex};

pub struct BakedSparseSupport {
    pub wires:&'static[u64],
    pub spans:&'static[(usize,usize)],
    pub fixed:&'static[(bool,usize,usize)],
}

pub struct BakedPointProjector {
    pub scopes:&'static[&'static[usize]],
    pub incidence:&'static[(usize,&'static[usize])],
    pub internal:&'static[usize],
    pub first_internal:usize,
    pub pending:&'static[(usize,usize,usize)],
    pub versions:&'static[(usize,usize)],
    pub constraints:usize,
    pub contractions:usize,
    pub copy_returns:&'static[(usize,usize,bool)],
}
pub struct BakedPointGraph {
    pub nodes:&'static[imasm_core::classic::Token],
    pub edges:&'static[(usize,usize)],
    pub word:&'static str,
    pub reference_inputs:&'static[imasm_core::imasm16_3::Reg16_3],
    pub reference_edges:&'static[imasm_core::imasm16_3::Reg16_3],
}
pub struct BakedPointProtocol {
    pub sic:&'static BakedSic,
    pub dual_denominator:crate::baked_resident::BakedInteger,
    pub dual_columns:&'static[[crate::baked_resident::BakedComplex;16];16],
    pub modulus_bits:u64,
    pub w_bits:u64,
    pub central_step:&'static[BakedWordComplex;16],
    pub engagement:&'static[BakedWordComplex;16],
    pub disassembler:&'static str,
    pub recompiler:&'static str,
    pub forward:[usize;16],
    pub backward:[usize;16],
    pub program_self_return:bool,
    pub executed_trie_steps:u64,
}
pub struct BakedPointY {
    pub postfix:&'static[u8],
    pub prefix_trace:&'static[&'static str],
    pub prefix_contractions:u64,
    pub contractions:u64,
    pub calls:u64,
    pub done:bool,
}
pub struct BakedPointRuntime {
    pub row:usize,
    pub column:usize,
    pub subtract:bool,
    pub reduced:bool,
    pub projected:bool,
    pub operations:u64,
    pub protocol_bank_count:usize,
    pub protocol_returns:&'static str,
}
pub struct BakedPoint {
    pub source_digits:&'static[u32],
    pub source_word:&'static str,
    pub hex_word:&'static str,
    pub binary_digits:&'static[char],
    pub powers:&'static[&'static[u32]],
    pub complement_word:&'static str,
    pub moment_offset:crate::baked_resident::BakedSignedWord,
    pub moment_weights:&'static[crate::baked_resident::BakedSignedWord],
    pub cells:usize,
    pub roots:&'static[usize],
    pub nodes:&'static[BakedNode],
    pub support:&'static BakedSparseSupport,
    pub point_wires:&'static[usize],
    pub residual_wires:&'static[usize],
    pub coordinates:[usize;2],
    pub fork_history:[usize;2],
    pub engagement_history:[[usize;2];2],
    pub projector:&'static BakedPointProjector,
    pub protocol:&'static BakedPointProtocol,
    pub graphs:&'static[BakedPointGraph;2],
    pub y:&'static BakedPointY,
    pub runtime:&'static BakedPointRuntime,
    pub preparation_nanoseconds:u64,
}
