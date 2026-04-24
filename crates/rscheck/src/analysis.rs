use crate::config::Policy;
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::{fs, io, path::PathBuf};

#[derive(Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
    pub ast: Option<syn::File>,
    pub parse_error: Option<String>,
}

pub struct Workspace {
    pub root: PathBuf,
    pub files: Vec<SourceFile>,
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error("failed to build glob matcher: {pattern}")]
    Glob {
        pattern: String,
        #[source]
        source: globset::Error,
    },
    #[error("failed to read file: {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl Workspace {
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: Vec::new(),
        }
    }

    pub fn load_files(mut self, policy: &Policy) -> Result<Self, DiscoverError> {
        let include = build_globset(&policy.workspace.include)?;
        let exclude = build_globset(&policy.workspace.exclude)?;

        let walker = WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let rel = path.strip_prefix(&self.root).unwrap_or(path);
            if !include.is_match(rel) || exclude.is_match(rel) {
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }

            let text = fs::read_to_string(path).map_err(|source| DiscoverError::Read {
                path: path.to_path_buf(),
                source,
            })?;
            let mut parse_error = None;
            let ast = match syn::parse_file(&text) {
                Ok(ast) => Some(ast),
                Err(err) => {
                    parse_error = Some(parse_error_text(&err));
                    None
                }
            };
            self.files.push(SourceFile {
                path: path.to_path_buf(),
                text,
                ast,
                parse_error,
            });
        }

        Ok(self)
    }
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, DiscoverError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|source| DiscoverError::Glob {
            pattern: pattern_text(pattern),
            source,
        })?;
        builder.add(glob);
    }
    builder.build().map_err(|source| DiscoverError::Glob {
        pattern: "<globset>".to_string(),
        source,
    })
}

fn parse_error_text(err: &syn::Error) -> String {
    err.to_string()
}

fn pattern_text(pattern: &str) -> String {
    pattern.to_string()
}
