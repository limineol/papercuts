use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::papercut::Papercut;

/// All papercuts live in one JSON Lines file, one report per line.
pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn open() -> Result<Store> {
        Ok(Store {
            path: data_dir()?.join("papercuts.jsonl"),
        })
    }

    /// Appends the papercut built from the next free id. The file stays exclusively
    /// locked from reading the current ids until the new line is written, so
    /// concurrent agents never share an id.
    pub fn append(&self, build: impl FnOnce(u64) -> Papercut) -> Result<Papercut> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
        }
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&self.path)
            .with_context(|| format!("cannot open {}", self.path.display()))?;
        file.lock()?;

        let last_id = self.read(&file)?.iter().map(|p| p.id).max().unwrap_or(0);
        let papercut = build(last_id + 1);
        let mut line = serde_json::to_string(&papercut)?;
        line.push('\n');
        let len = file.metadata()?.len();
        if let Err(err) = file
            .write_all(line.as_bytes())
            .and_then(|()| file.sync_data())
        {
            // Drop a partly written line (e.g. on a full disk) so the store stays readable.
            let _ = file.set_len(len);
            return Err(err.into());
        }
        Ok(papercut)
    }

    pub fn load(&self) -> Result<Vec<Papercut>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(err).with_context(|| format!("cannot open {}", self.path.display()));
            }
        };
        file.lock_shared()?;
        self.read(&file)
    }

    fn read(&self, file: &File) -> Result<Vec<Papercut>> {
        let mut papercuts = Vec::new();
        for (index, line) in BufReader::new(file).lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let papercut = serde_json::from_str(&line)
                .with_context(|| format!("{} line {}", self.path.display(), index + 1))?;
            papercuts.push(papercut);
        }
        Ok(papercuts)
    }
}

fn data_dir() -> Result<PathBuf> {
    let var = |name| env::var_os(name).filter(|value| !value.is_empty());
    if let Some(dir) = var("PAPERCUTS_HOME") {
        return Ok(dir.into());
    }
    if let Some(dir) = var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(dir).join("papercuts"));
    }
    let home = env::home_dir().context("cannot find the home directory; set PAPERCUTS_HOME")?;
    Ok(home.join(".local/share/papercuts"))
}
