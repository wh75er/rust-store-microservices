use crate::db::MainDbOps;
use crate::model::*;
use crate::UsersDatabase;

use serde::{Deserialize, Serialize};

use rocket::http::hyper::header;
use rocket::http::{ContentType, Status};
use rocket::request::{Request, FromRequest, Outcome};
use rocket::response::{self, Responder, Response};
use rocket_contrib::json::Json;

use http_auth_basic::Credentials;

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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderResponseJson {
    order_uid: uuid::Uuid,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WarrantyStatusResponseJson {
    pub item_uid: uuid::Uuid,
    pub warranty_date: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OrderWarrantyRequestJson {
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderWarrantyResponseJson {
    pub order_uid: Option<uuid::Uuid>,
    pub warranty_date: String,
    pub decision: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemJson {
    pub model: String,
    pub size: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderInfoResponseJson {
    pub order_uid: uuid::Uuid,
    pub order_date: String,
    pub item_uid: uuid::Uuid,
    pub status: String,
}


#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SolidOrderInfo {
    pub order_uid: uuid::Uuid,
    pub date: String,
    pub model: Option<String>,
    pub size: Option<String>,
    pub warranty_date: Option<String>,
    pub warranty_status: Option<String>,
}

#[derive(Responder, Debug)]
enum JsonRespond {
    OrdersRespond(Json<Vec<SolidOrderInfo>>),
    OrderRespond(Json<SolidOrderInfo>),
    WarrantyRespond(Json<OrderWarrantyResponseJson>),
    Error(Json<ErrorJson>),
    Empty(()),
}

#[derive(Debug)]
pub struct ApiResponder {
    inner: JsonRespond,
    status: Status,
    location: Option<String>,
}

impl<'r> Responder<'r> for ApiResponder {
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        let mut build = Response::build_from(self.inner.respond_to(&req).unwrap());
        if let Some(location) = self.location {
            build.merge(
                Response::build()
                    .header(header::Location(location))
                    .finalize(),
            );
        }
        build.status(self.status).header(ContentType::JSON).ok()
    }
}

#[get("/api/v1/store/<user_uid>/orders")]
pub fn user_orders_handler(
    conn: Result<UsersDatabase, ()>,
    user_uid: String,
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
            location: None,
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
                location: None,
            }
        }
    };

    let order_host = match env::var("ORDER_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };

    let warehouse_host = match env::var("WAREHOUSE_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };

    let warranty_host = match env::var("WARRANTY_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };

    match get_orders_info(&conn, MainDbOps, user_uid, &order_host, &warehouse_host, &warranty_host) {
        Ok(v) => {
            ApiResponder {
                inner: JsonRespond::OrdersRespond(Json(v)),
                status: Status::Ok,
                location: None,
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::UserNotFoundErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                    location: None,
                }
            }
            DaoError::DataError(DataError::OrderServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarehouseServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarrantyServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            _ => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::BadRequest,
                    location: None,
                }
            }
        }
    }
}

#[get("/api/v1/store/<user_uid>/<order_uid>", rank=1)]
pub fn user_order_handler(
    conn: Result<UsersDatabase, ()>,
    user_uid: String,
    order_uid: String,
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
            location: None,
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
                location: None,
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
                location: None,
            }
        }
    };

    let order_host = match env::var("ORDER_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };

    let warehouse_host = match env::var("WAREHOUSE_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };

    let warranty_host = match env::var("WARRANTY_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };

    match get_order_info(&conn, MainDbOps, user_uid, order_uid, &order_host, &warehouse_host, &warranty_host) {
        Ok(v) => {
            ApiResponder {
                inner: JsonRespond::OrderRespond(Json(v)),
                status: Status::Ok,
                location: None,
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::UserNotFoundErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                    location: None,
                }
            }
            DaoError::DataError(DataError::OrderServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarehouseServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarrantyServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            _ => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::BadRequest,
                    location: None,
                }
            }
        }
    }
}

#[post("/api/v1/store/<user_uid>/<order_uid>/warranty", data="<body>")]
pub fn warranty_verdict_handler(
    conn: Result<UsersDatabase, ()>,
    user_uid: String,
    order_uid: String,
    body: Json<OrderWarrantyRequestJson>
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
            location: None,
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
                location: None,
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
                location: None,
            }
        }
    };

    let order_host = match env::var("ORDER_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };

    match get_warranty_decision(&conn, MainDbOps, user_uid, order_uid, &order_host, &body.into_inner()) {
        Ok(v) => {
            ApiResponder {
                inner: JsonRespond::WarrantyRespond(Json(v)),
                status: Status::Ok,
                location: None,
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::UserNotFoundErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                    location: None,
                }
            }
            DaoError::DataError(DataError::OrderServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarehouseServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarrantyServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            _ => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::BadRequest,
                    location: None,
                }
            }
        }
    }
}

#[post("/api/v1/store/<user_uid>/purchase", data="<body>")]
pub fn purchase_handler(
    conn: Result<UsersDatabase, ()>,
    user_uid: String,
    body: Json<ItemJson>
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
            location: None,
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
                location: None,
            }
        }
    };

    let order_host = match env::var("ORDER_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };
    
    match purchase_item(&conn, MainDbOps, user_uid, &order_host, &body.into_inner()) {
        Ok(v) => {
            ApiResponder {
                inner: JsonRespond::Empty(()),
                status: Status::Created,
                location: Some(
                    "/".to_string() + v.order_uid.to_string().as_str()
                ),
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::UserNotFoundErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                    location: None,
                }
            }
            DaoError::DataError(DataError::ItemIsNotAvailable) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::Conflict,
                    location: None,
                }
            }
            DaoError::DataError(DataError::OrderServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarehouseServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarrantyServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            _ => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::BadRequest,
                    location: None,
                }
            }
        }
    }
}

#[delete("/api/v1/store/<user_uid>/<order_uid>/refund")]
pub fn return_order_handler(
    conn: Result<UsersDatabase, ()>,
    order_uid: String,
    user_uid: String,
) -> ApiResponder {
    if conn.is_err() {
        return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: DatabaseError::ConnectionFailed.to_string(),
            })),
            status: Status::ServiceUnavailable,
            location: None,
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
                location: None,
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
                location: None,
            }
        }
    };

    let order_host = match env::var("ORDER_HOST") {
        Ok(v) => v,
        Err(e) => return ApiResponder {
            inner: JsonRespond::Error(Json(ErrorJson {
                message: e.to_string(),
            })),
            status: Status::UnprocessableEntity,
            location: None,
        }
    };

    match return_item(&conn, MainDbOps, user_uid, order_uid, &order_host) {
        Ok(_) => {
            ApiResponder {
                inner: JsonRespond::Empty(()),
                status: Status::NoContent,
                location: None,
            }
        }
        Err(e) => match e {
            DaoError::DataError(DataError::UserNotFoundErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::NotFound,
                    location: None,
                }
            }
            DaoError::DataError(DataError::OrderServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarehouseServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            DaoError::DataError(DataError::WarrantyServiceAccessErr) => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::UnprocessableEntity,
                    location: None,
                }
            }
            _ => {
                ApiResponder {
                    inner: JsonRespond::Error(Json(ErrorJson {
                        message: e.to_string(),
                    })),
                    status: Status::BadRequest,
                    location: None,
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
    conn: Result<UsersDatabase, ()>,
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
