//! Numerical evidence is indexed by proposition and independent source.
use super::certificate::EvidencePolicy;
use crate::belnap_residual::V;
use std::collections::BTreeMap;
#[derive(Default)]
pub struct FourEvidence {
    records: BTreeMap<(String, String), V>,
}
impl FourEvidence {
    pub fn record(
        &mut self,
        proposition: &str,
        source: &str,
        residual: Option<f64>,
        policy: EvidencePolicy,
    ) {
        self.records.insert(
            (proposition.into(), source.into()),
            policy.classify(residual),
        );
    }
    pub fn verdict(&self, proposition: &str) -> V {
        self.records
            .iter()
            .filter(|((p, _), _)| p == proposition)
            .fold(V::N, |v, (_, e)| v.join(*e))
    }
}
