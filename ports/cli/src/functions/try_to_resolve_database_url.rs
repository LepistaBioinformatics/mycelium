/// Try to resolve database url
///
/// Try to fetch the DATABASE_URL from the environment variable. If not found,
/// ask for the user to paste the database url.
///
pub(crate) fn try_to_resolve_database_url() -> String {
    //
    // Get the database url from the environment variable
    //
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();

    //
    // If the database url is not set, ask for it
    //
    if !database_url.is_empty() {
        return database_url;
    }

    //
    // Ask for the the user to paste the database url
    //
    rpassword::prompt_password("Database URL: ".to_string())
        .expect("Failed to read database url")
        .trim()
        .to_string()
}
