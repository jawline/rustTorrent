use std::fs::File;
use std::io::Read;
use bencoder::{Entry, decode};

#[derive(Debug)]
#[derive(Clone)]
pub struct Info {
    pub name: String,
    pub piece_length: usize,
    pub pieces: Vec<Vec<u8>>,
    pub files: Vec<FileInfo>
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

pub fn split_pieces(pieces: &[u8]) -> Vec<Vec<u8>> {
    const SIZE_PIECE: usize = 20;

    let mut result = Vec::new();
    let num_iterations = pieces.len() / SIZE_PIECE;
    let mut pieces = pieces.iter();

    for _ in 0..num_iterations {
        result.push(pieces.take(SIZE_PIECE).map(|x| *x).collect());
    }

    result
}

pub fn info(torrent: &Entry) -> Result<Info, &'static str> {
    let info = torrent.field("info")?;
    let name = info.field("name")?;
    let piece_length = info.field("piece length")?;
    let files = info.field("files");


    let pieces = 

    let mut extracted = Info {
        name: name.to_string(),
        piece_length: piece_length.as_usize()?,
        pieces: split_pieces(info.field("pieces")?.as_bytes()?),
        files: Vec::new()
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
