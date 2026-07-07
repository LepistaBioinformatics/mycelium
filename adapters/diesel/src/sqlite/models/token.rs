use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};

#[derive(Clone, Debug, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = crate::sqlite::schema::token)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Token {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Text)]
    pub expiration: String,
    #[diesel(sql_type = Text)]
    pub meta: String,
}
