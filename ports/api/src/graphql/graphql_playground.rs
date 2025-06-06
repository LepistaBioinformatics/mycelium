use actix_web::HttpResponse;
use async_graphql::http::GraphQLPlaygroundConfig;

pub(crate) async fn graphql_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(async_graphql::http::playground_source(
            GraphQLPlaygroundConfig::new("/graphql"),
        ))
}
