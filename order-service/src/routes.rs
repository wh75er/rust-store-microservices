use crate::db::MainDbOps;
use crate::model::*;
use crate::OrdersDatabase;

use serde::{Deserialize, Serialize};

use rocket::State;
use rocket::http::{ContentType, Status};
use rocket::request::{Request, FromRequest, Outcome};
use rocket::response::{self, Responder, Response};
use rocket_contrib::json::Json;

use amiquip::{Connection};

use http_auth_basic::Credentials;

use std::{env, error, fmt};
use std::sync::Mutex;
use std::fmt::Display;

#[derive(Debug)]
enum DatabaseError {
    ConnectionFailed,
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DatabaseError::ConnectionFailed => f.write_str("Failed to connect to database!"),
        }
    }
}

impl error::Error for DatabaseError {}

#[derive(Serialize, Debug)]
struct ErrorJson {
    message: String,
}

#[derive(Deserialize, Debug)]
pub struct CreateOrderRequestJson {
    pub model: String,
    pub size: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderResponseJson {
    order_uid: uuid::Uuid,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WarehouseItemRequestJson {
    pub order_uid: uuid::Uuid,
    pub model: String,
    pub size: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WarehouseItemResponseJson {
    pub order_item_uid: uuid::Uuid,
    pub order_uid: uuid::Uuid,
    pub model: String,
    pub size: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OrderWarrantyRequestJson {
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderWarrantyResponseJson {
    pub warranty_date: String,
    pub decision: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderInfoResponseJson {
    order_uid: uuid::Uuid,
    order_date: String,
    item_uid: uuid::Uuid,
    status: String,
}

#[derive(Responder, Debug)]
enum JsonRespond {
    OrderInfoResponse(Json<OrderInfoResponseJson>),
    OrdersInfoResponse(Json<Vec<OrderInfoResponseJson>>),
    CreateOrderResponse(Json<CreateOrderResponseJson>),
    OrderWarrantyResponse(Json<OrderWarrantyResponseJson>),
    Error(Json<ErrorJson>),
    Empty(()),
}

#[derive(Debug)]
pub struct ApiResponder {
    inner: JsonRespond,
    status: Status,
}

impl<'r> Responder<'r> for ApiResponder {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        let mut build = Response::build_from(self.inner.respond_to(&req).unwrap());
        build.status(self.status).header(ContentType::JSON).ok()
    }
}

#[post("/api/v1/orders/<user_uid>", data="<body>")]
pub fn make_order_handler(
    conn: Result<OrdersDatabase, ()>,
    queue_conn: State<Option<Mutex<Connection>>>,
    user_uid: String,
    body: Json<CreateOrderRequestJson>,
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
        }
    }

    let conn = conn.unwrap();

    let user_uid = match validate_uid(user_uid).map_err(|e| DaoError::from(e)) {
        Ok(v) => v,
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    };

    let warehouse_host = match env::var("WAREHOUSE_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
        }
    };

    let warranty_host = match env::var("WARRANTY_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
        }
    };

    let order_uid = match create_order(
        &conn,
        &queue_conn,
        MainDbOps,
        &warehouse_host,
        &warranty_host,
        user_uid,
        &body,
    ) {
        Ok(v) => v,
        Err(e) => match e {
            DaoError::DataError(DataError::ItemIsNotAvailable) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::Conflict,
                }
            }
            DaoError::DataError(DataError::WarehouseServiceAccessErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                }
            }
            DaoError::DataError(DataError::WarrantyServiceAccessErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                }
            }
            DaoError::AmpqError => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::InternalServerError,
                }
            }
            _ => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::BadRequest,
                }
            }
        }
    };

    ApiResponder {
        inner: JsonRespond::CreateOrderResponse(Json(CreateOrderResponseJson {
            order_uid: order_uid,
        })),
        status: Status::Ok,
    }
}

#[get("/api/v1/orders/<user_uid>/<order_uid>")]
pub fn get_order_info_handler(
    conn: Result<OrdersDatabase, ()>,
    user_uid: String,
    order_uid: String,
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
        }
    }

    let conn = conn.unwrap();

    let user_uid = match validate_uid(user_uid).map_err(|e| DaoError::from(e)) {
        Ok(v) => v,
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    };

    let order_uid = match validate_uid(order_uid).map_err(|e| DaoError::from(e)) {
        Ok(v) => v,
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    };

    match get_user_order(&conn, MainDbOps, order_uid, user_uid) {
        Ok(v) => {
            return ApiResponder {
                inner: JsonRespond::OrderInfoResponse(Json(OrderInfoResponseJson {
                    order_uid: order_uid,
                    order_date: v.order_date.to_string(),
                    item_uid: v.item_uid,
                    status: v.status,

                })),
                status: Status::Ok,
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::OrderNotFoundErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson { 
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                }
            }
            _ => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson { 
                        message: e.to_string(),
                    })),
                    status: Status::BadRequest,
                }
            }
        }
    }
}

