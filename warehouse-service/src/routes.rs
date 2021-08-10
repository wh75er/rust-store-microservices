use crate::db::MainDbOps;
use crate::model::*;
use crate::WarehouseDatabase;

use serde::{Deserialize, Serialize};

use http_auth_basic::Credentials;

use rocket::http::{ContentType, Status};
use rocket::request::{Request, FromRequest, Outcome};
use rocket::response::{self, Responder, Response};
use rocket_contrib::json::Json;

use std::env;
use std::error;
use std::fmt;
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

#[derive(Serialize, Debug)]
pub struct ItemInfoResponseJson {
    model: String,
    size: String,
}

#[derive(Deserialize, Debug)]
pub struct OrderItemRequestJson {
    model: String,
    #[serde(rename = "orderUid")]
    order_uid: uuid::Uuid,
    size: String,
}

#[derive(Serialize, Debug)]
pub struct OrderItemResponseJson {
    model: String,
    #[serde(rename = "orderItemUid")]
    item_uid: uuid::Uuid,
    #[serde(rename = "orderUid")]
    order_uid: uuid::Uuid,
    size: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OrderWarrantyRequestJson {
    reason: String,
    #[serde(rename = "availableCount")]
    pub available_count: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OrderWarrantyResponseJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "warrantyDate")]
    pub warranty_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Responder, Debug)]
enum JsonRespond {
    ItemInfoResponse(Json<ItemInfoResponseJson>),
    OrderItemResponse(Json<OrderItemResponseJson>),
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

#[get("/api/v1/warehouse/<item_uid>")]
pub fn get_item_info(
    conn: Result<WarehouseDatabase, ()>,
    item_uid: String,
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

    let item_uid = match validate_uid(item_uid).map_err(|e| DaoError::from(e)) {
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

    match get_item(&conn, MainDbOps, item_uid) { 
        Ok(v) => {
            return ApiResponder {
                inner: JsonRespond::ItemInfoResponse(Json(ItemInfoResponseJson {
                    model: v.model,
                    size: v.size,
                })),
                status: Status::Ok,
            }
        }
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::NotFound,
            }
        }
    }
}

#[post("/api/v1/warehouse", data="<body>")]
pub fn add_order_item(
    conn: Result<WarehouseDatabase, ()>,
    body: Json<OrderItemRequestJson>
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

    match create_order(&conn, MainDbOps, body.order_uid, body.model.as_str(), body.size.as_str()) {
        Ok(v) => {
            return ApiResponder {
                inner: JsonRespond::OrderItemResponse(Json(OrderItemResponseJson {
                    model: body.model.to_string(),
                    item_uid: v.order_item_uid,
                    order_uid: v.order_uid,
                    size: body.size.to_string(),
                })),
                status: Status::Ok,
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::ItemNotFoundErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                }
            }
            DaoError::DataError(DataError::OrderNotFoundErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                }
            }
            DaoError::DataError(DataError::ItemIsNotAvailableErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::Conflict,
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

#[post("/api/v1/warehouse/<item_uid>/warranty", data = "<body>")]
pub fn request_item_warranty(
    conn: Result<WarehouseDatabase, ()>,
    body: Json<OrderWarrantyRequestJson>,
    item_uid: String,
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

    let item_uid = match validate_uid(item_uid).map_err(|e| DaoError::from(e)) {
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

    let warranty_host = match env::var("WARRANTY_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
        }
    };

    match get_warranty_verdict(&conn, MainDbOps, warranty_host.as_str(), item_uid, &mut body.into_inner()) {
        Ok(v) => {
            return ApiResponder {
                inner: JsonRespond::OrderWarrantyResponse(Json(v)),
                status: Status::Ok,
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::ItemNotFoundErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: String::from("Warranty not found for itemUid \'") + item_uid.to_string().as_str() + "\'",
                    })),
                    status: Status::NotFound,
                }
            }
            DaoError::DataError(DataError::OrderNotFoundErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: String::from("Warranty not found for itemUid \'") + item_uid.to_string().as_str() + "\'",
                    })),
                    status: Status::NotFound,
                }
            }
            DaoError::DataError(DataError::WarrantyServiceItemNotFoundErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: String::from("Warranty not found for itemUid \'") + item_uid.to_string().as_str() + "\'",
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
    }

}

#[delete("/api/v1/warehouse/<item_uid>")]
pub fn delete_order_item(
    conn: Result<WarehouseDatabase, ()>,
    item_uid: String,
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

    let item_uid = match validate_uid(item_uid).map_err(|e| DaoError::from(e)) {
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

    match cancel_order(&conn, MainDbOps, item_uid) {
        Ok(_) => {
            return ApiResponder {
                inner: JsonRespond::Empty(()),
                status: Status::NoContent,
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::ItemNotFoundErr) => {
                return ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                }
            }
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
    }
}

#[derive(Serialize, Debug)]
struct DetailsBody {
    database: String,
    #[serde(rename = "validationQuery")]
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
    conn: Result<WarehouseDatabase, ()>,
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
