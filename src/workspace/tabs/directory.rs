use std::path::{Path, PathBuf};

use crate::settings::StartIn;

pub(super) fn resolve(
    group: bool,
    explicit: Option<PathBuf>,
    start_in: StartIn,
    session: Option<&Path>,
    current: Option<&Path>,
    home: impl FnOnce() -> Option<PathBuf>,
) -> anyhow::Result<PathBuf> {
    if !group {
        anyhow::ensure!(
            explicit.is_none(),
            "app: a new tab cannot override its session cwd"
        );
        return session
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow::anyhow!("app: no session for a new tab"));
    }
    if let Some(folder) = explicit {
        return Ok(folder);
    }
    if let StartIn::Session = start_in
        && let Some(folder) = current
    {
        return Ok(folder.to_path_buf());
    }
    home().ok_or_else(|| anyhow::anyhow!("app: no home folder, so no new session"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_use_their_owner_not_live_folder_or_settings() {
        for setting in [StartIn::Home, StartIn::Session] {
            for owner in ["/projects/a", "/projects/b"] {
                let cwd = resolve(
                    false,
                    None,
                    setting,
                    Some(Path::new(owner)),
                    Some(Path::new("/tmp")),
                    || panic!("a tab does not consult home"),
                );
                assert_eq!(cwd.unwrap(), Path::new(owner));
            }
        }
    }

    #[test]
    fn sessions_keep_explicit_and_start_in_policies() {
        let owner = Some(Path::new("/project"));
        let live = Some(Path::new("/tmp"));
        for setting in [StartIn::Home, StartIn::Session] {
            assert_eq!(
                resolve(
                    true,
                    Some("/explicit".into()),
                    setting,
                    owner,
                    live,
                    || panic!("explicit")
                )
                .unwrap(),
                Path::new("/explicit")
            );
        }
        assert_eq!(
            resolve(true, None, StartIn::Home, owner, live, || Some(
                "/home".into()
            ))
            .unwrap(),
            Path::new("/home")
        );
        assert_eq!(
            resolve(true, None, StartIn::Session, owner, live, || panic!("live")).unwrap(),
            Path::new("/tmp")
        );
        assert_eq!(
            resolve(true, None, StartIn::Session, None, None, || Some(
                "/home".into()
            ))
            .unwrap(),
            Path::new("/home")
        );
    }

    #[test]
    fn missing_context_and_tab_overrides_are_errors() {
        assert!(
            resolve(false, None, StartIn::Home, None, None, || Some(
                "/home".into()
            ))
            .is_err()
        );
        assert!(
            resolve(
                false,
                Some("/override".into()),
                StartIn::Home,
                Some(Path::new("/owner")),
                None,
                || None
            )
            .is_err()
        );
        assert!(resolve(true, None, StartIn::Home, None, None, || None).is_err());
    }
}
