use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", untagged)]
pub enum GenericMapValue<T> {
    Number(T),
    Text(String),
    List(Vec<T>),
    Map(HashMap<String, T>),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GenericMap<T>(HashMap<String, GenericMapValue<T>>);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_as_integer_map() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), GenericMapValue::Number(1));
        map.insert("b".to_string(), GenericMapValue::Text("text".to_string()));
        map.insert("c".to_string(), GenericMapValue::List(vec![1, 2, 3]));

        let generic_map = GenericMap(map);
        let json = json!({
            "a": 1,
            "b": "text",
            "c": [1, 2, 3],
        });

        assert_eq!(serde_json::to_value(&generic_map).unwrap(), json);
        assert_eq!(
            serde_json::from_value::<GenericMap<i32>>(json).unwrap(),
            generic_map
        );
    }

    #[test]
    fn test_as_map_with_submap() {
        let mut map = HashMap::new();

        map.insert("a".to_string(), GenericMapValue::Number(1));
        map.insert("b".to_string(), GenericMapValue::Text("text".to_string()));
        map.insert(
            "c".to_string(),
            GenericMapValue::Map(
                vec![("d".to_string(), 1), ("e".to_string(), 2)]
                    .into_iter()
                    .collect(),
            ),
        );

        let generic_map = GenericMap(map);
        let json = json!({
            "a": 1,
            "b": "text",
            "c": {
                "d": 1,
                "e": 2,
            },
        });

        assert_eq!(serde_json::to_value(&generic_map).unwrap(), json);
        assert_eq!(
            serde_json::from_value::<GenericMap<i32>>(json).unwrap(),
            generic_map
        );
    }
}
