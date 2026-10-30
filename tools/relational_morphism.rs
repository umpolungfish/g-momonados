//! Lift native IMASM graph morphisms to a joint arithmetic relation.
//!
//! Rows are never replaced by a Cartesian product of port marginals. Each
//! edge carries the complete joint row bank, indexed by the same ancestry.
//! A later environment receives the previous environment's actual row values.
use imasm_core::{check::Graph, classic::Token, flow, imasm16_3::Reg16_3};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JointRelation {
    /// Exact source boundary supplied by the caller, not a descriptor tuple.
    pub source_word: String,
    /// Resident factor/product/carry wire identities, in port order.
    pub ports: Vec<usize>,
    /// Immutable assignment ancestry. Equal images do not identify two rows.
    pub origins: Vec<Vec<u8>>,
    /// Current whole rows in the environment's full SIXTEEN_3 carrier.
    pub rows: Vec<Vec<u8>>,
    pub environments: Vec<String>,
}

pub struct Execution {
    pub relation: JointRelation,
    /// Node and edge banks retain row, then port order.
    pub node_inputs: Vec<Vec<Vec<u8>>>,
    pub edge_values: Vec<Vec<Vec<u8>>>,
    pub committed_nodes: Vec<usize>,
    pub rounds: usize,
}

fn value(mask: u8) -> Reg16_3 {
    Reg16_3 {big_t:mask&1!=0,big_f:mask&2!=0,
             small_t:mask&4!=0,small_f:mask&8!=0}
}
fn mask(v: Reg16_3) -> u8 {
    v.big_t as u8 | (v.big_f as u8)<<1 | (v.small_t as u8)<<2 | (v.small_f as u8)<<3
}

impl JointRelation {
    pub fn new(source_word: String, ports: Vec<usize>, rows: Vec<Vec<u8>>) -> Result<Self,String> {
        if source_word.is_empty() || ports.is_empty() ||
            rows.iter().any(|r|r.len()!=ports.len() || r.iter().any(|&v|v>15)) {
            return Err("joint relation lost source, ports or full-carrier rows".into());
        }
        let mut unique=ports.clone();unique.sort_unstable();unique.dedup();
        if unique.len()!=ports.len() {return Err("duplicate arithmetic port".into());}
        Ok(Self {source_word,ports,origins:rows.clone(),rows,environments:Vec::new()})
    }

