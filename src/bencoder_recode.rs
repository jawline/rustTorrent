use bencoder::EntryData;


impl EntryData {
    //TODO: The bencode for List and Dictionary looks horrible, could be an iterator
    pub fn bencode(&self) -> Vec<u8> {
        match self {
            &EntryData::Str(ref v) => (v.len().to_string() + ":").as_bytes().iter().chain(v).map(|x| *x).collect(),
            &EntryData::Int(ref v) => ("i".to_string() + &v.to_string() + "e").as_bytes().iter().map(|x| *x).collect(),
            &EntryData::List(ref v) => {
                let mut res = Vec::new();
                res.extend("l".as_bytes());
                v.iter().for_each(|i| res.extend(&i.data.bencode()));
                res.extend("e".as_bytes());
                res                
            },
            &EntryData::Dictionary(ref v) => {
                let mut res = Vec::new();
                res.extend("d".as_bytes());
 
                for (name, data) in v {
                    res.extend(&EntryData::Str(name.as_bytes().to_vec()).bencode());
                    res.extend(&data.data.bencode());
                }

                res.extend("e".as_bytes());
                res
            }
        }
    }
}
