use std::error::Error;

use futures::{
    channel::{mpsc, oneshot},
    SinkExt,
};
use libp2p::{Multiaddr, PeerId};

use crate::network::address::Address;

use super::worker::WorkerCommand;

pub struct NodeClient {
    sender: mpsc::Sender<WorkerCommand>,
}

impl NodeClient {
    pub fn new(sender: mpsc::Sender<WorkerCommand>) -> Self {
        Self { sender }
    }

    pub async fn start_listening(
        &mut self,
        multiaddr: Multiaddr,
    ) -> Result<(), Box<dyn Error + Send>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(WorkerCommand::StartListening { multiaddr, sender })
            .await
            .expect("Command receiver not to be dropped");
        receiver.await.expect("Sender not to be dropped")
    }

    pub async fn get_listeners(&mut self) -> Multiaddr {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(WorkerCommand::GetListenerAddress { sender })
            .await
            .expect("Command receiver not to be dropped");
        receiver.await.expect("Sender not to be dropped")
    }

    pub async fn get_peer_id(&mut self) -> PeerId {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(WorkerCommand::GetPeerID { sender })
            .await
            .expect("Command receiver not to be dropped");
        receiver.await.expect("Sender not to be dropped")
    }

    pub fn shutdown(&mut self) {
        self.sender.close_channel();
    }

    pub async fn get_own_identities(&mut self) -> Vec<Address> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(WorkerCommand::GetOwnIdentities { sender })
            .await
            .expect("Receiver not to be dropped");
        receiver.await.expect("Sender not to be dropped").unwrap()
    }

    pub async fn generate_new_identity(&mut self, label: String) -> String {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(WorkerCommand::GenerateIdentity { label, sender })
            .await
            .expect("Receiver not to be dropped");
        receiver
            .await
            .expect("Sender not to be dropped")
            .expect("repo not to fail")
    }

    pub async fn delete_identity(&mut self, address: String) {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(WorkerCommand::DeleteIdentity { address, sender })
            .await
            .expect("Receiver not to be dropped");
        receiver
            .await
            .expect("Sender not to be dropped")
            .expect("repo not to fail")
    }

    pub async fn rename_identity(&mut self, address: String, new_label: String) {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(WorkerCommand::RenameIdentity {
                new_label,
                address,
                sender,
            })
            .await
            .expect("Receiver not to be dropped");
        receiver
            .await
            .expect("Sender not to be dropped")
            .expect("repo not to fail")
    }
}
