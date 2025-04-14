use actix_web::http::uri::PathAndQuery;

#[tracing::instrument(name = "extract_path_parts")]
pub(crate) fn extract_path_parts(path: PathAndQuery) -> (String, String) {
    let path_string = path.to_string();
    let path_parts = path_string.split("/").collect::<Vec<&str>>();
    let service_name = path_parts[1];
    let rest = path_string.replace(&format!("/{}", service_name), "");

    (service_name.to_string(), rest)
}
