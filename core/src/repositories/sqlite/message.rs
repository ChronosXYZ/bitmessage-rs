use std::error::Error;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

use crate::{network::messages::UnencryptedMsg, repositories::message::MessageRepository};

use super::models::{self, MessageStatus};

#[derive(Clone)]
pub struct SqliteMessageRepository {
    pool: SqlitePool,
}

impl SqliteMessageRepository {
    pub fn new(conn_pool: SqlitePool) -> Self {
        SqliteMessageRepository { pool: conn_pool }
    }
}

#[async_trait]
impl MessageRepository for SqliteMessageRepository {
    /// Save message in repository
    async fn save(
        &mut self,
        hash: String,
        msg: UnencryptedMsg,
        signature: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        let model = models::Message {
            hash,
            sender: msg.sender_ripe,
            recipient: msg.destination_ripe,
            data: msg.message,
            created_at: Utc::now(),
            status: MessageStatus::Received.to_string(),
            signature,
        };

        sqlx::query("INSERT INTO messages (hash, sender, recipient, data, created_at, status, signature) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .bind(model.hash)
            .bind(model.sender)
            .bind(model.recipient)
            .bind(model.data)
            .bind(model.created_at)
            .bind(model.status)
            .bind(model.signature)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get all messages in repository
    async fn get_messages(&self) -> Result<Vec<models::Message>, Box<dyn Error>> {
        let results = sqlx::query_as("SELECT * FROM messages")
            .fetch_all(&self.pool)
            .await?;
        Ok(results)
    }

    async fn get_messages_by_recipient(
        &self,
        address: String,
    ) -> Result<Vec<models::Message>, Box<dyn Error>> {
        let results = sqlx::query_as("SELECT * FROM messages WHERE recipient = ?")
            .bind(address)
            .fetch_all(&self.pool)
            .await?;
        Ok(results)
    }

    async fn get_messages_by_sender(
        &self,
        address: String,
    ) -> Result<Vec<models::Message>, Box<dyn Error>> {
        let results = sqlx::query_as("SELECT * FROM messages WHERE sender = ?")
            .bind(address)
            .fetch_all(&self.pool)
            .await?;
        Ok(results)
    }

    async fn save_model(&mut self, model: models::Message) -> Result<(), Box<dyn Error>> {
        sqlx::query("INSERT INTO messages (hash, sender, recipient, data, created_at, status, signature) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .bind(model.hash)
            .bind(model.sender)
            .bind(model.recipient)
            .bind(model.data)
            .bind(model.created_at)
            .bind(model.status)
            .bind(model.signature)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_message_status(
        &mut self,
        hash: String,
        status: MessageStatus,
    ) -> Result<(), Box<dyn Error>> {
        sqlx::query("UPDATE messages SET status = ? WHERE hash = ?")
            .bind(status.to_string())
            .bind(hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_messages_by_status(
        &self,
        status: MessageStatus,
    ) -> Result<Vec<models::Message>, Box<dyn Error>> {
        let results = sqlx::query_as("SELECT * FROM messages WHERE status = ?")
            .bind(status.to_string())
            .fetch_all(&self.pool)
            .await?;
        Ok(results)
    }

    async fn remove_message(&mut self, hash: String) -> Result<(), Box<dyn Error>> {
        sqlx::query("DELETE FROM messages WHERE hash = ?")
            .bind(hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_hash(
        &mut self,
        old_hash: String,
        new_hash: String,
    ) -> Result<(), Box<dyn Error>> {
        sqlx::query("UPDATE messages SET hash = ? WHERE hash = ?")
            .bind(new_hash)
            .bind(old_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
