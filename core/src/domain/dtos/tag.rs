use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

pub type TagMeta = HashMap<String, String>;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: Uuid,
    pub value: String,
    pub meta: Option<TagMeta>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tag_serde() {
        let tag = Tag {
            id: Uuid::new_v4(),
            value: "tag".to_string(),
            meta: Some(HashMap::new()),
        };

        let json = json!({
            "id": tag.id.to_string(),
            "value": tag.value,
            "meta": tag.meta,
        });

        assert_eq!(serde_json::to_value(&tag).unwrap(), json);
        assert_eq!(serde_json::from_value::<Tag>(json).unwrap(), tag);
    }
}
