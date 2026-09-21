use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml;

use crate::reconcile::{AdapterError, ProjectConfig, ProjectRoot, StatePaths, WorkspaceAdapter};

pub(crate) struct OsWorkspaceAdapter;

pub(crate) struct FakeWorkspaceAdapter;

impl OsWorkspaceAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl WorkspaceAdapter for OsWorkspaceAdapter {
    fn discover(&mut self, start: PathBuf) -> Result<ProjectRoot, AdapterError> {
        let start = fs::canonicalize(start).map_err(io_error)?;
        let mut candidate = if start.is_dir() {
            start.clone()
        } else {
            start
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| AdapterError {
                    category: "project-discovery".to_owned(),
                    message: "starting path has no parent".to_owned(),
                })?
        };

        loop {
            if candidate.join(".spawnbx.yml").is_file() {
                return Ok(ProjectRoot {
                    canonical_path: candidate,
                });
            }

            if !candidate.pop() {
                break;
            }
        }

        let fallback = fs::canonicalize(start).map_err(io_error)?;
        Ok(ProjectRoot {
            canonical_path: fallback,
        })
    }

    fn read_config(&mut self, root: &ProjectRoot) -> Result<Option<ProjectConfig>, AdapterError> {
        let path = root.canonical_path.join(".spawnbx.yml");
        if !path.is_file() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path).map_err(io_error)?;
        serde_yaml::from_str(&contents)
            .map(Some)
            .map_err(|error| AdapterError {
                category: "configuration".to_owned(),
                message: format!("{}: {error}", path.display()),
            })
    }

    fn write_config(
        &mut self,
        root: &ProjectRoot,
        config: &ProjectConfig,
    ) -> Result<(), AdapterError> {
        let path = safe_child(&root.canonical_path, Path::new(".spawnbx.yml"))?;
        let temporary = path.with_extension("yml.tmp");
        let contents = serde_yaml::to_string(config).map_err(|error| AdapterError {
            category: "configuration".to_owned(),
            message: error.to_string(),
        })?;

        fs::write(&temporary, contents).map_err(io_error)?;
        fs::rename(&temporary, &path).map_err(io_error)
    }

    fn ensure_state(
        &mut self,
        root: &ProjectRoot,
        create: bool,
    ) -> Result<StatePaths, AdapterError> {
        let state_dir = safe_child(&root.canonical_path, Path::new(".spawnbx"))?;
        let home_dir = safe_child(&root.canonical_path, Path::new(".spawnbx/home"))?;
        let nix_dir = safe_child(&root.canonical_path, Path::new(".spawnbx/nix"))?;
        let lock_path = safe_child(&root.canonical_path, Path::new(".spawnbx/nix/flake.lock"))?;

        if create {
            fs::create_dir_all(&home_dir).map_err(io_error)?;
            fs::create_dir_all(&nix_dir).map_err(io_error)?;
        }

        Ok(StatePaths {
            state_dir,
            home_dir,
            nix_dir,
            lock_path,
        })
    }

    fn write_generated(&mut self, path: PathBuf, contents: String) -> Result<(), AdapterError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        fs::write(path, contents).map_err(io_error)
    }
}

fn safe_child(root: &Path, child: &Path) -> Result<PathBuf, AdapterError> {
    let path = root.join(child);
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if metadata.file_type().is_symlink() {
            return Err(AdapterError {
                category: "unsafe-project-state".to_owned(),
                message: format!("state path is a symlink: {}", path.display()),
            });
        }
    }
    if let Ok(existing) = fs::canonicalize(&path) {
        if !existing.starts_with(root) {
            return Err(AdapterError {
                category: "unsafe-project-state".to_owned(),
                message: format!("path escapes project root: {}", path.display()),
            });
        }
    } else if let Some(parent) = path.parent() {
        let mut existing_parent = parent;
        while !existing_parent.exists() {
            existing_parent = existing_parent.parent().ok_or_else(|| AdapterError {
                category: "unsafe-project-state".to_owned(),
                message: format!("cannot resolve state parent {}", parent.display()),
            })?;
        }
        let parent = fs::canonicalize(existing_parent).map_err(|error| AdapterError {
            category: "unsafe-project-state".to_owned(),
            message: format!(
                "cannot resolve state parent {}: {error}",
                existing_parent.display()
            ),
        })?;
        if !parent.starts_with(root) {
            return Err(AdapterError {
                category: "unsafe-project-state".to_owned(),
                message: format!("path escapes project root: {}", path.display()),
            });
        }
    }
    Ok(path)
}

fn io_error(error: std::io::Error) -> AdapterError {
    AdapterError {
        category: "filesystem".to_owned(),
        message: error.to_string(),
    }
}
