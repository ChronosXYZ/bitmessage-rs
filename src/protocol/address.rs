use hmac_sha512::Hash;
use libp2p::{
    identity::{Keypair, PublicKey},
    PeerId,
};
use log::error;

struct Address {
    hash: PeerId,
    tag: Vec<u8>,
    public_decryption_key: Keypair,
    public_key: Option<PublicKey>,
}

impl Address {
    fn new(pid: PeerId) -> Self {
        let mut checksum = Hash::hash(Hash::hash(pid.to_base58()));
        let public_decryption_key = Keypair::ed25519_from_bytes(&mut checksum[0..32]).unwrap();
        let tag = checksum[32..].to_vec();
        Address {
            hash: pid,
            tag,
            public_decryption_key,
            public_key: Option::None,
        }
    }

    fn new_with_public_key(pid: PeerId, public_key: PublicKey) -> Self {
        let mut address = Self::new(pid);
        if let Some(res) = pid.is_public_key(&public_key) {
            if res {
                address.public_key = Option::Some(public_key);
            } else {
                error!("public key doesn't match with actual address");
            }
        }
        return address;
    }
}
