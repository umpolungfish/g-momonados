//! Source-bound inverse product constraints over the native numeral words.
//! Bounds are obtained from the retained alternatives, never supplied widths.
use crate::{fixed_point_imasm::ImasmNumeralWord, fixed_point_word_arithmetic as arithmetic};
use serde_json::{json,Value};

fn word(domains:&[u8],maximum:bool)->ImasmNumeralWord {
    let bits:Vec<_>=domains.iter().map(|&d|{
        let set=if maximum{d&1!=0}else{d==1};if set{1}else{2}
    }).collect();
    let end=bits.iter().rposition(|&d|d==1).map_or(0,|i|i+1);
    let mut result=String::from("⊢");
    for &bit in &bits[..end] {result.push_str(if bit==1{"≻⋈∈⊥∋"}else{"≻⋈∈⊤∋"});}
    result.push_str("⊙⊡⊣");ImasmNumeralWord::checked(&result).unwrap()
}
fn marks(word:&ImasmNumeralWord)->Vec<u8>{word.as_str().chars().filter_map(|c|match c{'⊤'=>Some(0),'⊥'=>Some(1),_=>None}).collect()}

/// All paths through the interval automaton are retained. Projection happens
/// only after both the lower and upper prefix conditions have been applied.
pub fn interval_support(domains:&[u8],lower:&ImasmNumeralWord,upper:&ImasmNumeralWord)->Vec<u8>{
    let width=domains.len();let lo=marks(lower);let hi=marks(upper);
    if lo.len()>width || arithmetic::compare(lower,upper).unwrap().is_gt(){return vec![0;width];}
    let transition=|position:usize,state:usize,bit:u8|->Option<usize>{
        let l=*lo.get(position).unwrap_or(&0);let h=*hi.get(position).unwrap_or(&0);
        if state&1!=0 && bit<l || state&2!=0 && bit>h{return None;}
        Some(usize::from(state&1!=0 && bit==l)|usize::from(state&2!=0 && bit==h)*2)
    };
    let mut forward=vec![[false;4];width+1];forward[0][if hi.len()>width{1}else{3}]=true;
    for depth in 0..width {let position=width-depth-1;
        for state in 0..4 {if forward[depth][state] {for bit in 0..2 {
            if domains[position]&(if bit==1{1}else{2})!=0 {
                if let Some(next)=transition(position,state,bit){forward[depth+1][next]=true;}
            }
        }}}
    }
    let mut backward=[true;4];let mut result=vec![0;width];
    for depth in (0..width).rev(){let position=width-depth-1;let mut previous=[false;4];
        for state in 0..4 {for bit in 0..2 {
            let mark=if bit==1{1}else{2};
            if domains[position]&mark==0{continue;}
            if let Some(next)=transition(position,state,bit){if backward[next] {
                previous[state]=true;
                if forward[depth][state]{result[position]|=mark;}
            }}
        }}
        backward=previous;
    }
    result
}

pub struct ProductReturn {pub p:Vec<u8>,pub q:Vec<u8>,pub bank:Value}
pub fn refine(source:&ImasmNumeralWord,p:&[u8],q:&[u8])->ProductReturn{
    assert_eq!(marks(source).first(),Some(&1),"proper-odd product return requires an odd source");
    let three=arithmetic::add(&arithmetic::one(),&arithmetic::add(&arithmetic::one(),&arithmetic::one()).unwrap()).unwrap();
    let minimum=|domains:&[u8]|{let value=word(domains,false);if arithmetic::compare(&value,&three).unwrap().is_lt(){three.clone()}else{value}};
    let (pmin,pmax,qmin,qmax)=(minimum(p),word(p,true),minimum(q),word(q,true));
    if arithmetic::compare(&pmin,&pmax).unwrap().is_gt() || arithmetic::compare(&qmin,&qmax).unwrap().is_gt(){
        return ProductReturn{p:vec![0;p.len()],q:vec![0;q.len()],bank:json!({"source_word":source.as_str(),"contradiction":"proper odd lane has no value at least three"})};
    }
    let (p_floor,p_lower_rem)=arithmetic::divide(source,&qmax).unwrap();
    let (p_upper,p_upper_rem)=arithmetic::divide(source,&qmin).unwrap();
    let (q_floor,q_lower_rem)=arithmetic::divide(source,&pmax).unwrap();
    let (q_upper,q_upper_rem)=arithmetic::divide(source,&pmin).unwrap();
    let ceil=|floor:&ImasmNumeralWord,remainder:&ImasmNumeralWord|if remainder==&arithmetic::zero(){floor.clone()}else{arithmetic::increment(floor).unwrap()};
    let p_lower=ceil(&p_floor,&p_lower_rem);let q_lower=ceil(&q_floor,&q_lower_rem);
    let lower=|a:&ImasmNumeralWord,b:&ImasmNumeralWord|if arithmetic::compare(a,b).unwrap().is_gt(){a.clone()}else{b.clone()};
    let upper=|a:&ImasmNumeralWord,b:&ImasmNumeralWord|if arithmetic::compare(a,b).unwrap().is_lt(){a.clone()}else{b.clone()};
    let pl=lower(&pmin,&p_lower);let pu=upper(&pmax,&p_upper);
    let ql=lower(&qmin,&q_lower);let qu=upper(&qmax,&q_upper);
    let p_return=interval_support(p,&pl,&pu);let q_return=interval_support(q,&ql,&qu);
    let bank=json!({"source_word":source.as_str(),"p_min":pmin.as_str(),"p_max":pmax.as_str(),
        "q_min":qmin.as_str(),"q_max":qmax.as_str(),"p_lower":pl.as_str(),"p_upper":pu.as_str(),
        "q_lower":ql.as_str(),"q_upper":qu.as_str(),"quotient_returns":[
            {"denominator":qmax.as_str(),"floor":p_floor.as_str(),"remainder":p_lower_rem.as_str()},
            {"denominator":qmin.as_str(),"floor":p_upper.as_str(),"remainder":p_upper_rem.as_str()},
            {"denominator":pmax.as_str(),"floor":q_floor.as_str(),"remainder":q_lower_rem.as_str()},
            {"denominator":pmin.as_str(),"floor":q_upper.as_str(),"remainder":q_upper_rem.as_str()}]});
    ProductReturn{p:p_return,q:q_return,bank}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_interval_support_fibres(){
        let mut words=vec![arithmetic::zero()];
        for i in 1..=32 {words.push(arithmetic::increment(&words[i-1]).unwrap());}
        for code in 0..243usize {
            let mut value=code;let mut domains=Vec::new();
            for _ in 0..5 {domains.push((value%3+1) as u8);value/=3;}
            for lower in 0..=32 {for upper in lower..=32 {
                let actual=interval_support(&domains,&words[lower],&words[upper]);
                let mut expected=vec![0;5];
                for n in lower..=upper {if n>=32{continue;}
                    let row:Vec<_>=(0..5).map(|bit|if n&(1<<bit)!=0{1}else{2}).collect();
                    if row.iter().zip(&domains).all(|(&bit,&domain)|bit&domain!=0){
                        for (result,&bit) in expected.iter_mut().zip(&row){*result|=bit;}
                    }
                }
                assert_eq!(actual,expected,"source interval fibre differs");
            }}
        }
    }
}
