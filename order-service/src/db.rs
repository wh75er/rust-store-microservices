use crate::model::Order;
use crate::schema::orders;
use crate::OrdersDatabase;
use diesel::prelude::*;
use std::result::Result;
use uuid;

pub struct MainDbOps;

pub trait DbOps {
    fn insert_order(
        &self,
        conn: &OrdersDatabase,
        order: &Order,
    ) -> Result<Vec<Order>, diesel::result::Error>;

    fn load_user_orders(
        &self,
        conn: &OrdersDatabase,
        user_uid: uuid::Uuid,
    ) -> Result<Vec<Order>, diesel::result::Error>;

    fn load_by_order_id(
        &self,
        conn: &OrdersDatabase,
        order_uid: uuid::Uuid,
    ) -> Result<Vec<Order>, diesel::result::Error>;

    fn load_by_order_user_id(
        &self,
        conn: &OrdersDatabase,
        order_uid: uuid::Uuid,
        user_uid: uuid::Uuid,
    ) -> Result<Vec<Order>, diesel::result::Error>;

    fn update_order_status(
        &self,
        conn: &OrdersDatabase,
        order_uid: uuid::Uuid,
        status: &str,
    ) -> Result<Order, diesel::result::Error>;

}

impl DbOps for MainDbOps {
    fn insert_order(
        &self,
        conn: &OrdersDatabase,
        order: &Order,
    ) -> Result<Vec<Order>, diesel::result::Error> {
        diesel::insert_into(orders::table)
            .values((
                orders::item_uid.eq(&order.item_uid),
                orders::order_date.eq(&order.order_date),
                orders::order_uid.eq(&order.order_uid),
                orders::status.eq(&order.status),
                orders::user_uid.eq(&order.user_uid),
            ))
            .get_results(&**conn)
    }

    fn load_user_orders(
        &self,
        conn: &OrdersDatabase,
        user_uid: uuid::Uuid,
    ) -> Result<Vec<Order>, diesel::result::Error> {
        orders::table
            .filter(orders::user_uid.eq(user_uid))
            .load::<Order>(&**conn)
    }

    fn load_by_order_id(
        &self,
        conn: &OrdersDatabase,
        order_uid: uuid::Uuid,
    ) -> Result<Vec<Order>, diesel::result::Error> {
        orders::table
            .filter(orders::order_uid.eq(order_uid))
            .load::<Order>(&**conn)
    }

    fn load_by_order_user_id(
        &self,
        conn: &OrdersDatabase,
        order_uid: uuid::Uuid,
        user_uid: uuid::Uuid,
    ) -> Result<Vec<Order>, diesel::result::Error> {
        orders::table
            .filter(orders::order_uid.eq(order_uid))
            .filter(orders::user_uid.eq(user_uid))
            .load::<Order>(&**conn)
    }

    fn update_order_status(
        &self,
        conn: &OrdersDatabase,
        order_uid: uuid::Uuid,
        status: &str,
    ) -> Result<Order, diesel::result::Error> {
        diesel::update(orders::table.filter(orders::order_uid.eq(order_uid)))
            .set(orders::status.eq(status))
            .get_result(&**conn)
    }
}
