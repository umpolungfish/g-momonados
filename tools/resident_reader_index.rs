//! Prepared dependency incidence, read directly from immutable executable data.
//! Building the per-wire lists belongs to compilation, not the trigger path.
pub struct ReaderIndex<'a> {raw:&'a[u8], wires:usize, data:usize}
fn read(raw:&[u8],at:usize)->usize {u64::from_le_bytes(raw[at..at+8].try_into().unwrap()) as usize}
impl<'a> ReaderIndex<'a> {
    pub fn new(raw:&'a[u8])->Self{
        let wires=read(raw,0);let data=8+8*(wires+1);
        assert_eq!(data+8*read(raw,8+8*wires),raw.len());
        Self{raw,wires,data}
    }
    pub fn wires(&self)->usize{self.wires}
    pub fn len(&self,wire:usize)->usize{assert!(wire<self.wires);read(self.raw,16+8*wire)-read(self.raw,8+8*wire)}
    pub fn get(&self,wire:usize,index:usize)->usize{
        assert!(index<self.len(wire));read(self.raw,self.data+8*(read(self.raw,8+8*wire)+index))
    }
    pub fn encode(readers:&[Vec<usize>])->Vec<u8>{
        let mut result=Vec::new();result.extend_from_slice(&(readers.len() as u64).to_le_bytes());
        let mut offset=0u64;result.extend_from_slice(&offset.to_le_bytes());
        for row in readers{offset+=row.len() as u64;result.extend_from_slice(&offset.to_le_bytes());}
        for row in readers{for &reader in row{result.extend_from_slice(&(reader as u64).to_le_bytes());}}
        result
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn prepared_incidence_roundtrip(){
        let expected=vec![vec![0,2,2],Vec::new(),vec![5],vec![6,9]];
        let raw=ReaderIndex::encode(&expected);let index=ReaderIndex::new(&raw);
        assert_eq!(index.wires(),expected.len());
        for (wire,row) in expected.iter().enumerate(){
            assert_eq!(index.len(wire),row.len());
            for (at,&reader) in row.iter().enumerate(){assert_eq!(index.get(wire,at),reader);}
        }
    }
}
