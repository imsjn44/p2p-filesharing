use swarmnet::SwarmNet;
use utilities::file_handler::FileDivider;
pub mod piece_request;
pub mod piece_response;
pub mod hash_response;
pub mod search;
pub mod swarmnet;
pub mod utilities;
fn main() {
    let mut peer = SwarmNet::start();
    // call TCP thread where each request is parsed in individual thread
    peer.start_tcp_thread(); 
    peer.start_udp_thread_for_search();

    peer.tcp_thread.unwrap().join().unwrap();
    peer.udp_thread.unwrap().join().unwrap();
    // FileDivider::upload("JohnWick3.mp4", 1024*1024);

}
