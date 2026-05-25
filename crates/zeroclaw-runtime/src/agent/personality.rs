//! Personality system — loads workspace identity and policy files (SOUL.md,
//! IDENTITY.md, USER.md, AGENTS.md, etc.) and injects them into the system
//! prompt pipeline.
//!
//! Ported from RustyClaw `src/agent/personality.rs`.  The loader reads markdown
//! files from the workspace root, validates size limits, and produces a
//! [`PersonalityProfile`] that the prompt builder can render.

use std::fmt::Write;
use std::path::{Path, PathBuf};

/// Maximum characters per personality file before truncation.
pub const MAX_FILE_CHARS: usize = 20_000;

/// Well-known personality files loaded from the workspace root.
pub const PERSONALITY_FILES: &[&str] = &[
    "AGENTS.md",
    "SOUL.md",
    "IDENTITY.md",
    "USER.md",
    "COMMANDS.md",
    "CHANNEL_GUIDE.md",
    "WORKFLOW.md",
    "TOOLS.md",
    "HEARTBEAT.md",
    "BOOTSTRAP.md",
    "MEMORY.md",
];

/// Subset of [`PERSONALITY_FILES`] that the dashboard exposes for
/// authoring. `BOOTSTRAP.md` is deliberately excluded: it's a
/// first-run scaffold the agent reads once and deletes, not a file
/// the user is meant to hand-edit. The runtime still injects it when
/// it exists on disk.
pub const EDITABLE_PERSONALITY_FILES: &[&str] = &[
    "AGENTS.md",
    "SOUL.md",
    "IDENTITY.md",
    "USER.md",
    "COMMANDS.md",
    "CHANNEL_GUIDE.md",
    "WORKFLOW.md",
    "TOOLS.md",
    "HEARTBEAT.md",
    "MEMORY.md",
];

/// A single personality file loaded from the workspace.
#[derive(Debug, Clone)]
pub struct PersonalityFile {
    /// Filename (e.g. `SOUL.md`).
    pub name: String,
    /// Raw content (possibly truncated).
    pub content: String,
    /// Whether the content was truncated due to size limits.
    pub truncated: bool,
    /// Full path on disk.
    pub path: PathBuf,
}

/// Aggregated personality profile loaded from a workspace.
#[derive(Debug, Clone, Default)]
pub struct PersonalityProfile {
    /// Successfully loaded personality files.
    pub files: Vec<PersonalityFile>,
    /// Files that were expected but not found.
    pub missing: Vec<String>,
}

impl PersonalityProfile {
    /// Returns the content of a specific file by name, if loaded.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.content.as_str())
    }

    /// Returns `true` if no personality files were loaded.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Render all loaded personality files into a prompt fragment.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for file in &self.files {
            let _ = writeln!(out, "### {}\n", file.name);
            out.push_str(&file.content);
            if file.truncated {
                let _ = writeln!(
                    out,
                    "\n\n[... truncated at {MAX_FILE_CHARS} chars — use `read` for full file]\n"
                );
            } else {
                out.push_str("\n\n");
            }
        }
        out
    }
}

/// Loads personality files from a workspace directory.
///
/// Each well-known file is read and validated.  Missing files are recorded
/// in `PersonalityProfile::missing` rather than treated as errors.
pub fn load_personality(workspace_dir: &Path) -> PersonalityProfile {
    load_personality_files(workspace_dir, PERSONALITY_FILES)
}

/// Load a specific set of personality files from a workspace directory.
pub fn load_personality_files(workspace_dir: &Path, filenames: &[&str]) -> PersonalityProfile {
    let mut profile = PersonalityProfile::default();

    for &filename in filenames {
        let Some((path, raw)) = read_workspace_personality_file(workspace_dir, filename) else {
            profile.missing.push(filename.to_string());
            continue;
        };

        match raw {
            Ok(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    profile.missing.push(filename.to_string());
                    continue;
                }
                let (content, truncated) = truncate_content(trimmed);
                profile.files.push(PersonalityFile {
                    name: filename.to_string(),
                    content,
                    truncated,
                    path,
                });
            }
            Err(_) => profile.missing.push(filename.to_string()),
        }
    }

    profile
}

fn read_workspace_personality_file(
    workspace_dir: &Path,
    filename: &str,
) -> Option<(PathBuf, std::io::Result<String>)> {
    let primary = workspace_dir.join(filename);
    if primary.exists() || filename != "MEMORY.md" {
        return Some((primary.clone(), std::fs::read_to_string(primary)));
    }

    let nested_memory = workspace_dir.join("memory").join("MEMORY.md");
    Some((
        nested_memory.clone(),
        std::fs::read_to_string(nested_memory),
    ))
}

