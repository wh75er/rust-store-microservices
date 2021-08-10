use std::result::Result;
use std::time::{Instant, Duration};

use crate::{SERVICES_STATUS,
            SERVICES_CALLOUT_TIMEOUT,
            SERVICES_CALLOUT_NUMBER,
            SERVICES_UPDATE_DURATION};

use crate::{Service};

use crate::routes::{OrderWarrantyRequestJson,
OrderWarrantyResponseJson,
WarrantyStatusResponseJson,
CreateOrderResponseJson,
OrderInfoResponseJson,
ItemJson};
use crate::model::{DataError, ServiceAccessError};

use uuid;
use reqwest;
use reqwest::StatusCode;

fn get_service_status(host: &str) -> bool {
    let url = host.to_string() + "/manage/health";

    let client = reqwest::blocking::Client::new();

    let result = client.get(&url)
        .timeout(Duration::new(*SERVICES_CALLOUT_TIMEOUT, 0))
        .send();

    match result {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn update_service_status(host: &str, service: &mut impl Service) {
    if !service.status() {
        if Instant::now().duration_since(service.updated()).as_secs() >= *SERVICES_UPDATE_DURATION {
            if get_service_status(host) {
                service.change_status(true);
            }
        }
    }
}

pub fn request_warehouse_service_item_info(
    host: &str,
    item_uid: uuid::Uuid,
) -> Result<ItemJson, ServiceAccessError> {
    let mut services_status = SERVICES_STATUS.get();

    update_service_status(host, &mut services_status.warehouse_service);

    if !services_status.warehouse_service.up {
        return Err(ServiceAccessError::from(DataError::WarehouseServiceAccessErr));
    }

    let url = host.to_string() + "/api/v1/warehouse/" + item_uid.to_string().as_str();

    let client = reqwest::blocking::Client::new();

    let mut res = None;
    for _ in 0..*SERVICES_CALLOUT_NUMBER {
        let result = client.get(&url)
            .timeout(Duration::new(*SERVICES_CALLOUT_TIMEOUT, 0))
            .send();

        match result {
            Ok(_) => {
                res = Some(result.unwrap());
                break;
            },
            Err(_) => (),
        }
    }

    if res.is_none() {
        services_status.warehouse_service.up = false;
        services_status.warehouse_service.updated = Instant::now();
    }

    let res = res
        .ok_or(ServiceAccessError::from(DataError::WarehouseServiceAccessErr))?;

    if res.status() == StatusCode::NOT_FOUND {
        return Err(ServiceAccessError::from(DataError::ItemNotFound).into());
    } else if res.status() != StatusCode::OK {
        return Err(ServiceAccessError::from(DataError::WarehouseServiceAccessErr).into())
    }
        
    res.json::<ItemJson>()
        .map_err(|e| e.into())
}

pub fn request_order_service_warranty_decision(
    host: &str,
    order_uid: uuid::Uuid,
    req_json: &OrderWarrantyRequestJson,
) -> Result<OrderWarrantyResponseJson, ServiceAccessError> {
    let mut services_status = SERVICES_STATUS.get();

    update_service_status(host, &mut services_status.order_service);

    if !services_status.order_service.up {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr));
    }

    let url = host.to_string() + "/api/v1/orders/" +
        order_uid.to_string().as_str() +
        "/warranty";

    let client = reqwest::blocking::Client::new();

    let mut res = None;
    for _ in 0..*SERVICES_CALLOUT_NUMBER {
        let result = client.post(&url)
            .json(req_json)
            .timeout(Duration::new(*SERVICES_CALLOUT_TIMEOUT, 0))
            .send();

        match result {
            Ok(_) => {
                res = Some(result.unwrap());
                break;
            },
            Err(_) => (),
        }
    }

    if res.is_none() {
        services_status.order_service.up = false;
        services_status.order_service.updated = Instant::now();
    }

    let res = res
        .ok_or(ServiceAccessError::from(DataError::OrderServiceAccessErr))?;

    if res.status() == StatusCode::NOT_FOUND {
        return Err(ServiceAccessError::from(DataError::OrderNotFoundErr).into());
    } else if res.status() != StatusCode::OK {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr).into())
    }
        
    res.json::<OrderWarrantyResponseJson>()
        .map_err(|e| e.into())
}

pub fn request_warranty_service_warranty_info(
    host: &str,
    item_uid: uuid::Uuid,
) -> Result<WarrantyStatusResponseJson, ServiceAccessError> {
    let mut services_status = SERVICES_STATUS.get();

    update_service_status(host, &mut services_status.warranty_service);

    if !services_status.warranty_service.up {
        return Err(ServiceAccessError::from(DataError::WarrantyServiceAccessErr));
    }

    let url = host.to_string() + "/api/v1/warranty/" +
        item_uid.to_string().as_str();

    let client = reqwest::blocking::Client::new();

    let mut res = None;
    for _ in 0..*SERVICES_CALLOUT_NUMBER {
        let result = client.get(&url)
            .timeout(Duration::new(*SERVICES_CALLOUT_TIMEOUT, 0))
            .send();

        match result {
            Ok(_) => {
                res = Some(result.unwrap());
                break;
            },
            Err(_) => (),
        }
    }

    if res.is_none() {
        services_status.warranty_service.up = false;
        services_status.warranty_service.updated = Instant::now();
    }

    let res = res
        .ok_or(ServiceAccessError::from(DataError::WarrantyServiceAccessErr))?;

    if res.status() == StatusCode::NOT_FOUND {
        return Err(ServiceAccessError::from(DataError::WarrantyNotFoundErr).into());
    } else if res.status() != StatusCode::OK {
        return Err(ServiceAccessError::from(DataError::WarrantyServiceAccessErr).into())
    }
        
    res.json::<WarrantyStatusResponseJson>()
        .map_err(|e| e.into())
}

