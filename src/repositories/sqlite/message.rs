use std::error::Error;

use async_trait::async_trait;
use chrono::Utc;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection,
};

use crate::{network::messages::UnencryptedMsg, repositories::message::MessageRepository};

use super::{
    models,
    schema::{self, messages::dsl},
};

#[derive(Clone)]
pub struct SqliteMessageRepository {
    connection_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl SqliteMessageRepository {
    pub fn new(conn_pool: Pool<ConnectionManager<SqliteConnection>>) -> Self {
        SqliteMessageRepository {
            connection_pool: conn_pool,
        }
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
        let mut conn = self.connection_pool.get().unwrap();

        let model = models::Message {
            hash,
            sender: msg.sender_ripe,
            recipient: msg.destination_ripe,
            data: msg.message,
            created_at: Utc::now().naive_utc(),
            status: "unknown".to_string(), // FIXME
            signature,
        };
        diesel::insert_into(schema::messages::table)
            .values(&model)
            .execute(&mut conn)?;
        Ok(())
    }

    /// Get all messages in repository
    async fn get_messages(&self) -> Result<Vec<models::Message>, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let results = dsl::messages.load::<models::Message>(&mut conn)?;
        Ok(results)
    }

    async fn get_messages_by_recipient(
        &self,
        address: String,
    ) -> Result<Vec<models::Message>, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let results = dsl::messages
            .filter(schema::messages::recipient.eq(address))
            .load::<models::Message>(&mut conn)?;
        Ok(results)
    }

    async fn get_messages_by_sender(
        &self,
        address: String,
    ) -> Result<Vec<models::Message>, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let results = dsl::messages
            .filter(schema::messages::sender.eq(address))
            .load::<models::Message>(&mut conn)?;
        Ok(results)
    }

    async fn save_model(&mut self, model: models::Message) -> Result<(), Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        diesel::insert_into(schema::messages::table)
            .values(&model)
            .execute(&mut conn)?;
        Ok(())
    }
}
