use std::iter;

use futures::StreamExt;
use libp2p::{
    core::upgrade::Version,
    gossipsub, identity, noise,
    request_response::{self, ProtocolSupport},
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, yamux, PeerId, Swarm, Transport,
};
use log::info;

use crate::network::behaviour::{BitmessageProtocol, BitmessageProtocolCodec};

use super::behaviour::BitmessageNetBehaviour;

pub struct Node {
    local_peer_id: PeerId,
    swarm: Swarm<BitmessageNetBehaviour>,
}

impl Node {
    pub fn new() -> Node {
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
                    gossipsub::MessageAuthenticity::Signed(local_key),
                    Default::default(),
                )
                .unwrap(),
                rpc: request_response::Behaviour::new(
                    BitmessageProtocolCodec(),
                    iter::once((BitmessageProtocol(), ProtocolSupport::Full)),
                    Default::default(),
                ),
            },
            local_peer_id,
        )
        .build();

        // Tell the swarm to listen on all interfaces and a random, OS-assigned
        // port.
        swarm
            .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
            .unwrap();

        Node {
            local_peer_id,
            swarm,
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {address:?}"),
                SwarmEvent::Behaviour(event) => info!("{event:?}"),
                _ => {}
            }
        }
    }
}
