use crate::hash_response;
use crate::piece_request;
use crate::piece_response;
use crate::retransmission;
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
use dirs;
use local_ip_address::local_ip;
use lazy_static::lazy_static;
//for randomly assigning piece to an IP_Addr
use rand::seq::SliceRandom;
use rand::thread_rng;
lazy_static! {
    static ref IP_ADDRS: String = initialize_ip();
}

fn initialize_ip() -> String {
    // Perform some initialization logic here and return the value for the global variable
    local_ip().unwrap().to_string()
}

//remember you have multple tcp udp request for third copy
// pub const IP_ADDRS: &str = "192.168.1.66";
// pub const *IP_ADDRS: &str = "127.0.0.1";
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

pub struct FileCompletionInfo {
    pub file_hash: String,
    pub no_of_pieces: usize,
    pub received_pieces: Vec<usize>,
}

pub struct SwarmNet {
    pub udp_thread: Option<JoinHandle<()>>,
    pub tcp_thread: Option<JoinHandle<()>>,
    //filehash, peers search result
    pub files_map: Arc<Mutex<HashMap<String, FileInfo>>>,
    //downloading files
    pub downloading_files_map: Arc<Mutex<HashMap<String, FileInfo>>>,
    //file_hash, PiecesFromPeer:ipAddr, Vec<FilePiece>
    pub peer_and_pieces: Arc<Mutex<HashMap<String, Vec<PiecesFromPeer>>>>,
    //file hash, piece hash and peers ;
    pub piece_and_peers: Arc<Mutex<HashMap<String, HashMap<FilePiece, Vec<IpAddr>>>>>,
    pub status: Arc<Mutex<Status>>,
    //file hash and collection of acquired piece numbers and ip
    pub pieces_got: Arc<Mutex<HashMap<String, Vec<PieceFromIpAndPiece>>>>,
    //transfer completion tracker,file hash and bool(completed?)
    pub completion_tracker: Arc<Mutex<Vec<String>>>,

    //seeded files and their path
    pub seeded_files: Arc<Mutex<HashMap<String, String>>>
}

impl SwarmNet {
    pub fn start() -> SwarmNet {
        let files_map: Arc<Mutex<HashMap<String, FileInfo>>> = Arc::new(Mutex::new(HashMap::new()));
        let downloading_files_map: Arc<Mutex<HashMap<String, FileInfo>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let piece_and_peers: Arc<Mutex<HashMap<String, HashMap<FilePiece, Vec<IpAddr>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let peer_and_pieces: Arc<Mutex<HashMap<String, Vec<PiecesFromPeer>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let completion_tracker: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        let pieces_got: Arc<Mutex<HashMap<String, Vec<PieceFromIpAndPiece>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let seeded_files:Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
        SwarmNet {
            seeded_files,
            files_map,
            downloading_files_map,
            piece_and_peers,
            completion_tracker,
            peer_and_pieces,
            pieces_got,
            udp_thread: None,
            tcp_thread: None,
            status: Arc::new(Mutex::new(Status::Idle)),

        }
    }

