use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub id: String,
    pub name: String,
    pub script_type: ScriptType,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScriptType {
    ShipBrain,
    MothershipBrain,
    Production,
}

impl ScriptType {
    pub fn as_str(&self) -> &str {
        match self {
            ScriptType::ShipBrain => "ship_brain",
            ScriptType::MothershipBrain => "mothership_brain",
            ScriptType::Production => "production",
        }
    }
}

pub struct ScriptStore {
    pub scripts: HashMap<String, Script>,
    pub scripts_dir: PathBuf,
}

impl ScriptStore {
    pub fn new(scripts_dir: PathBuf) -> Self {
        let mut store = Self {
            scripts: HashMap::new(),
            scripts_dir,
        };
        store.load_all();
        store
    }

    pub fn load_all(&mut self) {
        let dir = &self.scripts_dir;
        if !dir.exists() {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "vs") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let id = uuid::Uuid::new_v4().to_string();
                        let script_type = Self::infer_type(&name);
                        self.scripts.insert(
                            id.clone(),
                            Script {
                                id,
                                name,
                                script_type,
                                content,
                            },
                        );
                    }
                }
            }
        }
    }

    fn infer_type(name: &str) -> ScriptType {
        if name.contains("mothership") {
            ScriptType::MothershipBrain
        } else if name.contains("production") {
            ScriptType::Production
        } else {
            ScriptType::ShipBrain
        }
    }

    pub fn save_script(&mut self, id: &str, content: String) {
        if let Some(script) = self.scripts.get_mut(id) {
            script.content = content.clone();
            let path = self.scripts_dir.join(format!("{}.vs", script.name));
            let _ = std::fs::write(path, content);
        }
    }

    pub fn get_script_infos(&self) -> Vec<crate::ipc::ScriptInfo> {
        self.scripts
            .values()
            .map(|s| crate::ipc::ScriptInfo {
                id: s.id.clone(),
                name: s.name.clone(),
                script_type: s.script_type.as_str().to_string(),
            })
            .collect()
    }
}
