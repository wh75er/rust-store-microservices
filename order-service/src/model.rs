use crate::OrdersDatabase;
use crate::db::DbOps;
use crate::routes::{WarehouseItemRequestJson,
    CreateOrderRequestJson,
    OrderWarrantyRequestJson,
    OrderWarrantyResponseJson};
use crate::gateway::{get_service_status, request_warehouse_service_item, request_warehouse_service_return, request_warranty_service_start, request_warranty_service_stop, request_warehouse_service_decision, request_warehouse_service_item_info};

use crate::{WARRANTY_POLLING_THREAD,
            SERVICES_UPDATE_DURATION,
            QUEUE_NAME,
};

use amiquip::{Connection, QueueDeclareOptions, ConsumerOptions, ConsumerMessage, Exchange, Publish};

use crate::schema::orders;

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, MutexGuard};
use std::{thread, thread::JoinHandle, error, fmt, result::Result};
use std::fmt::Display;
use chrono;
use uuid;
use reqwest;

#[derive(Debug, Deserialize, Serialize, Queryable, Insertable, AsChangeset, Clone, PartialEq)]
pub struct Order {
    #[serde(default)]
    pub id: i32,
    pub item_uid: uuid::Uuid,
    pub order_date: chrono::NaiveDateTime,
    pub order_uid: uuid::Uuid,
    pub status: String,
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
    OrderCreateErr,
    ItemIsNotAvailable,
    ItemNotFound,
    WarehouseServiceAccessErr,
    WarrantyServiceAccessErr,
}

impl Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DataError::OrderNotFoundErr => f.write_str("Requested order is not found!"),
            DataError::UserNotFoundErr => f.write_str("Requested user is not found!"),
            DataError::OrderCreateErr => f.write_str("Failed to create order!"),
            DataError::ItemIsNotAvailable => f.write_str("Item not available!"),
            DataError::ItemNotFound => f.write_str("Requested item not found!"),
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
    AmpqError,
}

impl Display for DaoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DaoError::DieselError(e) => f.write_str(e.to_string().as_str()),
            DaoError::DataError(e) => f.write_str(e.to_string().as_str()),
            DaoError::ValidateError(e) => f.write_str(e.to_string().as_str()),
            DaoError::AmpqError => f.write_str("AMPQ internal error!"),
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

fn create_queue_consumer(
    queue_conn: &Mutex<Connection>,
    warranty_host: &str,
    mut warranty_polling_thread: MutexGuard<Option<JoinHandle<()>>>
) -> Result<(), amiquip::Error> {
    let mut queue_conn_unwrapped = queue_conn.lock().unwrap();
    let channel = queue_conn_unwrapped.open_channel(None)?;
    let warranty_host_copy = String::from(warranty_host);

    *warranty_polling_thread = Some(thread::spawn(move || -> () {
        loop {
            if get_service_status(warranty_host_copy.as_str()) {
                let queue = channel.queue_declare(QUEUE_NAME, QueueDeclareOptions::default()).unwrap();
                let consumer = queue.consume(ConsumerOptions::default()).unwrap();

                for message in consumer.receiver().iter() {
                    match message {
                        ConsumerMessage::Delivery(delivery) => {
                            let body = String::from_utf8_lossy(&delivery.body);
                            let item_uid = uuid::Uuid::parse_str(&(*body)).unwrap();

                            let result = request_warranty_service_start(warranty_host_copy.as_str(), item_uid);

                            if result.is_ok() {
                                consumer.ack(delivery).unwrap();
                            } else {
                                break;
                            }
                        }
                        _ => {
                            break;
                        }
                    }
                }

                consumer.cancel().unwrap();
            } else {
                channel.recover(true).unwrap();
                thread::sleep(std::time::Duration::from_secs(*SERVICES_UPDATE_DURATION));
            }
        }
    }));

    Ok(())
}

pub fn validate_uid(uid: String) -> Result<uuid::Uuid, ValidateError> {
    uid.parse::<uuid::Uuid>()
        .map_err(|_| ValidateError::InvalidUidErr)
}

pub fn get_user_order(
    conn: &OrdersDatabase,
    dbops: impl DbOps,
    order_uid: uuid::Uuid,
    user_uid: uuid::Uuid,
) -> Result<Order, DaoError> {
    let mut vec = dbops.load_by_order_user_id(conn, order_uid, user_uid)?;

    vec.pop()
        .ok_or(DaoError::from(DataError::OrderNotFoundErr))
}

pub fn get_user_orders(
    conn: &OrdersDatabase,
    dbops: impl DbOps,
    user_uid: uuid::Uuid,
) -> Result<Vec<Order>, DaoError> {
    dbops.load_user_orders(conn, user_uid)
        .map_err(|e| e.into())
}

