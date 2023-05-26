use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::{fs, str};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::{IpAddr, TcpStream},
};

use crate::swarmnet::PieceFromIpAndPiece;
use crate::utilities::file_handler::FilePiece;

pub fn handle_pieces(
    reader: &mut BufReader<&mut TcpStream>,
    addrs: IpAddr,
    piece_and_peers: &mut Arc<Mutex<HashMap<String, HashMap<FilePiece, Vec<IpAddr>>>>>,
    pieces_got: Arc<Mutex<HashMap<String, Vec<PieceFromIpAndPiece>>>>,
)
-> String
 {
    // let mut file_type_buf: Vec<u8> = Vec::new();
    let mut file_name_buf: Vec<u8> = Vec::new();
    let mut file_hash_buf: Vec<u8> = Vec::new();
    let mut file_size_buf: Vec<u8> = Vec::new();
    let mut total_pieces_buf: Vec<u8> = Vec::new();
    let mut piece_length_buf: Vec<u8> = Vec::new();
    let mut no_of_pieces_buf: Vec<u8> = Vec::new();

    // reader.read_until(b'\n', &mut file_type_buf).unwrap();
    reader.read_until(b'\n', &mut file_name_buf).unwrap();
    reader.read_until(b'\n', &mut file_hash_buf).unwrap();
    reader.read_until(b'\n', &mut file_size_buf).unwrap();
    reader.read_until(b'\n', &mut total_pieces_buf).unwrap();
    reader.read_until(b'\n', &mut piece_length_buf).unwrap();
    reader.read_until(b'\n', &mut no_of_pieces_buf).unwrap();

    // file_type_buf.pop();
    file_name_buf.pop();
    file_hash_buf.pop();
    file_size_buf.pop();
    total_pieces_buf.pop();
    piece_length_buf.pop();
    no_of_pieces_buf.pop();

    let file_hash = str::from_utf8(&file_hash_buf).unwrap();
    let file_hash = &file_hash[10..];
    println!("file_hash {}", file_hash);
    let file_size = str::from_utf8(&file_size_buf).unwrap();
    let file_size = file_size[10..].parse::<usize>().unwrap();
    println!("file_size: {:?}", file_size);
    let no_of_pieces = str::from_utf8(&no_of_pieces_buf).unwrap();
    let no_of_pieces = no_of_pieces[13..].parse::<usize>().unwrap();
    let piece_length = str::from_utf8(&piece_length_buf).unwrap();
    let piece_length = piece_length[13..].parse::<usize>().unwrap();
    let total_pieces = str::from_utf8(&total_pieces_buf).unwrap();
    let total_pieces = total_pieces[13..].parse::<usize>().unwrap();

    let mut piece_info_buf: Vec<u8> = Vec::new();
    let mut data_buf = vec![0; piece_length];

    //keep track of the received pieces
    let mut pieces_got_arr_temp: Vec<PieceFromIpAndPiece> = Vec::new();
    let mut file_piece_arr: Vec<FilePiece> = Vec::new();

    //to update the piece of the file that the peer has
    let piece_info_path = format!("piece_info/{file_hash}.txt");
    let mut piece_info_file =
        if let Ok(file) = fs::OpenOptions::new().append(true).open(&piece_info_path) {
            println!("file already exits");
            file
        } else {
            println!("creating new file");
            File::create(&piece_info_path).unwrap()
        };
    
    //check if piece_info is already full
    let f = File::open(&piece_info_path).unwrap();
    let r = BufReader::new(f);
    if(r.lines().count() == total_pieces){
        println!("file pieces are already fulfilled");
        return file_hash.to_string();
    }
    //create folder inside pieces to store the pieces
    let path = String::from("pieces/") + file_hash;
    let folder_path = Path::new(&path);
    if !(folder_path.exists() && folder_path.is_dir()) {
        fs::create_dir(path).unwrap()
    }
    for _ in 0..no_of_pieces {
        reader.read_until(b'\n', &mut piece_info_buf).unwrap();
        if piece_info_buf.is_empty() {
            continue;
        }

        piece_info_buf.pop();

        let (piece_no, hash) = parse_piece_no(str::from_utf8(&piece_info_buf).unwrap()).unwrap();
        file_piece_arr.push(FilePiece::new(piece_no as u64, hash.clone()));

        let path = format!("pieces/{file_hash}/{piece_no}.txt");
        let mut file = File::create(path).unwrap();
        reader
            .read_exact(&mut data_buf)
            .expect("failed to read data bytes");

        //to remove possible null characters from last peice
        if piece_no == total_pieces - 1 {
            println!("found the last piece");
            let assumed_size = total_pieces * piece_length;
            let error = assumed_size - file_size;
            let piece_size = piece_length - error;
            file.write_all(&data_buf[..piece_size])
                .expect("failed to write file");
        } else {
            file.write_all(&data_buf).expect("failed to write file");
        }

        // println!("received piece:{}", piece_no);
        let ip_and_piece = PieceFromIpAndPiece::new(addrs, piece_no);
        pieces_got_arr_temp.push(ip_and_piece);
        let msg = format!("piece_no:{piece_no}epiece_hash:{hash}\n");
        piece_info_file
            .write_all(msg.as_bytes())
            .expect("failed to write piece_info");

        piece_info_buf.clear();
    }

    let mut pieces_got = pieces_got.lock().unwrap();
    let pieces_got_arr = pieces_got.get_mut(file_hash).expect("pieces_got is empty");
    //if remove got piece from piece and peers
    let mut piece_and_peers = piece_and_peers.lock().unwrap();
    let piece_and_peers_inner = piece_and_peers.get_mut(file_hash).unwrap();
    for piece in file_piece_arr {
        piece_and_peers_inner.remove(&piece);
    }
    drop(piece_and_peers);
    
    pieces_got_arr.append(&mut pieces_got_arr_temp);

    file_hash.to_string()


}

fn parse_piece_no(text: &str) -> Option<(usize, String)> {
    let re = Regex::new(r"piece_no:(\d+)epiece_hash:(\w+)").unwrap();

    if let Some(captures) = re.captures(text) {
        let number_str = captures.get(1).unwrap().as_str();
        let number = number_str.parse::<usize>().unwrap();
        let hash = captures.get(2).unwrap().as_str();
        Some((number, String::from(hash)))
    } else {
        None
    }
}
