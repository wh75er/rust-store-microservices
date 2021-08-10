#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate lazy_static;

pub mod model;
pub mod schema;

mod db;
mod routes;
mod gateway;

use diesel::result::DatabaseErrorKind::__Unknown;
use diesel::result::Error::DatabaseError;
use diesel_migrations::RunMigrationsError::QueryError;
use rocket::fairing::AdHoc;
use rocket::Rocket;

use dotenv::dotenv;

use std::sync::{Mutex, MutexGuard};
use std::time::{Instant};
use std::env;

use routes::*;

lazy_static! {
    static ref SERVICES_UPDATE_DURATION: u64 = {
        match env::var("SERVICES_UPDATE_DURATION") {
            Ok(v) => v.parse().unwrap(),
            Err(_) => 60,
        }
    };
}

lazy_static! {
    static ref SERVICES_CALLOUT_NUMBER: u8 = {
        match env::var("SERVICES_CALLOUT_NUMBER") {
            Ok(v) => v.parse().unwrap(),
            Err(_) => 4,
        }
    };
}

lazy_static! {
    static ref SERVICES_CALLOUT_TIMEOUT: u64 = {
        match env::var("SERVICES_CALLOUT_TIMEOUT") {
            Ok(v) => v.parse().unwrap(),
            Err(_) => 3,
        }
    };
}

trait Service {
    fn status(&self) -> bool;
    fn change_status(&mut self, up: bool);
    fn updated(&self) -> Instant;
}

struct ServiceStruct {
    up: bool,
    updated: Instant,
}

impl Service for ServiceStruct {
    fn status(&self) -> bool {
        self.up
    }

    fn change_status(&mut self, up: bool) {
        self.up = up;
        self.updated = Instant::now();
    }

    fn updated(&self) -> Instant {
        self.updated
    }
}

struct ServicesStatus {
    warranty_service: ServiceStruct,
    warehouse_service: ServiceStruct,
    order_service: ServiceStruct,
}

lazy_static! {
    static ref SERVICES_STATUS: Mutex<ServicesStatus> = Mutex::new(ServicesStatus {
        warranty_service: ServiceStruct {
            up: true,
            updated: Instant::now(),
        },
        warehouse_service: ServiceStruct {
            up: true,
            updated: Instant::now(),
        },
        order_service: ServiceStruct {
            up: true,
            updated: Instant::now(),
        }
    });
}

impl SERVICES_STATUS {
    pub fn get(&self) -> MutexGuard<ServicesStatus> {
        self.lock().unwrap()
    }
}

embed_migrations!();

#[database("pgdb")]
pub struct UsersDatabase(diesel::PgConnection);

fn run_db_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = UsersDatabase::get_one(&rocket).expect("database connection");
    match embedded_migrations::run(&*conn) {
        Ok(()) => Ok(rocket),
        Err(e) => match e {
            QueryError(e2) => match e2 {
                DatabaseError(e3, _) => match e3 {
                    __Unknown => {
                        println!("Warning!: Migration failure due to possible relation existence!(Ignoring)");
                        Ok(rocket)
                    }
                    _ => Err(rocket),
                },
                _ => Err(rocket),
            },
            _ => {
                println!("Failed to run database migrations: {:?}", e);
                Err(rocket)
            }
        },
    }
}

fn cors() -> impl rocket::fairing::Fairing {
    let mut default = rocket_cors::CorsOptions::default();

    default = default.allow_credentials(true);
    default = default.expose_headers(["Content-Type", "X-Custom", "Location"]
                .iter()
                .map(|s| (*s).to_string())
                .collect());

    default.to_cors().unwrap()
}

fn rocket<T>(db: T) -> rocket::Rocket
where
    T: rocket::fairing::Fairing,
{
    rocket::ignite()
        .mount(
            "/",
            routes![
                user_orders_handler,
                user_order_handler,
                warranty_verdict_handler,
                purchase_handler,
                return_order_handler,
                health_check,
            ],
        )
        .attach(cors())
        .attach(db)
        .attach(AdHoc::on_attach("Database Migrations", run_db_migrations))
}

fn main() {
    dotenv().ok();

    rocket(UsersDatabase::fairing()).launch();
}
