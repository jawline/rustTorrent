use std::net::TcpListener;

pub fn peer_server(port: u16) -> Result<(), String> {
   let incoming_server = TcpListener::bind(("0.0.0.0", port));

    if let Err(v) = incoming_server {
        return Err(v.to_string());
    }

    let incoming_server = incoming_server.unwrap();

    println!("TODO: Peer server threading");
    Ok(())
}
