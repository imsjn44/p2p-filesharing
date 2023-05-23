use crate::hash_response;
use crate::piece_request;
use crate::piece_response;
use crate::search;
use crate::utilities::availability;
use crate::utilities::file_handler::PiecesFromPeer;
use crate::utilities::file_handler::{FileInfo, FilePiece};
use custom_thread::ThreadPool;
use std::clone::Clone;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::net::UdpSocket;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{
    collections::HashMap,
    net::TcpListener,
    sync::{Arc, Mutex, MutexGuard},
    thread,
};
//for randomly assigning piece to an IP_Addr
use rand::seq::SliceRandom;
use rand::thread_rng;

//remember you have multple tcp udp request for third copy
pub const IP_ADDRS: &str = "127.0.0.1";
const TCP_LISTENER_PORT: u16 = 7878;
const UDP_LISTENER_PORT: u16 = 8787;

// const TCP_RECEIVER_PORT: u16 = 7878;
// const UDP_RECEIVER_PORT: u16 = 8787;
const TCP_RECEIVER_PORT: u16 = 9898;
const UDP_RECEIVER_PORT: u16 = 8989;

#[derive(Clone, PartialEq)]
pub enum Status {
    Searching,
    PieceHandling,
    Idle,
}
#[derive(Debug)]
pub struct PieceFromIpAndPiece {
    pub ip: IpAddr,
    pub piece_no: usize,
}

impl PieceFromIpAndPiece {
    pub fn new(ip: IpAddr, piece_no: usize) -> PieceFromIpAndPiece {
        PieceFromIpAndPiece { ip, piece_no }
    }
}
pub struct SwarmNet {
    pub udp_thread: Option<JoinHandle<()>>,
    pub tcp_thread: Option<JoinHandle<()>>,
    //filehash, peers
    pub files_map: Arc<Mutex<HashMap<String, FileInfo>>>,
    //file_hash, PiecesFromPeer:ipAddr, Vec<FilePiece>
    pub peer_and_pieces: Arc<Mutex<Vec<PiecesFromPeer>>>,
    //piece hash and peers ;only for a single file
    pub pieces: Option<HashMap<FilePiece, Vec<IpAddr>>>,
    pub status: Arc<Mutex<Status>>,
    //file hash and collection of acquired piece numbers and ip
    pub pieces_got: Arc<Mutex<HashMap<String, Vec<PieceFromIpAndPiece>>>>,
}

impl SwarmNet {
    pub fn start() -> SwarmNet {
        let files_map: Arc<Mutex<HashMap<String, FileInfo>>> = Arc::new(Mutex::new(HashMap::new()));
        let peer_and_pieces: Arc<Mutex<Vec<PiecesFromPeer>>> = Arc::new(Mutex::new(Vec::new()));
        let pieces_got: Arc<Mutex<HashMap<String, Vec<PieceFromIpAndPiece>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        SwarmNet {
            files_map,
            peer_and_pieces,
            pieces_got,
            udp_thread: None,
            tcp_thread: None,
            pieces: None,
            status: Arc::new(Mutex::new(Status::Idle)),
        }
    }

    pub fn start_udp_thread_for_search(&mut self) {
        let udp_thread = thread::spawn(|| {
            let socket = UdpSocket::bind(format!("{IP_ADDRS}:{UDP_LISTENER_PORT}"))
                .expect("couldn't bind to that address");
            let udp_threadpool = ThreadPool::new(8);
            println!("Listening for UDP requests");

            loop {
                let mut buf = [0; 1024];
                let (_, socket_addrs) = socket.recv_from(&mut buf).unwrap();
                udp_threadpool.execute(move || {
                    let ip_addr = socket_addrs.ip();
                    let datagram = std::str::from_utf8(&buf)
                        .expect("failed to parse datagram from udp socket");
                    search::process_search(datagram, ip_addr);
                });
            }
        });
        self.udp_thread = Some(udp_thread);
    }

    pub fn start_tcp_thread(&mut self) {
        let files_map = self.files_map.clone();
        let peer_and_pieces = self.peer_and_pieces.clone();
        let status = self.status.clone();
        let pieces_got = self.pieces_got.clone();

        let tcp_thread = thread::spawn(move || {
            let listener = TcpListener::bind(format!("{IP_ADDRS}:{TCP_LISTENER_PORT}")).unwrap();
            let requests_thread_pool = ThreadPool::new(15);
            println!("Listening for TCP requests");

            for stream in listener.incoming() {
                println!("receiving data");
                let files_map = files_map.clone();
                let peer_and_pieces = peer_and_pieces.clone();
                let status = status.clone();
                let stream = stream.unwrap();
                let pieces_got = pieces_got.clone();

                requests_thread_pool.execute(move || {
                    let files_map = files_map.lock().unwrap();
                    let peer_and_pieces = peer_and_pieces.lock().unwrap();
                    let status = status.lock().unwrap();
                    let pieces_got = pieces_got.lock().unwrap();

                    Self::handle_request(stream, files_map, peer_and_pieces, status, pieces_got);
                });
                println!("PROCESSED A STREAM");
            }
        });
        self.tcp_thread = Some(tcp_thread);
    }

