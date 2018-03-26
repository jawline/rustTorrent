#![feature(try_trait)]
#[allow(dead_code)]

extern crate url;
extern crate byteorder;

mod bencoder;
mod torrent;
mod tracker;

use std::env;
use torrent::*;
use tracker::{TrackerData, connect};

pub fn main() {
    let filename = env::args().nth(1).unwrap();
    println!("Loading {}", filename);
    let root = from_file(&filename).unwrap();
    let info = info(&root).unwrap();

    println!("Loading {}", info.name);

    let (ctrl, data) = connect(&info);

    loop {
        let tracker_data = data.try_recv();
        if let Ok(TrackerData::Close) = tracker_data {
            println!("Tracker Closed");
            break;
        }
    } 
}
