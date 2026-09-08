use crate::file_replace::replace_file;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const MANAGED_MARKER: &str = "hookdock-hook.exe";
const LEGACY_WINDOWS_PATH: &str = "ping island\\resources\\ping-island-hook.exe";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookInstallState {
    pub title: String,
    pub installed: bool,
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Copy)]
struct HookEventDescriptor {
    name: &'static str,
    matcher: Option<&'static str>,
    timeout: Option<u64>,
}

struct HookProfile {
    title: &'static str,
    relative_path: &'static [&'static str],
    events: &'static [HookEventDescriptor],
}

const CLAUDE_EVENTS: &[HookEventDescriptor] = &[
    HookEventDescriptor {
        name: "UserPromptSubmit",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "PermissionRequest",
        matcher: Some("*"),
        timeout: Some(86_400),
    },
    HookEventDescriptor {
        name: "Notification",
        matcher: Some("*"),
        timeout: None,
    },
    HookEventDescriptor {
        name: "Stop",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "SessionEnd",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "PostToolUseFailure",
        matcher: Some("*"),
        timeout: None,
    },
];

const CODEX_EVENTS: &[HookEventDescriptor] = &[
    HookEventDescriptor {
        name: "SessionStart",
        matcher: Some("startup|resume"),
        timeout: None,
    },
    HookEventDescriptor {
        name: "UserPromptSubmit",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "PermissionRequest",
        matcher: Some(".*"),
        timeout: Some(86_400),
    },
    HookEventDescriptor {
        name: "Stop",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "SessionEnd",
        matcher: None,
        timeout: None,
    },
];

const GEMINI_EVENTS: &[HookEventDescriptor] = &[
    HookEventDescriptor {
        name: "SessionStart",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "SessionEnd",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "AfterAgent",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "Notification",
        matcher: None,
        timeout: None,
    },
    HookEventDescriptor {
        name: "AfterTool",
        matcher: Some(".*"),
        timeout: None,
    },
];

fn profile(source: &str) -> Option<HookProfile> {
    match source {
        "claude" => Some(HookProfile {
            title: "Claude Code",
            relative_path: &[".claude", "settings.json"],
            events: CLAUDE_EVENTS,
        }),
        "codex" => Some(HookProfile {
            title: "Codex",
            relative_path: &[".codex", "hooks.json"],
            events: CODEX_EVENTS,
        }),
        "gemini" => Some(HookProfile {
            title: "Gemini CLI",
            relative_path: &[".gemini", "settings.json"],
            events: GEMINI_EVENTS,
        }),
        _ => None,
    }
}

pub struct HookInstaller {
    home_directory: PathBuf,
    bridge_path: PathBuf,
}

impl HookInstaller {
    pub fn new(home_directory: PathBuf, bridge_path: PathBuf) -> Self {
        Self {
            home_directory,
            bridge_path,
        }
    }

    pub fn generic_command(&self) -> String {
        format!("\"{}\" --source generic", self.bridge_path.display())
    }

    pub fn install(
        &self,
        sources: &[String],
    ) -> Result<BTreeMap<String, HookInstallState>, String> {
        if !self.bridge_path.is_file() {
            return Err(format!(
                "找不到 Hook Bridge：{}",
                self.bridge_path.display()
            ));
        }
        for source in sources {
            let profile = profile(source).ok_or_else(|| format!("不支持的 Hook 来源：{source}"))?;
            let path = self.profile_path(&profile);
            let current = read_configuration(&path)?;
            write_configuration(&path, merge_profile(current, source, &self.bridge_path)?)?;
        }
        Ok(self.status())
    }

    pub fn uninstall(
        &self,
        sources: &[String],
    ) -> Result<BTreeMap<String, HookInstallState>, String> {
        for source in sources {
            let profile = profile(source).ok_or_else(|| format!("不支持的 Hook 来源：{source}"))?;
            let path = self.profile_path(&profile);
            if path.exists() {
                write_configuration(&path, remove_managed_entries(read_configuration(&path)?))?;
            }
        }
        Ok(self.status())
    }

    pub fn status(&self) -> BTreeMap<String, HookInstallState> {
        ["claude", "codex", "gemini"]
            .into_iter()
            .map(|source| {
                let profile = profile(source).expect("static hook profile");
                let path = self.profile_path(&profile);
                let result = read_configuration(&path);
                let (installed, error) = match result {
                    Ok(configuration) => (configuration_has_managed_entry(&configuration), None),
                    Err(error) => (false, Some(error)),
                };
                (
                    source.to_owned(),
                    HookInstallState {
                        title: profile.title.to_owned(),
                        installed,
                        file_path: path.display().to_string(),
                        error,
                    },
                )
            })
            .collect()
    }

    fn profile_path(&self, profile: &HookProfile) -> PathBuf {
        profile
            .relative_path
            .iter()
            .fold(self.home_directory.clone(), |path, component| {
                path.join(component)
            })
    }
}

fn read_configuration(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Ok(json!({}));
    }
    let configuration: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} 不是有效 JSON：{error}", path.display()))?;
    if !configuration.is_object() {
        return Err(format!("{} 不是 JSON 对象，未进行修改", path.display()));
    }
    Ok(configuration)
}

