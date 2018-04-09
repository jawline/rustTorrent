use torrent::{from_file, prepare};
use tracker::{TrackerState, PeerAddress, connect};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::thread;
use std::net::TcpListener;
use peer_client::{peer_client, ClientState};
use torrent_data::TorrentData;

pub enum DownloadState {
    Close
}

struct Peer {
    id: PeerAddress,
    channel: (Sender<ClientState>, Receiver<ClientState>)
}

const MAX_PEERS: usize = 20;

pub fn download(filename: &str) -> (Sender<DownloadState>, Receiver<DownloadState>) {
    
    let filename = filename.to_string();

    let (thread_send, main_recv): (Sender<DownloadState>, Receiver<DownloadState>) = mpsc::channel();
    let (main_send, thread_recv): (Sender<DownloadState>, Receiver<DownloadState>) = mpsc::channel();

    thread::spawn(move || {

        let peer_port = 6898;
        let tracker_port = 11993;

        let root = from_file(&filename).unwrap();
        let info = prepare(&root).unwrap();
        
        println!("Loading {}", info.name);

        let torrent_data = TorrentData::allocate(&("/home/blake/".to_string() + &info.name), info.pieces.clone(), info.piece_length); 

        if let Err(v) = torrent_data { 
            println!("Bad Allocate {}", v);
            thread_send.send(DownloadState::Close);
            return;
        }

        let torrent_data = torrent_data.unwrap();

        let (tracker_send, tracker_recv) = connect(&info, peer_port, tracker_port);

        let mut active_peers: Vec<Peer> = Vec::new();

        loop {

            //Check if a control signal has been sent
            let ctrl_data = thread_recv.try_recv();
            
            if let Ok(DownloadState::Close) = ctrl_data {
                tracker_send.send(TrackerState::Close("Requested".to_string()));
            }

            //Update peer-wire client info
            let mut closed = Vec::new();

            //Read all signals from clients & process
            for client_num in 0..active_peers.len() {
                let (send, recv) = &active_peers[client_num].channel;

                loop {
                    let client_data = recv.try_recv();
                    if let Ok(signal) = client_data {
                        match signal {
                            ClientState::Close(reason) => {
                                println!("Flagging {:?} for close due to {}", active_peers[client_num].id, reason);
                                closed.push(client_num);
                            }
                        }
                    } else { //No More Incoming Data
                        break;
                    } 
                }           
            }

            //Back-to-front remove each index in vector (Preserves removal-index)
            closed.iter().rev().for_each(|&i| {
                println!("Remove {:?}", active_peers[i].id);
                active_peers.remove(i);
            });

            //Update tracker information
            let tracker_data = tracker_recv.try_recv();

            match tracker_data {
                Ok(TrackerState::Close(v)) => {
                    println!("Closed because {}", v);
                    thread_send.send(DownloadState::Close);
                    break; 
                },
                Ok(TrackerState::Connected(cid)) => {
                    println!("Connected to the tracker with connection id {}", cid);
                },
                Ok(TrackerState::Announced(peers)) => {
                    //println!("Acquired peers {:?}", peers);
                    for peer in &peers {
                        if active_peers.len() < MAX_PEERS {
                            active_peers.push(Peer {
                                id: peer.clone(),
                                channel: peer_client(&info, peer)
                            });
                        }
                    }
                },
                Err(_) => {}
            }

        } 
    });

    (main_send, main_recv)
}
