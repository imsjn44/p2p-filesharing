use hex;
use regex::Regex;
use sha1::{Digest, Sha1};
use std::io::Write;
use std::iter::Iterator;
use std::net::IpAddr;
use std::{fs::File, os::windows::prelude::FileExt};

#[derive(Debug)]
pub struct PiecesFromPeer {
    pub ip_addr: IpAddr,
    pub pieces: Vec<FilePiece>,
    pub no_of_pieces:usize
}
impl PiecesFromPeer {
    pub fn new(ip_addr: IpAddr, pieces: Vec<FilePiece>, no_of_pieces:usize) -> PiecesFromPeer {
        PiecesFromPeer { ip_addr, pieces, no_of_pieces }
    }
}

#[derive(Debug)]
pub struct FileInfo {
    pub file: FileDivider,
    pub peers: Vec<IpAddr>,
}
impl FileInfo {
    pub fn new(file: FileDivider, peers: Vec<IpAddr>) -> FileInfo {
        FileInfo { file, peers }
    }
    pub fn push_peer(&mut self, value: IpAddr) {
        self.peers.push(value);
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct FilePiece {
    pub piece_no: u64,
    pub hash: String,
}
impl FilePiece {
    pub fn parser(pieces: Vec<String>, no_of_pieces:usize) -> Vec<FilePiece> {
        let re = Regex::new(r"piece_no:(\w+)ehash:(\w+)").unwrap();
        let mut pieces_holder: Vec<FilePiece> = Vec::with_capacity(no_of_pieces);
        for piece in pieces {
            if let Some(captures) = re.captures(&piece) {
                let piece_no = captures.get(1).unwrap().as_str().parse::<u64>().unwrap();
                let hash = captures.get(2).unwrap().as_str().to_string();
                pieces_holder.push(FilePiece::new(piece_no, hash));
            }
        }
        pieces_holder
    }

    pub fn new(piece_no: u64, hash: String) -> FilePiece {
        FilePiece { piece_no, hash }
    }
    pub fn convert_to_str(&self) -> String {
        let out = format!("piece_no:{}ehash:{}\n", self.piece_no, self.hash);
        out
    }
}

#[derive(Debug)]
pub struct FileDivider {
    pub file_name: String,
    pub file_hash: String,
    pub file_size: u64,
    pub no_of_pieces: u64,
    pub piece_length: u64,
    pub pieces: Vec<FilePiece>,
}

impl FileDivider {
    pub fn divide_and_hash(file: File, filename: &str, piece_size: u64) -> FileDivider {
        let file_size = file.metadata().expect("No metadata found").len();
        let no_of_pieces = (file_size / piece_size) + 1;
        let mut pieces: Vec<FilePiece> = Vec::new();
        let mut whole_file_hasher = Sha1::new();
        for i in 0..no_of_pieces {
            let mut hasher = Sha1::new();
            let mut buf = vec![0; piece_size as usize];
            let bytes_read = file.seek_read(&mut buf, i * piece_size).unwrap();
    
            hasher.update(&buf[0..bytes_read]);
            whole_file_hasher.update(&buf[0..bytes_read]);
            let result = hasher.finalize();
            let hex_str = hex::encode(result);
            let piece = FilePiece::new(i, hex_str);
            pieces.push(piece);
        }
        let file_hash = hex::encode(whole_file_hasher.finalize());
        FileDivider {
            file_name: filename.to_string(),
            file_hash,
            file_size,
            no_of_pieces,
            piece_length: piece_size,
            pieces,
        }
    }

    pub fn parse(lines: &mut dyn Iterator<Item = String>, no_of_files: usize) -> Vec<FileDivider> {
        let mut files = Vec::new();
        for _ in 0..no_of_files {
            let file_name = lines.next().unwrap()[10..].to_string();
            println!("{}", file_name);
            let file_hash = &lines.next().unwrap()[10..];
            let file_size = &lines.next().unwrap()[10..];
            let file_size = file_size.parse::<u64>().unwrap();
            let no_of_pieces = &lines.next().unwrap()[13..];
            let no_of_pieces = no_of_pieces.parse::<usize>().unwrap();

            let piece_length = &lines.next().unwrap()[13..];
            // let piece_length = piece_length.trim();
            println!("piece length {:?} hello",piece_length);
            let piece_length = piece_length.parse::<u64>().unwrap();
            let pieces = lines.take(no_of_pieces).collect::<Vec<String>>();
            // let pieces = lines.collect::<Vec<String>>();
            let pieces_holder = FilePiece::parser(pieces, no_of_pieces);
            let fd = FileDivider {
                file_name,
                file_hash: String::from(file_hash),
                file_size,
                pieces: pieces_holder,
                no_of_pieces: no_of_pieces as u64,
                piece_length,
            };
            files.push(fd);
        }
        files
    }

    pub fn get_pieces_string(&self) -> String {
        let mut data = String::new();
        for piece in &self.pieces {
            data.push_str(&piece.convert_to_str());
        }
        data
    } 

    pub fn get_string(&self) -> String {
        let mut str_out = String::new();
        str_out.push_str(format!("file_name:{}\n", self.file_name).as_str());
        str_out.push_str(format!("file_hash:{}\n", self.file_hash).as_str());
        str_out.push_str(format!("file_size:{}\n", self.file_size).as_str());
        str_out.push_str(format!("no_of_pieces:{}\n", self.no_of_pieces).as_str());
        str_out.push_str(format!("piece_length:{}\n", self.piece_length).as_str());
        for piece in &self.pieces {
            str_out.push_str(piece.convert_to_str().as_str());
        }
        str_out
    }

    pub fn upload(filename: &str, piece_size: u64) {
        let file = File::open(filename).unwrap();
        let f = FileDivider::divide_and_hash(file, filename, piece_size);
        let binding = f.get_string();
        let data_hash = binding.as_bytes();
        let mut hash_file = File::create(format!("file_hash/{filename}.txt"))
        .expect("failed to hash file");

        let mut pieces_info_file = File::create(format!("piece_info/{}.txt",f.file_hash))
        .expect("failed to create piece_info file");
        let pieces_string = f.get_pieces_string();
        let no_of_pieces = format!("no_of_pieces:{}\n",f.no_of_pieces);
        let data_piece_info = no_of_pieces + &pieces_string ;

        pieces_info_file.write_all(data_piece_info.as_bytes()).expect("failed to write pieces info");
        hash_file.write_all(data_hash).expect("failed to write");
    }
}
