#[derive(Debug)]
pub(crate) enum InternalError {
    Database(diesel::result::Error),

    #[allow(unused)]
    Unknown,
}

impl From<diesel::result::Error> for InternalError {
    fn from(err: diesel::result::Error) -> Self {
        InternalError::Database(err)
    }
}

impl ToString for InternalError {
    fn to_string(&self) -> String {
        match self {
            InternalError::Database(e) => format!("Database error: {}", e),
            InternalError::Unknown => "Unknown error".to_string(),
        }
    }
}
