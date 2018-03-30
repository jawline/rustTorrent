#![feature(try_trait)]
#[allow(dead_code)]

extern crate sha1;
extern crate url;
extern crate byteorder;
extern crate rand;

mod bencoder;
mod bencoder_recode;
mod torrent;
mod tracker;
mod download;
mod peer_id;

use std::env;

pub fn main() {
    download::download(&env::args().nth(1).unwrap());    
}
