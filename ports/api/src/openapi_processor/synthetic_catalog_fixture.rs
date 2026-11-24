//! Synthetic OpenAPI catalog used by the operations listing tests.

use crate::dtos::{ServiceWrapper, ToolOperation};

use myc_core::domain::dtos::{
    health_check_info::HealthStatus, http::HttpMethod,
    security_group::SecurityGroup,
};
use mycelium_openapi::{OpenApiSchema, Operation};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

pub(crate) struct SyntheticCatalog {
    pub operations: Vec<ToolOperation>,
    pub docs: HashMap<String, OpenApiSchema>,
}

pub(crate) fn build_synthetic_catalog(
    service_count: usize,
    operations_per_service: usize,
) -> SyntheticCatalog {
    let mut operations = Vec::new();
    let mut docs = HashMap::new();

    for service_index in 1..=service_count {
        let service_name = format!("service-{service_index:02}");
        let document =
            build_service_document(&service_name, operations_per_service);

        for operation_index in 0..operations_per_service {
            operations.push(build_tool_operation(
                &service_name,
                operation_index,
                &document,
            ));
        }

        docs.insert(
            service_name,
            serde_json::from_value::<OpenApiSchema>(document).unwrap(),
        );
    }

    SyntheticCatalog { operations, docs }
}

fn record_path(service_name: &str, operation_index: usize) -> String {
    format!("/{service_name}/records-{operation_index}/{{id}}")
}

fn build_service_document(
    service_name: &str,
    operations_per_service: usize,
) -> Value {
    let mut paths = Map::new();
    let mut schemas = Map::new();

    for operation_index in 0..operations_per_service {
        paths.insert(
            record_path(service_name, operation_index),
            json!({ "post": build_operation_json(operation_index) }),
        );

        insert_record_schemas(&mut schemas, operation_index);
    }

    json!({
        "openapi": "3.0.0",
        "info": { "title": service_name, "version": "1.0.0" },
        "paths": paths,
        "components": { "schemas": schemas }
    })
}

fn insert_record_schemas(
    schemas: &mut Map<String, Value>,
    operation_index: usize,
) {
    let child_ref = format!("#/components/schemas/Child{operation_index}");

    schemas.insert(
        format!("Record{operation_index}"),
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "format": "uuid" },
                "name": { "type": "string" },
                "child": { "$ref": child_ref }
            }
        }),
    );

    schemas.insert(
        format!("Child{operation_index}"),
        json!({
            "type": "object",
            "properties": {
                "value": { "type": "integer" },
                "labels": { "type": "array", "items": { "type": "string" } }
            }
        }),
    );
}

fn build_operation_json(operation_index: usize) -> Value {
    let record_ref = format!("#/components/schemas/Record{operation_index}");

    json!({
        "operationId": format!("create_record_{operation_index}"),
        "tags": ["records"],
        "summary": format!("Create record {operation_index}"),
        "parameters": [{
            "name": "id",
            "in": "path",
            "required": true,
            "schema": { "type": "string" }
        }],
        "requestBody": {
            "content": {
                "application/json": { "schema": { "$ref": record_ref } }
            }
        },
        "responses": {
            "201": {
                "description": "Created",
                "content": {
                    "application/json": { "schema": { "$ref": record_ref } }
                }
            }
        }
    })
}

fn build_tool_operation(
    service_name: &str,
    operation_index: usize,
    document: &Value,
) -> ToolOperation {
    let path = record_path(service_name, operation_index);
    let operation_json = document["paths"][&path]["post"].clone();

    ToolOperation {
        path,
        method: HttpMethod::Post,
        operation: serde_json::from_value::<Operation>(operation_json).unwrap(),
        operation_value: None,
        security_group: SecurityGroup::Public,
        service: ServiceWrapper {
            name: service_name.to_string(),
            health_status: HealthStatus::Unknown,
            capabilities: None,
            description: None,
        },
    }
}
