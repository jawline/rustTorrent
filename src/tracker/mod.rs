use torrent::Info;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::thread;

mod state;
mod http;
mod udp;

pub use tracker::state::{TrackerState, PeerAddress};

pub fn tracker_thread(info: &Info, peer_port: u16, tracker_port: u16, send: Sender<TrackerState>, recv: Receiver<TrackerState>) {
    if info.announce.starts_with("udp://") {
        udp::udp_tracker(info, peer_port, tracker_port, send, recv); 
    } else if info.announce.starts_with("http://") || info.announce.starts_with("https://") {
        http::http_tracker(info, peer_port, tracker_port, send, recv);
    } else {
        send.send(
            TrackerState::Close("Unknown tracker protocol".to_string())
        );
    }
}

pub fn connect(info: &Info, peer_port: u16, tracker_port: u16) -> (Sender<TrackerState>, Receiver<TrackerState>) {
     let info = info.clone();

    let (thread_send, main_recv): (Sender<TrackerState>, Receiver<TrackerState>) = mpsc::channel();
    let (main_send, thread_recv): (Sender<TrackerState>, Receiver<TrackerState>) = mpsc::channel();

    thread::spawn(move || {
        tracker_thread(&info, peer_port, tracker_port, thread_send, thread_recv);
    });
   
    (main_send, main_recv)
}
