use torrent::{from_file, prepare};
use tracker::{TrackerData, connect};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::thread;

pub enum DownloadState {
    Close
}

pub fn download(filename: &str) -> (Sender<DownloadState>, Receiver<DownloadState>) {
    
    let filename = filename.to_string();

    let (thread_send, main_recv): (Sender<DownloadState>, Receiver<DownloadState>) = mpsc::channel();
    let (main_send, thread_recv): (Sender<DownloadState>, Receiver<DownloadState>) = mpsc::channel();


    thread::spawn(move || {
        let root = from_file(&filename).unwrap();
        let info = prepare(&root).unwrap();

        println!("Loading {}", info.name);

        let (tracker_send, tracker_recv) = connect(&info);

        loop {

            let ctrl_data = thread_recv.try_recv();
            
            if let Ok(DownloadState::Close) = ctrl_data {
                tracker_send.send(TrackerData::Close);
            }

            let tracker_data = tracker_recv.try_recv();

            if let Ok(TrackerData::Close) = tracker_data {
                println!("Tracker Closed");
                thread_send.send(DownloadState::Close);
                break;
            }

        } 
    });

    (main_send, main_recv)
}
