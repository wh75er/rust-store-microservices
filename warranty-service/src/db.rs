use crate::model::Warranty;
use crate::schema::warranty;
use crate::WarrantyDatabase;
use diesel::prelude::*;
use std::result::Result;
use uuid;

pub struct MainDbOps;

pub trait DbOps {
    fn insert(
        &self,
        w: &Warranty,
        conn: &WarrantyDatabase,
    ) -> Result<Vec<Warranty>, diesel::result::Error>;
    fn load(&self, conn: &WarrantyDatabase) -> Result<Vec<Warranty>, diesel::result::Error>;
    fn load_id(
        &self,
        uid: uuid::Uuid,
        conn: &WarrantyDatabase,
    ) -> Result<Vec<Warranty>, diesel::result::Error>;
    fn update(
        &self,
        id: uuid::Uuid,
        status: &str,
        conn: &WarrantyDatabase,
    ) -> Result<Warranty, diesel::result::Error>;
    fn delete(
        &self,
        id: uuid::Uuid,
        conn: &WarrantyDatabase,
    ) -> Result<usize, diesel::result::Error>;
}

impl DbOps for MainDbOps {
    fn insert(
        &self,
        w: &Warranty,
        conn: &WarrantyDatabase,
    ) -> Result<Vec<Warranty>, diesel::result::Error> {
        diesel::insert_into(warranty::table)
            .values((
                warranty::comment.eq(&w.comment),
                warranty::item_uid.eq(&w.item_uid),
                warranty::status.eq(&w.status),
                warranty::warranty_date.eq(&w.warranty_date),
            ))
            .get_results(&**conn)
    }

    fn load(&self, conn: &WarrantyDatabase) -> Result<Vec<Warranty>, diesel::result::Error> {
        warranty::table.load::<Warranty>(&**conn)
    }

    fn load_id(
        &self,
        uid: uuid::Uuid,
        conn: &WarrantyDatabase,
    ) -> Result<Vec<Warranty>, diesel::result::Error> {
        warranty::table
            .filter(warranty::item_uid.eq(uid))
            .load::<Warranty>(&**conn)
    }

    fn update(
        &self,
        uid: uuid::Uuid,
        status: &str,
        conn: &WarrantyDatabase,
    ) -> Result<Warranty, diesel::result::Error> {
        diesel::update(warranty::table.filter(warranty::item_uid.eq(uid)))
            .set(warranty::status.eq(status.to_string()))
            .get_result(&**conn)
    }

    fn delete(
        &self,
        uid: uuid::Uuid,
        conn: &WarrantyDatabase,
    ) -> Result<usize, diesel::result::Error> {
        diesel::delete(warranty::table.filter(warranty::item_uid.eq(uid))).execute(&**conn)
    }
}
