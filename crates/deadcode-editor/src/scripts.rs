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
    UnitBrain,
    Behavior,
    /// Brain type script — drives entity execution.
    TypeBrain,
    /// Non-brain type script — provides library functions.
    TypeLibrary,
}

impl ScriptType {
    pub fn as_str(&self) -> &str {
        match self {
            ScriptType::UnitBrain => "unit_brain",
            ScriptType::Behavior => "behavior",
            ScriptType::TypeBrain => "type_brain",
            ScriptType::TypeLibrary => "type_library",
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
        // Load top-level .gs files.
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "gs") {
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
        // Load type scripts from types/ subdirectory.
        let types_dir = dir.join("types");
        if types_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&types_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "gs") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let name = path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let id = uuid::Uuid::new_v4().to_string();
                            // Type scripts infer brain vs library from registered type defs.
                            // Default to TypeBrain; app layer can override.
                            let script_type = ScriptType::TypeBrain;
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
    }

    fn infer_type(name: &str) -> ScriptType {
        if name.contains("behavior") {
            ScriptType::Behavior
        } else {
            ScriptType::UnitBrain
        }
    }

    pub fn save_script(&mut self, id: &str, content: String) {
        if let Some(script) = self.scripts.get_mut(id) {
            script.content = content.clone();
            // Type scripts live in types/ subdirectory.
            let path = match script.script_type {
                ScriptType::TypeBrain | ScriptType::TypeLibrary => {
                    self.scripts_dir.join("types").join(format!("{}.gs", script.name))
                }
                _ => self.scripts_dir.join(format!("{}.gs", script.name)),
            };
            let _ = std::fs::write(path, content);
        }
    }

    /// Ensure type scripts exist in `scripts/types/` directory.
    /// Creates files from mod defaults if they don't already exist.
    /// `type_defs` maps type name → (is_brain, default_source).
    pub fn ensure_type_scripts(
        &mut self,
        type_defs: &[(String, bool, String)], // (name, is_brain, default_source)
    ) {
        let types_dir = self.scripts_dir.join("types");
        let _ = std::fs::create_dir_all(&types_dir);

        for (name, is_brain, default_source) in type_defs {
            let path = types_dir.join(format!("{name}.gs"));
            if !path.exists() {
                let _ = std::fs::write(&path, default_source);
            }
            // Check if we already loaded this script.
            let already_loaded = self.scripts.values().any(|s| {
                s.name == *name && matches!(s.script_type, ScriptType::TypeBrain | ScriptType::TypeLibrary)
            });
            if !already_loaded {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let id = uuid::Uuid::new_v4().to_string();
                let script_type = if *is_brain { ScriptType::TypeBrain } else { ScriptType::TypeLibrary };
                self.scripts.insert(
                    id.clone(),
                    Script {
                        id,
                        name: name.clone(),
                        script_type,
                        content,
                    },
                );
            }
        }
    }

    /// Find a type script by its type name.
    pub fn find_type_script(&self, type_name: &str) -> Option<&Script> {
        self.scripts.values().find(|s| {
            s.name == type_name
                && matches!(s.script_type, ScriptType::TypeBrain | ScriptType::TypeLibrary)
        })
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
