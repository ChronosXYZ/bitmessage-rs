use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Debug)]
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
    GetData { inventory: Vec<Vec<u8>> },
    Inv { inventory: Vec<Vec<u8>> },
    Object(Object),
    None,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageCommand {
    GetData,
    Inv,
    ReqInv,
    Object,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkMessage {
    pub command: MessageCommand,
    pub payload: MessagePayload,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum MsgEncoding {
    Ignore = 0,
    Trivial = 1,
    Simple = 2,
    Extended = 3,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnencryptedMsg {
    behavior_bitfield: u32,
    public_signing_key: Vec<u8>,
    public_encryption_key: Vec<u8>,
    destination_ripe: Vec<u8>,
    encoding: MsgEncoding,
    message: Vec<u8>,
    signature: Vec<u8>,
}
