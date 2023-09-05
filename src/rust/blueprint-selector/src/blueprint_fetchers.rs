use std::{fmt::Display, path::Path, rc::Rc};
use anyhow::{anyhow, Result};
use strum::{IntoEnumIterator, EnumIter};
use reqwest::{Url, blocking,};
use tempdir::TempDir;
use git2::Repository;

#[derive(Debug, EnumIter)]
pub enum FetcherKind {
    HttpSingleFile,
    Git
}

pub struct Fetcher {
    kind: FetcherKind,
    uri: Url,
    output_dir: Rc<Path>
}

impl Display for FetcherKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match  self {
            FetcherKind::HttpSingleFile => write!(f, "Single Blueprint File Over HTTP"),
            FetcherKind::Git => write!(f, "Clone a Git repository over HTTP(S)")
        }
    }
}

pub fn copy_recursively(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> Result<()> {
    std::fs::create_dir_all(&destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if filetype.is_dir() {
            copy_recursively(entry.path(), destination.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), destination.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

impl FetcherKind {
    pub fn get_variants_list() -> Vec<FetcherKind> {
        FetcherKind::iter().collect()
    }
}

impl Fetcher {
    pub fn new(kind: FetcherKind, uri: &str, output_dir: &Path)-> Result<Self> {
        let output_dir = Rc::from(std::fs::canonicalize(output_dir)?);
        let uri = Url::parse(uri)?;
        Ok(Fetcher { kind, uri, output_dir})
    }

    fn http_single_file(self) -> Result<()> {
        // use the last portion of  the path in the url as the filename (similar to wget)
        let filename = self.uri.path().rsplit("/").next().ok_or_else(|| anyhow!("No file name in path"))?;
        let filename = String::from(filename);
        let request_body = blocking::get(self.uri)?.text()?;
        std::fs::write(self.output_dir.join(filename), request_body)?;
        Ok(())
    }

    fn git_repo(self) -> Result<()> {
        let temp_dir = TempDir::new("blueprints_repo")?;
        println!("Cloning into repository.");
        let _repo = Repository::clone(self.uri.as_str(), &temp_dir)?;
        copy_recursively(&temp_dir, self.output_dir)?;
        // temp_dir is a RAII object and is deleted on instance Drop
        Ok(())
    }

    // move out of self, preventing multiple use of he same fetcher
    pub fn fetch(self) -> Result<()> {
        match self.kind {
            FetcherKind::HttpSingleFile => self.http_single_file()?,
            FetcherKind::Git => self.git_repo()?,
        }

        Ok(())
    }

}

