use crate::WarehouseDatabase;
use crate::db::DbOps;
use crate::routes::{OrderWarrantyResponseJson, OrderWarrantyRequestJson};
use crate::gateway::{request_warranty_service_item_verdict};

use crate::schema::{items, order_items};

use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;
use std::fmt::Display;
use uuid;
use reqwest;

#[derive(Debug, Deserialize, Serialize, Queryable, Insertable, AsChangeset, Clone, PartialEq)]
#[table_name = "items"]
pub struct Item {
    #[serde(default)]
    pub id: i32,
    pub available_count: i32,
    pub model: String,
    pub size: String,
}

#[derive(Debug, Deserialize, Serialize, Queryable, Insertable, AsChangeset, Clone, PartialEq)]
#[table_name = "order_items"]
pub struct OrderItem {
    #[serde(default)]
    pub id: i32,
    pub canceled: Option<bool>,
    pub order_item_uid: uuid::Uuid,
    pub order_uid: uuid::Uuid,
    pub item_id: Option<i32>,
}

#[derive(Debug, PartialEq)]
pub enum ValidateError {
    InvalidUidErr,
}

impl Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ValidateError::InvalidUidErr => f.write_str("UUID is incorrect! Failed to parse it!"),
        }
    }
}

impl error::Error for ValidateError {}

#[derive(Debug, PartialEq)]
pub enum DataError {
    OrderNotFoundErr,
    ItemNotFoundErr,
    ItemIsNotAvailableErr,
    OrderCreateErr,
    WarrantyServiceAccessErr,
    WarrantyServiceItemNotFoundErr,
}

impl Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DataError::OrderNotFoundErr => f.write_str("Requested order is not found!"),
            DataError::ItemNotFoundErr => f.write_str("Requested item is not found!"),
            DataError::ItemIsNotAvailableErr => f.write_str("Item is not available!"),
            DataError::OrderCreateErr => f.write_str("Failed to create order!"),
            DataError::WarrantyServiceAccessErr => f.write_str("Failed to access warranty service!"),
            DataError::WarrantyServiceItemNotFoundErr => f.write_str("Requested item not found!"),
        }
    }
}

impl error::Error for DataError {}

#[derive(Debug, PartialEq)]
pub enum DaoError {
    DieselError(diesel::result::Error),
    DataError(DataError),
    ValidateError(ValidateError),
}

impl Display for DaoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DaoError::DieselError(e) => f.write_str(e.to_string().as_str()),
            DaoError::DataError(e) => f.write_str(e.to_string().as_str()),
            DaoError::ValidateError(e) => f.write_str(e.to_string().as_str()),
        }
    }
}

impl error::Error for DaoError {}

impl From<diesel::result::Error> for DaoError {
    fn from(err: diesel::result::Error) -> DaoError {
        DaoError::DieselError(err)
    }
}

impl From<DataError> for DaoError {
    fn from(err: DataError) -> DaoError {
        DaoError::DataError(err)
    }
}

impl From<ValidateError> for DaoError {
    fn from(err: ValidateError) -> DaoError {
        DaoError::ValidateError(err)
    }
}

#[derive(Debug)]
pub enum ServiceAccessError {
    ReqwestError(reqwest::Error),
    DataError(DataError),
}

impl Display for ServiceAccessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServiceAccessError::ReqwestError(e) => f.write_str(e.to_string().as_str()),
            ServiceAccessError::DataError(e) => f.write_str(e.to_string().as_str()),
        }
    }
}

impl error::Error for ServiceAccessError {}

impl From<reqwest::Error> for ServiceAccessError {
    fn from(err: reqwest::Error) -> ServiceAccessError {
        ServiceAccessError::ReqwestError(err)
    }
}

impl From<DataError> for ServiceAccessError {
    fn from(err: DataError) -> ServiceAccessError {
        ServiceAccessError::DataError(err)
    }
}

pub fn validate_uid(uid: String) -> Result<uuid::Uuid, ValidateError> {
    uid.parse::<uuid::Uuid>()
        .map_err(|_| ValidateError::InvalidUidErr)
}

impl Item {
    fn decrement_count(&mut self) -> Result<(), DaoError> {
        if self.available_count <= 0 {
            return Err(DaoError::from(DataError::ItemIsNotAvailableErr));
        }

        self.available_count -= 1;

        Ok(())
    }

    fn increment_count(&mut self) -> () {
        self.available_count += 1;
    }
}

pub fn get_item(
    conn: &WarehouseDatabase,
    dbops: impl DbOps,
    item_uid: uuid::Uuid,
) -> Result<Item, DaoError> {
    let mut vec = dbops.load_order_item_uid(item_uid, conn)?;

    let order = vec
        .pop()
        .ok_or(DaoError::from(DataError::OrderNotFoundErr))?;
    
    let mut vec = dbops.load_item_id(order.item_id.unwrap(), conn)?;

    vec.pop().ok_or(DaoError::from(DataError::ItemNotFoundErr))
}

pub fn create_order(
    conn: &WarehouseDatabase,
    dbops: impl DbOps,
    order_uid: uuid::Uuid,
    model: &str,
    size: &str,
) -> Result<OrderItem, DaoError> {
    let mut vec = dbops.load_item(model.to_string(), size.to_string(), conn)?;
    let mut item = vec.pop().ok_or(DaoError::from(DataError::ItemNotFoundErr))?;

    item.decrement_count()?;

    dbops.update_item(&item, conn)?;

    let mut vec = dbops.load_order_uid(order_uid, conn)?;

    if !vec.is_empty() {
        dbops.update_order_status(order_uid, false, conn)?;
    } else {
        let item_uid = uuid::Uuid::new_v4();

        vec = dbops.insert_order(
            &OrderItem {
                id: 0,
                canceled: Some(false),
                order_item_uid: item_uid,
                order_uid: order_uid,
                item_id: Some(item.id),
            },
            conn,
        )?;
    }

    vec.pop().ok_or(DaoError::from(DataError::OrderCreateErr))
}

pub fn get_warranty_verdict(
    conn: &WarehouseDatabase,
    dbops: impl DbOps,
    host: &str,
    item_uid: uuid::Uuid,
    req_json: &mut OrderWarrantyRequestJson,
) -> Result<OrderWarrantyResponseJson, DaoError> {
    let item = get_item(conn, dbops, item_uid)?;

    req_json.available_count = Some(item.available_count);

    let response = request_warranty_service_item_verdict(host, item_uid, req_json)
        .map_err(|e| match e {
            ServiceAccessError::DataError(DataError::WarrantyServiceItemNotFoundErr) => {
                DaoError::from(DataError::WarrantyServiceItemNotFoundErr)
            }
            _ => {
                DaoError::from(DataError::WarrantyServiceAccessErr)
            }
        })?;

    Ok(response)
}

pub fn cancel_order(
    conn: &WarehouseDatabase,
    dbops: impl DbOps,
    item_uid: uuid::Uuid,
) -> Result<(), DaoError> {
    let mut vec = dbops.load_order_item_uid(item_uid, conn)?;

    let order = vec.pop()
        .ok_or(DaoError::from(DataError::OrderNotFoundErr))?;

    dbops.update_order_status(order.order_uid, true, conn)?;

    let mut vec = dbops.load_item_id(order.item_id.unwrap(), conn)?;

    let mut item = vec.pop().
        ok_or(DaoError::from(DataError::ItemNotFoundErr))?;

    item.increment_count();

    dbops.update_item(&item, conn)?;

    Ok(())
}
