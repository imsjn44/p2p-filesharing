use crate::utilities::file_handler::{FilePiece, PiecesFromPeer};
use std::collections::HashMap;
use std::fmt::format;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Lines, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use crate::swarmnet::TCP_RECEIVER_PORT;
pub fn piece_availability(reader: &mut BufReader<&mut TcpStream>, addrs: IpAddr) {
    println!("im inside piece request handler");
    let mut lines = reader.lines().map(|l| l.unwrap());

    let file_hash = &lines.next().unwrap()[10..];
    // let dir_path = "./piece_info";
    // let files = fs::read_dir(dir_path).unwrap();
    //
    let path = format!("piece_info/{file_hash}.txt");
    let f = File::open(&path).unwrap();
    let reader = BufReader::new(f);
    let no_of_pieces = reader.lines().count();
    //
    let contents = fs::read_to_string(path).expect("failed to read file");
    let addrs = SocketAddr::new(addrs, TCP_RECEIVER_PORT);
    let timeout = Duration::from_secs(5);
    let mut stream = TcpStream::connect_timeout(&addrs, timeout).unwrap();
    let init_msg =
        format!("type:available_pieces\nfile_hash:{file_hash}\nno_of_pieces:{no_of_pieces}\n");
    stream.write(init_msg.as_bytes()).unwrap();
    stream.write(contents.as_bytes()).unwrap();

    ///
    // for path in files {
    //     let entry = path.unwrap();
    //     let path = entry.path();
    //     let file_name = entry.file_name().into_string().unwrap();
    //     if file_name == file_hash.to_string() + ".txt" {
    //         //
    //         let f = File::open(&path).unwrap();
    //         let reader = BufReader::new(f);
    //         let no_of_pieces = reader.lines().count();
    //         //
    //         let contents = fs::read_to_string(path).expect("failed to read file");
    //         let addrs = SocketAddr::new(addrs, 9898);
    //         let timeout = Duration::from_secs(5);
    //         let mut stream = TcpStream::connect_timeout(&addrs, timeout).unwrap();
    //         let init_msg = format!(
    //             "type:available_pieces\nfile_hash:{file_hash}\nno_of_pieces:{no_of_pieces}\n"
    //         );
    //         stream.write(init_msg.as_bytes()).unwrap();
    //         stream.write(contents.as_bytes()).unwrap();
    //     }
    // }
    println!("exiting from piece req handler");
}

pub fn piece_response_handler(
    reader: &mut BufReader<&mut TcpStream>,
    addrs: IpAddr,
    peer_and_pieces: Arc<Mutex<HashMap<String, Vec<PiecesFromPeer>>>>,
) {
    println!("im inside piece response handler");
    let mut lines = reader.lines().map(|l| l.unwrap());
    let file_hash = &lines.next().unwrap()[10..];
    let no_of_pieces = &lines.next().unwrap()[13..];
    let no_of_pieces = String::from(no_of_pieces).parse::<usize>().unwrap();
    let pieces = lines.take(no_of_pieces).collect::<Vec<String>>();
    let pieces_holder = FilePiece::parser(pieces, no_of_pieces);

    let piece_from_peer = PiecesFromPeer::new(addrs, pieces_holder, no_of_pieces);

    let mut peer_and_pieces = peer_and_pieces.lock().unwrap();
    let arr = peer_and_pieces.get_mut(file_hash).unwrap();
    arr.push(piece_from_peer);
    println!("completed processing the received peer pieces info\n");
}
