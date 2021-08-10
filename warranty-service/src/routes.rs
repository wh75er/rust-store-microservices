use crate::db::MainDbOps;
use crate::model::*;
use crate::WarrantyDatabase;

use serde::{Deserialize, Serialize};

use rocket::http::{ContentType, Status};
use rocket::request::{Request, FromRequest, Outcome};
use rocket::response::{self, Responder, Response};

use http_auth_basic::Credentials;

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

#[derive(Serialize, Debug)]
struct ErrorJson {
    message: String,
}

#[derive(Serialize, Debug)]
struct WarrantyInfoResponseJson {
    #[serde(rename = "itemUid")]
    item_uid: String,
    status: String,
    #[serde(rename = "warrantyDate")]
    warranty_date: String,
}

#[derive(Deserialize, Debug)]
pub struct ItemWarrantyRequestJson {
    #[serde(rename = "availableCount")]
    available_count: i32,
    reason: String,
}

#[derive(Serialize, Debug)]
struct OrderWarrantyResponseJson {
    #[serde(rename = "decision")]
    verdict: String,
    #[serde(rename = "warrantyDate")]
    warranty_date: String,
}

#[derive(Responder, Debug)]
enum JsonRespond {
    WarrantyInfoResponse(Json<WarrantyInfoResponseJson>),
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

#[get("/api/v1/warranty/<item_uid>")]
pub fn get_info(conn: Result<WarrantyDatabase, ()>, item_uid: String) -> ApiResponder {
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

    match get_warranty_status(&conn, MainDbOps, item_uid) {
        Ok(v) => {
            return ApiResponder {
                inner: JsonRespond::WarrantyInfoResponse(Json(WarrantyInfoResponseJson {
                    item_uid: item_uid.to_string(),
                    status: v.status,
                    warranty_date: v.warranty_date.to_string(),
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

#[post("/api/v1/warranty/<item_uid>/warranty", data = "<body>")]
pub fn request_warranty_verdict(
    conn: Result<WarrantyDatabase, ()>,
    body: Json<ItemWarrantyRequestJson>,
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

    let available_count =
        match validate_available_count(body.available_count).map_err(|e| DaoError::from(e)) {
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

    match get_warranty_verdict(&conn, MainDbOps, item_uid, available_count) {
        Ok(v) => {
            return ApiResponder {
                inner: JsonRespond::OrderWarrantyResponse(Json(OrderWarrantyResponseJson {
                    verdict: v.verdict.unwrap(),
                    warranty_date: v.obj.warranty_date.to_string(),
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
    };
}

#[post("/api/v1/warranty/<item_uid>")]
pub fn request_warranty(conn: Result<WarrantyDatabase, ()>, item_uid: String) -> ApiResponder {
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

    match add_warranty(&conn, MainDbOps, item_uid) {
        Ok(_) => {
            return ApiResponder {
                inner: JsonRespond::Empty(()),
                status: Status::NoContent,
            }
        }
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    }
}

#[delete("/api/v1/warranty/<item_uid>")]
pub fn delete_warranty(conn: Result<WarrantyDatabase, ()>, item_uid: String) -> ApiResponder {
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

    match close_warranty(&conn, MainDbOps, item_uid) {
        Ok(_) => {
            return ApiResponder {
                inner: JsonRespond::Empty(()),
                status: Status::NoContent,
            }
        }
        Err(e) => {
            return ApiResponder {
                inner: JsonRespond::Error(Json(ErrorJson {
                    message: e.to_string(),
                })),
                status: Status::BadRequest,
            }
        }
    }
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
    conn: Result<WarrantyDatabase, ()>,
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
