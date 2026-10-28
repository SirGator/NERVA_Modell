use crate::{Result, error, fingerprint};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub struct ResultCollector {
    root: PathBuf,
    _lock: File,
}
impl ResultCollector {
    pub fn open(root: &Path, plan: &Value) -> Result<Self> {
        fs::create_dir_all(root).map_err(|e| error(e.to_string()))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(".lock"))
            .map_err(|e| error(e.to_string()))?;
        lock.try_lock()
            .map_err(|e| error(format!("output is locked: {e}")))?;
        let result = Self {
            root: root.to_owned(),
            _lock: lock,
        };
        let path = root.join("manifest.json");
        if path.exists() {
            if result.read(&path)? != *plan {
                return Err(error(
                    "existing manifest differs; choose a new output directory",
                ));
            }
        } else {
            if fs::read_dir(root)
                .map_err(|e| error(e.to_string()))?
                .any(|entry| entry.map(|e| e.file_name() != ".lock").unwrap_or(true))
            {
                return Err(error("nonempty output has no matching manifest"));
            }
            result.write_json("manifest.json", plan)?;
        }
        fs::create_dir_all(root.join("records")).map_err(|e| error(e.to_string()))?;
        Ok(result)
    }
    fn read(&self, path: &Path) -> Result<Value> {
        serde_json::from_slice(&fs::read(path).map_err(|e| error(e.to_string()))?)
            .map_err(|e| error(e.to_string()))
    }
    pub fn write_json(&self, relative: &str, value: &Value) -> Result<()> {
        self.write(
            relative,
            &serde_json::to_vec_pretty(value).map_err(|e| error(e.to_string()))?,
        )
    }
    pub fn write(&self, relative: &str, bytes: &[u8]) -> Result<()> {
        let path = self.root.join(relative);
        let partial = self.root.join(format!("{relative}.partial"));
        let mut file = File::create(&partial).map_err(|e| error(e.to_string()))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| error(e.to_string()))?;
        drop(file);
        fs::rename(partial, path).map_err(|e| error(e.to_string()))
    }
    pub fn save(&self, id: &str, record: &Value) -> Result<()> {
        let relative = format!("records/{id}.json");
        if self.root.join(&relative).exists() {
            return Err(error("refusing to overwrite an existing trial"));
        }
        let checksum = fingerprint(&serde_json::to_vec(record).unwrap());
        self.write_json(
            &relative,
            &json!({"checksum_fnv1a64":checksum,"record":record}),
        )
    }
    pub fn cached(&self, id: &str, job: &Value) -> Result<Option<Value>> {
        let path = self.root.join("records").join(format!("{id}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let envelope = self.read(&path)?;
        let record = &envelope["record"];
        let checksum = fingerprint(&serde_json::to_vec(record).unwrap());
        if envelope["checksum_fnv1a64"] != checksum || record["job"] != *job {
            return Err(error(format!(
                "cached result integrity/identity differs: {id}"
            )));
        }
        Ok(Some(record.clone()))
    }
}

pub fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
