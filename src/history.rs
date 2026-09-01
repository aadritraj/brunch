use std::{
    collections::HashMap,
    fs, io,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

const HALF_LIFE_DAYS: f64 = 30.0;
const LAMBDA: f64 = std::f64::consts::LN_2 / HALF_LIFE_DAYS;
const FILE_VERSION: u32 = 1;
const SECONDS_PER_DAY: f64 = 86_400.0;

#[derive(Debug, Default, Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    launches: HashMap<String, Vec<i64>>,
}

#[derive(Debug, Default)]
pub struct History {
    path: Option<PathBuf>,
    launches: HashMap<String, Vec<i64>>,
}

impl History {
    pub fn load(path: Option<PathBuf>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
            Err(error) => {
                eprintln!(
                    "warning: could not read launch history {}: {error}",
                    path.display()
                );
                return Self {
                    path: Some(path),
                    ..Self::default()
                };
            }
        };
        if contents.is_empty() {
            return Self {
                path: Some(path),
                ..Self::default()
            };
        }

        match serde_json::from_str::<HistoryFile>(&contents) {
            Ok(file) if file.version == FILE_VERSION => Self {
                path: Some(path),
                launches: file.launches,
            },
            Ok(file) => {
                eprintln!(
                    "warning: unsupported launch history version {}",
                    file.version
                );
                Self {
                    path: Some(path),
                    ..Self::default()
                }
            }
            Err(error) => {
                eprintln!(
                    "warning: could not parse launch history {}: {error}",
                    path.display()
                );
                Self {
                    path: Some(path),
                    ..Self::default()
                }
            }
        }
    }

    pub fn record_launch(&mut self, appid: &str) {
        self.record_launch_at(appid, unix_seconds());
    }

    pub fn record_launch_at(&mut self, appid: &str, timestamp: i64) {
        self.launches
            .entry(appid.to_owned())
            .or_default()
            .push(timestamp);
    }

    pub fn frecency(&self, appid: &str) -> f64 {
        self.launches
            .get(appid)
            .map_or(0.0, |launches| frecency_at(launches))
    }

    // returns decayed launch count, sum of decayed launches
    // this is used to boost search results based on launch frequency
    pub fn decayed_launches(&self, appid: &str) -> f64 {
        let now = unix_seconds();
        self.launches.get(appid).map_or(0.0, |launches| {
            launches
                .iter()
                .map(|launch| {
                    let age_days = (now - *launch).max(0) as f64 / SECONDS_PER_DAY;
                    (-LAMBDA * age_days).exp()
                })
                .sum()
        })
    }

    pub fn persist(&self) -> io::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let file = HistoryFile {
            version: FILE_VERSION,
            launches: self.launches.clone(),
        };
        let serialized = serde_json::to_vec_pretty(&file).map_err(io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serialized)?;
        fs::rename(temporary, path)
    }
}

fn frecency_at(launches: &[i64]) -> f64 {
    let Some(reference) = launches.iter().copied().max() else {
        return 0.0;
    };
    let score = launches
        .iter()
        .map(|launch| {
            let age_days = (reference - *launch).max(0) as f64 / SECONDS_PER_DAY;
            (-LAMBDA * age_days).exp()
        })
        .sum::<f64>()
        / launches.len() as f64
        * launches.len() as f64;
    reference as f64 / SECONDS_PER_DAY + score.ln() / LAMBDA
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
