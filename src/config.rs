use serde_json::Value;

use crate::scene::SceneParameters;

pub struct SimConfig {
    pub n_worms: usize,
    pub n_rewards: usize,
    pub scene_params: SceneParameters,
    pub interval: u64,
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
            interval: 200,
        }
    }
}

impl SimConfig {
    pub fn read_default() -> Self {
        let default_conf_file = "./conf/default.json";
        Self::from_json(default_conf_file).unwrap_or_else(|error| {
            println!("Error loading default configuration file {default_conf_file}:\n{error}");
            SimConfig::default()
        })
    }

    fn from_json(file_path: &str) -> Result<Self, String> {
        let file_content = std::fs::read_to_string(file_path).map_err(|err| format!("{err}"))?;

        let json_config =
            serde_json::from_str::<Value>(&file_content).map_err(|err| format!("{err}"))?;

        let get_int_attr = |attr: &str| {
            json_config[attr]
                .as_u64()
                .ok_or_else(|| format!("Error reading {attr}"))
        };
        let get_float_attr = |attr: &str| {
            json_config[attr]
                .as_f64()
                .ok_or_else(|| format!("Error reading {attr}"))
        };

        Ok(Self {
            n_worms: get_int_attr("n_worms")? as _,
            n_rewards: get_int_attr("n_rewards")? as _,
            scene_params: SceneParameters {
                worm_size: get_int_attr("worm_size")? as _,
                body_size: get_float_attr("part_size")? as _,
                starvation: get_int_attr("starvation")? as _,
                expiration: get_int_attr("expiration")? as _,
            },
            interval: get_int_attr("milisec")? as _,
        })
    }
}
