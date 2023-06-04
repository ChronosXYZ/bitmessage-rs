use async_std::task;
use chrono::Utc;
use diesel::{
    connection::SimpleConnection,
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use rand::distributions::{Alphanumeric, DistString};
use std::{borrow::Cow, collections::HashMap, error::Error, fs, io, iter, time::Duration};

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
use serde::Serialize;

use crate::{
    network::{
        address::Address,
        behaviour::{
            BitmessageBehaviourEvent, BitmessageNetBehaviour, BitmessageProtocol,
            BitmessageProtocolCodec, BitmessageResponse,
        },
        messages::{
            self, MessageCommand, MessagePayload, MsgEncoding, Object, ObjectKind, UnencryptedMsg,
        },
    },
    repositories::{
        address::AddressRepositorySync,
        inventory::InventoryRepositorySync,
        message::MessageRepositorySync,
        sqlite::{
            address::SqliteAddressRepository,
            inventory::SqliteInventoryRepository,
            message::SqliteMessageRepository,
            models::{self, MessageStatus},
        },
    },
};

use super::handler::Handler;

const IDENTIFY_PROTO_NAME: &str = "/bitmessage/id/1.0.0";
const KADEMLIA_PROTO_NAME: &[u8] = b"/bitmessage/kad/1.0.0";

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/repositories/sqlite/migrations");
const COMMON_PUBSUB_TOPIC: &'static str = "common";

#[derive(Debug)]
pub enum Folder {
    Inbox,
    Sent,
}

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

type DynError = Box<dyn Error + Send + Sync>;

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
    GetOwnIdentities {
        sender: oneshot::Sender<Result<Vec<Address>, DynError>>,
    },
    GenerateIdentity {
        label: String,
        sender: oneshot::Sender<Result<String, DynError>>,
    },
    RenameIdentity {
        new_label: String,
        address: String,
        sender: oneshot::Sender<Result<(), DynError>>,
    },
    DeleteIdentity {
        address: String,
        sender: oneshot::Sender<Result<(), DynError>>,
    },
    GetMessages {
        address: String,
        folder: Folder,
        sender: oneshot::Sender<Result<Vec<models::Message>, DynError>>,
    },
    SendMessage {
        msg: models::Message,
        from: String,
        sender: oneshot::Sender<Result<(), DynError>>,
    },
}

pub struct NodeWorker {
    local_peer_id: PeerId,
    swarm: Swarm<BitmessageNetBehaviour>,
    handler: Handler,
    command_sender: mpsc::Sender<WorkerCommand>,
    command_receiver: mpsc::Receiver<WorkerCommand>,

    pubkey_notifier: mpsc::Receiver<String>,
    tracked_pubkeys: HashMap<String, bool>, // TODO populate it on startup

    pending_commands: Vec<WorkerCommand>,
    _sqlite_connection_pool: Pool<ConnectionManager<SqliteConnection>>,
    common_topic: Sha256Topic,