    pub fn through(&self, name: &str, graph: &Graph) -> Result<Execution,String> {
        if !graph.validate().is_empty() {return Err("ill-typed environment graph".into());}
        if !graph.nodes.contains(&Token::Vinit) || !graph.nodes.contains(&Token::Tanch) {
            return Err("environment requires source and return boundaries".into());
        }
        // Spell out the entire alphabet. Extensions require their own lift;
        // an unimplemented operator must never silently become identity.
        for op in &graph.nodes {
            match op {
                Token::Vinit|Token::Tanch|Token::Afwd|Token::Arev|Token::Clink|
                Token::Evalt|Token::Fsplit|Token::Ffuse|Token::Imscrib|
                Token::Evalf|Token::Engagr|Token::Ifix=>{},
                _=>return Err("environment uses an unsupported token morphism".into()),
            }
        }
        let empty=vec![vec![0;self.ports.len()];self.rows.len()];
        let mut edges=vec![empty.clone();graph.edges.len()];
        let mut nodes=vec![empty.clone();graph.nodes.len()];
        let bound=4*graph.edges.len().max(1)+4;
        let mut rounds=0;
        for _ in 0..bound {
            rounds+=1;let mut changed=false;
            for (i,&op) in graph.nodes.iter().enumerate() {
                let mut input=empty.clone();
                for (e,&(_,to)) in graph.edges.iter().enumerate() {
                    if to==i {
                        for (row,received) in input.iter_mut().zip(&edges[e]) {
                            for (v,&w) in row.iter_mut().zip(received) {*v|=w;}
                        }
                    }
                }
                nodes[i]=input;
                let fan=graph.out_degree(i);let mut slot=0;
                for (e,&(from,_)) in graph.edges.iter().enumerate() {
                    if from!=i {continue;}
                    for row in 0..self.rows.len() {for port in 0..self.ports.len() {
                        let out=mask(flow::gate_out(op,value(nodes[i][row][port]),
                            value(self.rows[row][port]),slot,fan));
                        if edges[e][row][port]!=out {edges[e][row][port]=out;changed=true;}
                    }}
                    slot+=1;
                }
            }
            if !changed {
                let mut result=self.clone();result.rows=empty;
                for (i,&op) in graph.nodes.iter().enumerate() {
                    if op==Token::Tanch {
                        for (row,input) in result.rows.iter_mut().zip(&nodes[i]) {
                            for (v,&w) in row.iter_mut().zip(input) {*v|=w;}
                        }
                    }
                }
                result.environments.push(name.into());
                return Ok(Execution {relation:result,node_inputs:nodes,edge_values:edges,
                    committed_nodes:graph.nodes.iter().enumerate().filter_map(|(i,&op)|
                        (op==Token::Ifix).then_some(i)).collect(),rounds});
            }
        }
        Err("joint native flow did not reach its fixed-point bound".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use imasm_core::check;
    #[test]
    fn native_flow_and_joint_lift_agree_for_all_carrier_values() {
        let ops=[Token::Afwd,Token::Arev,Token::Clink,Token::Evalt,
                 Token::Evalf,Token::Engagr,Token::Imscrib,Token::Ifix];
        for op in ops {
            let g=check::from_sequence(&[Token::Vinit,Token::Fsplit,op,Token::Ffuse,Token::Tanch],&[(1,3)]);
            let relation=JointRelation::new("bound-source".into(),vec![2],(0..16).map(|v|vec![v]).collect()).unwrap();
            let actual=relation.through("native",&g).unwrap();
            for seed in 0..16 {
                let (nodes,_)=flow::flow_values(&g,value(seed));
                assert_eq!(actual.relation.rows[seed as usize][0],mask(nodes[4]));
            }
        }
    }
    #[test]
    fn successive_environments_act_on_the_joint_carry_rows() {
        let rows:Vec<_>=(0..8).map(|i| {
            let a=i&1;let b=(i>>1)&1;let c=(i>>2)&1;let total=a+b+c;
            [a,b,c,total&1,(total>>1)&1].map(|b|if b==1 {1}else{2}).to_vec()
        }).collect();
        let source=JointRelation::new("bound-product-source".into(),vec![2,3,4,5,6],rows.clone()).unwrap();
        let reverse=check::from_sequence(&[Token::Vinit,Token::Arev,Token::Tanch],&[]);
        let first=source.through("reverse",&reverse).unwrap();
        assert_ne!(first.relation.rows,source.rows);
        for (before,after) in source.rows.iter().zip(&first.relation.rows) {
            assert_eq!(after,&before.iter().map(|&v|if v==1 {2}else{1}).collect::<Vec<_>>());
        }
        let second=first.relation.through("reverse-again",&reverse).unwrap();
        assert_eq!(second.relation.rows,rows);
        assert_eq!(second.relation.origins,source.origins);
        assert_eq!(second.relation.ports,source.ports);
        assert_eq!(second.relation.source_word,source.source_word);
        assert_eq!(second.relation.environments,vec!["reverse","reverse-again"]);
        let projection=check::from_sequence(&[Token::Vinit,Token::Evalt,Token::Tanch],&[]);
        let projected=first.relation.through("truth",&projection).unwrap();
        assert!(projected.relation.rows.iter().flatten().all(|&v|v==0 || v==1));
        assert_eq!(projected.relation.origins.len(),8);
        // The intermediate complement matters: projecting the original is different.
        assert_ne!(projected.relation.rows,source.through("truth",&projection).unwrap().relation.rows);
    }
}
