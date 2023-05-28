use hex;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write, BufRead, BufReader};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::os::windows::fs::FileExt;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::utilities::file_handler::FilePiece;
pub fn handle_piece_request(
    reader: &mut BufReader<&mut TcpStream>,
    addrs: IpAddr,
    seeded_files: Arc<Mutex<HashMap<String, String>>>
) {
    let mut lines = reader.lines().map(|l| l.unwrap());
    let file_name = &lines.next().unwrap()[10..];

    let file_hash = &lines.next().unwrap()[10..];
    let file_size = &lines.next().unwrap()[10..];
    let file_size = file_size.parse::<usize>().unwrap();

    let total_pieces = &lines.next().unwrap()[13..];
    let total_pieces = total_pieces.parse::<u64>().unwrap();
    let piece_size = &lines.next().unwrap()[13..];
    let piece_size = piece_size.parse::<u64>().unwrap();
    let no_of_pieces = &lines.next().unwrap()[13..];
    let no_of_pieces = no_of_pieces.parse::<usize>().unwrap();
    let pieces_req = lines.collect::<Vec<_>>();
    let pieces = FilePiece::parser(pieces_req, no_of_pieces);

    let socket = SocketAddr::new(addrs, 7878);
    let stream = TcpStream::connect(socket).unwrap();
    
    //transferring from the original file
    let seeded_files = seeded_files.lock().unwrap();
    let file_path = seeded_files.get(file_hash).unwrap().clone();
    println!("file p {}", file_path);
    drop(seeded_files);
    let file_path = Path::new(&file_path);
    if let Ok(exists) = file_path.try_exists(){
        if file_path.is_file(){
            let file_path = file_path.to_str().unwrap();
            from_file(file_name, file_path , file_hash, file_size, total_pieces, no_of_pieces, piece_size, stream, pieces);
            println!("completed");
            return ();
        }
    }


    // if file_path.try_exists().unwrap() && file_path.is_file() {
    //     from_file(file_name, file_hash, file_size, total_pieces, no_of_pieces, piece_size, stream, pieces);
    //     println!("completed");
    //     return ();
    // }
    //transferring fromt the pieces of a file
    let pieces_path = format!("pieces/{}", file_hash);
    let path = Path::new(&pieces_path);
    if path.exists() && path.is_dir() {
        from_pieces_folder(file_name, file_hash, file_size, total_pieces, no_of_pieces, piece_size, stream, pieces);
    }      
    println!("completed");
}

fn from_file(file_name:&str, file_path:&str, file_hash:&str, file_size:usize, total_pieces:u64, no_of_pieces:usize, piece_size:u64, mut stream: TcpStream, pieces:Vec<FilePiece>){
    let file = File::open(file_path).expect("failed to read file");
    let init_msg = format!("type:pieces\nfile_name:{file_name}\nfile_hash:{file_hash}\nfile_size:{file_size}\ntotal_pieces:{total_pieces}\npiece_length:{piece_size}\nno_of_pieces:{no_of_pieces}\n");
    println!("{}", init_msg);
    stream
        .write(init_msg.as_bytes())
        .expect("failed to send initial msg before sending pieces");

    // let mut tfile = File::create("test.bin").unwrap();
    // tfile.write(init_msg.as_bytes()).unwrap();
    for piece in pieces {
        let mut buf = vec![0; piece_size as usize];
        let mut hasher = Sha1::new();

        let offset = piece_size * piece.piece_no;
        let bytes_read = file.seek_read(&mut buf, offset as u64).unwrap();
        println!("bytes read {}\n", bytes_read);
        hasher.update(&buf[0..bytes_read]);
        let result = hasher.finalize();
        let hex_str = hex::encode(result);

        if hex_str == piece.hash {
            let piece_info = format!("piece_no:{}epiece_hash:{}\n", piece.piece_no, piece.hash);
            stream.write(piece_info.as_bytes()).unwrap();
            // tfile.write(piece_info.as_bytes()).unwrap();
            stream.write(&buf).expect("failed to send piece");
            // tfile.write(&buf).expect("error in file");
        } else {
            println!("file hash didn't match so not sending the piece");
        }
    }
}

fn from_pieces_folder(file_name:&str, file_hash:&str, file_size:usize, total_pieces:u64, no_of_pieces:usize, piece_size:u64, mut stream: TcpStream, pieces:Vec<FilePiece>) {
    let init_msg = format!("type:pieces\nfile_name:{file_name}\nfile_hash:{file_hash}\nfile_size:{file_size}\ntotal_pieces:{total_pieces}\npiece_length:{piece_size}\nno_of_pieces:{no_of_pieces}\n");
    stream
        .write(init_msg.as_bytes())
        .expect("failed to send initial msg before sending pieces");
    println!("sending from pieces");
    for piece in pieces {
        let piece_path = format!("pieces/{}/{}.txt", file_hash, piece.piece_no);
        let mut file = File::open(piece_path).expect("failed to read file");
        let mut buf = vec![0; piece_size as usize];
        let mut hasher = Sha1::new();


        let bytes_read = file.read(&mut buf).unwrap();
        println!("piece  {}\n", piece.piece_no);
        hasher.update(&buf[0..bytes_read]);
        let result = hasher.finalize();
        let hex_str = hex::encode(result);

        if hex_str == piece.hash {
            let piece_info = format!("piece_no:{}epiece_hash:{}\n", piece.piece_no, piece.hash);
            stream.write(piece_info.as_bytes()).unwrap();
            // tfile.write(piece_info.as_bytes()).unwrap();
            stream.write(&buf).expect("failed to send piece");
            // tfile.write(&buf).expect("error in file");
        } else {
            println!("file hash didn't match so not sending the piece");
        }
    }
}