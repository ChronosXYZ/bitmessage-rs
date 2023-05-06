use std::{collections::HashMap, error::Error};

use async_trait::async_trait;
use libp2p::PeerId;

use crate::{network::address::Address, repositories::address::AddressRepository};

pub struct MemoryAddressRepository {
    addresses: HashMap<PeerId, Address>,
}

impl MemoryAddressRepository {
    pub fn new() -> MemoryAddressRepository {
        MemoryAddressRepository {
            addresses: HashMap::new(),
        }
    }
}

#[async_trait]
impl AddressRepository for MemoryAddressRepository {
    async fn get(&self, hash: PeerId) -> Result<Option<Address>, Box<dyn Error>> {
        let address = self.addresses.get(&hash);
        Ok(address.cloned())
    }

    async fn store(&mut self, a: Address) -> Result<(), Box<dyn Error>> {
        self.addresses.insert(a.hash, a);
        Ok(())
    }

    async fn get_contacts(&self) -> Result<Vec<Address>, Box<dyn Error>> {
        let contacts: Vec<Address> = self
            .addresses
            .values()
            .filter(|&v| match &v.public_key {
                Some(_) => true,
                None => false,
            })
            .cloned()
            .collect();
        Ok(contacts)
    }

    async fn get_identities(&self) -> Result<Vec<Address>, Box<dyn Error>> {
        let identities: Vec<Address> = self
            .addresses
            .values()
            .filter(|&v| match &v.keypair {
                Some(_) => true,
                None => false,
            })
            .cloned()
            .collect();
        Ok(identities)
    }
}
