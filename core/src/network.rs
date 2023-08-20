use libp2p::Multiaddr;

use self::node::{client::NodeClient, worker::NodeWorker};

pub(crate) mod address;
pub(crate) mod behaviour;
pub(crate) mod messages;
pub mod node;

pub fn new(bootstrap_nodes: Option<Vec<Multiaddr>>) -> (NodeClient, NodeWorker) {
    let (worker, sender) = NodeWorker::new(bootstrap_nodes);
    let client = NodeClient::new(sender);
    (client, worker)
}
