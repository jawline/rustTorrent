use rand::{thread_rng, Rng};

pub fn gen_peer_id() -> Vec<u8> {
    let mut peer_string: [u8; 20] = [0u8; 20];
    peer_string[0] = 'r' as u8;
    peer_string[1] = 'T' as u8;
    thread_rng().fill_bytes(&mut peer_string[2..20]);
    peer_string.to_vec() 
}
