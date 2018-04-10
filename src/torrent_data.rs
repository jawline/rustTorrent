/**
 * Manage writes and hashing of downloaded portions of a torrent
 */

use std::path::Path;
use std::fs::File;
use std::io;
use std::io::Write;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom};
use bitfield::Bitfield;

pub struct TorrentData {
    pub data_path: String,
    pub handle: File,
    pub have: Bitfield,
    pub pieces: Vec<Vec<u8>>, //Sha1 hashes of each piece of the torrent
    pub piece_size: usize
}

fn zeros(size: usize) -> Vec<u8> {
    (0..size).map(|_| 0).collect()
}

impl TorrentData {
    pub fn allocate(name: &str, pieces: Vec<Vec<u8>>, piece_size: usize) -> Result<TorrentData, io::Error> {

        if !Path::new(name).exists() {
            println!("Pre-allocating space for the torrent");

            let mut torrent_file = File::create(name)?;
            
            let zero_piece = zeros(piece_size);

            for _ in 0..pieces.len() {
                torrent_file.write(&zero_piece)?;
            }

            println!("Pre-allocated");
        } else {
            println!("TODO: File exists, should check");
        }

        Ok(TorrentData {
            data_path: name.to_string(),
            handle: OpenOptions::new().read(true).write(true).open(name)?,
            have: Bitfield::new((0..(pieces.len() / 8) + 1).map(|_| 0).collect()),
            pieces: pieces,
            piece_size: piece_size
        })
    }

    pub fn write(&mut self, piece: usize, data: &[u8]) -> io::Result<()> {
        //println!("TODO: TorrentData preserve handle");
        //println!("TODO: TorrentData check piece hash");
 
        self.handle.seek(SeekFrom::Start((piece * self.piece_size) as u64))?;
        self.handle.write(data)?;
        self.have.set(piece); 
        Ok(())
    }
}
