use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct MsGraphDecode {
    pub mail: String,
}