    pub fn handle_request(
        mut stream: TcpStream,
        mut files_map: MutexGuard<HashMap<String, FileInfo>>,
        peer_and_pieces: MutexGuard<Vec<PiecesFromPeer>>,
        status: MutexGuard<Status>,
        pieces_got: MutexGuard<HashMap<String, Vec<PieceFromIpAndPiece>>>,
    ) {
        let addrs = stream.peer_addr().unwrap().ip();

        let mut file_type_buf: Vec<u8> = Vec::new();
        let mut reader = BufReader::new(&mut stream);
        reader.read_until(b'\n', &mut file_type_buf).unwrap();
        file_type_buf.pop();
        let req_type = std::str::from_utf8(&file_type_buf).unwrap();
        let req_type = &req_type[5..];

        println!("req type:{} from {}", req_type, addrs.to_string());
        match req_type {
            "hash" => {
                if *status == Status::Searching {
                    hash_response::handle_hash(&mut reader, &mut files_map, addrs);
                }
            }

            "asking_available_pieces" => {
                availability::piece_availability(&mut reader, addrs);
            }
            //response to ask_available_pieces
            "available_pieces" => {
                availability::piece_response_handler(&mut reader, addrs, peer_and_pieces);
            }
            //handle the actual piece request
            "asking_pieces" => {
                piece_request::handle_piece_request(&mut reader, addrs);
            }

            //actual pieces accept handler
            "pieces" => {
                piece_response::handle_pieces(&mut reader, addrs, pieces_got);
            }
            _ => (),
        }
    }

    pub fn request_info_about_file_pieces(&mut self, file_hash: &str) {
        self.set_piece_handling();

        let files_map = self.files_map.lock().unwrap();
        //initialize pieces got
        let mut pieces_got = self.pieces_got.lock().unwrap();
        let piece_nums: Vec<PieceFromIpAndPiece> = Vec::new();
        pieces_got.insert(file_hash.to_string(), piece_nums);

        let file_info = files_map
            .get(file_hash)
            .expect("error while getting val for give key");
        let peers = &file_info.peers;
        // ask for the pieces a peer has
        for peer in peers {
            let msg = format!("type:asking_available_pieces\nfile_hash:{}", file_hash);
            let socket_addrs = SocketAddr::new(peer.clone(), TCP_RECEIVER_PORT);
            let timeout = Duration::from_secs(5);
            let mut stream = TcpStream::connect_timeout(&socket_addrs, timeout).unwrap();
            stream.write(msg.as_bytes()).unwrap();
        }
    }

    pub fn select_pieces_from_peers(&mut self, selected_file_hash: String) {
        // let pieces: = self.pieces.as_mut().unwrap();
        let mut pieces: HashMap<FilePiece, Vec<IpAddr>> = HashMap::new();
        let files_map = self.files_map.lock().unwrap();
        let file_info = files_map.get(&selected_file_hash).unwrap();
        let file_name = &file_info.file.file_name;
        let file_hash = &file_info.file.file_hash;
        let file_size = &file_info.file.file_size;
        let piece_length = &file_info.file.piece_length;
        let total_pieces = &file_info.file.no_of_pieces;
        //initialize pieces with empty ip addresses
        for piece in &file_info.file.pieces {
            let ip: Vec<IpAddr> = Vec::new();
            pieces.insert(piece.clone(), ip);
        }

        //holding for peers response to fill up the peer_and_pieces
        Self::hold_for(2);
        let peer_and_pieces = self.peer_and_pieces.lock().unwrap();
        for peer in peer_and_pieces.iter() {
            for piece in &peer.pieces {
                pieces.get_mut(piece).unwrap().push(peer.ip_addr);
            }
        }

        //ask for pieces to the peers
        let mut rng = thread_rng();
        let timeout = Duration::from_secs(5);
        let mut peer_pieces: HashMap<IpAddr, Vec<FilePiece>> = HashMap::new();
        //divide pieces to ip addresses
        for (piece, addrs) in &pieces {
            let ip_addr = addrs.choose(&mut rng).unwrap();
            if peer_pieces.contains_key(ip_addr) {
                peer_pieces.get_mut(ip_addr).unwrap().push(piece.clone());
            } else {
                let mut pieces: Vec<FilePiece> = Vec::new();
                pieces.push(piece.clone());
                peer_pieces.insert(ip_addr.clone(), pieces);
            }
        }

        println!("peer and pieces: {:?} ", peer_pieces);
        for (ip_addr, pieces) in peer_pieces {
            println!("asking piece to {}", ip_addr.to_string());
            let addr = SocketAddr::new(ip_addr, TCP_RECEIVER_PORT);
            let mut stream =
                TcpStream::connect_timeout(&addr, timeout).expect("error while asking for piece");
            let initial_msg = format!("type:asking_pieces\nfile_name:{file_name}\nfile_hash:{file_hash}\nfile_size:{file_size}\ntotal_pieces:{total_pieces}\npiece_length:{piece_length}\nno_of_pieces:{}\n",pieces.len());
            stream.write(initial_msg.as_bytes()).unwrap();
            for piece in pieces {
                stream.write(piece.convert_to_str().as_bytes()).unwrap();
            }
        }
    }