    inventory_repo: Box<InventoryRepositorySync>,
    address_repo: Box<AddressRepositorySync>,
    messages_repo: Box<MessageRepositorySync>,
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
        let (pubkey_notifier_sink, pubkey_notifier) = mpsc::channel(0);
        let inventory_repo = Box::new(SqliteInventoryRepository::new(pool.clone()));
        let address_repo = Box::new(SqliteAddressRepository::new(pool.clone()));
        let message_repo = Box::new(SqliteMessageRepository::new(pool.clone()));
        (
            Self {
                local_peer_id,
                swarm,
                handler: Handler::new(
                    address_repo.clone(),
                    inventory_repo.clone(),
                    message_repo.clone(),
                    sender.clone(),
                    pubkey_notifier_sink,
                ),
                command_sender: sender.clone(),
                pubkey_notifier,
                tracked_pubkeys: HashMap::new(),
                command_receiver: receiver,
                pending_commands: Vec::new(),
                _sqlite_connection_pool: pool,
                common_topic: topic,

                address_repo: address_repo.clone(),
                inventory_repo: inventory_repo.clone(),
                messages_repo: message_repo.clone(),
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
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
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
                match &obj.kind {
                    ObjectKind::Msg { encrypted: _ } => self
                        .messages_repo
                        .update_message_status(
                            bs58::encode(&obj.hash).into_string(),
                            MessageStatus::Sent,
                        )
                        .await
                        .unwrap(),
                    _ => {}
                }

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
            WorkerCommand::GetOwnIdentities { sender } => {
                let result = self.address_repo.get_identities().await;
                match result {
                    Ok(a) => {
                        sender.send(Ok(a)).expect("receiver not to be dropped");
                    }
                    Err(e) => {
                        sender
                            .send(Err(Box::from(e.to_string())))
                            .expect("receiver not to be dropped");
                        return;
                    }
                }
            }
            WorkerCommand::GenerateIdentity { label, sender } => {
                let mut address = Address::generate();
                address.label = label;
                let res = self.address_repo.store(address.clone()).await;
                match res {
                    Ok(_) => {
                        sender
                            .send(Ok(address.string_repr))
                            .expect("receiver not to be dropped");
                    }
                    Err(e) => sender
                        .send(Err(Box::from(e.to_string())))
                        .expect("receiver not to be dropped"),
                }
            }
            WorkerCommand::RenameIdentity {
                new_label,
                address,
                sender,
            } => match self.address_repo.update_label(address, new_label).await {
                Ok(_) => {
                    sender.send(Ok(())).expect("receiver not to be dropped");
                }
                Err(e) => sender
                    .send(Err(Box::from(e.to_string())))
                    .expect("receiver not to be dropped"),
            },
            WorkerCommand::DeleteIdentity { address, sender } => {
                match self.address_repo.delete_address(address).await {
                    Ok(_) => {
                        sender.send(Ok(())).expect("receiver not to be dropped");
                    }
                    Err(e) => sender
                        .send(Err(Box::from(e.to_string())))
                        .expect("receiver not to be dropped"),
                }
            }
            WorkerCommand::GetMessages {
                address,
                folder,
                sender,
            } => match folder {
                Folder::Inbox => {
                    match self.messages_repo.get_messages_by_recipient(address).await {
                        Ok(v) => sender.send(Ok(v)).expect("receiver not to be dropped"),
                        Err(e) => sender
                            .send(Err(Box::from(e.to_string())))
                            .expect("receiver not to be dropped"),
                    }
                }
                Folder::Sent => match self.messages_repo.get_messages_by_sender(address).await {
                    Ok(v) => sender.send(Ok(v)).expect("receiver not to be dropped"),
                    Err(e) => sender
                        .send(Err(Box::from(e.to_string())))
                        .expect("receiver not to be dropped"),
                },
            },
            WorkerCommand::SendMessage {
                mut msg,
                from,
                sender,
            } => {
                let identity = self
                    .address_repo
                    .get_by_ripe_or_tag(from)
                    .await
                    .unwrap()
                    .unwrap();
                let recipient: Option<Address> = self
                    .address_repo
                    .get_by_ripe_or_tag(msg.recipient.clone())
                    .await
                    .unwrap();
                match recipient {
                    Some(v) => {
                        msg.status = MessageStatus::WaitingForPOW.to_string();
                        let object = create_object_from_msg(&identity, &v, msg.clone());
                        msg.hash = bs58::encode(&object.hash).into_string();
                        self.messages_repo.save_model(msg.clone()).await.unwrap();
                        object.do_proof_of_work(self.command_sender.clone());
                    }
                    None => {
                        msg.status = MessageStatus::WaitingForPubkey.to_string();
                        // we generate random hash value, cuz we don't really know real hash value of the message at the moment, and it's not that important
                        msg.hash = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
                        self.messages_repo.save_model(msg.clone()).await.unwrap();
                        // send getpubkey request
                        let obj = messages::Object::with_signing(
                            &identity,
                            ObjectKind::Getpubkey {
                                tag: Address::new(bs58::decode(msg.recipient).into_vec().unwrap())
                                    .tag,
                            },
                            Utc::now() + chrono::Duration::days(7),
                        );
                        obj.do_proof_of_work(self.command_sender.clone());
                    }
                }
                sender.send(Ok(())).unwrap();
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
                },
                pubkey_notification = self.pubkey_notifier.next() => self.handle_pubkey_notification(pubkey_notification.unwrap()).await,
            }
        }
    }

    async fn handle_pubkey_notification(&mut self, tag: String) {
        if let Some(_) = self.tracked_pubkeys.get(&tag) {
            // TODO seek for messages which haven't been sent yet because of empty public key of recipient
            let addr = self
                .address_repo
                .get_by_ripe_or_tag(tag)
                .await
                .unwrap()
                .expect("Address entity exists in db");
            let msgs = self
                .messages_repo
                .get_messages_by_recipient(addr.string_repr.clone())
                .await
                .unwrap();
            msgs.into_iter()
                .filter(|x| x.status == MessageStatus::WaitingForPubkey.to_string())
                .for_each(|mut x| {
                    let identity =
                        task::block_on(self.address_repo.get_by_ripe_or_tag(x.sender.clone()))
                            .unwrap()
                            .expect("identity exists in address repo");
                    x.status = MessageStatus::WaitingForPOW.to_string();
                    let object = create_object_from_msg(&identity, &addr, x.clone());
                    x.hash = bs58::encode(&object.hash).into_string();
                    task::block_on(self.messages_repo.save_model(x)).unwrap();
                    object.do_proof_of_work(self.command_sender.clone());
                });
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

                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .add_explicit_peer(&peer_id);
            }
        }
    }

    pub fn serialize_and_encrypt_payload<T>(
        object: T,
        secret_key: &libsecp256k1::SecretKey,
    ) -> Vec<u8>
    where
        T: Serialize,
    {
        let encrypted = ecies::encrypt(
            &ecies::PublicKey::from_secret_key(secret_key).serialize(),
            serde_cbor::to_vec(&object).unwrap().as_ref(),
        )
        .unwrap();
        encrypted
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

fn create_object_from_msg(identity: &Address, recipient: &Address, msg: models::Message) -> Object {
    let unenc_msg = UnencryptedMsg {
        behavior_bitfield: 0,
        sender_ripe: msg.sender.clone(),
        destination_ripe: msg.recipient.clone(),
        encoding: MsgEncoding::Simple,
        message: msg.data.clone(),
        public_encryption_key: recipient
            .public_encryption_key
            .unwrap()
            .serialize()
            .to_vec(),
        public_signing_key: identity.public_signing_key.unwrap().serialize().to_vec(),
    };
    let encrypted =
        serialize_and_encrypt_payload_pub(unenc_msg, &recipient.public_encryption_key.unwrap());
    messages::Object::with_signing(
        &identity,
        ObjectKind::Msg { encrypted },
        Utc::now() + chrono::Duration::days(7), // FIXME
    )
}

pub fn serialize_and_encrypt_payload_pub<T>(
    object: T,
    public_key: &libsecp256k1::PublicKey,
) -> Vec<u8>
where
    T: Serialize,
{
    let encrypted = ecies::encrypt(
        &public_key.serialize(),
        serde_cbor::to_vec(&object).unwrap().as_ref(),
    )
    .unwrap();
    encrypted
}
