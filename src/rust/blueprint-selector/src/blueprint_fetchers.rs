use std::{fmt::Display, path::Path, rc::Rc};
use anyhow::{anyhow, Result};
use strum::{IntoEnumIterator, EnumIter};
use reqwest::{Url, blocking};

#[derive(Debug, EnumIter)]
pub enum FetcherKind {
    HttpSingleFile,
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
        }
    }
}

impl FetcherKind {
    pub fn get_variants_list() -> Vec<FetcherKind> {
        FetcherKind::iter().collect()
    }
}

impl Fetcher {
    pub fn new(kind: FetcherKind, uri: &str, output_dir: &Path)-> Result<Self> {
        let output_dir = Rc::from(output_dir);
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

    // move out of self, preventing multiple use of he same fetcher
    pub fn fetch(self) -> Result<()> {
        match self.kind {
            FetcherKind::HttpSingleFile => self.http_single_file()?
        }

        Ok(())
    }

}

