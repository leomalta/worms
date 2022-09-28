use serde_json::Value;

use crate::scene::SceneParameters;

pub struct SimConfig {
    pub n_worms: usize,
    pub n_rewards: usize,
    pub scene_params: SceneParameters,
    pub milisec: u64,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            n_worms: 15,
            n_rewards: 5,
            scene_params: SceneParameters {
                worm_size: 8,
                body_size: 7.0,
                starvation: 2000,
                expiration: 25,
            },
            milisec: 200,
        }
    }
}

pub enum ReadConfigError {
    FileRead(String, std::io::Error),
    FileParse(String, serde_json::Error),
    FileContent(String, String),
}

impl std::fmt::Display for ReadConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match self {
            ReadConfigError::FileRead(path, error) => {
                format!("ERROR: Couldn't open file {path}. \n{error}")
            }
            ReadConfigError::FileParse(path, error) => {
                format!("ERROR: Couldn't parse file {path}. \n{error}")
            }
            ReadConfigError::FileContent(path, param) => {
                format!("ERROR: Couldn't parse parameter '{param}' on file '{path}'")
            }
        };
        write!(f, "{result}")
    }
}

impl SimConfig {
    pub fn from_json(file_path: &str) -> Result<Self, ReadConfigError> {
        let file_content = std::fs::read_to_string(file_path)
            .map_err(|x| ReadConfigError::FileRead(file_path.to_string(), x))?;

        let json_config = serde_json::from_str::<Value>(&file_content)
            .map_err(|x| ReadConfigError::FileParse(file_path.to_string(), x))?;

        let scene_params = SceneParameters {
            worm_size: json_config["worm_size"].as_u64().ok_or_else(|| {
                ReadConfigError::FileContent(file_path.to_string(), "worm_size".to_owned())
            })? as _,
            body_size: json_config["part_size"].as_f64().ok_or_else(|| {
                ReadConfigError::FileContent(file_path.to_string(), "part_size".to_owned())
            })? as _,
            starvation: json_config["starvation"].as_u64().ok_or_else(|| {
                ReadConfigError::FileContent(file_path.to_string(), "starvation".to_owned())
            })? as _,
            expiration: json_config["expiration"].as_u64().ok_or_else(|| {
                ReadConfigError::FileContent(file_path.to_string(), "expiration".to_owned())
            })? as _,
        };

        Ok(Self {
            n_worms: json_config["n_worms"].as_u64().ok_or_else(|| {
                ReadConfigError::FileContent(file_path.to_string(), "n_worms".to_owned())
            })? as _,
            n_rewards: json_config["n_rewards"].as_u64().ok_or_else(|| {
                ReadConfigError::FileContent(file_path.to_string(), "n_rewards".to_owned())
            })? as _,
            scene_params,
            milisec: json_config["milisec"].as_u64().ok_or_else(|| {
                ReadConfigError::FileContent(file_path.to_string(), "milisec".to_owned())
            })? as _,
        })
    }
}
