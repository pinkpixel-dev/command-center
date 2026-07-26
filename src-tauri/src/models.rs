use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// What kind of entry this is. The UI calls them all "entries", the database
/// keeps the distinction so filtering and syntax highlighting stay accurate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandKind {
    #[default]
    Command,
    Script,
    Sequence,
    Snippet,
    Reference,
}

impl CommandKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Script => "script",
            Self::Sequence => "sequence",
            Self::Snippet => "snippet",
            Self::Reference => "reference",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "command" => Ok(Self::Command),
            "script" => Ok(Self::Script),
            "sequence" => Ok(Self::Sequence),
            "snippet" => Ok(Self::Snippet),
            "reference" => Ok(Self::Reference),
            other => Err(AppError::invalid(format!("unknown command kind: {other}"))),
        }
    }
}

/// Informational risk label. Never blocks anything, only warns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    #[default]
    Safe,
    Caution,
    Destructive,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Caution => "caution",
            Self::Destructive => "destructive",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "safe" => Ok(Self::Safe),
            "caution" => Ok(Self::Caution),
            "destructive" => Ok(Self::Destructive),
            other => Err(AppError::invalid(format!("unknown risk level: {other}"))),
        }
    }
}

/// A full entry with its tags and collection memberships resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub description: String,
    pub kind: CommandKind,
    pub language: Option<String>,
    pub shell: Option<String>,
    pub operating_system: Option<String>,
    pub risk_level: RiskLevel,
    pub risk_reasons: Vec<String>,
    pub favorite: bool,
    pub working_directory: Option<String>,
    pub source_url: Option<String>,
    pub notes: String,
    pub content_hash: String,
    pub copy_count: i64,
    pub last_copied_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub collections: Vec<CollectionRef>,
    pub variables: Vec<String>,
}

impl Command {
    /// Turns a stored entry back into an editable input. Used by the importer
    /// when merging an incoming entry into one that already exists.
    pub fn to_input(&self) -> CommandInput {
        CommandInput {
            title: self.title.clone(),
            content: self.content.clone(),
            description: self.description.clone(),
            kind: self.kind,
            language: self.language.clone(),
            shell: self.shell.clone(),
            operating_system: self.operating_system.clone(),
            risk_level: Some(self.risk_level),
            favorite: self.favorite,
            working_directory: self.working_directory.clone(),
            source_url: self.source_url.clone(),
            notes: self.notes.clone(),
            tags: self.tags.clone(),
            collection_ids: self.collections.iter().map(|entry| entry.id).collect(),
        }
    }
}

/// Lightweight collection reference embedded in a command payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionRef {
    pub id: i64,
    pub name: String,
}

/// What the frontend sends when creating or updating an entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInput {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: CommandKind,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub shell: Option<String>,
    #[serde(default)]
    pub operating_system: Option<String>,
    /// `None` means "let the local risk rules decide".
    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub working_directory: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub collection_ids: Vec<i64>,
}

impl CommandInput {
    /// Trims everything, drops empty optionals, and rejects the two fields we
    /// genuinely cannot store without.
    pub fn normalized(mut self) -> AppResult<Self> {
        self.title = self.title.trim().to_string();
        self.content = self.content.trim_end().to_string();
        self.description = self.description.trim().to_string();
        self.notes = self.notes.trim().to_string();
        self.language = clean_optional(self.language);
        self.shell = clean_optional(self.shell);
        self.operating_system = clean_optional(self.operating_system);
        self.working_directory = clean_optional(self.working_directory);
        self.source_url = clean_optional(self.source_url);

        if self.content.trim().is_empty() {
            return Err(AppError::invalid("A command needs some content"));
        }
        if self.title.is_empty() {
            self.title = derive_title(&self.content);
        }

        let mut tags: Vec<String> = self
            .tags
            .iter()
            .map(|tag| tag.trim().to_lowercase())
            .filter(|tag| !tag.is_empty())
            .collect();
        tags.sort();
        tags.dedup();
        self.tags = tags;

        self.collection_ids.sort_unstable();
        self.collection_ids.dedup();

        Ok(self)
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|inner| inner.trim().to_string())
        .filter(|inner| !inner.is_empty())
}

/// Falls back to the first meaningful line when the user did not name the entry.
fn derive_title(content: &str) -> String {
    let candidate = content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or("Untitled command");

    let mut title: String = candidate.chars().take(72).collect();
    if candidate.chars().count() > 72 {
        title.push('…');
    }
    title
}

/// Sidebar counts, refreshed after every mutation.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub total: i64,
    pub favorites: i64,
    pub scripts: i64,
    pub recent: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub command_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub command_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

impl CollectionInput {
    pub fn normalized(mut self) -> AppResult<Self> {
        self.name = self.name.trim().to_string();
        self.description = self.description.trim().to_string();
        if self.name.is_empty() {
            return Err(AppError::invalid("A collection needs a name"));
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(title: &str, content: &str) -> CommandInput {
        CommandInput {
            title: title.to_string(),
            content: content.to_string(),
            description: String::new(),
            kind: CommandKind::Command,
            language: None,
            shell: None,
            operating_system: None,
            risk_level: None,
            favorite: false,
            working_directory: None,
            source_url: None,
            notes: String::new(),
            tags: vec![],
            collection_ids: vec![],
        }
    }

    #[test]
    fn empty_content_is_rejected() {
        let result = input("Something", "   \n  ").normalized();
        assert!(result.is_err());
    }

    #[test]
    fn missing_title_falls_back_to_first_line() {
        let normalized = input("  ", "# a comment\ngit status\n").normalized().unwrap();
        assert_eq!(normalized.title, "git status");
    }

    #[test]
    fn long_derived_titles_are_truncated() {
        let long = "echo ".to_string() + &"a".repeat(200);
        let normalized = input("", &long).normalized().unwrap();
        assert_eq!(normalized.title.chars().count(), 73);
        assert!(normalized.title.ends_with('…'));
    }

    #[test]
    fn tags_are_lowercased_sorted_and_deduped() {
        let mut raw = input("t", "ls");
        raw.tags = vec![" Docker ".into(), "docker".into(), "Cleanup".into(), " ".into()];
        let normalized = raw.normalized().unwrap();
        assert_eq!(normalized.tags, vec!["cleanup", "docker"]);
    }

    #[test]
    fn blank_optionals_become_none() {
        let mut raw = input("t", "ls");
        raw.shell = Some("   ".into());
        raw.source_url = Some(" https://example.com ".into());
        let normalized = raw.normalized().unwrap();
        assert_eq!(normalized.shell, None);
        assert_eq!(normalized.source_url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn kind_and_risk_round_trip() {
        for kind in [
            CommandKind::Command,
            CommandKind::Script,
            CommandKind::Sequence,
            CommandKind::Snippet,
            CommandKind::Reference,
        ] {
            assert_eq!(CommandKind::parse(kind.as_str()).unwrap(), kind);
        }
        for risk in [RiskLevel::Safe, RiskLevel::Caution, RiskLevel::Destructive] {
            assert_eq!(RiskLevel::parse(risk.as_str()).unwrap(), risk);
        }
        assert!(CommandKind::parse("nonsense").is_err());
        assert!(RiskLevel::parse("nonsense").is_err());
    }
}
