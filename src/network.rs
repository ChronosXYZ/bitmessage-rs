use futures::channel::mpsc;
use libp2p::Multiaddr;

use self::node::{client::NodeClient, worker::NodeWorker};

pub mod address;
pub mod behaviour;
pub mod messages;
pub mod node;

pub fn new(bootstrap_nodes: Option<Vec<Multiaddr>>) -> (NodeClient, NodeWorker) {
    let (command_sender, command_receiver) = mpsc::channel(1);
    let worker = NodeWorker::new(command_receiver, bootstrap_nodes);
    let client = NodeClient::new(command_sender);
    (client, worker)
}
