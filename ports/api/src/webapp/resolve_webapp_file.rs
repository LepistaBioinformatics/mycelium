use std::path::{Path, PathBuf};

/// What a path under `/_app/` resolves to in the webapp build
#[derive(Debug, PartialEq)]
pub(super) enum WebappFile {
    /// A file of the build
    Found(PathBuf),

    /// A client-side route, answered with `index.html` (WEBAPP-02)
    ClientRoute,

    /// A missing file, or one outside the build (WEBAPP-03, WEBAPP-04)
    Missing,
}

/// Resolve `tail`, the path after `/_app/`, inside `dist_dir` (canonical)
///
/// The candidate is canonicalized, so `..` segments and symlinks pointing out
/// of the build never resolve to a file outside it.
///
pub(super) fn resolve_webapp_file(dist_dir: &Path, tail: &str) -> WebappFile {
    let relative = tail.trim_start_matches('/');

    if relative.is_empty() {
        return WebappFile::ClientRoute;
    }

    let found = dist_dir
        .join(relative)
        .canonicalize()
        .ok()
        .filter(|path| path.starts_with(dist_dir) && path.is_file());

    if let Some(path) = found {
        return WebappFile::Found(path);
    }

    if names_a_file(relative) {
        return WebappFile::Missing;
    }

    WebappFile::ClientRoute
}

/// A last segment with an extension names a file, not a client-side route
fn names_a_file(relative: &str) -> bool {
    relative
        .rsplit('/')
        .next()
        .is_some_and(|segment| segment.contains('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_dir() -> PathBuf {
        let root = std::env::temp_dir()
            .join(format!("myc-webapp-{}", uuid::Uuid::new_v4().simple()));
        let dist = root.join("dist");

        std::fs::create_dir_all(dist.join("assets")).unwrap();
        std::fs::write(dist.join("index.html"), "<html></html>").unwrap();
        std::fs::write(dist.join("assets/app-1.js"), "").unwrap();
        std::fs::write(root.join("secret.txt"), "outside").unwrap();

        dist.canonicalize().unwrap()
    }

    #[test]
    fn finds_a_file_of_the_build() {
        let dist = build_dir();

        assert_eq!(
            resolve_webapp_file(&dist, "assets/app-1.js"),
            WebappFile::Found(dist.join("assets/app-1.js"))
        );
    }

    #[test]
    fn answers_client_routes_with_the_index() {
        let dist = build_dir();

        for tail in ["", "dashboard", "dashboard/federation", "assets"] {
            assert_eq!(
                resolve_webapp_file(&dist, tail),
                WebappFile::ClientRoute,
                "{tail}"
            );
        }
    }

    #[test]
    fn misses_an_absent_file() {
        let dist = build_dir();

        assert_eq!(
            resolve_webapp_file(&dist, "assets/missing.js"),
            WebappFile::Missing
        );
    }

    #[test]
    fn never_leaves_the_build() {
        let dist = build_dir();

        assert_eq!(
            resolve_webapp_file(&dist, "../secret.txt"),
            WebappFile::Missing
        );
        assert_eq!(
            resolve_webapp_file(&dist, "assets/../../secret.txt"),
            WebappFile::Missing
        );
    }

    #[cfg(unix)]
    #[test]
    fn never_follows_a_symlink_out_of_the_build() {
        let dist = build_dir();
        std::os::unix::fs::symlink(
            dist.parent().unwrap().join("secret.txt"),
            dist.join("leak.txt"),
        )
        .unwrap();

        assert_eq!(resolve_webapp_file(&dist, "leak.txt"), WebappFile::Missing);
    }
}
