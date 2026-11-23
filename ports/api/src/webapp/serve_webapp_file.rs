use super::{
    resolve_webapp_file::{resolve_webapp_file, WebappFile},
    webapp_response_headers::with_webapp_headers,
    ServedWebapp,
};
use crate::settings::WEBAPP_MOUNT_PATH;

use actix_files::NamedFile;
use actix_web::{http::header, web, HttpRequest, HttpResponse};

/// `GET`/`HEAD` on `/_app` and `/_app/{tail}`; other methods answer 405
pub(super) fn configure(config: &mut web::ServiceConfig, served: ServedWebapp) {
    config
        .app_data(web::Data::new(served))
        .service(
            web::resource(WEBAPP_MOUNT_PATH)
                .route(web::get().to(redirect_to_mount))
                .route(web::head().to(redirect_to_mount)),
        )
        .service(
            web::resource(format!("{WEBAPP_MOUNT_PATH}/{{tail:.*}}"))
                .route(web::get().to(serve_webapp_file))
                .route(web::head().to(serve_webapp_file)),
        );
}

/// `/_app` to `/_app/`, where the relative asset URLs resolve (WEBAPP-05)
async fn redirect_to_mount() -> HttpResponse {
    HttpResponse::PermanentRedirect()
        .insert_header((header::LOCATION, format!("{WEBAPP_MOUNT_PATH}/")))
        .finish()
}

/// A file of the build, `index.html` for a client-side route, 404 otherwise
async fn serve_webapp_file(
    req: HttpRequest,
    served: web::Data<ServedWebapp>,
) -> HttpResponse {
    let path = match resolve_webapp_file(
        served.dist_dir(),
        req.match_info().query("tail"),
    ) {
        WebappFile::Found(path) => path,
        WebappFile::ClientRoute => served.dist_dir().join("index.html"),
        WebappFile::Missing => {
            return with_webapp_headers(HttpResponse::NotFound().finish(), None)
        }
    };

    let file_name = path.file_name().and_then(|name| name.to_str());

    match NamedFile::open(&path) {
        Ok(file) => with_webapp_headers(file.into_response(&req), file_name),
        Err(_) => with_webapp_headers(HttpResponse::NotFound().finish(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::webapp_config::WebappConfig;

    use actix_web::{
        body::to_bytes, dev::ServiceResponse, http::StatusCode, test, App,
    };

    fn served() -> ServedWebapp {
        let dist = std::env::temp_dir()
            .join(format!("myc-webapp-{}", uuid::Uuid::new_v4().simple()));

        std::fs::create_dir_all(dist.join("assets")).unwrap();
        std::fs::write(dist.join("index.html"), "<html>index</html>").unwrap();
        std::fs::write(dist.join("assets/app-1.js"), "app").unwrap();

        let config = WebappConfig {
            dist_dir: dist,
            hosted_login_client_id: None,
        };

        ServedWebapp::prepare(Some(&config), &[]).unwrap().unwrap()
    }

    async fn call(req: test::TestRequest) -> ServiceResponse {
        let app = test::init_service(
            App::new()
                .configure(|config| configure(config, served()))
                .default_service(web::to(|| async {
                    HttpResponse::ImATeapot().finish()
                })),
        )
        .await;

        test::call_service(&app, req.to_request()).await
    }

    fn header_of(response: &ServiceResponse, name: &str) -> String {
        response
            .headers()
            .get(name)
            .map(|value| value.to_str().unwrap().to_string())
            .unwrap_or_default()
    }

    #[actix_web::test]
    async fn serves_a_deep_link_with_the_uncached_index() {
        let response =
            call(test::TestRequest::get().uri("/_app/dashboard/federation"))
                .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            header_of(&response, "cache-control"),
            "no-store, no-cache, must-revalidate"
        );
        assert_eq!(header_of(&response, "x-frame-options"), "SAMEORIGIN");
        let body = to_bytes(response.into_body()).await.unwrap();
        assert_eq!(body, "<html>index</html>");
    }

    #[actix_web::test]
    async fn serves_an_asset_as_immutable() {
        let response =
            call(test::TestRequest::get().uri("/_app/assets/app-1.js")).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            header_of(&response, "cache-control"),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(header_of(&response, "x-content-type-options"), "nosniff");
    }

    #[actix_web::test]
    async fn answers_404_for_a_missing_asset() {
        let response =
            call(test::TestRequest::get().uri("/_app/assets/missing.js")).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn never_serves_an_encoded_traversal() {
        let response = call(
            test::TestRequest::get().uri("/_app/assets/%2e%2e/%2e%2e/x.txt"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn redirects_the_bare_mount() {
        let response = call(test::TestRequest::get().uri("/_app")).await;

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(header_of(&response, "location"), "/_app/");
    }

    #[actix_web::test]
    async fn refuses_other_methods() {
        let response =
            call(test::TestRequest::post().uri("/_app/dashboard")).await;

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[actix_web::test]
    async fn leaves_other_paths_to_the_router() {
        let response =
            call(test::TestRequest::get().uri("/_application")).await;

        assert_eq!(response.status(), StatusCode::IM_A_TEAPOT);
    }
}