/// Truncate content to `MAX_FILE_CHARS` if necessary.
fn truncate_content(content: &str) -> (String, bool) {
    if content.chars().count() <= MAX_FILE_CHARS {
        return (content.to_string(), false);
    }
    let truncated = content
        .char_indices()
        .nth(MAX_FILE_CHARS)
        .map(|(idx, _)| &content[..idx])
        .unwrap_or(content);
    (truncated.to_string(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_workspace(files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "zeroclaw_personality_test_{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, content) in files {
            std::fs::write(dir.join(name), content).unwrap();
        }
        dir
    }

    #[test]
    fn load_personality_reads_existing_files() {
        let ws = setup_workspace(&[
            ("SOUL.md", "I am a helpful assistant."),
            ("IDENTITY.md", "Name: Nova"),
        ]);

        let profile = load_personality(&ws);
        assert_eq!(profile.files.len(), 2);
        assert_eq!(profile.get("SOUL.md").unwrap(), "I am a helpful assistant.");
        assert_eq!(profile.get("IDENTITY.md").unwrap(), "Name: Nova");
        assert!(!profile.is_empty());

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn load_personality_records_missing_files() {
        let ws = setup_workspace(&[("SOUL.md", "soul content")]);

        let profile = load_personality(&ws);
        assert_eq!(profile.files.len(), 1);
        assert!(profile.missing.contains(&"IDENTITY.md".to_string()));
        assert!(profile.missing.contains(&"USER.md".to_string()));

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn load_personality_treats_empty_files_as_missing() {
        let ws = setup_workspace(&[("SOUL.md", "   \n  ")]);

        let profile = load_personality(&ws);
        assert!(profile.is_empty());
        assert!(profile.missing.contains(&"SOUL.md".to_string()));

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn load_personality_truncates_large_files() {
        let large = "x".repeat(MAX_FILE_CHARS + 500);
        let ws = setup_workspace(&[("SOUL.md", &large)]);

        let profile = load_personality(&ws);
        let soul = profile.files.iter().find(|f| f.name == "SOUL.md").unwrap();
        assert!(soul.truncated);
        assert_eq!(soul.content.chars().count(), MAX_FILE_CHARS);

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn render_produces_markdown_sections() {
        let ws = setup_workspace(&[("SOUL.md", "Be kind."), ("IDENTITY.md", "Name: Nova")]);

        let profile = load_personality(&ws);
        let rendered = profile.render();
        assert!(rendered.contains("### SOUL.md"));
        assert!(rendered.contains("Be kind."));
        assert!(rendered.contains("### IDENTITY.md"));
        assert!(rendered.contains("Name: Nova"));

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn render_truncated_file_shows_notice() {
        let large = "y".repeat(MAX_FILE_CHARS + 100);
        let ws = setup_workspace(&[("SOUL.md", &large)]);

        let profile = load_personality(&ws);
        let rendered = profile.render();
        assert!(rendered.contains("[... truncated at"));

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn load_personality_reads_policy_files() {
        let ws = setup_workspace(&[
            ("COMMANDS.md", "run commands directly"),
            ("CHANNEL_GUIDE.md", "split bubbles"),
            ("WORKFLOW.md", "verify output"),
        ]);

        let profile = load_personality(&ws);
        assert_eq!(profile.get("COMMANDS.md").unwrap(), "run commands directly");
        assert_eq!(profile.get("CHANNEL_GUIDE.md").unwrap(), "split bubbles");
        assert_eq!(profile.get("WORKFLOW.md").unwrap(), "verify output");

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn load_personality_uses_nested_memory_when_root_missing() {
        let ws = setup_workspace(&[]);
        let nested_dir = ws.join("memory");
        std::fs::create_dir_all(&nested_dir).unwrap();
        std::fs::write(nested_dir.join("MEMORY.md"), "nested memory").unwrap();

        let profile = load_personality(&ws);
        let memory = profile
            .files
            .iter()
            .find(|file| file.name == "MEMORY.md")
            .unwrap();
        assert_eq!(memory.content, "nested memory");
        assert!(memory.path.ends_with("memory/MEMORY.md"));

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn load_personality_prefers_root_memory() {
        let ws = setup_workspace(&[("MEMORY.md", "root memory")]);
        let nested_dir = ws.join("memory");
        std::fs::create_dir_all(&nested_dir).unwrap();
        std::fs::write(nested_dir.join("MEMORY.md"), "nested memory").unwrap();

        let profile = load_personality(&ws);
        let memory = profile
            .files
            .iter()
            .find(|file| file.name == "MEMORY.md")
            .unwrap();
        assert_eq!(memory.content, "root memory");
        assert!(memory.path.ends_with("MEMORY.md"));
        assert!(!memory.path.ends_with("memory/MEMORY.md"));

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn get_returns_none_for_missing_file() {
        let ws = setup_workspace(&[]);
        let profile = load_personality(&ws);
        assert!(profile.get("SOUL.md").is_none());
        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn load_personality_files_custom_subset() {
        let ws = setup_workspace(&[("SOUL.md", "soul"), ("USER.md", "user")]);

        let profile = load_personality_files(&ws, &["SOUL.md", "USER.md"]);
        assert_eq!(profile.files.len(), 2);
        assert!(profile.missing.is_empty());

        let _ = std::fs::remove_dir_all(ws);
    }

    #[test]
    fn empty_workspace_yields_empty_profile() {
        let ws = setup_workspace(&[]);
        let profile = load_personality(&ws);
        assert!(profile.is_empty());
        assert!(!profile.missing.is_empty());
        let _ = std::fs::remove_dir_all(ws);
    }
}
