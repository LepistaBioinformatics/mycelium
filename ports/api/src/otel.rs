use std::str::FromStr;
use tonic::metadata::{Ascii, MetadataKey, MetadataMap, MetadataValue};

/// Parse headers from environment variable into MetadataMap
///
/// This function is used to parse headers from environment variable
/// `OTEL_EXPORTER_OTLP_HEADERS` into MetadataMap. The headers are expected to
/// be in the format `name1=value1,name2=value2,...`. The function will return a
/// MetadataMap containing the headers.
pub(super) fn metadata_from_headers(
    headers: Vec<(String, String)>,
) -> MetadataMap {
    let mut metadata = MetadataMap::new();

    headers.into_iter().for_each(|(name, value)| {
        let value = value
            .parse::<MetadataValue<Ascii>>()
            .expect("Header value invalid");
        metadata.insert(MetadataKey::from_str(&name).unwrap(), value);
    });

    metadata
}

/// Parse OTLP headers from environment variable
///
/// This function is used to parse headers from environment variable
/// `OTEL_EXPORTER_OTLP_HEADERS` into a vector of tuples. The headers are
/// expected to be in the format `name1=value1,name2=value2,...`. The function
/// will return a vector of tuples containing the headers.
pub(super) fn parse_otlp_headers_from_env() -> Vec<(String, String)> {
    let mut headers = Vec::new();

    if let Ok(hdrs) = std::env::var("OTEL_EXPORTER_OTLP_HEADERS") {
        hdrs.split(',')
            .map(|header| {
                header
                    .split_once('=')
                    .expect("Header should contain '=' character")
            })
            .for_each(|(name, value)| {
                headers.push((name.to_owned(), value.to_owned()))
            });
    }
    headers
}
