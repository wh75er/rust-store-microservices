use crate::db::DbOps;
use crate::schema::warranty;
use crate::WarrantyDatabase;
use chrono;
use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;
use std::fmt::Display;
use std::result::Result;
use uuid;

#[derive(Debug, Deserialize, Serialize, Queryable, Insertable, AsChangeset, Clone, PartialEq)]
#[table_name = "warranty"]
pub struct Warranty {
    #[serde(default)]
    pub id: i32,
    pub comment: Option<String>,
    pub item_uid: uuid::Uuid,
    pub status: String,
    pub warranty_date: chrono::NaiveDateTime,
}

pub struct WarrantyVerdict {
    pub obj: Warranty,
    pub verdict: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ValidateError {
    InvalidUidErr,
    InvalidItemNumErr,
}

impl Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ValidateError::InvalidUidErr => f.write_str("UUID is incorrect! Failed to parse it!"),
            ValidateError::InvalidItemNumErr => {
                f.write_str("Available item number is incorrect! Number should be positive!")
            }
        }
    }
}

impl error::Error for ValidateError {}

#[derive(Debug, PartialEq)]
pub enum DataError {
    NotFoundErr,
    InsertErr,
    DeleteErr,
}

impl Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DataError::NotFoundErr => f.write_str("Requested value is not found!"),
            DataError::InsertErr => f.write_str("Failed to insert value!"),
            DataError::DeleteErr => f.write_str("Failed to delete value!"),
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

pub fn validate_uid(uid: String) -> Result<uuid::Uuid, ValidateError> {
    uid.parse::<uuid::Uuid>()
        .map_err(|_| ValidateError::InvalidUidErr)
}

pub fn validate_available_count(item_num: i32) -> Result<i32, ValidateError> {
    if item_num < 0 {
        return Err(ValidateError::InvalidItemNumErr);
    } else {
        return Ok(item_num);
    }
}

pub fn get_warranty_status(
    conn: &WarrantyDatabase,
    dbops: impl DbOps,
    uid: uuid::Uuid,
) -> Result<Warranty, DaoError> {
    let mut vec = dbops.load_id(uid, conn)?;

    vec.pop().ok_or(DaoError::from(DataError::NotFoundErr))
}

pub fn add_warranty(
    conn: &WarrantyDatabase,
    dbops: impl DbOps,
    uid: uuid::Uuid,
) -> Result<Warranty, DaoError> {
    let w = Warranty {
        id: 0,
        comment: None,
        item_uid: uid,
        status: String::from("ON_WARRANTY"),
        warranty_date: chrono::Utc::now().naive_utc(),
    };

    let mut vec = dbops.insert(&w, conn)?;

    vec.pop().ok_or(DaoError::from(DataError::InsertErr))
}

pub fn close_warranty(
    conn: &WarrantyDatabase,
    dbops: impl DbOps,
    uid: uuid::Uuid,
) -> Result<Warranty, DaoError> {
    dbops
        .update(uid, "REMOVED_FROM_WARRANTY", conn)
        .map_err(|e| DaoError::from(e))
}

pub fn get_warranty_verdict(
    conn: &WarrantyDatabase,
    dbops: impl DbOps,
    uid: uuid::Uuid,
    item_num: i32,
) -> Result<WarrantyVerdict, DaoError> {
    let mut vec = dbops.load_id(uid, conn)?;

    let mut verdict = vec
        .pop()
        .ok_or(DaoError::from(DataError::NotFoundErr))
        .map(|v| WarrantyVerdict {
            obj: v,
            verdict: None,
        })?;

    if verdict.obj.status != "ON_WARRANTY" {
        verdict.verdict = Some(String::from("REFUSED"));
        return Ok(verdict);
    }

    if item_num > 0 {
        verdict.verdict = Some(String::from("RETURN"));
        return Ok(verdict);
    } else {
        verdict.verdict = Some(String::from("FIXING"));
        return Ok(verdict);
    }
}
