use crate::utilities::file_handler::{FileDivider, FileInfo};
use ::std::collections::HashMap;
use std::fs::{File, self};
use std::io::{BufReader, BufRead, Write, Read};
use std::net::{TcpStream, SocketAddr};
use std::net::{IpAddr};
use std::sync::{Arc, Mutex, MutexGuard};
use std::io;
pub fn handle_hash(
    reader: &mut BufReader<&mut TcpStream>,
    files_map: Arc<Mutex<HashMap<String, FileInfo>>>,
    addrs: IpAddr,
) {
    let mut files_map = files_map.lock().unwrap();
    let mut lines = reader.lines().map(|l| l.unwrap());
    let no_of_files = lines.next().unwrap()[12..].parse::<usize>().unwrap();

    let files = FileDivider::parse(&mut lines, no_of_files);
    for file in files {
        create_hash_map(file, &mut files_map, addrs);
    }
}

fn create_hash_map(
    file: FileDivider,
    files_map: &mut MutexGuard<HashMap<String, FileInfo>>,
    addrs: IpAddr,
) {
    if files_map.contains_key(&file.file_hash) {
        println!("value already inserted");
        let file_info = files_map.get_mut(&file.file_hash).unwrap();
        file_info.push_peer(addrs);
    } else {
        let peers: Vec<IpAddr> = vec![addrs];
        let key = file.file_hash.clone();
        let file_info = FileInfo::new(file, peers);
        files_map.insert(key, file_info);
    }
}

