use crate::{
    network::messages::{MessageCommand, MessagePayload, NetworkMessage, Object},
    repositories::{address::AddressRepository, inventory::InventoryRepository},
};

pub struct Handler {
    address_repo: Box<dyn AddressRepository + Send + Sync>,
    inventory_repo: Box<dyn InventoryRepository + Send + Sync>,
}

impl Handler {
    pub fn new(
        address_repo: Box<dyn AddressRepository + Send + Sync>,
        inventory_repo: Box<dyn InventoryRepository + Send + Sync>,
    ) -> Handler {
        Handler {
            address_repo,
            inventory_repo,
        }
    }

    pub async fn handle_message(&self, msg: NetworkMessage) -> Option<NetworkMessage> {
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
        let inv = if let MessagePayload::Inv { inventory } = payload {
            inventory
        } else {
            Vec::new()
        };
        let missing_objects = self
            .inventory_repo
            .get_missing_objects(inv)
            .await
            .expect("Repo not to fail");
        if !missing_objects.is_empty() {
            return Some(NetworkMessage {
                command: MessageCommand::GetData,
                payload: MessagePayload::GetData {
                    inventory: missing_objects,
                },
            });
        }
        None
    }

    async fn handle_objects(&self, payload: MessagePayload) {
        let objects: Vec<Object> = if let MessagePayload::Objects(obj) = payload {
            obj
        } else {
            log::warn!("incorrent payload passed to handle_object function");
            return;
        };

        for obj in objects {
            match obj {
                Object::Msg { encrypted } => todo!(),
                Object::Broadcast { tag, encrypted } => todo!(),
                Object::Getpubkey { tag } => todo!(),
                Object::Pubkey { tag, encrypted } => todo!(),
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
