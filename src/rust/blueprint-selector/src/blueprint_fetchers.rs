// ********************************************************************************
// * Copyright (c) 2023 Contributors to the Eclipse Foundation
// *
// * See the NOTICE file(s) distributed with this work for additional
// * information regarding copyright ownership.
// *
// * This program and the accompanying materials are made available under the
// * terms of the Apache License 2.0 which is available at
// * https://www.apache.org/licenses/LICENSE-2.0
// *
// * SPDX-License-Identifier: Apache-2.0
// ********************************************************************************

use anyhow::{anyhow, Result};
use git2::Repository;
use reqwest::{blocking, Url};
use std::{fmt::Display, fs, path::Path, rc::Rc};
use strum::{EnumIter, IntoEnumIterator};
use tempfile::tempdir;

#[derive(Debug, EnumIter)]
pub enum FetcherKind {
    HttpSingleFile,
    Git,
    FileSystemFolder,
}

pub struct Fetcher {
    kind: FetcherKind,
    uri: Url,
    output_dir: Rc<Path>,
}

impl Display for FetcherKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetcherKind::HttpSingleFile => write!(f, "Single Blueprint File Over HTTP"),
            FetcherKind::Git => write!(f, "Clone a Git repository over HTTP(S)"),
            FetcherKind::FileSystemFolder => write!(
                f,
                "Directory on the filesystem (absolute path prefixed with file://)"
            ),
        }
    }
}

fn copy_dir_recursive(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(entry.path(), dest.as_ref().join(entry.file_name()))?;
        } else {
            // better error traces on which file/entry the copy failed
            match fs::copy(entry.path(), dest.as_ref().join(entry.file_name())) {
                Ok(_) => continue,
                Err(err) => {
                    return Err(anyhow!("Failed when copying {:#?} with {:#?}", entry, err))
                }
            }
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
    pub fn new(kind: FetcherKind, uri: &str, output_dir: &Path) -> Result<Self> {
        let output_dir = Rc::from(std::fs::canonicalize(output_dir)?);
        let uri = Url::parse(uri)?;
        Ok(Fetcher {
            kind,
            uri,
            output_dir,
        })
    }

    fn http_single_file(self) -> Result<()> {
        // use the last portion of  the path in the url as the filename (similar to wget)
        let filename = self
            .uri
            .path()
            .rsplit('/')
            .next()
            .ok_or_else(|| anyhow!("No file name in path"))?;
        let filename = String::from(filename);
        let request_body = blocking::get(self.uri)?.text()?;
        std::fs::write(self.output_dir.join(filename), request_body)?;
        Ok(())
    }

    fn git_repo(self) -> Result<()> {
        let temp_dir = tempdir()?;
        println!("Cloning into repository.");
        let _repo = Repository::clone(self.uri.as_str(), &temp_dir)?;
        copy_dir_recursive(&temp_dir, self.output_dir)?;
        // temp_dir is a RAII object and is deleted on instance Drop
        Ok(())
    }

    fn local_folder(self) -> Result<()> {
        let source = fs::canonicalize(self.uri.path())?;
        copy_dir_recursive(source, self.output_dir)?;

        Ok(())
    }

    // move out of self, preventing multiple use of he same fetcher
    pub fn fetch(self) -> Result<()> {
        match self.kind {
            FetcherKind::HttpSingleFile => self.http_single_file()?,
            FetcherKind::Git => self.git_repo()?,
            FetcherKind::FileSystemFolder => self.local_folder()?,
        }

        Ok(())
    }
}
