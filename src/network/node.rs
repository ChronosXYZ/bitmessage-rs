use std::{borrow::Cow, error::Error, iter};

use futures::StreamExt;
use libp2p::{
    core::upgrade::Version,
    gossipsub, identify, identity,
    kad::{store::MemoryStore, Kademlia, KademliaConfig},
    noise,
    request_response::{self, ProtocolSupport},
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm, Transport,
};
use log::{debug, info};

use crate::network::behaviour::{BitmessageProtocol, BitmessageProtocolCodec};

use super::{
    behaviour::{BitmessageNetBehaviour, BitmessageRequest, BitmessageResponse},
    messages::{MessageCommand, MessagePayload, NetworkMessage},
};

struct Handler {}

impl Handler {
    fn new() -> Handler {
        Handler {}
    }

    fn handle_request(&self, req: BitmessageRequest) -> NetworkMessage {
        match req.0.command {
            MessageCommand::GetData => todo!(),
            MessageCommand::Inv => todo!(),
            MessageCommand::ReqInv => self.handle_get_inv_message(req.0.payload),
            MessageCommand::Object => todo!(),
        }
    }

    fn handle_get_inv_message(&self, _: MessagePayload) -> NetworkMessage {
        return NetworkMessage {
            command: MessageCommand::Inv,
            payload: MessagePayload::Inv {
                inventory: Vec::new(),
            },
        };
    }
}

const IDENTIFY_PROTO_NAME: &str = "/bitmessage/id/1.0.0";
const KADEMLIA_PROTO_NAME: &[u8] = b"/bitmessage/kad/1.0.0";

pub struct Node {
    local_peer_id: PeerId,
    swarm: Swarm<BitmessageNetBehaviour>,
    handler: Handler,
}

impl Node {
    pub fn new(bootstrap_nodes: Option<Vec<Multiaddr>>) -> Node {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Local peer id: {local_peer_id:?}");

        let transport = tcp::async_io::Transport::default()
            .upgrade(Version::V1Lazy)
            .authenticate(noise::NoiseAuthenticated::xx(&local_key).unwrap())
            .multiplex(yamux::YamuxConfig::default())
            .boxed();

        let mut swarm = SwarmBuilder::with_async_std_executor(
            transport,
            BitmessageNetBehaviour {
                gossipsub: gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(local_key.clone()),
                    Default::default(),
                )
                .unwrap(),
                rpc: request_response::Behaviour::new(
                    BitmessageProtocolCodec(),
                    iter::once((BitmessageProtocol(), ProtocolSupport::Full)),
                    Default::default(),
                ),
                kademlia: Kademlia::with_config(
                    local_peer_id,
                    MemoryStore::new(local_peer_id),
                    KademliaConfig::default()
                        .set_protocol_names(
                            iter::once(Cow::Borrowed(KADEMLIA_PROTO_NAME)).collect(),
                        )
                        .to_owned(),
                ),
                identify: identify::Behaviour::new(identify::Config::new(
                    IDENTIFY_PROTO_NAME.to_string(),
                    local_key.public(),
                )),
            },
            local_peer_id,
        )
        .build();

        if let Some(bootstrap_peers) = bootstrap_nodes {
            // First, we add the addresses of the bootstrap nodes to our view of the DHT
            for peer_address in &bootstrap_peers {
                let peer_id = extract_peer_id_from_multiaddr(peer_address).unwrap(); // FIXME
                swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, peer_address.clone());
            }

            // Next, we add our own info to the DHT. This will then automatically be shared
            // with the other peers on the DHT. This operation will fail if we are a bootstrap peer.
            swarm
                .behaviour_mut()
                .kademlia
                .bootstrap()
                .map_err(|err| err)
                .unwrap();
        }

        // Tell the swarm to listen on all interfaces and a random, OS-assigned
        // port.
        swarm
            .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
            .unwrap();

        Node {
            local_peer_id,
            swarm,
            handler: Handler::new(),
        }
    }

    pub async fn run(&mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {address:?}"),
                SwarmEvent::Behaviour(super::behaviour::BitmessageNetBehaviourEvent::Rpc(
                    request_response::Event::Message { message, .. },
                )) => match message {
                    request_response::Message::Request {
                        request_id,
                        request,
                        channel,
                    } => {
                        debug!("received request {request_id}: {:?}", request);
                        let msg = self.handler.handle_request(request);
                        self.swarm
                            .behaviour_mut()
                            .rpc
                            .send_response(channel, BitmessageResponse(msg))
                            .unwrap()
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        debug!("received response on {request_id}: {:?}", response);
                    }
                },
                _ => {}
            }
        }
    }

    /// When we receive IdentityInfo, if the peer supports our Kademlia protocol, we add
    /// their listen addresses to the DHT, so they will be propagated to other peers.
    fn handle_identify_event(&mut self, identify_event: Box<identify::Event>) {
        log::debug!("Received identify::Event: {:?}", *identify_event);

        if let identify::Event::Received {
            peer_id,
            info:
                identify::Info {
                    listen_addrs,
                    protocols,
                    ..
                },
        } = *identify_event
        {
            if protocols
                .iter()
                .any(|p| p.as_bytes() == KADEMLIA_PROTO_NAME)
            {
                for addr in listen_addrs {
                    log::debug!("Adding received IdentifyInfo matching protocol '{}' to the DHT. Peer: {}, addr: {}", String::from_utf8_lossy(KADEMLIA_PROTO_NAME), peer_id, addr);
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }
        }
    }
}

fn extract_peer_id_from_multiaddr(
    address_with_peer_id: &Multiaddr,
) -> Result<PeerId, Box<dyn Error>> {
    match address_with_peer_id.iter().last() {
        Some(multiaddr::Protocol::P2p(hash)) => PeerId::from_multihash(hash).map_err(|multihash| {
            format!("Invalid PeerId '{multihash:?}' in Multiaddr '{address_with_peer_id}'").into()
        }),
        _ => Err("Multiaddr does not contain peer_id".into()),
    }
}

#[cfg(test)]
mod tests {

    // TODO
    #[test]
    fn it_works() {}
}
