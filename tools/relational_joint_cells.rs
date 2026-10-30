//! Retain the native adder's whole joint relation across its five ports.
//! Port discovery verifies the existing RelationProgram lowering exactly;
//! numeric cell returns come from the live native word arithmetic interpreter.
use crate::{fixed_point_imasm::ImasmNumeralWord, fixed_point_word_arithmetic as arithmetic};

pub fn full_adder_rows() -> [[u8;5];8] {
    let zero=arithmetic::zero();let one=arithmetic::one();
    core::array::from_fn(|index| {
        let inputs:[bool;3]=core::array::from_fn(|bit|index & (1<<bit)!=0);
        let word=|value|if value {&one}else{&zero};
        let pair=arithmetic::add(word(inputs[0]),word(inputs[1])).unwrap();
        let sum=arithmetic::add(&pair,word(inputs[2])).unwrap();
        let bits:Vec<_>=sum.as_str().chars().filter(|c|matches!(c,'⊤'|'⊥')).collect();
        let sum_mark=if bits.first()==Some(&'⊥'){1}else{2};
        let carry_mark=if bits.get(1)==Some(&'⊥'){1}else{2};
        [if inputs[0]{1}else{2},if inputs[1]{1}else{2},if inputs[2]{1}else{2},sum_mark,carry_mark]
    })
}

pub fn full_adder_ports(ops:&[[u32;4]]) -> Vec<[usize;5]> {
    let signature=[3,3,4,4,7,3,3,4,4,7,4,4,7];
    let mut result=Vec::new();
    for s in 0..ops.len().saturating_sub(12) {
        let block=&ops[s..s+13];
        if !block.iter().zip(signature).all(|(row,op)|row[0]==op) {continue;}
        let(a,b,c)=(block[0][1],block[1][1],block[6][1]);
        if a==b || b==c || a==c {continue;}
        let i=s as u32;
        let expected=[[3,a,0],[3,b,0],[4,a,i+1],[4,i,b],[7,i+2,i+3],
            [3,i+4,0],[3,c,0],[4,i+4,i+6],[4,i+5,c],[7,i+7,i+8],
            [4,a,b],[4,i+4,c],[7,i+10,i+11]];
        if block.iter().zip(expected).all(|(row,want)|row[..3]==want && row[3]==0) {
            result.push([a as usize,b as usize,c as usize,s+9,s+12]);
        }
    }
    result
}

pub fn check_lowering(ops:&[[u32;4]], ports:&[[usize;5]], rows:&[[u8;5];8]) {
    for &[a,b,c,sum,carry] in ports {
        let s=sum-9;
        for row in rows {
            let mut internal=[false;13];
            let input=|wire:u32, internal:&[bool;13]| {
                let wire=wire as usize;
                if wire==a {row[0]==1} else if wire==b {row[1]==1}
                else if wire==c {row[2]==1} else {internal[wire-s]}
            };
            for offset in 0..13 {
                let[op,x,y,_]=ops[s+offset];
                let xv=input(x,&internal);
                internal[offset]=if op==3 {!xv} else {
                    let yv=input(y,&internal); if op==4 {xv&&yv}else{xv||yv}
                };
            }
            assert_eq!(internal[sum-s],row[3]==1,"native sum disagrees with prepared lowering");
            assert_eq!(internal[carry-s],row[4]==1,"native carry disagrees with prepared lowering");
        }
    }
}

/// Return the union of surviving JOINT rows at each port. This is relational
/// projection after compatibility, never an independent product of marginals.
pub fn supported(rows:&[[u8;5];8], domains:[u8;5], image:[u8;2]) -> [u8;5] {
    let mut result=[0;5];
    for row in rows {
        if (0..5).all(|port| (0..2).any(|fibre|
            domains[port] & (1<<fibre)!=0
                && image[fibre]==image[if row[port]==1 {0}else{1}]
                && (1<<fibre)==row[port])) {
            for port in 0..5 {result[port]|=row[port];}
        }
    }
    result
}

pub fn native_row_words(rows:&[[u8;5];8]) -> Vec<ImasmNumeralWord> {
    rows.iter().map(|row| {
        let mut word=String::from("⊢");
        for &mark in row {word.push_str(if mark==1 {"≻⋈∈⊥∋"}else{"≻⋈∈⊤∋"});}
        word.push_str("⊙⊡⊣");ImasmNumeralWord::checked(&word).unwrap()
    }).collect()
}