    pub fn handle_left_out_pieces(&mut self, file_hash: &str) {
        println!("processing left out pieces and combining");
        let mut pieces_got = self.pieces_got.lock().unwrap();
        let mut files_map = self.files_map.lock().unwrap();
        // let piece_and_peers = self.pieces.clone().unwrap();

        let file_info = files_map.get(file_hash).unwrap();
        let pieces = file_info.file.pieces.clone();
        let total_pieces = pieces.len();

        let mut leftout_pieces: Vec<(&usize, &IpAddr)> = Vec::new();
        let ip_and_piece_no = pieces_got.get(file_hash).unwrap();
        let mut received_pieces: HashMap<usize, IpAddr> = HashMap::new();

        for elem in ip_and_piece_no {
            let ip = elem.ip;
            let piece_no = elem.piece_no;
            received_pieces.insert(piece_no, ip);
        }

        for piece in pieces {
            if !received_pieces.contains_key(&(piece.piece_no as usize)) {
                leftout_pieces.push(
                    received_pieces
                        .get_key_value(&(piece.piece_no as usize))
                        .unwrap(),
                );
            }
        }

        for (piece_no, ip) in leftout_pieces {
            break;
        }

        //if successful, combine
        Self::combine_pieces(file_hash, total_pieces, &mut files_map);
    }

    fn combine_pieces(
        file_hash: &str,
        total_pieces: usize,
        files_map: &mut MutexGuard<HashMap<String, FileInfo>>,
    ) {
        let folder_path = format!("pieces/{file_hash}");
        let file_info = files_map.get(file_hash).unwrap();
        let file_name = &file_info.file.file_name;
        let file_size = file_info.file.file_size as usize;
        let piece_length = file_info.file.piece_length as usize;

        let mut output_file = File::create(file_name).unwrap();
        let mut buffer = vec![0u8; piece_length];

        for piece_no in 0..total_pieces {
            let file_path = format!("pieces/{file_hash}/{piece_no}.txt");
            let mut file = fs::File::open(file_path).unwrap();
            file.read(&mut buffer).unwrap();
            if piece_no == total_pieces - 1 {
                println!("found the last piece");
                let assumed_size = total_pieces * piece_length;
                let error = assumed_size - file_size;
                let piece_size = piece_length - error;
                output_file.write_all(&buffer[..piece_size])
                    .expect("failed to write output_file");
            } else {
                output_file.write_all(&buffer).expect("failed to write file");
            }
        }
        println!("completed");

       
    }

    pub fn broadcast_search(&mut self, value: &str) {
        self.set_searching();
        println!("broadcasting search query");
        let msg = format!("type:search_query\nquery:{}", value);
        // broadcast my search query to all the connected devices
        let socket = UdpSocket::bind(format!("{IP_ADDRS}:3400"))
            .expect("couldn't bind udp socket to that address");
        socket.set_broadcast(true).expect("set_broadcast failed");
        socket
            .send_to(
                msg.as_bytes(),
                format!("255.255.255.255:{UDP_RECEIVER_PORT}"),
            )
            .expect("failed to send search_query using UDP");
        socket
            .send_to(msg.as_bytes(), "255.255.255.255:8888")
            .expect("failed to send search_query using UDP");
    }

    pub fn hold_for(secs: u64) {
        thread::sleep(Duration::from_secs(secs));
        println!("times up!");
    }

    pub fn set_searching(&mut self) {
        *self.status.lock().unwrap() = Status::Searching;
    }
    pub fn set_piece_handling(&mut self) {
        *self.status.lock().unwrap() = Status::PieceHandling;
    }
    pub fn reset_searching(&mut self) {
        *self.status.lock().unwrap() = Status::Idle;
        self.files_map.lock().unwrap().clear();
    }
    pub fn reset_piece_handling(&mut self) {
        *self.status.lock().unwrap() = Status::Idle;
        self.peer_and_pieces.lock().unwrap().clear();
    }
}
