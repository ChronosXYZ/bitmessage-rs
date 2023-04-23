use std::error::Error;

mod pow;
mod protocol;

use futures::StreamExt;
use libp2p::core::upgrade::Version;
use libp2p::swarm::keep_alive::Behaviour;
use libp2p::swarm::SwarmEvent;
use libp2p::{identity, noise, swarm::SwarmBuilder, tcp, yamux};
use libp2p::{PeerId, Transport};
use log::info;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer id: {local_peer_id:?}");

    let transport = tcp::async_io::Transport::default()
        .upgrade(Version::V1Lazy)
        .authenticate(noise::NoiseAuthenticated::xx(&local_key).unwrap())
        .multiplex(yamux::YamuxConfig::default())
        .boxed();

    let mut swarm =
        SwarmBuilder::with_async_std_executor(transport, Behaviour::default(), local_peer_id)
            .build();

    // Tell the swarm to listen on all interfaces and a random, OS-assigned
    // port.
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {address:?}"),
            SwarmEvent::Behaviour(event) => info!("{event:?}"),
            _ => {}
        }
    }
}
