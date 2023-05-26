use std::{net::{IpAddr, SocketAddr, TcpStream}, fs, io::BufReader};
use std::io::{Read, Write};

pub fn handle_peers_details_retransmission(file_hash:&str, addrs: IpAddr){
    let file_path = format!("piece_info/{file_hash}");
    if let Ok(mut file) = fs::OpenOptions::new().append(true).open(&file_path){
        if let Ok(mut stream) = TcpStream::connect(SocketAddr::new(addrs,7878)){
            let mut msg = String::new();
            file.read_to_string(&mut msg).unwrap();
            let buffer = format!("type:re_peer_info\nfile_hash:{file_hash}\n{msg}\n");
            stream.write(buffer.as_bytes()).expect("failed retransmission");
        }

    }

}

pub fn retransmisson_response_handler(
    reader: &mut BufReader<&mut TcpStream>,
    addrs: IpAddr,
){
    
}
