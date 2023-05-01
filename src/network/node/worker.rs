use std::{borrow::Cow, error::Error, io, iter};

use futures::{
    channel::{mpsc, oneshot},
    select, StreamExt,
};
use libp2p::{
    core::upgrade::Version,
    gossipsub::{self, HandlerError},
    identify, identity,
    kad::{store::MemoryStore, Kademlia, KademliaConfig},
    noise,
    request_response::{self, ProtocolSupport},
    swarm::{derive_prelude::Either, ConnectionHandlerUpgrErr, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm, Transport,
};
use log::{debug, info};

use crate::network::{
    behaviour::{
        BitmessageBehaviourEvent, BitmessageNetBehaviour, BitmessageProtocol,
        BitmessageProtocolCodec, BitmessageRequest, BitmessageResponse,
    },
    messages::{MessageCommand, MessagePayload, NetworkMessage},
};

struct Handler;

impl Handler {
    fn new() -> Handler {
        Handler
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

#[derive(Debug)]
pub enum WorkerCommand {
    StartListening {
        multiaddr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    Dial {
        peer: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    GetListenerAddresses {
        sender: oneshot::Sender<Vec<Multiaddr>>,
    },
}

pub struct NodeWorker {
    local_peer_id: PeerId,
    swarm: Swarm<BitmessageNetBehaviour>,
    handler: Handler,
    command_receiver: mpsc::Receiver<WorkerCommand>,
}

impl NodeWorker {
    pub fn new(
        command_receiver: mpsc::Receiver<WorkerCommand>,
        bootstrap_nodes: Option<Vec<Multiaddr>>,
    ) -> NodeWorker {
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

        Self {
            local_peer_id,
            swarm,
            handler: Handler::new(),
            command_receiver,
        }
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<
            BitmessageBehaviourEvent,
            Either<
                Either<Either<HandlerError, io::Error>, io::Error>,
                ConnectionHandlerUpgrErr<io::Error>,
            >,
        >,
    ) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {address:?}"),
            SwarmEvent::Behaviour(BitmessageBehaviourEvent::RequestResponse(
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

    async fn handle_command(&mut self, command: WorkerCommand) {
        match command {
            WorkerCommand::StartListening { multiaddr, sender } => {
                debug!("Starting listening to the network...");
                let _ = match self.swarm.listen_on(multiaddr) {
                    Ok(_) => sender.send(Ok(())),
                    Err(e) => sender.send(Err(Box::new(e))),
                };
            }
            WorkerCommand::Dial { peer, sender } => todo!(),
            WorkerCommand::GetListenerAddresses { sender } => todo!(),
        }
    }

    pub async fn run(mut self) {
        loop {
            select! {
                event = self.swarm.next() => self.handle_event(event.expect("Swarm stream to be infinite.")).await,
                command = self.command_receiver.next() => match command {
                    Some(c) => self.handle_command(c).await,
                    // Command channel closed, thus shutting down the network event loop.
                    None=>  return,
               }
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

//#[cfg(test)]
//mod tests {
//    use async_std::task;
//
//    use super::*;
//
//    #[async_std::test]
//    async fn it_works() {
//        let n1 = NodeWorker::new(None);
//        let n1_listeners = n1.swarm.listeners().last().unwrap().clone();
//        task::spawn(n1.run());
//
//        let n2 = NodeWorker::new(Some(vec![n1_listeners]));
//        task::block_on(n2.run());
//    }
//}