#[get("/api/v1/orders/<user_uid>")]
pub fn get_all_user_orders_handler(
    conn: Result<OrdersDatabase, ()>,
    user_uid: String,
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
        }
    }

    let conn = conn.unwrap();

    let user_uid = match validate_uid(user_uid).map_err(|e| DaoError::from(e)) {
        Ok(v) => v,
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    };

    let orders = get_user_orders(&conn, MainDbOps, user_uid)
        .map_err(|e| return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::BadRequest,
        });

    let mut orders_response: Vec<OrderInfoResponseJson> = Vec::new();
    
    for order in orders.unwrap().iter() {
        orders_response.push(OrderInfoResponseJson {
            order_uid: order.order_uid,
            order_date: order.order_date.to_string(),
            item_uid: order.item_uid,
            status: order.status.to_string(),
        });
    };

    ApiResponder {
        inner: JsonRespond::OrdersInfoResponse(Json(orders_response)),
        status: Status::Ok,
    }
}

#[post("/api/v1/orders/<order_uid>/warranty", data="<body>")]
pub fn get_order_warranty_handler(
    conn: Result<OrdersDatabase, ()>,
    order_uid: String,
    body: Json<OrderWarrantyRequestJson>
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
        }
    }

    let conn = conn.unwrap();

    let order_uid = match validate_uid(order_uid).map_err(|e| DaoError::from(e)) {
        Ok(v) => v,
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    };

    let warehouse_host = match env::var("WAREHOUSE_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
        }
    };

    let response = match get_warranty_decision(
        &conn,
        MainDbOps,
        &warehouse_host,
        order_uid,
        &body,
    ) {
        Ok(v) => v,
        Err(e) => match e {
            DaoError::DataError(DataError::OrderNotFoundErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                }
            }
            DaoError::DataError(DataError::WarrantyServiceAccessErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                }
            }
            _ => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::BadRequest,
                }
            }
        }
    };

    ApiResponder {
        inner: JsonRespond::OrderWarrantyResponse(Json(response)),
        status: Status::Ok,
    }
}

#[delete("/api/v1/orders/<order_uid>")]
pub fn return_order_handler(
    conn: Result<OrdersDatabase, ()>,
    order_uid: String,
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
        }
    }

    let conn = conn.unwrap();

    let order_uid = match validate_uid(order_uid).map_err(|e| DaoError::from(e)) {
        Ok(v) => v,
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    };

    let warehouse_host = match env::var("WAREHOUSE_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
        }
    };

    let warranty_host = match env::var("WARRANTY_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
        }
    };

    let _ = return_order(
        &conn,
        MainDbOps,
        &warehouse_host,
        &warranty_host,
        order_uid,
    ).map_err(|e| match e {
        DaoError::DataError(DataError::OrderNotFoundErr) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::NotFound,
            }
        }
        DaoError::DataError(DataError::WarrantyServiceAccessErr) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::UnprocessableEntity,
            }
        }
        DaoError::DataError(DataError::WarehouseServiceAccessErr) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::UnprocessableEntity,
            }
        }
        _ => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    });

    ApiResponder {
        inner: JsonRespond::Empty(()),
        status: Status::NoContent,
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DetailsBody {
    database: String,
    validation_query: String,
}

#[derive(Serialize, Debug)]
struct DbBody {
    status: String,
    details: DetailsBody,
}

#[derive(Serialize, Debug)]
struct ComponentsBody {
    db: DbBody,
}

#[derive(Serialize, Debug)]
struct PingBody {
    status: String,
}

#[derive(Serialize, Debug)]
pub struct HealthBody {
    status: String,
    components: ComponentsBody,
    ping: PingBody,
}

#[derive(PartialEq)]
struct User {
    username: String,
    password: String,
}

impl User {
    fn user_from(
        uname: String,
        pass: String, 
    ) -> User {
        User {
            username: uname,
            password: pass,
        }
    }

    fn is_admin(
        &self,
    ) -> bool {
        let admin_uname = match env::var("ADMIN_USERNAME") {
            Ok(v) => v,
            Err(_) => "root".to_string(),
        };

        let admin_pass = match env::var("ADMIN_PASSWORD") {
            Ok(v) => v,
            Err(_) => "root".to_string(),
        };

        let admin = User {
            username: admin_uname,
            password: admin_pass,
        };

        if self == &admin {
            true
        } else {
            false
        }
    }
}

pub struct Admin(User);

impl<'a, 'r> FromRequest<'a, 'r> for Admin {
    type Error = ();

    fn from_request(request: &Request) -> Outcome<Self, Self::Error> {
        let auth_header = request.headers().get_one("Authorization");

        match auth_header {
            Some(v) => {
                let credentials = Credentials::from_header(v.to_string()).unwrap();

                let user = User::user_from(credentials.user_id, credentials.password);

                if user.is_admin() {
                    Outcome::Success(Admin(user))
                } else {
                    Outcome::Failure((Status::Unauthorized, ()))
                }
            }
            _ => Outcome::Failure((Status::Unauthorized, ()))
        }

    }
}

#[get("/manage/health")]
pub fn health_check(
    _user: Admin,
    conn: Result<OrdersDatabase, ()>,
) -> Json<HealthBody> {
    let mut validation_query = String::from("IsValid()");
    let mut status = String::from("UP");

    if conn.is_err() {
        validation_query = String::from("!IsValid()");
        status = String::from("DOWN");
    }

    let details =  DetailsBody {
        database: String::from("PostgreSQL"),
        validation_query,
    };

    let db = DbBody {
        status,
        details,
    };

    let components = ComponentsBody {
        db: db,
    };

    let ping_status = String::from("UP");

    let ping = PingBody {
        status: ping_status,
    };

    let server_status = String::from("UP");

    Json(HealthBody {
        status: server_status,
        components: components,
        ping: ping,
    })
}
