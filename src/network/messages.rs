use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub type InventoryVector = Vec<Vec<u8>>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Object {
    Msg { encrypted: Vec<u8> },
    Broadcast { tag: Vec<u8>, encrypted: Vec<u8> },
    Getpubkey { tag: Vec<u8> },
    Pubkey { tag: Vec<u8>, encrypted: Vec<u8> },
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
    behavior_bitfield: u32,
    public_signing_key: Vec<u8>,
    public_encryption_key: Vec<u8>,
    destination_ripe: Vec<u8>,
    encoding: MsgEncoding,
    message: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnencryptedPubkey {
    behaviour_bitfield: u32,
    public_signing_key: Vec<u8>,
    public_encryption_key: Vec<u8>,
    nonce_trials_per_byte: u64,
    extra_bytes: u64,
    signature: Vec<u8>,
}
