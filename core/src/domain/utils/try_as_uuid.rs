use mycelium_base::utils::errors::{general_err, MappedErrors};
use uuid::Uuid;

pub fn try_as_uuid(uuid: &str) -> Result<Uuid, MappedErrors> {
    match Uuid::parse_str(uuid) {
        Ok(res) => Ok(res),
        Err(_) => general_err(
            format!("{uuid} is not a valid UUID"),
            "uuid".to_string(),
        )
        .with_exp_true()
        .as_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_try_as_uuid() {
        let uuid = Uuid::new_v4();
        let uuid_str = uuid.to_string();
        assert_eq!(try_as_uuid(&uuid_str).unwrap(), uuid);
        assert!(try_as_uuid("not a uuid").is_err());
    }
}
