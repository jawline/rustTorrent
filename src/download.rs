use torrent::{Info, from_file, prepare};
use tracker::{TrackerState, PeerAddress, connect};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::time::Duration;
use std::thread;
use peer_client::{peer_client, ClientState};
use torrent_data::TorrentData;
use bitfield::Bitfield;

pub enum DownloadState {
    Close
}

struct Peer {
    id: PeerAddress,
    locked: Option<usize>,
    channel: (Sender<ClientState>, Receiver<ClientState>)
}

fn remaining(torrent_data: &TorrentData) -> usize {
    (0..torrent_data.pieces.len()).filter(|&x| !torrent_data.have.get(x)).count()
}

const MAX_PEERS: usize = 50;

struct Download {

    send: Sender<DownloadState>,
    recv: Receiver<DownloadState>,

    tracker: (Sender<TrackerState>, Receiver<TrackerState>),

    info: Info,
    data: TorrentData,

    active_clients: Vec<Peer>
}

impl Download {
    
    pub fn shutdown(&mut self, reason: &str) {
        println!("TODO: Shut Down because {}", reason);
    }

    pub fn sync_ctrl(&mut self) {
        //Check if a control signal has been sent
        let ctrl_data = self.recv.try_recv();
            
        if let Ok(DownloadState::Close) = ctrl_data {
            self.shutdown("Requested");
        }
    }

    pub fn sync_tracker(&mut self) {

        //Update tracker information
        let tracker_data = self.tracker.1.try_recv();

        match tracker_data {
            Ok(TrackerState::Close(v)) => {
                self.shutdown(&format!("Tracker Closed because {}", v));
            },
            Ok(TrackerState::Connected(cid)) => {
                println!("Connected to the tracker with connection id {}", cid);
            },
            Ok(TrackerState::Announced(peers)) => {
                //println!("Acquired peers {:?}", peers);
                for peer in &peers {
                    let mut active_peers = &mut self.active_clients;
                    let can_add = active_peers.len() < MAX_PEERS;
                    let already_have = active_peers.iter().any(|x| peer.ip == x.id.ip);
                    if can_add && !already_have {
                        active_peers.push(Peer {
                            id: peer.clone(),
                            locked: None,
                            channel: peer_client(&self.info, peer)
                        });
                    }
                }
            },
            Err(_) => {}
        }
    }

    fn flag_remove(id: usize, to_remove: &mut Vec<usize>) {
        if !to_remove.iter().any(|&x| x == id) {
            to_remove.push(id);
        }
    }

    fn s_client(&mut self, id: usize, msg: ClientState, to_remove: &mut Vec<usize>) {
        if self.active_clients[id].channel.0.send(msg).is_err() {
            Download::flag_remove(id, to_remove);
        }
    }

    fn r_client(&mut self, id: usize) -> Result<ClientState, mpsc::TryRecvError> {
        self.active_clients[id].channel.1.try_recv()
    }

    /** Find a piece we don't have and they do have **/
    fn find_needed_piece(&self, field: &Bitfield) -> Option<usize> {
        (0..self.data.pieces.len())
        .find(|&x| {
            //println!("Find {}", x);
            let i_have = self.data.have.get(x);
            let they_have = field.get(x);
            let is_unlocked = remaining(&self.data) < MAX_PEERS || !self.active_clients.iter().any(|cl| cl.locked == Some(x));
            !i_have && they_have && is_unlocked
        })
    }

    fn process_client_msg(&mut self, id: usize, msg: ClientState, to_remove: &mut Vec<usize>) {
        match msg {
            ClientState::Close(reason) => {
                println!("Flagging {:?} for close due to {}", self.active_clients[id].id, reason);
                Download::flag_remove(id, to_remove);
            },
            ClientState::Need(field) => { 
                let target = self.find_needed_piece(&field); 

                if let Some(i) = target {
                    self.active_clients[id].locked = Some(i);
                    self.s_client(id, ClientState::Want(i), to_remove);
                } else {
                    self.s_client(id, ClientState::Close("Nothing of interest".to_string()), to_remove);
                }
            },
            ClientState::Commit(piece, data) => {
                self.data.write(piece, &data).unwrap();
                self.update_data_state();
            },
            _ => {
                println!("Unexpected message from {:?}", self.active_clients[id].id);
                self.s_client(id, ClientState::Close("Bad message".to_string()), to_remove);
            }
        } 
    }

    pub fn sync_clients(&mut self) {
        //Update peer-wire client info
        let mut closed = Vec::new();

        //Read all signals from clients & process
        for client_num in 0..self.active_clients.len() {
            while let Ok(signal) = self.r_client(client_num) {
                self.process_client_msg(client_num, signal, &mut closed);
            }
        }

        //Back-to-front remove each index in vector (Preserves removal-index)
        closed.iter().rev().for_each(|&i| {
            self.active_clients.remove(i);
        });
    }

    fn update_data_state(&mut self) {
        let total_pieces = self.data.pieces.len();
        let remaining_pieces = remaining(&self.data);
        let piece_length = self.data.piece_size;

        println!("{}MB / {}MB", ((total_pieces - remaining_pieces) * piece_length) / 1024 / 1024, (total_pieces * piece_length) / 1024 / 1024);
    }
}

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
        let mut tracker = connect(&info, peer_port, tracker_port);

        let mut state = Download {
            send: thread_send, 
            recv: thread_recv,
            tracker: tracker,
            data: torrent_data,
            info: info,
            active_clients: Vec::new()
        };

        loop {

            state.sync_ctrl();
            state.sync_tracker();
            state.sync_clients();

            thread::sleep(Duration::from_millis(150));
        } 
    });

    (main_send, main_recv)
}
