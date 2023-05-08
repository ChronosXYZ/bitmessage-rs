use std::error::Error;

use async_trait::async_trait;
use diesel::{
    query_dsl::methods::FilterDsl,
    r2d2::{ConnectionManager, Pool},
    BoolExpressionMethods, ExpressionMethods, RunQueryDsl, SqliteConnection,
};
use ecies::{PublicKey, SecretKey};

use crate::{network::address::Address, repositories::address::AddressRepository};

use super::{
    models,
    schema::{self, addresses::dsl},
};

pub struct SqliteAddressRepository {
    connection_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl SqliteAddressRepository {
    pub fn new(conn_poll: Pool<ConnectionManager<SqliteConnection>>) -> SqliteAddressRepository {
        SqliteAddressRepository {
            connection_pool: conn_poll,
        }
    }

    fn serialize(a: Address) -> models::Address {
        let mut pub_sig_key_ser: Option<Vec<u8>> = None;
        let mut priv_sig_key_ser: Option<Vec<u8>> = None;
        let mut pub_enc_key_ser: Option<Vec<u8>> = None;
        let mut priv_enc_key_ser: Option<Vec<u8>> = None;
        if let Some(k) = a.public_signing_key {
            pub_sig_key_ser = Some(k.serialize().to_vec());
        }
        if let Some(k) = a.private_signing_key {
            priv_sig_key_ser = Some(k.serialize().to_vec());
        }
        if let Some(k) = a.public_encryption_key {
            pub_enc_key_ser = Some(k.serialize().to_vec());
        }
        if let Some(k) = a.private_encryption_key {
            priv_enc_key_ser = Some(k.serialize().to_vec());
        }
        models::Address {
            address: a.string_repr,
            tag: bs58::encode(a.tag).into_string(),
            public_signing_key: pub_sig_key_ser,
            private_signing_key: priv_sig_key_ser,
            public_encryption_key: pub_enc_key_ser,
            private_encryption_key: priv_enc_key_ser,
        }
    }

    fn deserialize(m: &models::Address) -> Result<Address, Box<dyn Error>> {
        let mut address = Address::with_string_repr(m.address.clone());
        let mut psk = None;
        let mut ppsk = None;
        let mut pek = None;
        let mut ppek = None;
        if let Some(d) = m.public_signing_key.clone() {
            psk = Some(PublicKey::parse_slice(d.as_slice(), None)?);
        }
        if let Some(d) = m.private_signing_key.clone() {
            ppsk = Some(SecretKey::parse_slice(d.as_slice())?);
        }
        if let Some(d) = m.public_encryption_key.clone() {
            pek = Some(PublicKey::parse_slice(d.as_slice(), None)?);
        }
        if let Some(d) = m.private_encryption_key.clone() {
            ppek = Some(SecretKey::parse_slice(d.as_slice())?);
        }

        address.public_signing_key = psk;
        address.private_signing_key = ppsk;
        address.public_encryption_key = pek;
        address.private_encryption_key = ppek;
        Ok(address)
    }
}

#[async_trait]
impl AddressRepository for SqliteAddressRepository {
    async fn store(&mut self, a: Address) -> Result<(), Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let model = Self::serialize(a);
        diesel::insert_into(schema::addresses::table)
            .values(&model)
            .execute(&mut conn)?;
        Ok(())
    }

    async fn delete_address(&mut self, hash: String) -> Result<(), Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        diesel::delete(dsl::addresses.filter(schema::addresses::address.eq(hash)))
            .execute(&mut conn)?;
        Ok(())
    }

    async fn get_by_ripe_or_tag(&self, hash: String) -> Result<Option<Address>, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let results = dsl::addresses
            .filter(
                schema::addresses::address
                    .eq(&hash)
                    .or(schema::addresses::tag.eq(&hash)),
            )
            .load::<models::Address>(&mut conn)?;
        if results.len() == 0 {
            return Ok(None);
        }

        let res: &models::Address = &results[0];
        let address = Self::deserialize(res)?;
        Ok(Some(address))
    }

    async fn get_contacts(&self) -> Result<Vec<Address>, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let results = schema::addresses::dsl::addresses
            .filter(
                schema::addresses::public_signing_key
                    .is_not_null()
                    .and(schema::addresses::public_encryption_key.is_not_null()),
            )
            .load::<models::Address>(&mut conn)?;
        let mut contacts = vec![];
        for res in results {
            let addr = Self::deserialize(&res)?;
            contacts.push(addr);
        }
        Ok(contacts)
    }

    async fn get_identities(&self) -> Result<Vec<Address>, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let results = schema::addresses::dsl::addresses
            .filter(
                schema::addresses::private_signing_key
                    .is_not_null()
                    .and(schema::addresses::private_encryption_key.is_not_null()),
            )
            .load::<models::Address>(&mut conn)?;
        let mut identities = vec![];
        for res in results {
            let addr = Self::deserialize(&res)?;
            identities.push(addr);
        }
        Ok(identities)
    }

    async fn update_public_keys(
        &mut self,
        ripe: String,
        public_signing_key: PublicKey,
        public_encryption_key: PublicKey,
    ) -> Result<(), Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        diesel::update(dsl::addresses.filter(schema::addresses::address.eq(ripe)))
            .set((
                schema::addresses::public_signing_key
                    .eq(Some(public_signing_key.serialize().to_vec())),
                schema::addresses::public_encryption_key
                    .eq(Some(public_encryption_key.serialize().to_vec())),
            ))
            .execute(&mut conn)?;
        Ok(())
    }
}
