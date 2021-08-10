use crate::model::User;
use crate::schema::users;
use crate::UsersDatabase;
use diesel::prelude::*;
use std::result::Result;
use uuid;

pub struct MainDbOps;

pub trait DbOps {
    fn load_user_by_id(
        &self,
        conn: &UsersDatabase,
        user_uid: uuid::Uuid,
    ) -> Result<Vec<User>, diesel::result::Error>;
}

impl DbOps for MainDbOps {
    fn load_user_by_id(
        &self,
        conn: &UsersDatabase,
        user_uid: uuid::Uuid,
    ) -> Result<Vec<User>, diesel::result::Error> {
        users::table
            .filter(users::user_uid.eq(user_uid))
            .load::<User>(&**conn)
    }
}
