use log::warn;

use crate::{
    network::messages::{MessageCommand, MessagePayload, NetworkMessage, Object, ObjectKind},
    pow,
    repositories::{
        address::AddressRepositorySync, inventory::InventoryRepositorySync,
        message::MessageRepositorySync,
    },
};

pub struct Handler {
    address_repo: Box<AddressRepositorySync>,
    inventory_repo: Box<InventoryRepositorySync>,
    message_repo: Box<MessageRepositorySync>,
}

impl Handler {
    pub fn new(
        address_repo: Box<AddressRepositorySync>,
        inventory_repo: Box<InventoryRepositorySync>,
        message_repo: Box<MessageRepositorySync>,
    ) -> Handler {
        Handler {
            address_repo,
            inventory_repo,
            message_repo,
        }
    }

    pub async fn handle_message(&mut self, msg: NetworkMessage) -> Option<NetworkMessage> {
        match msg.command {
            MessageCommand::GetData => Some(self.handle_get_data(msg.payload).await),
            MessageCommand::Inv => self.handle_inv(msg.payload).await,
            MessageCommand::ReqInv => Some(self.handle_get_inv_message(msg.payload).await),
            MessageCommand::Objects => {
                self.handle_objects(msg.payload).await;
                None
            }
        }
    }

    async fn handle_get_inv_message(&self, _: MessagePayload) -> NetworkMessage {
        let inv = self
            .inventory_repo
            .get()
            .await
            .expect("Inventory repo not to fail");
        NetworkMessage {
            command: MessageCommand::Inv,
            payload: MessagePayload::Inv { inventory: inv },
        }
    }

    async fn handle_inv(&self, payload: MessagePayload) -> Option<NetworkMessage> {
        let mut inv = if let MessagePayload::Inv { inventory } = payload {
            inventory
        } else {
            Vec::new()
        };
        self.inventory_repo
            .get_missing_objects(&mut inv)
            .await
            .expect("Repo not to fail");
        if !inv.is_empty() {
            return Some(NetworkMessage {
                command: MessageCommand::GetData,
                payload: MessagePayload::GetData { inventory: inv },
            });
        }
        None
    }

    async fn handle_objects(&mut self, payload: MessagePayload) {
        let objects: Vec<Object> = if let MessagePayload::Objects(obj) = payload {
            obj
        } else {
            log::warn!("incorrent payload passed to handle_object function");
            return;
        };

        'outer: for obj in objects {
            let target = pow::get_pow_target(
                &obj,
                pow::NETWORK_MIN_NONCE_TRIALS_PER_BYTE,
                pow::NETWORK_MIN_EXTRA_BYTES,
            );
            let pow_check_res = pow::check_pow(target, obj.nonce.clone(), obj.hash.clone());
            if pow_check_res.is_err() {
                log::warn!(
                    "object with hash {:?} has invalid nonce! skipping it",
                    bs58::encode(obj.hash).into_string()
                );
                continue;
            }

            match obj.kind {
                ObjectKind::Msg { encrypted } => {
                    let identities = self
                        .address_repo
                        .get_identities()
                        .await
                        .expect("Address repo not to fail");
                    for i in identities {
                        let decryption_result = ecies::decrypt(
                            &i.private_encryption_key.unwrap().serialize(),
                            &encrypted,
                        );
                        if let Ok(msg) = decryption_result {
                            log::debug!("message object successfully decrypted! saving it...");
                            match serde_cbor::from_slice(msg.as_slice()) {
                                Ok(msg) => {
                                    self.message_repo
                                        .save(bs58::encode(&obj.hash).into_string(), msg)
                                        .await
                                        .expect("repo not to fail");
                                }
                                Err(e) => {
                                    log::error!("received malformed message! skipping it");
                                    continue 'outer;
                                }
                            }
                        }
                    }
                }
                ObjectKind::Broadcast { tag, encrypted } => {
                    log::warn!("we don't support broadcast msgs at the moment");
                    continue 'outer;
                }
                ObjectKind::Getpubkey { tag } => {
                    let identities = self
                        .address_repo
                        .get_identities()
                        .await
                        .expect("repo not to fail");
                    for i in identities {
                        if i.tag == tag {
                            log::debug!("someone requested our pubkey! sending it out...");
                            // TODO send out our pubkey
                        }
                    }
                }
                ObjectKind::Pubkey { tag, encrypted } => todo!(),
            }
        }
    }

    async fn handle_get_data(&self, payload: MessagePayload) -> NetworkMessage {
        let inv = if let MessagePayload::GetData { inventory } = payload {
            inventory
        } else {
            Vec::new()
        };

        let mut objects: Vec<Object> = Vec::new();

        for hash in inv {
            if let Some(obj) = self
                .inventory_repo
                .get_object(hash)
                .await
                .expect("Repository not to fail")
            {
                objects.push(obj);
            }
        }

        NetworkMessage {
            command: MessageCommand::Objects,
            payload: MessagePayload::Objects(objects),
        }
    }
}
