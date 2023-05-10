use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub type InventoryVector = Vec<String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum ObjectKind {
    Msg { encrypted: Vec<u8> },
    Broadcast { tag: Vec<u8>, encrypted: Vec<u8> },
    Getpubkey { tag: Vec<u8> },
    Pubkey { tag: Vec<u8>, encrypted: Vec<u8> },
}

impl ObjectKind {
    pub fn object_type(&self) -> u8 {
        match self {
            ObjectKind::Msg { encrypted } => 0,
            ObjectKind::Broadcast { tag, encrypted } => 1,
            ObjectKind::Getpubkey { tag } => 2,
            ObjectKind::Pubkey { tag, encrypted } => 3,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Object {
    pub hash: Vec<u8>,
    pub nonce: u64,
    pub kind: ObjectKind,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum MessagePayload {
    GetData { inventory: InventoryVector },
    Inv { inventory: InventoryVector },
    Objects(Vec<Object>),
    None,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageCommand {
    GetData,
    Inv,
    ReqInv,
    Objects,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkMessage {
    pub command: MessageCommand,
    pub payload: MessagePayload,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
#[repr(u8)]
pub enum MsgEncoding {
    Ignore = 0,
    Trivial = 1,
    Simple = 2,
    Extended = 3,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnencryptedMsg {
    pub behavior_bitfield: u32,
    pub public_signing_key: Vec<u8>,
    pub public_encryption_key: Vec<u8>,
    pub sender_ripe: String,
    pub destination_ripe: String,
    pub encoding: MsgEncoding,
    pub message: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnencryptedPubkey {
    pub behaviour_bitfield: u32,
    pub public_signing_key: Vec<u8>,
    pub public_encryption_key: Vec<u8>,
    pub nonce_trials_per_byte: u64,
    pub extra_bytes: u64,
    pub signature: Vec<u8>,
}
