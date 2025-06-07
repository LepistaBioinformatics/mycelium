use crate::graphql::QueryRoot;

use actix_web::web;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};

#[tracing::instrument(name = "graphql_handler", skip_all)]
pub(crate) async fn graphql_handler(
    schema: web::Data<Schema<QueryRoot, EmptyMutation, EmptySubscription>>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}
