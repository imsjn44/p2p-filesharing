use std::fs;
use std::sync::MutexGuard;
use std::io::{Write, BufReader, BufRead};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;
use crate::utilities::file_handler::{PiecesFromPeer, FilePiece};
pub fn piece_availability(reader: &mut BufReader<&mut TcpStream>, addrs: IpAddr) {
    let mut lines = reader.lines().map(|l| l.unwrap());

    let file_hash = &lines.next().unwrap()[10..];
    let dir_path = "./piece_info";
    let files = fs::read_dir(dir_path).unwrap();

    for path in files {
        let entry = path.unwrap();
        let path = entry.path();
        let file_name = entry.file_name().into_string().unwrap();
        if file_name == file_hash.to_string() + ".txt" {
            let contents = fs::read_to_string(path).expect("failed to read file");
            let addrs = SocketAddr::new(addrs, 7878);
            let timeout = Duration::from_secs(5);
            let mut stream = TcpStream::connect_timeout(&addrs, timeout).unwrap();
            stream.write(b"type:available_pieces\n").unwrap();
            stream.write(contents.as_bytes()).unwrap();
        }
    }
}

pub fn piece_response_handler(
    reader: &mut BufReader<&mut TcpStream>,
    addrs: IpAddr,
    mut peer_and_pieces: MutexGuard<Vec<PiecesFromPeer>>
) {
    println!("inside piece response handler");
    let mut lines = reader.lines().map(|l| l.unwrap());
    // let _file_hash = &lines.next().unwrap()[10..];
    let no_of_pieces = &lines.next().unwrap()[13..];
    let no_of_pieces = String::from(no_of_pieces).parse::<usize>().unwrap();
    let pieces = lines.take(no_of_pieces).collect::<Vec<String>>();
    let pieces_holder = FilePiece::parser(pieces, no_of_pieces);
    
    let piece_from_peer = PiecesFromPeer::new(addrs,pieces_holder, no_of_pieces);
    peer_and_pieces.push(piece_from_peer);
    println!("completed processing the received peer pieces info\n");

}
