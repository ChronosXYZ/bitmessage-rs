use std::error::Error;

use async_trait::async_trait;

use crate::{network::address::Address, repositories::address::AddressRepository};

pub struct MemoryAddressRepository {}

impl MemoryAddressRepository {
    pub fn new() -> MemoryAddressRepository {
        MemoryAddressRepository {}
    }
}

#[async_trait]
impl AddressRepository for MemoryAddressRepository {
    async fn get(&self, hash: Vec<u8>) -> Result<Option<Address>, Box<dyn Error>> {
        todo!()
    }

    async fn store(&mut self, a: crate::network::address::Address) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    async fn get_contacts(&self) -> Result<Vec<Address>, Box<dyn Error>> {
        todo!()
    }

    async fn get_identities(&self) -> Result<Vec<Address>, Box<dyn Error>> {
        todo!()
    }
}
