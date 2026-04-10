//! File-driven action system.
//!
//! Actions are loaded from `action.yaml` files which map user-visible
//! command names (possibly multi-word) to built-in command strings.
//!
//! # `action.yaml` format
//!
//! ```yaml
//! open world: ls .
//! explor: cd $1
//! go home: cd ~
//! quit: quit
//! save: save
//! read: cat $1
//! back: cd ..
//! go: cd $1
//! find: grep $1
//! farm: farm
//! breed: breed
//! rest: rest
//! status: status
//! ```
//!
//! `$1`, `$2`, … are replaced with the actual arguments supplied by the
//! player when they type the command (space-separated tokens after the key).

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// A map of game action names → built-in command strings.
///
/// Keys are action names as typed by the player (may contain spaces).
/// Values are built-in command strings, e.g. `"cd $1"`, `"ls ."`, `"quit"`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActionMap(pub HashMap<String, String>);

/// A parsed, executable built-in command.
#[derive(Debug, Clone)]
pub enum BuiltinCmd {
    /// `ls [path]` — list entities in the current area or a named area.
    Ls { path: Option<String> },
    /// `cd <path>` — change area; special values: `~` (home), `..` (back).
    Cd { path: String },
    /// `cat <file>` — read and display an entity YAML file.
    Cat { file: String },
    /// `echo <content> > <file>` — write text content to a file in the area.
    EchoTo { content: String, file: String },
    /// `grep <pattern>` — search for entities/content matching a pattern.
    Grep { pattern: String },
    /// Open the farm sub-menu.
    Farm,
    /// Open the breed-animals sub-menu.
    Breed,
    /// Rest and heal.
    Rest,
    /// Show player status.
    Status,
    /// Save the game.
    Save,
    /// Quit the game.
    Quit,
}

impl ActionMap {
    /// Load an action map from a YAML file at `path`.
    pub fn load(path: &Path) -> Result<Self, String> {
        let yaml = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let map: HashMap<String, String> =
            serde_yaml::from_str(&yaml).map_err(|e| e.to_string())?;
        Ok(ActionMap(map))
    }

    /// Serialize this action map to a YAML string.
    pub fn to_yaml(&self) -> Result<String, String> {
        // Write entries in a stable, human-friendly order.
        let mut entries: Vec<(&String, &String)> = self.0.iter().collect();
        entries.sort_by_key(|(k, _)| k.as_str());
        let mut out = String::new();
        for (k, v) in &entries {
            out.push_str(&format!("{}: {}\n", k, v));
        }
        Ok(out)
    }

    /// Return the default built-in action map.
    pub fn default_map() -> Self {
        let mut m = HashMap::new();
        m.insert("open world".to_string(), "ls .".to_string());
        m.insert("explor".to_string(), "cd $1".to_string());
        m.insert("go home".to_string(), "cd ~".to_string());
        m.insert("quit".to_string(), "quit".to_string());
        m.insert("save".to_string(), "save".to_string());
        m.insert("read".to_string(), "cat $1".to_string());
        m.insert("back".to_string(), "cd ..".to_string());
        m.insert("go".to_string(), "cd $1".to_string());
        m.insert("find".to_string(), "grep $1".to_string());
        m.insert("farm".to_string(), "farm".to_string());
        m.insert("breed".to_string(), "breed".to_string());
        m.insert("rest".to_string(), "rest".to_string());
        m.insert("status".to_string(), "status".to_string());
        ActionMap(m)
    }

    /// Match player input against the action map and return the corresponding
    /// [`BuiltinCmd`], or `None` if no action matches.
    ///
    /// Matching is case-insensitive. When multiple keys are a prefix of the
    /// input, the longest (most-specific) key wins. Arguments (`$1`, `$2`, …)
    /// are substituted with the remaining input tokens after the matched key.
    pub fn match_input(&self, input: &str) -> Option<BuiltinCmd> {
        let input_tokens: Vec<&str> = input.trim().split_whitespace().collect();
        if input_tokens.is_empty() {
            return None;
        }

        // Find the longest matching action key.
        let mut best: Option<(&str, &str, usize)> = None; // (key, value, key_token_count)

        for (key, value) in &self.0 {
            let key_tokens: Vec<&str> = key.split_whitespace().collect();
            let klen = key_tokens.len();

            if input_tokens.len() < klen {
                continue;
            }

            let prefix_match = input_tokens[..klen]
                .iter()
                .zip(key_tokens.iter())
                .all(|(a, b)| a.eq_ignore_ascii_case(b));

            if prefix_match {
                if best.map_or(true, |(_, _, n)| klen > n) {
                    best = Some((key.as_str(), value.as_str(), klen));
                }
            }
        }

        let (_, builtin_str, klen) = best?;

        // Substitute $1, $2, … with the remaining tokens.
        let args: Vec<&str> = input_tokens[klen..].to_vec();
        let mut substituted = builtin_str.to_string();
        for (i, arg) in args.iter().enumerate() {
            substituted = substituted.replace(&format!("${}", i + 1), arg);
        }

        parse_builtin(&substituted)
    }

    /// Merge another action map into this one (other's keys override ours).
    pub fn merge(&mut self, other: &ActionMap) {
        for (k, v) in &other.0 {
            self.0.insert(k.clone(), v.clone());
        }
    }

    /// Return a sorted list of `(action_name, builtin_str)` pairs for display.
    pub fn display_list(&self) -> Vec<(String, String)> {
        let mut list: Vec<_> = self.0.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        list.sort_by(|a, b| a.0.cmp(&b.0));
        list
    }
}

/// Parse a built-in command string (after argument substitution) into a
/// [`BuiltinCmd`].  Returns `None` for unknown commands.
fn parse_builtin(s: &str) -> Option<BuiltinCmd> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut tokens = s.splitn(3, ' ');
    let cmd = tokens.next().unwrap_or("");

    match cmd {
        "ls" => {
            let path = tokens
                .next()
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty() && p != ".");
            Some(BuiltinCmd::Ls { path })
        }
        "cd" => {
            let path = tokens
                .next()
                .map(|p| p.trim().to_string())
                .unwrap_or_else(|| "~".to_string());
            Some(BuiltinCmd::Cd { path })
        }
        "cat" => {
            let file = tokens
                .next()
                .map(|f| f.trim().to_string())
                .unwrap_or_default();
            Some(BuiltinCmd::Cat { file })
        }
        "echo" => {
            // Expected form: echo <content> > <file>
            if let Some(rest) = s.strip_prefix("echo ") {
                if let Some(arrow_pos) = rest.rfind('>') {
                    let content = rest[..arrow_pos].trim().to_string();
                    let file = rest[arrow_pos + 1..].trim().to_string();
                    return Some(BuiltinCmd::EchoTo { content, file });
                }
            }
            None
        }
        "grep" => {
            let pattern = tokens
                .next()
                .map(|p| p.trim().to_string())
                .unwrap_or_default();
            Some(BuiltinCmd::Grep { pattern })
        }
        "farm" => Some(BuiltinCmd::Farm),
        "breed" => Some(BuiltinCmd::Breed),
        "rest" => Some(BuiltinCmd::Rest),
        "status" => Some(BuiltinCmd::Status),
        "save" => Some(BuiltinCmd::Save),
        "quit" => Some(BuiltinCmd::Quit),
        _ => None,
    }
}
