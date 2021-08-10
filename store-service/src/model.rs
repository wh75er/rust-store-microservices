use crate::UsersDatabase;
use crate::db::DbOps;
use crate::routes::{OrderWarrantyRequestJson,
    OrderWarrantyResponseJson,
    SolidOrderInfo,
    OrderInfoResponseJson,
    ItemJson,
    WarrantyStatusResponseJson,
    CreateOrderResponseJson};
use crate::gateway::*;

use crate::schema::users;

use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;
use std::fmt::Display;
use uuid;
use reqwest;

#[derive(Debug, Deserialize, Serialize, Queryable, Insertable, AsChangeset, Clone, PartialEq)]
pub struct User {
    #[serde(default)]
    pub id: i32,
    pub name: String,
    pub user_uid: uuid::Uuid,
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
    UserNotFoundErr,
    WarrantyNotFoundErr,
    OrderCreateErr,
    ItemIsNotAvailable,
    ItemNotFound,
    OrderServiceAccessErr,
    WarehouseServiceAccessErr,
    WarrantyServiceAccessErr,
}

impl Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DataError::OrderNotFoundErr => f.write_str("Requested order is not found!"),
            DataError::UserNotFoundErr => f.write_str("Requested user is not found!"),
            DataError::WarrantyNotFoundErr => f.write_str("Warranty info not found!"),
            DataError::OrderCreateErr => f.write_str("Failed to create order!"),
            DataError::ItemIsNotAvailable => f.write_str("Item not available!"),
            DataError::ItemNotFound => f.write_str("Requested item not found!"),
            DataError::OrderServiceAccessErr => f.write_str("Failed to access order service!"),
            DataError::WarehouseServiceAccessErr => f.write_str("Failed to access warehouse service!"),
            DataError::WarrantyServiceAccessErr => f.write_str("Failed to access warranty service!"),
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

pub fn verify_user(
    conn: &UsersDatabase,
    dbops: impl DbOps,
    user_uid: uuid::Uuid
) -> Result<User, DaoError> {
    let mut vec = dbops.load_user_by_id(conn, user_uid)?;
    
    vec.pop()
        .ok_or(DaoError::from(DataError::UserNotFoundErr))
}

pub fn get_solid_info(
    order: &OrderInfoResponseJson,
    warehouse_host: &str,
    warranty_host: &str,
) -> Result<SolidOrderInfo, DaoError> {
    let item_uid = order.item_uid;

    let mut solid_order_info = SolidOrderInfo {
        order_uid: order.order_uid,
        date: order.order_date.to_string(),
        model: None,
        size: None,
        warranty_date: None,
        warranty_status: None,
    };

    let item_info = request_warehouse_service_item_info(warehouse_host, item_uid)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::WarehouseServiceAccessErr)
            }
        });

    let item_info: Option<ItemJson> = item_info.ok();

    match item_info {
        Some(v) => {
            solid_order_info.model = Some(v.model);
            solid_order_info.size = Some(v.size);
        },
        None => {},
    }

    let warranty_info = request_warranty_service_warranty_info(warranty_host, item_uid)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::WarrantyServiceAccessErr)
            }
        });

    let warranty_info: Option<WarrantyStatusResponseJson> = warranty_info.ok();

    match warranty_info {
        Some(v) => {
            solid_order_info.warranty_date = Some(v.warranty_date);
            solid_order_info.warranty_status = Some(v.status);
        },
        None => {},
    }

    Ok(solid_order_info)
}

pub fn get_orders_info(
    conn: &UsersDatabase,
    dbops: impl DbOps,
    user_uid: uuid::Uuid,
    order_host: &str,
    warehouse_host: &str,
    warranty_host: &str,
) -> Result<Vec<SolidOrderInfo>, DaoError> {
    let _ = verify_user(conn, dbops, user_uid)?;

    let orders: Vec<OrderInfoResponseJson> = request_order_service_user_orders(order_host, user_uid)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::OrderServiceAccessErr)
            }
        })?;

    let mut solid_orders_info = vec!();

    for order in orders.iter() {
        let solid_order_info = get_solid_info(&order, warehouse_host, warranty_host)?;

        solid_orders_info.push(
            solid_order_info
        );
    };

    Ok(solid_orders_info)
}

pub fn get_order_info(
    conn: &UsersDatabase,
    dbops: impl DbOps,
    user_uid: uuid::Uuid,
    order_uid: uuid::Uuid,
    order_host: &str,
    warehouse_host: &str,
    warranty_host: &str,
) -> Result<SolidOrderInfo, DaoError> {
    let _ = verify_user(conn, dbops, user_uid)?;

    let order: OrderInfoResponseJson = request_order_service_user_order(order_host, user_uid, order_uid)  
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::OrderServiceAccessErr)
            }
        })?;

    get_solid_info(&order, warehouse_host, warranty_host)
}

pub fn get_warranty_decision(
    conn: &UsersDatabase,
    dbops: impl DbOps,
    user_uid: uuid::Uuid,
    order_uid: uuid::Uuid,
    order_host: &str,
    req_json: &OrderWarrantyRequestJson,
) -> Result<OrderWarrantyResponseJson, DaoError> {
    let _ = verify_user(conn, dbops, user_uid)?;

    request_order_service_warranty_decision(order_host, order_uid, req_json)   
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::OrderServiceAccessErr)
            }
        })
        .map(|mut v| {
            v.order_uid = Some(order_uid); 
            v
        })
}

pub fn purchase_item(
    conn: &UsersDatabase,
    dbops: impl DbOps,
    user_uid: uuid::Uuid,
    order_host: &str,
    req_json: &ItemJson,
) -> Result<CreateOrderResponseJson, DaoError> {
    let _ = verify_user(conn, dbops, user_uid)?;
 
    request_order_service_create_order(order_host, user_uid, req_json)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::OrderServiceAccessErr)
            }
        })
}

pub fn return_item(
    conn: &UsersDatabase,
    dbops: impl DbOps,
    user_uid: uuid::Uuid,
    order_uid: uuid::Uuid,
    order_host: &str,
) -> Result<(), DaoError> {
    let _ = verify_user(conn, dbops, user_uid)?;

    request_order_service_return_order(order_host, order_uid)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::OrderServiceAccessErr)
            }
        })
        .map(|_| ())
}
