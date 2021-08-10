use crate::model::{Item, OrderItem};
use crate::schema::{items, order_items};
use crate::WarehouseDatabase;
use diesel::prelude::*;
use std::result::Result;
use uuid;

pub struct MainDbOps;

pub trait DbOps {
    fn insert_order(
        &self,
        order_item: &OrderItem,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<OrderItem>, diesel::result::Error>;
    fn load_orders(&self, conn: &WarehouseDatabase) -> Result<Vec<OrderItem>, diesel::result::Error>;

    fn load_order_uid(
        &self,
        order_uid: uuid::Uuid,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<OrderItem>, diesel::result::Error>;

    fn load_order_item_uid(
        &self,
        item_uid: uuid::Uuid,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<OrderItem>, diesel::result::Error>;

    fn load_item(
        &self,
        model: String,
        size: String,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<Item>, diesel::result::Error>;

    fn load_item_id(
        &self,
        id: i32,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<Item>, diesel::result::Error>;

    fn update_order_status(
        &self,
        order_uid: uuid::Uuid,
        canceled: bool,
        conn: &WarehouseDatabase,
    ) -> Result<OrderItem, diesel::result::Error>;

    fn update_item(
        &self,
        item: &Item,
        conn: &WarehouseDatabase,
    ) -> Result<Item, diesel::result::Error>;
}

impl DbOps for MainDbOps {
    fn insert_order(
        &self,
        order_item: &OrderItem,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<OrderItem>, diesel::result::Error> {
        diesel::insert_into(order_items::table)
            .values((
                order_items::canceled.eq(&order_item.canceled),
                order_items::order_item_uid.eq(&order_item.order_item_uid),
                order_items::order_uid.eq(&order_item.order_uid),
                order_items::item_id.eq(&order_item.item_id),
            ))
            .get_results(&**conn)
    }

    fn load_orders(&self, conn: &WarehouseDatabase) -> Result<Vec<OrderItem>, diesel::result::Error> {
        order_items::table.load::<OrderItem>(&**conn)
    }

    fn load_order_uid(
        &self,
        order_uid: uuid::Uuid,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<OrderItem>, diesel::result::Error> {
        order_items::table
            .filter(order_items::order_uid.eq(order_uid))
            .load::<OrderItem>(&**conn)
    }

    fn load_order_item_uid(
        &self,
        item_uid: uuid::Uuid,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<OrderItem>, diesel::result::Error> {
        order_items::table
            .filter(order_items::order_item_uid.eq(item_uid))
            .load::<OrderItem>(&**conn)
    }

    fn load_item(
        &self,
        model: String,
        size: String,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<Item>, diesel::result::Error> {
        items::table
            .filter(items::model.eq(model))
            .filter(items::size.eq(size))
            .load::<Item>(&**conn)
    }

    fn load_item_id(
        &self,
        id: i32,
        conn: &WarehouseDatabase,
    ) -> Result<Vec<Item>, diesel::result::Error> {
        items::table
            .filter(items::id.eq(id))
            .load::<Item>(&**conn)
    }

    fn update_order_status(
        &self,
        order_uid: uuid::Uuid,
        canceled: bool,
        conn: &WarehouseDatabase,
    ) -> Result<OrderItem, diesel::result::Error> {
        diesel::update(order_items::table.filter(order_items::order_uid.eq(order_uid)))
            .set(order_items::canceled.eq(canceled))
            .get_result(&**conn)
    }

    fn update_item(
        &self,
        item: &Item,
        conn: &WarehouseDatabase,
    ) -> Result<Item, diesel::result::Error> {
        diesel::update(items::table.filter(items::id.eq(item.id)))
            .set(item)
            .get_result(&**conn)
    }
}
