use std::error::Error;

use async_trait::async_trait;
use dyn_clone::{clone_trait_object, DynClone};

use crate::network::messages::UnencryptedMsg;

use super::sqlite::models::{self, MessageStatus};

#[async_trait]
pub trait MessageRepository: DynClone {
    /// Save message in repository
    async fn save(
        &mut self,
        hash: String,
        msg: UnencryptedMsg,
        signature: Vec<u8>,
    ) -> Result<(), Box<dyn Error>>;

    async fn save_model(&mut self, model: models::Message) -> Result<(), Box<dyn Error>>;

    /// Get all messages in repository
    async fn get_messages(&self) -> Result<Vec<models::Message>, Box<dyn Error>>;

    async fn get_messages_by_recipient(
        &self,
        address: String,
    ) -> Result<Vec<models::Message>, Box<dyn Error>>;

    async fn get_messages_by_sender(
        &self,
        address: String,
    ) -> Result<Vec<models::Message>, Box<dyn Error>>;

    async fn update_message_status(
        &mut self,
        hash: String,
        status: MessageStatus,
    ) -> Result<(), Box<dyn Error>>;

    /// Update hash of message when inventory object is created
    async fn update_hash(
        &mut self,
        old_hash: String,
        new_hash: String,
    ) -> Result<(), Box<dyn Error>>;

    async fn get_messages_by_status(
        &self,
        status: MessageStatus,
    ) -> Result<Vec<models::Message>, Box<dyn Error>>;

    async fn remove_message(&mut self, hash: String) -> Result<(), Box<dyn Error>>;
}

clone_trait_object!(MessageRepository);

pub type MessageRepositorySync = dyn MessageRepository + Sync + Send;