fn command_from_entry(entry: &Value) -> Option<&str> {
    let object = entry.as_object()?;
    if let Some(command) = object.get("command").and_then(Value::as_str) {
        return Some(command);
    }
    object
        .get("hooks")
        .and_then(Value::as_array)?
        .iter()
        .find_map(command_from_entry)
}

fn is_managed_entry(entry: &Value) -> bool {
    command_from_entry(entry).is_some_and(|command| {
        let command = command.to_ascii_lowercase().replace('/', "\\");
        command.contains(MANAGED_MARKER) || command.contains(LEGACY_WINDOWS_PATH)
    })
}

fn configuration_has_managed_entry(configuration: &Value) -> bool {
    configuration
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|hooks| {
            hooks.values().any(|entries| {
                entries
                    .as_array()
                    .is_some_and(|entries| entries.iter().any(is_managed_entry))
            })
        })
}

fn remove_managed_entries(mut configuration: Value) -> Value {
    let Some(root) = configuration.as_object_mut() else {
        return configuration;
    };
    let mut remove_hooks = false;
    if let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) {
        hooks.retain(|_, entries| {
            let Some(entries) = entries.as_array_mut() else {
                return true;
            };
            entries.retain(|entry| !is_managed_entry(entry));
            !entries.is_empty()
        });
        remove_hooks = hooks.is_empty();
    }
    if remove_hooks {
        root.remove("hooks");
    }
    configuration
}

fn merge_profile(
    mut configuration: Value,
    source: &str,
    bridge_path: &Path,
) -> Result<Value, String> {
    let profile = profile(source).ok_or_else(|| format!("不支持的 Hook 来源：{source}"))?;
    configuration = remove_managed_entries(configuration);
    let root = configuration
        .as_object_mut()
        .ok_or_else(|| "Hook 配置必须是 JSON 对象".to_owned())?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "现有 hooks 字段不是 JSON 对象，未进行修改".to_owned())?;
    let command = format!("\"{}\" --source {source}", bridge_path.display());
    for descriptor in profile.events {
        let entries = hooks
            .entry(descriptor.name)
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| format!("现有 {} Hook 不是数组，未进行修改", descriptor.name))?;
        let mut command_entry = json!({ "type": "command", "command": command });
        if let Some(timeout) = descriptor.timeout {
            command_entry["timeout"] = json!(timeout);
        }
        let mut entry = json!({ "hooks": [command_entry] });
        if let Some(matcher) = descriptor.matcher {
            entry["matcher"] = json!(matcher);
        }
        entries.push(entry);
    }
    Ok(configuration)
}

fn write_configuration(path: &Path, configuration: Value) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "无效的 Hook 配置路径".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let backup = path.with_extension("hookdock.backup");
    if path.exists() && !backup.exists() {
        fs::copy(path, &backup).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("hookdock.tmp");
    let mut bytes = serde_json::to_vec_pretty(&configuration).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    replace_file(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn merge_preserves_unrelated_settings_and_hooks() {
        let original = json!({
            "model": "opus",
            "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "keep-me.exe" }] }] }
        });
        let merged = merge_profile(
            original.clone(),
            "claude",
            Path::new("C:\\HookDock\\hookdock-hook.exe"),
        )
        .unwrap();
        assert_eq!(merged["model"], "opus");
        assert_eq!(merged["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert_eq!(original["hooks"]["Stop"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn removal_keeps_other_hook_commands() {
        let merged = merge_profile(
            json!({ "hooks": { "Notification": [{ "hooks": [{ "type": "command", "command": "custom.exe" }] }] } }),
            "gemini",
            Path::new("C:\\HookDock\\hookdock-hook.exe"),
        )
        .unwrap();
        let removed = remove_managed_entries(merged);
        assert_eq!(
            removed["hooks"]["Notification"].as_array().unwrap().len(),
            1
        );
        assert!(removed["hooks"].get("AfterAgent").is_none());
    }

    #[test]
    fn migration_removes_only_known_windows_preview_entry() {
        let configuration = json!({
            "hooks": {
                "Notification": [
                    { "hooks": [{ "type": "command", "command": "C:\\Program Files\\Ping Island\\resources\\ping-island-hook.exe --source generic" }] },
                    { "hooks": [{ "type": "command", "command": "C:\\tools\\ping-island-hook.exe" }] }
                ]
            }
        });
        let migrated = remove_managed_entries(configuration);
        let entries = migrated["hooks"]["Notification"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            command_from_entry(&entries[0]),
            Some("C:\\tools\\ping-island-hook.exe")
        );
    }

    #[test]
    fn installer_round_trip_preserves_file_content() {
        let directory = tempdir().unwrap();
        let bridge = directory.path().join("hookdock-hook.exe");
        fs::write(&bridge, b"bridge").unwrap();
        let config = directory.path().join(".claude").join("settings.json");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(&config, br#"{"theme":"dark"}"#).unwrap();
        let installer = HookInstaller::new(directory.path().to_owned(), bridge);

        installer.install(&["claude".to_owned()]).unwrap();
        assert!(installer.status()["claude"].installed);
        installer.uninstall(&["claude".to_owned()]).unwrap();
        assert!(!installer.status()["claude"].installed);
        assert_eq!(read_configuration(&config).unwrap()["theme"], "dark");
    }
}
