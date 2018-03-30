use std::fs::File;
use std::io::Read;
use sha1;
use bencoder::{Entry, decode};
use peer_id::gen_peer_id;

#[derive(Debug)]
#[derive(Clone)]
pub struct Info {
    pub name: String,
    pub announce: String,
    pub piece_length: usize,
    pub pieces: Vec<Vec<u8>>,
    pub files: Vec<FileInfo>,
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>
}

#[derive(Debug)]
#[derive(Clone)]
pub struct FileInfo {
    pub path: String,
    pub length: usize
}

pub fn from_string(input: &mut &[u8]) -> Result<Entry, &'static str> {
    let res = decode(input);
    if res.is_err() {
        Err("Parsing Error")
    } else {
        Ok(res.unwrap())
    }
}

pub fn from_file(file_path: &str) -> Result<Entry, &'static str> {
    let file = File::open(file_path);

    if file.is_err() {
        return Err("File Open");
    }

    let mut file = file.unwrap();
    let mut buffer = Vec::new();
    
    if file.read_to_end(&mut buffer).is_err() { 
        return Err("File Read");
    }

    let mut c_slice: &[u8] = &buffer;
    from_string(&mut c_slice)
}

pub fn prepare(torrent: &Entry) -> Result<Info, &'static str> {
    let info = torrent.field("info")?;
    let announce = torrent.field("announce")?;
    let name = info.field("name")?;
    let piece_length = info.field("piece length")?;
    let files = info.field("files");

    println!("TODO: Extract pieces hash");

    let mut info_digest = sha1::Sha1::new();
    info_digest.update(&info.bencode());

    let mut extracted = Info {
        name: name.to_string(),
        announce: announce.to_string(),
        piece_length: piece_length.as_usize()?,
        pieces: Vec::new(),
        files: Vec::new(),
        info_hash: info_digest.digest().bytes().to_vec(),
        peer_id: gen_peer_id() 
    }; 

    if files.is_ok() {
        let files = files.unwrap();
        
        if let Entry::List(files) = files {
            for file in files {
                let path = file.field("path")?;
                let length = file.field("length")?;
                extracted.files.push(FileInfo {
                    path: path.to_string(),
                    length: length.as_usize()?
                });
            }
        }

    } else {
        let length = info.field("length")?;
        extracted.files.push(FileInfo {
            path: name.to_string(),
            length: length.as_usize()?
        });
    }

    Ok(extracted)
}
