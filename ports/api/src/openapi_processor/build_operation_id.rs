use rand::{distributions::Alphanumeric, Rng};
use slugify::slugify;

/// Build operation id
///
/// This function builds an operation id from a method, operation id, service
/// name, and path.
///
pub(crate) fn build_operation_id(
    method: &str,
    operation_id: Option<&String>,
    service_name: &str,
    path: &str,
) -> String {
    let method = method.to_string().to_uppercase();
    let operation_id = (match &operation_id {
        Some(id) => slugify!(id.to_string().as_str()),
        None => slugify!(path.to_string().as_str()),
    })
    .replace("-", "_");

    let random_suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    format!(
        "{}__{}__{}__{}",
        service_name.to_string(),
        method,
        operation_id,
        random_suffix
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_operation_id() {
        let operation_id_named = build_operation_id(
            "GET",
            Some(&String::from("get_user")),
            "user",
            "user/1",
        );

        let operation_id_unnamed =
            build_operation_id("GET", None, "user", "user/1");

        assert_eq!(operation_id_named, "user:GET:get_user");

        assert_eq!(operation_id_unnamed, "user:GET:user_1");
    }
}
