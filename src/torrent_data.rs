/**
 * Manage writes and hashing of downloaded portions of a torrent
 */

use std::fs::File;
use std::io;
use std::io::Write;
use bitfield::Bitfield;

pub struct TorrentData {
    pub data_path: String,
    pub data_have: Bitfield,
    pub pieces: Vec<Vec<u8>>, //Sha1 hashes of each piece of the torrent
    pub piece_size: usize
}

fn zeros(size: usize) -> Vec<u8> {
    (0..size).map(|_| 0).collect()
}

impl TorrentData {
    pub fn allocate(name: &str, pieces: Vec<Vec<u8>>, piece_size: usize) -> Result<TorrentData, io::Error> {

        println!("Pre-allocating space for the torrent");

        let mut torrent_file = File::create(name)?;
        
        let zero_piece = zeros(piece_size);

        for _ in 0..pieces.len() {
            torrent_file.write(&zero_piece)?;
        }

        println!("Pre-allocated");

        Ok(TorrentData {
            data_path: name.to_string(),
            data_have: Bitfield::new((0..pieces.len() / 8).map(|_| 0).collect()),
            pieces: pieces,
            piece_size: piece_size
        })
    }
}
