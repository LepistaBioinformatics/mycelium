mod bootstrap_claim_page;
mod bootstrap_complete;
mod bootstrap_request_code;

use actix_web::web;

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config
        .service(bootstrap_claim_page::bootstrap_claim_page_url)
        .service(bootstrap_request_code::bootstrap_request_code_url)
        .service(bootstrap_complete::bootstrap_complete_url);
}
