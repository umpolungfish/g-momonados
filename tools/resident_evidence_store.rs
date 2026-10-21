//! Append-only retained evidence. A checkpoint names an immutable prefix.
//! Static continuation loads that retained prefix from the compiled image.
use std::{fs::{File,OpenOptions},io::{Read,Write,Seek,SeekFrom},path::{Path,PathBuf},marker::PhantomData};
use serde::{Serialize,de::DeserializeOwned};
use serde_json::{Value,json};

pub struct Archive<T> {
    memory:Vec<T>, file:Option<File>, path:Option<PathBuf>,
    records:Vec<(u64,u64)>, bytes:u64, marker:PhantomData<T>,
    embedded:Option<&'static [u8]>,
}
impl<T:Clone+Serialize+DeserializeOwned> Archive<T> {
    pub fn new()->Self {Self{memory:Vec::new(),file:None,path:None,records:Vec::new(),bytes:0,marker:PhantomData,embedded:None}}
    pub fn len(&self)->usize {if self.file.is_some(){self.records.len()}else if self.embedded.is_some(){self.records.len()+self.memory.len()}else{self.memory.len()}}
    pub fn push(&mut self,value:T) {
        if let Some(file)=&mut self.file {
            let mut encoder=flate2::write::GzEncoder::new(Vec::new(),flate2::Compression::fast());
            serde_json::to_writer(&mut encoder,&value).unwrap();let encoded=encoder.finish().unwrap();
            let length=encoded.len() as u64;file.write_all(&length.to_le_bytes()).unwrap();file.write_all(&encoded).unwrap();
            self.records.push((self.bytes+8,length));self.bytes+=8+length;
        }else{self.memory.push(value);}
    }
    pub fn get(&self,index:usize)->T {
        if let Some(raw)=self.embedded {
            if index>=self.records.len(){return self.memory[index-self.records.len()].clone();}
            let(offset,length)=self.records[index];
            return serde_json::from_reader(flate2::read::GzDecoder::new(&raw[offset as usize..(offset+length) as usize])).unwrap();
        }
        if let Some(file)=&self.file {
            let(offset,length)=self.records[index];let mut bytes=vec![0;length as usize];
            // An independent descriptor leaves the append position untouched.
            let mut reader=File::open(self.path.as_ref().unwrap()).unwrap();
            reader.seek(SeekFrom::Start(offset)).unwrap();reader.read_exact(&mut bytes).unwrap();
            let _=file;serde_json::from_reader(flate2::read::GzDecoder::new(bytes.as_slice())).unwrap()
        }else{self.memory[index].clone()}
    }
    pub fn iter(&self)->impl Iterator<Item=T>+'_ {(0..self.len()).map(|index|self.get(index))}
    pub fn enable(&mut self,path:&Path) {
        assert!(self.file.is_none() && self.embedded.is_none());let values=std::mem::take(&mut self.memory);
        self.file=Some(OpenOptions::new().create_new(true).write(true).open(path).unwrap());self.path=Some(path.to_path_buf());
        for value in values{self.push(value);}
    }
    pub fn sync(&self) {if let Some(file)=&self.file{file.sync_all().unwrap();}}
    pub fn view(&self)->Value {
        assert!(self.embedded.is_none(),"publish the embedded prefix to the execution evidence archive before readout");
        if let Some(path)=&self.path {json!({"external_archive":path,"encoding":"length-u64le-gzip-json","records":self.len(),"bytes":self.bytes})}
        else{serde_json::to_value(&self.memory).unwrap()}
    }
    pub fn load(value:&Value)->Self {
        if let Some(path)=value.get("external_archive").and_then(Value::as_str) {
            assert_eq!(value["encoding"],"length-u64le-gzip-json");
            let count=value["records"].as_u64().unwrap() as usize;let boundary=value["bytes"].as_u64().unwrap();
            let mut file=File::open(path).unwrap();assert!(file.metadata().unwrap().len()>=boundary);
            let mut records=Vec::new();let mut offset=0;
            for _ in 0..count {let mut header=[0;8];file.read_exact(&mut header).unwrap();let length=u64::from_le_bytes(header);
                assert!(offset+8+length<=boundary);records.push((offset+8,length));offset+=8+length;
                file.seek(SeekFrom::Start(offset)).unwrap();}
            assert_eq!(offset,boundary);
            // Loaded prefixes stay read-only until enable() copies them into
            // the next execution's own archive, record by record.
            let mut result=Self::new();result.path=Some(path.into());result.records=records;result.bytes=boundary;
            result.file=Some(file);result
        }else{let mut result=Self::new();result.memory=serde_json::from_value(value.clone()).unwrap();result}
    }
    pub fn load_bound(value:&Value,bindings:&[(&str,&'static [u8])])->Self {
        let Some(path)=value.get("external_archive").and_then(Value::as_str) else{return Self::load(value);};
        if bindings.is_empty(){return Self::load(value);}
        let boundary=value["bytes"].as_u64().unwrap();
        let raw=bindings.iter().find(|(name,raw)|*name==path && raw.len() as u64==boundary)
            .expect("retained archive is absent from the compiled prefix bindings").1;
        assert_eq!(value["encoding"],"length-u64le-gzip-json");
        assert_eq!(raw.len() as u64,boundary,"compiled archive prefix boundary differs");
        let count=value["records"].as_u64().unwrap() as usize;
        let mut result=Self::new();let mut offset=0usize;
        for _ in 0..count {
            let length=u64::from_le_bytes(raw[offset..offset+8].try_into().unwrap());
            let end=offset.checked_add(8).unwrap().checked_add(usize::try_from(length).unwrap()).unwrap();
            assert!(end<=raw.len());result.records.push(((offset+8) as u64,length));offset=end;
        }
        assert_eq!(offset,raw.len());result.bytes=boundary;result.embedded=Some(raw);result
    }
    pub fn migrate(&mut self,path:&Path) {
        let mut old=std::mem::replace(self,Self::new());
        if old.embedded.is_some() || old.file.is_some() {
            let mut target=OpenOptions::new().create_new(true).write(true).open(path).unwrap();
            if let Some(raw)=old.embedded {target.write_all(raw).unwrap();}
            else {let mut source=File::open(old.path.as_ref().unwrap()).unwrap();
                assert_eq!(std::io::copy(&mut Read::by_ref(&mut source).take(old.bytes),&mut target).unwrap(),old.bytes);}
            self.records=std::mem::take(&mut old.records);self.bytes=old.bytes;
            self.path=Some(path.into());self.file=Some(target);
            for value in old.memory{self.push(value);}return;
        }
        self.enable(path);
        for value in old.iter(){self.push(value);}
    }
}
impl<T:Clone+Serialize+DeserializeOwned> Serialize for Archive<T> {
    fn serialize<S:serde::Serializer>(&self,serializer:S)->Result<S::Ok,S::Error>{self.view().serialize(serializer)}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resident_embedded_prefix_recovers_without_its_original_file() {
        let directory=PathBuf::from(std::env::var("RESIDENT_EVIDENCE_TEST_DIR").unwrap()).join("embedded");
        std::fs::create_dir_all(&directory).unwrap();
        let path=directory.join("parent.records");let next=directory.join("returned.records");
        let mut original=Archive::<(usize,Vec<u8>)>::new();
        original.push((0,vec![]));original.enable(&path);original.push((0,vec![3,2,1]));
        let prefix=original.view();original.push((1,vec![9;10000]));original.sync();
        let all=std::fs::read(&path).unwrap();let boundary=prefix["bytes"].as_u64().unwrap() as usize;
        let baked:&'static[u8]=Box::leak(all[..boundary].to_vec().into_boxed_slice());
        std::fs::rename(&path,directory.join("original-retained.records")).unwrap();
        let bindings=[(path.to_str().unwrap(),baked)];
        let mut restored=Archive::<(usize,Vec<u8>)>::load_bound(&prefix,&bindings);
        assert_eq!(restored.len(),2);assert_eq!(restored.get(1),(0,vec![3,2,1]));
        restored.push((1,vec![8]));assert_eq!(restored.get(2),(1,vec![8]));
        restored.migrate(&next);restored.sync();
        assert_eq!(&std::fs::read(&next).unwrap()[..boundary],baked);
        assert_eq!(restored.get(1),(0,vec![3,2,1]));assert_eq!(restored.get(2),(1,vec![8]));
        assert_eq!(restored.view()["records"],3);
    }
    #[test]
    fn resident_evidence_prefix_survives_append_and_reopen() {
        let directory=std::env::var("RESIDENT_EVIDENCE_TEST_DIR").unwrap();std::fs::create_dir_all(&directory).unwrap();
        let path=Path::new(&directory).join("prefix.records");let next=Path::new(&directory).join("next.records");
        let mut records=Archive::<(usize,Vec<u8>)>::new();records.push((0,vec![]));records.enable(&path);
        records.push((0,vec![3,2,1]));let prefix=records.view();records.push((1,vec![9;10000]));records.sync();
        let mut restored=Archive::<(usize,Vec<u8>)>::load(&prefix);assert_eq!(restored.len(),2);
        assert_eq!(restored.get(1),(0,vec![3,2,1]));restored.migrate(&next);restored.push((1,vec![8]));
        assert_eq!(records.get(2),(1,vec![9;10000]));assert_eq!(restored.get(2),(1,vec![8]));
    }
}
