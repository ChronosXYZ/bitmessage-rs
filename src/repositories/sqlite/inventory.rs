use diesel::{
    r2d2::{ConnectionManager, Pool},
    RunQueryDsl,
};
use std::error::Error;

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl, SqliteConnection};

use crate::{
    network::messages::{Object, ObjectKind},
    repositories::inventory::InventoryRepository,
};

use super::{
    models,
    schema::{self, inventory::dsl},
};

pub struct SqliteInventoryRepository {
    connection_pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl SqliteInventoryRepository {
    pub fn new(conn_pool: Pool<ConnectionManager<SqliteConnection>>) -> SqliteInventoryRepository {
        SqliteInventoryRepository {
            connection_pool: conn_pool,
        }
    }
}

#[async_trait]
impl InventoryRepository for SqliteInventoryRepository {
    async fn get(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let results = dsl::inventory
            .filter(schema::inventory::expires.le(Utc::now().naive_utc()))
            .select(schema::inventory::hash)
            .load::<String>(&mut conn)?;
        Ok(results)
    }

    async fn get_object(&self, hash: String) -> Result<Option<Object>, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let obj: Vec<models::Object> = dsl::inventory
            .filter(schema::inventory::hash.eq(&hash))
            .load::<models::Object>(&mut conn)?;
        if obj.is_empty() {
            return Ok(None);
        }

        let model = &obj[0];
        let kind: ObjectKind =
            serde_cbor::from_slice(&model.data).expect("data not to be malformed");
        let nonce = u64::from_le_bytes(model.nonce.clone().try_into().unwrap());

        Ok(Some(Object {
            hash: bs58::decode(&hash).into_vec().unwrap(),
            nonce,
            expires: model.expires.timestamp(),
            kind,
            signature: model.signature.clone(),
        }))
    }

    /// Filter inventory vector with missing objects
    async fn get_missing_objects(&self, hashes: &mut Vec<String>) -> Result<(), Box<dyn Error>> {
        // delete items from the vector if they are in database
        let mut conn = self.connection_pool.get().unwrap();
        hashes.retain(|h| {
            let res: Vec<models::Object> = dsl::inventory
                .filter(schema::inventory::hash.eq(h))
                .load::<models::Object>(&mut conn)
                .expect("repo not to fail");
            if res.is_empty() {
                true
            } else {
                false
            }
        });
        Ok(())
    }

    /// Store received object
    async fn store_object(&mut self, hash: String, o: Object) -> Result<(), Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();

        let data = serde_cbor::to_vec(&o.kind).expect("data not to be malformed");

        let model = models::Object {
            hash,
            nonce: o.nonce.to_le_bytes().to_vec(),
            object_type: o.kind.object_type() as i32,
            data,
            expires: NaiveDateTime::from_timestamp_opt(o.expires, 0).unwrap(),
            signature: o.signature,
        };
        diesel::insert_into(schema::inventory::table)
            .values(&model)
            .execute(&mut conn)?;
        Ok(())
    }

    /// Cleanup the storage of expired items
    async fn cleanup(&mut self) -> Result<usize, Box<dyn Error>> {
        let mut conn = self.connection_pool.get().unwrap();
        let updated_count: usize = diesel::delete(
            dsl::inventory.filter(schema::inventory::expires.le(Utc::now().naive_utc())),
        )
        .execute(&mut conn)?;

        Ok(updated_count)
    }
}
