use slugify::slugify;

pub(crate) fn build_operation_id(
    operation_id: Option<String>,
    service_name: &str,
    path: &str,
) -> String {
    (match &operation_id {
        Some(id) => format!("{}_{}", service_name, id),
        None => slugify!(&format!("{}_{}", service_name, path)),
    })
    .replace("-", "_")
}
