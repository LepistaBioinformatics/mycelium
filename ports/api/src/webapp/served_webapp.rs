use crate::{models::webapp_config::WebappConfig, settings::WEBAPP_MOUNT_PATH};

use myc_core::domain::dtos::service::Service;
use std::path::PathBuf;

/// A webapp build checked at boot: `dist_dir` is canonical and holds
/// `index.html`
#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "webapp"), allow(dead_code))]
pub(crate) struct ServedWebapp {
    dist_dir: PathBuf,
}

impl ServedWebapp {
    /// `None` without `[api.webapp]`
    ///
    /// Errs, and the boot stops, when the section is set but cannot be served:
    /// no `webapp` feature, an unreadable `distDir` or no `index.html` in it
    /// (WEBAPP-09).
    ///
    pub(crate) fn prepare(
        config: Option<&WebappConfig>,
        services: &[Service],
    ) -> Result<Option<Self>, String> {
        let Some(config) = config else {
            return Ok(None);
        };

        if !cfg!(feature = "webapp") {
            return Err("[api.webapp] is set but this gateway was built \
                without the `webapp` feature"
                .to_string());
        }

        let dist_dir = config.dist_dir.canonicalize().map_err(|err| {
            format!(
                "[api.webapp] distDir {} cannot be read: {err}",
                config.dist_dir.display()
            )
        })?;

        if !dist_dir.join("index.html").is_file() {
            return Err(format!(
                "[api.webapp] distDir {} has no index.html",
                dist_dir.display()
            ));
        }

        warn_about_shadowed_routes(services);

        Ok(Some(Self { dist_dir }))
    }

    #[cfg(feature = "webapp")]
    pub(crate) fn dist_dir(&self) -> &std::path::Path {
        &self.dist_dir
    }
}

/// A downstream route under `/_app` is unreachable while the webapp is served
fn warn_about_shadowed_routes(services: &[Service]) {
    let mount = format!("{WEBAPP_MOUNT_PATH}/");

    for service in services {
        service
            .routes
            .iter()
            .filter(|route| {
                route.path == WEBAPP_MOUNT_PATH
                    || route.path.starts_with(&mount)
            })
            .for_each(|route| {
                tracing::warn!(
                    service = %service.name,
                    path = %route.path,
                    "route shadowed by the webapp served at {WEBAPP_MOUNT_PATH}"
                )
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(dist_dir: PathBuf) -> WebappConfig {
        WebappConfig {
            dist_dir,
            hosted_login_client_id: None,
        }
    }

    fn build_dir(with_index: bool) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("myc-webapp-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();

        if with_index {
            std::fs::write(dir.join("index.html"), "<html></html>").unwrap();
        }

        dir
    }

    #[test]
    fn serves_nothing_without_the_section() {
        assert!(ServedWebapp::prepare(None, &[]).unwrap().is_none());
    }

    #[cfg(feature = "webapp")]
    #[test]
    fn accepts_a_build_with_its_index() {
        let dir = build_dir(true);

        let served = ServedWebapp::prepare(Some(&config(dir.clone())), &[])
            .unwrap()
            .unwrap();

        assert_eq!(served.dist_dir(), dir.canonicalize().unwrap());
    }

    #[cfg(feature = "webapp")]
    #[test]
    fn refuses_a_build_without_index() {
        let err = ServedWebapp::prepare(Some(&config(build_dir(false))), &[])
            .unwrap_err();

        assert!(err.contains("has no index.html"), "{err}");
    }

    #[cfg(feature = "webapp")]
    #[test]
    fn refuses_a_missing_directory() {
        let missing = std::env::temp_dir().join("myc-webapp-missing-dir");

        let err =
            ServedWebapp::prepare(Some(&config(missing)), &[]).unwrap_err();

        assert!(err.contains("cannot be read"), "{err}");
    }

    #[cfg(not(feature = "webapp"))]
    #[test]
    fn refuses_the_section_without_the_feature() {
        let err = ServedWebapp::prepare(Some(&config(build_dir(true))), &[])
            .unwrap_err();

        assert!(err.contains("without the `webapp` feature"), "{err}");
    }
}
