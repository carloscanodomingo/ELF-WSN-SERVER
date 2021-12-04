use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
struct ConfigFile {
    devices: HashMap<String, String>,
}