pub fn request_order_service_user_orders(
    host: &str,
    user_uid: uuid::Uuid,
) -> Result<Vec<OrderInfoResponseJson>, ServiceAccessError> {
    let mut services_status = SERVICES_STATUS.get();

    update_service_status(host, &mut services_status.order_service);

    if !services_status.order_service.up {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr));
    }

    let url = host.to_string() + "/api/v1/orders/" +
        user_uid.to_string().as_str();

    let client = reqwest::blocking::Client::new();

    let mut res = None;
    for _ in 0..*SERVICES_CALLOUT_NUMBER {
        let result = client.get(&url)
            .timeout(Duration::new(*SERVICES_CALLOUT_TIMEOUT, 0))
            .send();

        match result {
            Ok(_) => {
                res = Some(result.unwrap());
                break;
            },
            Err(_) => (),
        }
    }

    if res.is_none() {
        services_status.order_service.up = false;
        services_status.order_service.updated = Instant::now();
    }

    let res = res
        .ok_or(ServiceAccessError::from(DataError::OrderServiceAccessErr))?;

    if res.status() != StatusCode::OK {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr).into())
    }
        
    res.json::<Vec<OrderInfoResponseJson>>()
        .map_err(|e| e.into())
}

pub fn request_order_service_user_order(
    host: &str,
    user_uid: uuid::Uuid,
    order_uid: uuid::Uuid,
) -> Result<OrderInfoResponseJson, ServiceAccessError> {
    let mut services_status = SERVICES_STATUS.get();

    update_service_status(host, &mut services_status.order_service);

    if !services_status.order_service.up {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr));
    }

    let url = host.to_string() + "/api/v1/orders/" +
        user_uid.to_string().as_str() + "/" +
        order_uid.to_string().as_str();

    let client = reqwest::blocking::Client::new();

    let mut res = None;
    for _ in 0..*SERVICES_CALLOUT_NUMBER {
        let result = client.get(&url)
            .timeout(Duration::new(*SERVICES_CALLOUT_TIMEOUT, 0))
            .send();

        match result {
            Ok(_) => {
                res = Some(result.unwrap());
                break;
            },
            Err(_) => (),
        }
    }

    if res.is_none() {
        services_status.order_service.up = false;
        services_status.order_service.updated = Instant::now();
    }

    let res = res
        .ok_or(ServiceAccessError::from(DataError::OrderServiceAccessErr))?;

    if res.status() == StatusCode::NOT_FOUND {
        return Err(ServiceAccessError::from(DataError::OrderNotFoundErr).into())
    } else if res.status() != StatusCode::OK {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr).into())
    }
        
    res.json::<OrderInfoResponseJson>()
        .map_err(|e| e.into())
}

pub fn request_order_service_create_order(
    host: &str,
    user_uid: uuid::Uuid,
    req_json: &ItemJson,
) -> Result<CreateOrderResponseJson, ServiceAccessError> {
    let mut services_status = SERVICES_STATUS.get();

    update_service_status(host, &mut services_status.order_service);

    if !services_status.order_service.up {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr));
    }

    let url = host.to_string() + "/api/v1/orders/" +
        user_uid.to_string().as_str();

    let client = reqwest::blocking::Client::new();

    let mut res = None;
    for _ in 0..*SERVICES_CALLOUT_NUMBER {
        let result = client.post(&url)
            .json(req_json)
            .timeout(Duration::new(*SERVICES_CALLOUT_TIMEOUT, 0))
            .send();

        match result {
            Ok(_) => {
                res = Some(result.unwrap());
                break;
            },
            Err(_) => (),
        }
    }

    if res.is_none() {
        services_status.order_service.up = false;
        services_status.order_service.updated = Instant::now();
    }

    let res = res
        .ok_or(ServiceAccessError::from(DataError::OrderServiceAccessErr))?;

    if res.status() == StatusCode::CONFLICT {
        return Err(ServiceAccessError::from(DataError::ItemIsNotAvailable).into())
    } else if res.status() != StatusCode::OK {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr).into())
    }
        
    res.json::<CreateOrderResponseJson>()
        .map_err(|e| e.into())
}

pub fn request_order_service_return_order(
    host: &str,
    order_uid: uuid::Uuid,
) -> Result<(), ServiceAccessError> {
    let mut services_status = SERVICES_STATUS.get();

    update_service_status(host, &mut services_status.order_service);

    if !services_status.order_service.up {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr));
    }

    let url = host.to_string() + "/api/v1/orders/" +
        order_uid.to_string().as_str();

    let client = reqwest::blocking::Client::new();

    let mut res = None;
    for _ in 0..*SERVICES_CALLOUT_NUMBER {
        let result = client.delete(&url)
            .timeout(Duration::new(*SERVICES_CALLOUT_TIMEOUT, 0))
            .send();

        match result {
            Ok(_) => {
                res = Some(result.unwrap());
                break;
            },
            Err(_) => (),
        }
    }

    if res.is_none() {
        services_status.order_service.up = false;
        services_status.order_service.updated = Instant::now();
    }

    let res = res
        .ok_or(ServiceAccessError::from(DataError::OrderServiceAccessErr))?;

    if res.status() == StatusCode::NOT_FOUND {
        return Err(ServiceAccessError::from(DataError::OrderNotFoundErr).into())
    } else if res.status() != StatusCode::NO_CONTENT {
        return Err(ServiceAccessError::from(DataError::OrderServiceAccessErr).into())
    }

    Ok(())
}
