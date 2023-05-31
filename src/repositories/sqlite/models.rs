use super::schema::{addresses, inventory, messages};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use strum::{Display, EnumString};

#[derive(Queryable, Insertable, Debug, PartialEq)]
#[diesel(table_name = addresses)]
pub(crate) struct Address {
    pub address: String,
    pub tag: String,
    pub public_encryption_key: Option<Vec<u8>>,
    pub public_signing_key: Option<Vec<u8>>,
    pub private_signing_key: Option<Vec<u8>>,
    pub private_encryption_key: Option<Vec<u8>>,
    pub label: Option<String>,
}

#[derive(Queryable, Insertable, Debug, PartialEq)]
#[diesel(table_name = inventory)]
pub(crate) struct Object {
    pub hash: String,
    pub object_type: i32,
    pub nonce: Vec<u8>,
    pub data: Vec<u8>,
    pub expires: NaiveDateTime,
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

#[derive(Queryable, Insertable, Debug, PartialEq, Clone, AsChangeset)]
#[diesel(table_name = messages)]
pub struct Message {
    pub hash: String,
    pub sender: String,
    pub recipient: String,
    pub data: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub status: String,
    pub signature: Vec<u8>,
}
