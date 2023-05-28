#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{fs, thread, time};
use std::{process::Command, vec};
use swarmnet::SwarmNet;
use tauri::{Manager, Window};
use utilities::file_handler::FileDivider;
pub mod api_handlers;
pub mod hash_response;
pub mod piece_request;
pub mod piece_response;
pub mod retransmission;
pub mod search;
pub mod swarmnet;
pub mod utilities;
use regex::Regex;
use serde_json;
use serde_json::Value;
use local_ip_address::local_ip;

fn initialize_seeded_files(peer: &mut SwarmNet) {
    let dir_path = "./files_path";
    let files = fs::read_dir(dir_path).unwrap();
    let mut seeded_files = peer.seeded_files.lock().unwrap();
    for path in files {
        let entry = path.unwrap();
        let path = entry.path();
        let mut file_hash = path.file_name().unwrap().to_str().unwrap().to_string();
        //remove .txt
        file_hash.pop();
        file_hash.pop();
        file_hash.pop();
        file_hash.pop();

        let file_path = fs::read_to_string(path).unwrap();
        seeded_files.insert(file_hash, file_path);
    }
    // println!("{:?}", seeded_files);
}

#[derive(Clone, serde::Serialize)]
struct Payload {
    file_hash: String,
    download_speed: String,
    count: usize,
}
fn download_status_tracker(window: Arc<Mutex<Window>>, file_hash: String, chunks: usize) {
    println!("inside speed tracker");
    let dir_path = format!("pieces/{file_hash}");
    let mut initial_count = 0 as usize;
    let mut count = 0 as usize;
    let event_name = format!("download_speed_{file_hash}");
    while count <= chunks {
        let start_time = time::Instant::now();
        thread::sleep(Duration::from_secs(1));
        let files = match fs::read_dir(&dir_path) {
            Ok(files) => files,
            Err(_) => {
                println!("folder doesn't exist");
                count += 100;
                continue;}
        };
        count = files.count();
        let count_diff = count - initial_count;
        initial_count = count;
        let end_time = time::Instant::now();
        let time_diff = end_time.duration_since(start_time).as_secs_f64();
        let downloaded_size = count_diff;
        let download_speed = downloaded_size as f64 / time_diff as f64;
        let download_speed = format!("{:.2} MB/s", download_speed);
        // let percent= (count as f64/ chunks as f64) * 100 as f64;
        // let percent = download_speed

        println!("download speed {}", download_speed);        
        let window = window.lock().unwrap();
        window
            .emit_all(
                &event_name,
                Payload {
                    file_hash:file_hash.clone(),
                    download_speed,
                    count
                },
            )
            .unwrap();

        // let files = fs::read_dir(dir_path).unwrap();
        // let no_of_files = files.count();
    }
}
fn main() {

    let mut peer = SwarmNet::start();
    // peer.ip_addrs = ip_addrs;
    initialize_seeded_files(&mut peer);
    peer.start_tcp_thread();
    peer.start_udp_thread_for_search();
    let completion_tracker = peer.completion_tracker.clone();
    let downloading_files_map = peer.downloading_files_map.clone();
    let peer_and_pieces = peer.peer_and_pieces.clone();
    let seeded_files = peer.seeded_files.clone();
    SwarmNet::completion_check_loop(
        completion_tracker,
        downloading_files_map,
        peer_and_pieces,
        seeded_files,
    );

    let peer = Arc::new(Mutex::new(peer));
    let peer_cloned = Arc::clone(&peer);
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            send_active_files,
            delete_file
        ])
        .setup(move |app| {
            let peer = peer_cloned.clone();
            let peer1 = peer_cloned.clone();
            let peer2 = peer_cloned.clone();
            let peer3 = peer_cloned.clone();
            // listen to the `event-name` (emitted on any window)
            let window = app.get_window("main").unwrap();
            let window_copy = Arc::new(Mutex::new(window));
            let window_download = window_copy.clone();

            let _open_downloads_path = app.listen_global("open_downloads", move|event|{
                let payload = event.payload().unwrap();
                let payload: Value = serde_json::from_str(payload).unwrap();
                let mut file_hash = payload["file_hash"].to_string();
                file_hash.pop();
                file_hash.remove(0);
                let peer = peer3.lock().unwrap();
                let seeded_files = peer.seeded_files.lock().unwrap();
                let file_path = seeded_files.get(&file_hash).unwrap().clone();
                drop(seeded_files);
                open_downloaded_file(file_path);
            });

            let _download_speed = app.listen_global("handle_download", move |event| {
                let payload = event.payload().unwrap();
                let payload: Value = serde_json::from_str(payload).unwrap();
                let mut file_hash = payload["file_hash"].to_string();
                file_hash.pop();
                file_hash.remove(0);
                let chunks = payload["chunks"].to_string().parse::<usize>().unwrap();

                println!("file_hash {} ", file_hash);
                println!("chunks {} ", chunks);
                download_status_tracker(window_download.clone(), file_hash, chunks);
            });

            let _search_id = app.listen_global("search_query", move |event| {
                let mut peer = peer.lock().unwrap();
                let mut files_map = peer.files_map.lock().unwrap();
                files_map.clear();
                drop(files_map);
                let query: &str = serde_json::from_str(event.payload().unwrap()).unwrap();
                peer.broadcast_search(query);
                let result = search_results(&peer);
                let window = window_copy.lock().unwrap();
                window.emit_all("search_response", result).unwrap();
            });

            let _upload_id = app.listen_global("file_upload", move |event| {
                let payload = event.payload().unwrap();
                let payload: Value = serde_json::from_str(payload).unwrap();
                let mut file_path = payload["file_path"].to_string();
                file_path.pop();
                file_path.remove(0);

                let file_name = parse_uploaded_file_name(file_path.clone());
                let file_hash = FileDivider::upload(&file_name, 1024 * 1024, file_path.clone());
                if let Some(file_hash) = file_hash {
                    let peer = peer2.lock().unwrap();
                    let mut seeded_files = peer.seeded_files.lock().unwrap();
                    seeded_files.insert(file_hash.clone(), file_path.clone());
                    SwarmNet::maintain_files_path(&file_hash, file_path);
                }
            });

            let _request_file_info = app.listen_global("file_info", move |event| {
                let file_hash: &str = serde_json::from_str(event.payload().unwrap()).unwrap();
                let mut peer = peer1.lock().unwrap();
                peer.request_info_about_file_pieces(file_hash);
                SwarmNet::hold_for(2);
                peer.select_pieces_from_peers(file_hash.to_string());
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

fn parse_uploaded_file_name(file_path: String) -> String {
    let pattern = r"\\";
    let re = Regex::new(pattern).unwrap();
    let elements: Vec<&str> = re.split(&file_path).collect();
    let mut temp = elements.last().unwrap().to_string();
    temp
}

#[derive(Debug, Serialize, Clone)]
struct AvailableFiles {
    name: String,
    chunks: usize,
    fileHash: String,
    fileSize: usize,
    seeds: usize,
    status: usize,
}

#[tauri::command]
fn send_active_files() -> Vec<AvailableFiles> {
    let dir_path = "./file_hash";
    let files = fs::read_dir(dir_path).unwrap();
    let mut no_of_files = 0;
    let mut overall_files_string = String::new();

    for path in files {
        no_of_files += 1;
        let entry = path.unwrap();
        let path = entry.path();

        let contents = fs::read_to_string(path).unwrap();
        overall_files_string.push_str(&contents);
    }

    let reader = BufReader::new(overall_files_string.as_bytes());

    let mut lines = reader.lines().map(|l| l.unwrap());
    let files_info = FileDivider::parse(&mut lines, no_of_files);

    let mut output_files_info: Vec<AvailableFiles> = Vec::with_capacity(no_of_files);
    for file in files_info {
        let temp = AvailableFiles {
            name: file.file_name,
            chunks: file.no_of_pieces as usize,
            fileHash: file.file_hash,
            fileSize: file.file_size as usize,
            seeds: 2,
            status: 100,
        };
        output_files_info.push(temp);
    }
    output_files_info
}

fn open_downloaded_file(file_path: String) {
    println!("file_path {}", file_path);
    Command::new("explorer")
        .args(["/select,", &file_path])
        .spawn()
        .unwrap();
}
#[tauri::command]
fn delete_file(file_name: String, file_hash: String) {
    let file_hash_path = format!("file_hash/{file_name}.txt");
    let files_path = format!("files_path/{file_hash}.txt");
    let piece_info_path = format!("piece_info/{file_hash}.txt");
    let paths = [file_hash_path, files_path, piece_info_path];
    for path in paths {
        match fs::remove_file(path) {
            Ok(()) => println!("File deleted successfully."),
            Err(e) => println!("Error deleting file: {}", e),
        }
    }
}

fn search_results(peer: &SwarmNet) -> Vec<AvailableFiles> {
    thread::sleep(Duration::from_secs(2));
    let mut files_map = peer.files_map.lock().unwrap();
    let mut output_files_info: Vec<AvailableFiles> = Vec::new();
    let keys = files_map.keys();
    for key in keys {
        let file_info = files_map.get(key).unwrap();
        let file = &file_info.file;
        let output = AvailableFiles {
            name: file.file_name.clone(),
            chunks: file.no_of_pieces as usize,
            fileHash: file.file_hash.clone(),
            fileSize: file.file_size.clone() as usize,
            seeds: file_info.peers.len(),
            status: 0,
        };
        output_files_info.push(output);
    }
    output_files_info
}
