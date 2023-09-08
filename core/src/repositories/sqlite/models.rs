use chrono::{DateTime, NaiveDateTime, Utc};
use strum::{Display, EnumString};

#[derive(sqlx::FromRow, Debug, PartialEq)]
pub(crate) struct Address {
    pub address: String,
    pub tag: String,
    pub public_encryption_key: Option<Vec<u8>>,
    pub public_signing_key: Option<Vec<u8>>,
    pub private_signing_key: Option<Vec<u8>>,
    pub private_encryption_key: Option<Vec<u8>>,
    pub label: Option<String>,
}

#[derive(sqlx::FromRow, Debug, PartialEq)]
pub(crate) struct Object {
    pub hash: String,
    pub object_type: i32,
    pub nonce: Vec<u8>,
    pub data: Vec<u8>,
    pub expires: DateTime<Utc>,
    pub signature: Vec<u8>,
}

#[derive(EnumString, Display)]
pub enum MessageStatus {
    WaitingForPubkey,
    WaitingForPOW,
    Sent,
    Received,
    Unknown,
}

#[derive(sqlx::FromRow, Debug, PartialEq, Clone)]
pub struct Message {
    pub hash: String,
    pub sender: String,
    pub recipient: String,
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub status: String,
    pub signature: Vec<u8>,
}