    pub fn start_udp_thread_for_search(&mut self) {
        let udp_thread = thread::spawn(|| {
            let socket = UdpSocket::bind(format!("{}:{UDP_LISTENER_PORT}",*IP_ADDRS))
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
                    let mut datagram = datagram.lines().map(|l| l);
                    let req_type = &datagram.next().unwrap()[5..];

                    match req_type {
                        "search_query" => {
                            let q = &datagram.next().unwrap()[6..];
                            search::process_search(q, ip_addr);
                        }
                        "retransmission" => {
                            let file_hash = &datagram.next().unwrap()[10..];
                            retransmission::handle_peers_details_retransmission(file_hash, ip_addr);
                        }
                        _ => {}
                    }
                });
            }
        });
        self.udp_thread = Some(udp_thread);
    }

    pub fn start_tcp_thread(&mut self) {
        let files_map = self.files_map.clone();
        let peer_and_pieces = self.peer_and_pieces.clone();
        let completion_tracker = self.completion_tracker.clone();
        let piece_and_peers = self.piece_and_peers.clone();
        let status = self.status.clone();
        let pieces_got = self.pieces_got.clone();
        let seeded_files = self.seeded_files.clone();

        let tcp_thread = thread::spawn(move || {
            let listener = TcpListener::bind(format!("{}:{TCP_LISTENER_PORT}",*IP_ADDRS)).unwrap();
            let requests_thread_pool = ThreadPool::new(15);
            println!("Listening for TCP requests");

            for stream in listener.incoming() {
                println!("receiving data");
                let files_map = files_map.clone();
                let peer_and_pieces = peer_and_pieces.clone();
                let piece_and_peers = piece_and_peers.clone();
                let completion_tracker = completion_tracker.clone();
                let status = status.clone();
                let stream = stream.unwrap();
                let pieces_got = pieces_got.clone();
                let seeded_files = seeded_files.clone();

                requests_thread_pool.execute(move || {

                    Self::handle_request(
                        stream,
                        files_map,
                        completion_tracker,
                        peer_and_pieces,
                        piece_and_peers,
                        status,
                        pieces_got,
                        seeded_files
                    );
                });
            }
        });
        self.tcp_thread = Some(tcp_thread);
    }

    pub fn handle_request(
        mut stream: TcpStream,
        files_map: Arc<Mutex<HashMap<String, FileInfo>>>,
        completion_tracker: Arc<Mutex<Vec<String>>>,
        peer_and_pieces: Arc<Mutex<HashMap<String, Vec<PiecesFromPeer>>>>,
        mut piece_and_peers: Arc<Mutex<HashMap<String, HashMap<FilePiece, Vec<IpAddr>>>>>,
        _status: Arc<Mutex<Status>>,
        pieces_got: Arc<Mutex<HashMap<String, Vec<PieceFromIpAndPiece>>>>,
        seeded_files: Arc<Mutex<HashMap<String, String>>>
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
                hash_response::handle_hash(&mut reader, files_map, addrs);
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
                piece_request::handle_piece_request(&mut reader, addrs, seeded_files);
            }

            //actual pieces accept handler
            "pieces" => {
                let file_hash = piece_response::handle_pieces(
                    &mut reader,
                    addrs,
                    &mut piece_and_peers,
                    pieces_got,
                );
                Self::update_if_transfer_is_completed(
                    &file_hash,
                    piece_and_peers,
                    completion_tracker,
                );
            }
            //retransimmison of peers info about a file RESPONSE
            "re_peer_info" => retransmission::retransmisson_response_handler(&mut reader, addrs),

            _ => (),
        }
    }

    fn update_if_transfer_is_completed(
        file_hash: &str,
        piece_and_peers: Arc<Mutex<HashMap<String, HashMap<FilePiece, Vec<IpAddr>>>>>,
        completion_tracker: Arc<Mutex<Vec<String>>>,
    ) {
        let mut piece_and_peers = piece_and_peers.lock().unwrap();

        if let Some(inner) = piece_and_peers.get(file_hash) {
            if inner.is_empty() {
                let mut completion_tracker = completion_tracker.lock().unwrap();
                completion_tracker.push(file_hash.to_string());
                piece_and_peers.remove(file_hash).unwrap();
            }
        }
    }

    pub fn update_file_hash(file_info: &FileInfo) {
        let file_path = format!("file_hash/{}.txt", file_info.file.file_name);
        let mut piece_info_file = if let Ok(file) = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&file_path)
        {
            //file already exists
            return ();
        } else {
            println!("creating new file");
            let mut file = File::create(&file_path).unwrap();

            let mut contents = format!(
                "file_name:{}\nfile_hash:{}\nfile_size:{}\nno_of_pieces:{}\npiece_length:{}\n",
                file_info.file.file_name,
                file_info.file.file_hash,
                file_info.file.file_size,
                file_info.file.no_of_pieces,
                file_info.file.piece_length
            );
            file.write(contents.as_bytes()).unwrap();

            file.write(file_info.file.get_pieces_string().as_bytes())
                .unwrap();
        };
    }
    pub fn request_info_about_file_pieces(&mut self, file_hash: &str) {
        self.set_piece_handling();

        let mut files_map = self.files_map.lock().unwrap();
        let mut peer_and_pieces = self.peer_and_pieces.lock().unwrap();
        let vtemp: Vec<PiecesFromPeer> = Vec::new();
        peer_and_pieces.insert(file_hash.to_string(), vtemp);
        let mut downloading_files_map = self.downloading_files_map.lock().unwrap();
        let file_info = files_map.remove(file_hash).unwrap();

        downloading_files_map.insert(file_hash.to_string(), file_info);
        files_map.clear();
        drop(files_map);

        //initialize pieces got
        let mut pieces_got = self.pieces_got.lock().unwrap();
        let piece_nums: Vec<PieceFromIpAndPiece> = Vec::new();
        pieces_got.insert(file_hash.to_string(), piece_nums);

        let file_info = downloading_files_map
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

    fn calculate_piece_and_peers(&mut self, file_hash: &str) {
        let peer_and_pieces = self.peer_and_pieces.lock().unwrap();
    }

    pub fn completion_check_loop(
        completion_tracker: Arc<Mutex<Vec<String>>>,
        downloading_files_map: Arc<Mutex<HashMap<String, FileInfo>>>,
        peer_and_pieces: Arc<Mutex<HashMap<String, Vec<PiecesFromPeer>>>>,
        seeded_files: Arc<Mutex<HashMap<String, String>>>,
    ) {
        thread::spawn(move || loop {
            let mut completion_tracker = completion_tracker.lock().unwrap();
            for (index, file_hash) in completion_tracker.clone().iter().enumerate() {
                let seeded_files = seeded_files.clone();
                let mut downloading_files_map = downloading_files_map.lock().unwrap();
                Self::combine_pieces(file_hash, &mut downloading_files_map, seeded_files);
                completion_tracker.remove(index);
                let mut peer_and_pieces = peer_and_pieces.lock().unwrap();
                peer_and_pieces.remove(file_hash);
                drop(peer_and_pieces);

                //remove pieces file
                let folder_path = format!("pieces/{file_hash}");

                // Remove the folder
                fs::remove_dir_all(folder_path).unwrap();
                println!("pieces combined successfully!")
            }
        });
    }

    pub fn select_pieces_from_peers(&mut self, selected_file_hash: String) {
        // let pieces: = self.pieces.as_mut().unwrap();
        let mut pieces: HashMap<FilePiece, Vec<IpAddr>> = HashMap::new();
        let downloading_files_map = self.downloading_files_map.lock().unwrap();
        let file_info = downloading_files_map.get(&selected_file_hash).unwrap();
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
        let peer_and_pieces_arr = peer_and_pieces.get(file_hash).unwrap();
        for peer in peer_and_pieces_arr.iter() {
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
        //
        let mut piece_and_peers = self.piece_and_peers.lock().unwrap();
        piece_and_peers.insert(file_hash.to_string(), pieces);
        drop(piece_and_peers);
        Self::update_file_hash(file_info);

        let mut seeded_files = self.seeded_files.lock().unwrap();
        seeded_files.insert(file_hash.to_string(), "downloading".to_string());
        Self::maintain_files_path(file_hash, "downloading".to_string());
    }

    fn combine_pieces(
        file_hash: &str,
        downloading_files_map: &mut MutexGuard<HashMap<String, FileInfo>>,
        seeded_files: Arc<Mutex<HashMap<String, String>>>
    ) {
        let file_info = downloading_files_map.get(file_hash).unwrap();
        let file_name = file_info.file.file_name.clone();
        let file_size = file_info.file.file_size.clone() as usize;
        let piece_length = file_info.file.piece_length.clone() as usize;
        let total_pieces = file_info.file.no_of_pieces as usize;

        let mut download_path = dirs::download_dir().unwrap();
        download_path.push(file_name);

        let mut output_file = File::create(&download_path).unwrap();
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
                output_file
                    .write_all(&buffer[..piece_size])
                    .expect("failed to write output_file");
            } else {
                output_file
                    .write_all(&buffer)
                    .expect("failed to write file");
            }
        }
        //releasing the downloading_files_map
        downloading_files_map.remove(file_hash).unwrap();
        let mut seeded_files = seeded_files.lock().unwrap();
        seeded_files.insert(file_hash.to_string(), download_path.to_str().unwrap().to_string());
        Self::maintain_files_path(file_hash, download_path.to_str().unwrap().to_string());
    }

    pub fn broadcast_search(&mut self, value: &str) {
        self.set_searching();
        println!("broadcasting search query");
        let msg = format!("type:search_query\nquery:{}\n", value);
        // broadcast my search query to all the connected devices
        let socket = UdpSocket::bind(format!("{}:3400",*IP_ADDRS))
            .expect("couldn't bind udp socket to that address");
        socket.set_broadcast(true).expect("set_broadcast failed");
        socket
            .send_to(
                msg.as_bytes(),
                format!("255.255.255.255:{UDP_RECEIVER_PORT}"),
            )
            .expect("failed to send search_query using UDP");
        // socket
        //     .send_to(msg.as_bytes(), "255.255.255.255:8888")
        //     .expect("failed to send search_query using UDP");
    }

    pub fn broadcast_file_hash_retransmission(&mut self, file_hash: &str) {
        let msg = format!("type:retransmission\nfile_hash:{}\n", file_hash);
        // broadcast my search query to all the connected devices
        let socket = UdpSocket::bind(format!("{}:3401",*IP_ADDRS))
            .expect("couldn't bind udp socket to that address");
        socket.set_broadcast(true).expect("set_broadcast failed");

        if let Ok(_b) = socket.send_to(
            msg.as_bytes(),
            format!("255.255.255.255:{UDP_RECEIVER_PORT}"),
        ) {
            println!("broadcasting file hash to gather missing pieces\n");
        } else {
            println!("failed to send search_query using UDP");
        }
    }

    pub fn maintain_files_path(file_hash: &str, file_path: String){
        let path = format!("files_path/{file_hash}.txt");
        let mut file = File::create(path).unwrap();
        file.write(file_path.as_bytes()).unwrap();

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