pub fn create_order(
    conn: &OrdersDatabase,
    queue_conn: &Option<Mutex<Connection>>,
    dbops: impl DbOps,
    warehouse_host: &str,
    warranty_host: &str,
    user_uid: uuid::Uuid,
    body: &CreateOrderRequestJson,
) -> Result<uuid::Uuid, DaoError> {
    let order_uid = uuid::Uuid::new_v4();

    let req_json = WarehouseItemRequestJson {
        order_uid: order_uid,
        model: body.model.to_string(),
        size: body.size.to_string(),
    };

    let response = request_warehouse_service_item(warehouse_host, &req_json)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::WarehouseServiceAccessErr)
            }
        })?;

    let order = Order {
        id: 0,
        item_uid: response.order_item_uid,
        order_date: chrono::Utc::now().naive_utc(),
        order_uid: order_uid,
        status: "PAID".to_string(),
        user_uid: user_uid,
    };

    let err = request_warranty_service_start(warranty_host, order.item_uid)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::WarrantyServiceAccessErr)
            }
        })
        .err();

    if err != None {
        if queue_conn.is_some() {
            let queue_conn = queue_conn.as_ref().unwrap();
            let warranty_polling_thread = WARRANTY_POLLING_THREAD.lock().unwrap();

            if warranty_polling_thread.is_none() {
                let _ = create_queue_consumer(queue_conn, warranty_host, warranty_polling_thread)
                    .map_err(|_| DaoError::AmpqError)?;
            }

            let mut queue_conn_unwrapped = queue_conn.lock().unwrap();

            let channel = queue_conn_unwrapped.open_channel(None)
                .map_err(|_| DaoError::AmpqError)?;

            channel.queue_declare(QUEUE_NAME, QueueDeclareOptions::default())
                .map_err(|_| DaoError::AmpqError)?;

            let exchange = Exchange::direct(&channel);

            exchange.publish(Publish::new(order.item_uid.to_string().as_bytes(), QUEUE_NAME))
                .map_err(|_| DaoError::AmpqError)?;
        } else {
            request_warehouse_service_return(warehouse_host, order.item_uid)
                .map_err(|e| match e {
                    ServiceAccessError::DataError(de) => {
                        de.into()
                    }
                    _ => {
                        DaoError::from(DataError::WarrantyServiceAccessErr)
                    }
                })?;

            return Err(err.unwrap());
        }
    }

    let mut vec = dbops.insert_order(conn, &order)?;

    vec.pop()
        .ok_or(DataError::OrderCreateErr)?;

    Ok(order_uid)
}

pub fn return_order(
    conn: &OrdersDatabase,
    dbops: impl DbOps,
    warehouse_host: &str,
    warranty_host: &str,
    order_uid: uuid::Uuid,
) -> Result<(), DaoError> {
    let mut vec = dbops.load_by_order_id(conn, order_uid)?;

    let order = vec.pop().ok_or(DataError::OrderNotFoundErr)?;
    
    let item_uid = order.item_uid;

    request_warehouse_service_return(warehouse_host, item_uid)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::WarehouseServiceAccessErr)
            }
        })?;

    let err = request_warranty_service_stop(warranty_host, item_uid)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::WarehouseServiceAccessErr)
            }
        })
        .err();

    if err != None {
        let item_info = request_warehouse_service_item_info(warehouse_host, item_uid)
            .map_err(|e| match e {
                ServiceAccessError::DataError(de) => {
                    de.into()
                }
                _ => {
                    DaoError::from(DataError::WarehouseServiceAccessErr)
                }
            })?;

        let req_json = WarehouseItemRequestJson {
            order_uid: order_uid,
            model: item_info.model.to_string(),
            size: item_info.size.to_string(),
        };

        request_warehouse_service_item(warehouse_host, &req_json)
            .map_err(|e| match e {
                ServiceAccessError::DataError(de) => {
                    de.into()
                }
                _ => {
                    DaoError::from(DataError::WarehouseServiceAccessErr)
                }
            })?;

        return Err(err.unwrap());
    }

    dbops.update_order_status(conn, order_uid, "CANCELED")?;

    Ok(())
}

pub fn get_warranty_decision(
    conn: &OrdersDatabase,
    dbops: impl DbOps,
    warehouse_host: &str,
    order_uid: uuid::Uuid,
    req_json: &OrderWarrantyRequestJson,
) -> Result<OrderWarrantyResponseJson, DaoError> {
    let mut vec = dbops.load_by_order_id(conn, order_uid)?;

    let order = vec.pop().ok_or(DataError::OrderNotFoundErr)?;

    request_warehouse_service_decision(warehouse_host, order.item_uid, req_json)
        .map_err(|e| match e {
            ServiceAccessError::DataError(de) => {
                de.into()
            }
            _ => {
                DaoError::from(DataError::WarehouseServiceAccessErr)
            }
        })
}
