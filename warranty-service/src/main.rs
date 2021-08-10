#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

pub mod model;
pub mod schema;

mod db;
mod routes;

use diesel::result::DatabaseErrorKind::__Unknown;
use diesel::result::Error::DatabaseError;
use diesel_migrations::RunMigrationsError::QueryError;
use rocket::fairing::AdHoc;
use rocket::Rocket;

use dotenv::dotenv;

use routes::*;

embed_migrations!();

#[database("pgdb")]
pub struct WarrantyDatabase(diesel::PgConnection);

fn run_db_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = WarrantyDatabase::get_one(&rocket).expect("database connection");
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
                get_info,
                request_warranty_verdict,
                request_warranty,
                delete_warranty,
                health_check,
            ],
        )
        .attach(cors())
        .attach(db)
        .attach(AdHoc::on_attach("Database Migrations", run_db_migrations))
}

fn main() {
    dotenv().ok();

    rocket(WarrantyDatabase::fairing()).launch();
}
