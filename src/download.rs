use torrent::{from_file, prepare};
use tracker::{TrackerState, PeerAddress, connect};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::time::Duration;
use std::{io, thread};
use std::net::TcpListener;
use peer_client::{peer_client, ClientState};
use torrent_data::TorrentData;

pub enum DownloadState {
    Close
}

struct Peer {
    id: PeerAddress,
    locked: Option<usize>,
    channel: (Sender<ClientState>, Receiver<ClientState>)
}

fn s_peer(peer_list: &mut Vec<Peer>, el: usize, elem: ClientState) {
    let (ref mut send, _) = peer_list[el].channel;
    send.send(elem).unwrap();
}

fn r_peer(peer_list: &mut Vec<Peer>, el: usize) -> Result<ClientState, mpsc::TryRecvError> {
    let (_, ref mut recv) = peer_list[el].channel;
    recv.try_recv()
}

fn remaining(torrent_data: &TorrentData) -> usize {
    (0..torrent_data.pieces.len()).filter(|&x| !torrent_data.have.get(x)).count()
}

const MAX_PEERS: usize = 50;

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

        let mut torrent_data = torrent_data.unwrap();

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

                loop {
                    let client_data = r_peer(&mut active_peers, client_num);
                    if let Ok(signal) = client_data {
                        match signal {
                            ClientState::Close(reason) => {
                                println!("Flagging {:?} for close due to {}", active_peers[client_num].id, reason);
                                closed.push(client_num);
                            },
                            ClientState::Need(field) => { 
                                let target = (0..torrent_data.pieces.len())
                                    .find(|&x| {
                                        //println!("Find {}", x);
                                        let i_have = torrent_data.have.get(x);
                                        let they_have = field.get(x);
                                        let is_unlocked = remaining(&torrent_data) < MAX_PEERS || !active_peers.iter().any(|cl| cl.locked == Some(x));
                                        !i_have && they_have && is_unlocked
                                    });

                                if let Some(i) = target {
                                    active_peers[client_num].locked = Some(i);
                                    s_peer(&mut active_peers, client_num, ClientState::Want(i));
                                } else {
                                    s_peer(&mut active_peers, client_num, ClientState::Close("Nothing of interest".to_string()));
                                }
                            },
                            ClientState::Commit(piece, data) => {
                                println!("{} / {} / {}", piece, torrent_data.pieces.len(), remaining(&torrent_data));
                                torrent_data.write(piece, &data).unwrap();
                            },
                            _ => { println!("TODO: Error handler"); }
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
                    println!("Design a way to trigger full shut down");
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
                        let can_add = active_peers.len() < MAX_PEERS;
                        let already_have = active_peers.iter().any(|x| peer.ip == x.id.ip);
                        if can_add && !already_have {
                            active_peers.push(Peer {
                                id: peer.clone(),
                                locked: None,
                                channel: peer_client(&info, peer)
                            });
                        }
                    }
                },
                Err(_) => {}
            }
            thread::sleep(Duration::from_millis(150));
        } 
    });

    (main_send, main_recv)
}
