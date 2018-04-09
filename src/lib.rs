#![feature(try_trait)]
#[allow(dead_code)]

extern crate sha1;
extern crate url;
extern crate byteorder;
extern crate rand;
extern crate reqwest;

mod bencoder;
mod bencoder_recode;
mod torrent;
mod torrent_data;
mod tracker;
mod download;
mod peer_server;
mod peer_client;
mod peer_id;
mod urlencode;
mod bitfield;
