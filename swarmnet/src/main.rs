use swarmnet::SwarmNet;
pub mod piece_request;
pub mod piece_response;
pub mod hash_response;
pub mod search;
pub mod swarmnet;
pub mod utilities;
fn main() {
    // the struct stores Hashmap, tcp_thread
    let mut peer = SwarmNet::start();
    // call TCP thread where each request is parsed in individual thread
    peer.start_tcp_thread(); 
    SwarmNet::hold_for(1);
    println!("Enter search value: ");
    let mut query = String::new();
    std::io::stdin().read_line(&mut query).expect("failed to read input");
    peer.broadcast_search(&query);

    // //holding for 10sec to give time for listener to get requests
    SwarmNet::hold_for(2);
    println!("asking for information about file hash:");

    

    //peer.files_map returns my Hashmap

    let files = peer.files_map.lock().unwrap();
    //just trying to print ipaddress of the peers
    let keys = files.keys();
    for k in keys {
        println!("file name:{:?}",files.get(k).unwrap().file.file_name );
        println!("file hash:{:?}",files.get(k).unwrap().file.file_hash );
        println!("file size:{:?}",files.get(k).unwrap().file.file_size );
        println!("peers\n{:?}\n",files.get(k).unwrap().peers );
    }
    drop(files);

   
    println!("asking for information about file hash, enter the hash: ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("failed to read input");
    peer.request_info_about_file_pieces(input.trim());

    println!("\nI will be asking for the actual piece wait for 5sec");
    SwarmNet::hold_for(5);
    peer.select_pieces_from_peers(input.trim().to_string());

    //at this point we've got the pieces
    SwarmNet::hold_for(5);
    println!("\nchecking if all pieces were received");
    peer.handle_left_out_pieces(input.trim());


    peer.tcp_thread.unwrap().join().unwrap();
}
