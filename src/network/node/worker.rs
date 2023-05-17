use std::{borrow::Cow, error::Error, fs, io, iter, time::Duration};

use diesel::{
    connection::SimpleConnection,
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use directories::ProjectDirs;
use futures::{
    channel::{mpsc, oneshot},
    select, StreamExt,
};
use libp2p::{
    core::upgrade::Version,
    gossipsub::{self, MessageId, PublishError, Sha256Topic},
    identify, identity,
    kad::{store::MemoryStore, Kademlia, KademliaConfig},
    mdns, noise,
    request_response::{self, ProtocolSupport},
    swarm::{derive_prelude::Either, ConnectionHandlerUpgrErr, SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm, Transport,
};
use log::{debug, info};

use crate::{
    network::{
        behaviour::{
            BitmessageBehaviourEvent, BitmessageNetBehaviour, BitmessageProtocol,
            BitmessageProtocolCodec, BitmessageResponse,
        },
        messages::{self, MessageCommand, MessagePayload},
    },
    repositories::{
        inventory::InventoryRepositorySync,
        sqlite::{
            address::SqliteAddressRepository, inventory::SqliteInventoryRepository,
            message::SqliteMessageRepository,
        },
    },
};

use super::handler::Handler;

const IDENTIFY_PROTO_NAME: &str = "/bitmessage/id/1.0.0";
const KADEMLIA_PROTO_NAME: &[u8] = b"/bitmessage/kad/1.0.0";

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/repositories/sqlite/migrations");
const COMMON_PUBSUB_TOPIC: &'static str = "common";

#[derive(Debug)]
pub struct DbConnectionOpts {
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for DbConnectionOpts {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            if self.enable_wal {
                conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            }
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }
            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

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
    GetListenerAddress {
        sender: oneshot::Sender<Multiaddr>,
    },
    GetPeerID {
        sender: oneshot::Sender<PeerId>,
    },
    BroadcastMsgByPubSub {
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
        msg: messages::NetworkMessage,
    },
    NonceCalculated {
        obj: messages::Object,
    },
}

pub struct NodeWorker {
    local_peer_id: PeerId,
    swarm: Swarm<BitmessageNetBehaviour>,
    handler: Handler,
    command_receiver: mpsc::Receiver<WorkerCommand>,
    pending_commands: Vec<WorkerCommand>,
    sqlite_connection_pool: Pool<ConnectionManager<SqliteConnection>>,
    common_topic: Sha256Topic,
    inventory_repo: Box<InventoryRepositorySync>,
}

impl NodeWorker {
    pub fn new(
        bootstrap_nodes: Option<Vec<Multiaddr>>,
    ) -> (NodeWorker, mpsc::Sender<WorkerCommand>) {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Local peer id: {local_peer_id:?}");

        let transport = tcp::async_io::Transport::default()
            .upgrade(Version::V1Lazy)
            .authenticate(noise::Config::new(&local_key).unwrap())
            .multiplex(yamux::Config::default())
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
                mdns: mdns::async_io::Behaviour::new(mdns::Config::default(), local_peer_id)
                    .unwrap(),
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

        let dirs = ProjectDirs::from("", "", "bitmessage-rs").unwrap();
        let data_dir = dirs.data_dir();
        let data_dir_buf = data_dir.join("db");
        fs::create_dir_all(&data_dir_buf).expect("db folder is created");
        let db_url = data_dir_buf.join("database.db");

        debug!("{:?}", db_url.to_str().unwrap());

        let pool = Pool::builder()
            .max_size(16)
            .connection_customizer(Box::new(DbConnectionOpts {
                enable_wal: true,
                enable_foreign_keys: true,
                busy_timeout: Some(Duration::from_secs(30)),
            }))
            .build(ConnectionManager::<SqliteConnection>::new(
                db_url.into_os_string().into_string().unwrap(),
            ))
            .unwrap();

        let conn = &mut pool.get().unwrap();
        conn.run_pending_migrations(MIGRATIONS)
            .expect("migrations won't fail");
        let topic = Sha256Topic::new(COMMON_PUBSUB_TOPIC);
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .expect("subscription not to fail");

        let (sender, receiver) = mpsc::channel(0);
        let inventory_repo = Box::new(SqliteInventoryRepository::new(pool.clone()));
        (
            Self {
                local_peer_id,
                swarm,
                handler: Handler::new(
                    Box::new(SqliteAddressRepository::new(pool.clone())),
                    inventory_repo.clone(),
                    Box::new(SqliteMessageRepository::new(pool.clone())),
                    sender.clone(),
                ),
                command_receiver: receiver,
                pending_commands: Vec::new(),
                sqlite_connection_pool: pool,
                common_topic: topic,
                inventory_repo: inventory_repo.clone(),
            },
            sender,
        )
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<
            BitmessageBehaviourEvent,
            Either<
                Either<
                    Either<Either<void::Void, io::Error>, io::Error>,
                    ConnectionHandlerUpgrErr<io::Error>,
                >,
                void::Void,
            >,
        >,
    ) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {address:?}");
                let indexes: Vec<usize> = self
                    .pending_commands
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| match v {
                        WorkerCommand::GetListenerAddress { sender: _ } => Some(i.clone()),
                        _ => None,
                    })
                    .collect();
                for i in indexes {
                    if let WorkerCommand::GetListenerAddress { sender } =
                        self.pending_commands.remove(i)
                    {
                        sender
                            .send(address.clone())
                            .expect("Receiver not to be dropped");
                    }
                }
            }
            SwarmEvent::Behaviour(BitmessageBehaviourEvent::RequestResponse(
                request_response::Event::Message { message, .. },
            )) => match message {
                request_response::Message::Request {
                    request_id,
                    request,
                    channel,
                } => {
                    debug!("received request {request_id}: {:?}", request);
                    let msg = self.handler.handle_message(request.0).await.unwrap();
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
            SwarmEvent::Behaviour(BitmessageBehaviourEvent::Identify(e)) => {
                self.handle_identify_event(e)
            }
            SwarmEvent::Behaviour(BitmessageBehaviourEvent::Mdns(mdns::Event::Discovered(
                list,
            ))) => {
                for (peer_id, multiaddr) in list {
                    log::debug!("Found new peer via mDNS: {:?}/{:?}", multiaddr, peer_id);
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, multiaddr);
                }
            }
            _ => {}
        }
    }

    async fn handle_command(&mut self, command: WorkerCommand) {
        match command {
            WorkerCommand::StartListening { multiaddr, sender } => {
                debug!("Starting listening to the network...");
                match self.swarm.listen_on(multiaddr.clone()) {
                    Ok(_) => sender.send(Ok(())).expect("Receiver not to be dropped"),
                    Err(e) => sender
                        .send(Err(Box::new(e)))
                        .expect("Receiver not to be dropped"),
                };
            }
            WorkerCommand::Dial { peer, sender } => todo!(),
            WorkerCommand::GetListenerAddress { sender } => match self.swarm.listeners().next() {
                Some(v) => {
                    sender.send(v.clone()).expect("Receiver not to be dropped");
                }
                None => {
                    self.pending_commands
                        .push(WorkerCommand::GetListenerAddress { sender });
                }
            },
            WorkerCommand::GetPeerID { sender } => sender
                .send(self.local_peer_id)
                .expect("Receiver not to be dropped"),
            WorkerCommand::BroadcastMsgByPubSub { sender, msg } => match self.publish_pubsub(msg) {
                Ok(_) => sender.send(Ok(())).expect("receiver not to be dropped"),
                Err(e) => sender
                    .send(Err(Box::new(e)))
                    .expect("receiver not to be dropped"),
            },
            WorkerCommand::NonceCalculated { obj } => {
                self.inventory_repo
                    .store_object(bs58::encode(&obj.hash).into_string(), obj)
                    .await
                    .expect("repo not to fail");

                let inventory = self.inventory_repo.get().await.expect("repo not to fail");
                let msg = messages::NetworkMessage {
                    command: MessageCommand::Inv,
                    payload: MessagePayload::Inv { inventory },
                };
                self.publish_pubsub(msg)
                    .expect("pubsub publish not to fail");
            }
        };
    }

    fn publish_pubsub(&mut self, msg: messages::NetworkMessage) -> Result<MessageId, PublishError> {
        let serialized_msg = serde_cbor::to_vec(&msg).unwrap();
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.common_topic.clone(), serialized_msg)
    }

    pub async fn run(mut self) {
        log::debug!("node worker event loop started");
        loop {
            select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                command = self.command_receiver.next() => match command {
                    Some(c) => self.handle_command(c).await,
                    // Command channel closed, thus shutting down the network event loop.
                    None => {
                        log::debug!("Shutting down network event loop...");
                        return;
                    },
               }
            }
        }
    }

    /// When we receive IdentityInfo, if the peer supports our Kademlia protocol, we add
    /// their listen addresses to the DHT, so they will be propagated to other peers.
    fn handle_identify_event(&mut self, identify_event: identify::Event) {
        log::debug!("Received identify::Event: {:?}", identify_event);

        if let identify::Event::Received {
            peer_id,
            info:
                identify::Info {
                    listen_addrs,
                    protocols,
                    ..
                },
        } = identify_event
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
