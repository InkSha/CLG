//! UI template system.
//!
//! A UI template is a YAML file (typically `world/config/ui.yaml`) that controls
//! how the player status bar is rendered. Templates support expression syntax,
//! table layout, separators, centering, and colored text.
//!
//! # Template file format
//! ```yaml
//! scope: /
//! exclude:
//!   - home/*
//! env:
//!   name: $player.name
//!   level: $player.level
//!   ch: $player.currentHealth
//!   mh: $player.health
//! include:
//!   - resources
//! content: |
//!   ---
//!   | ${{ $name }} Lv.${{ $level }} |-❤️ ${{ $ch }}/${{ $mh }} |
//!   ---
//! ```
//!
//! # Content syntax
//! - `---` → horizontal separator line (50 `─` characters)
//! - `${{ expr }}` → render expression value
//! - `$RRGGBB{{ expr }}` → render expression with 24-bit foreground colour
//! - `[ content ]` → render line centred within the terminal width
//! - `| col1 |-col2 |` → table row  
//!   * cell starting with `-` → right-aligned
//!   * cell ending with `-` → left-aligned
//!   * otherwise → centred

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;

/// Terminal output width used for padding/separators.
pub const LINE_WIDTH: usize = 50;

// ── Template struct ───────────────────────────────────────────────────────────

/// A parsed UI template loaded from YAML.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct UiTemplate {
    /// Path scope: `"/"` matches every area (default).
    #[serde(default = "default_scope")]
    pub scope: String,

    /// Glob-style path patterns to exclude from rendering.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Short variable name → `$player.<field>` / `$CURRENT_AREA.<field>` binding.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Names of other UI templates whose rendered output is substituted by name.
    #[serde(default)]
    pub include: Vec<String>,

    /// Multi-line template content string.
    #[serde(default)]
    pub content: String,
}

fn default_scope() -> String {
    "/".to_string()
}

// ── Render context ────────────────────────────────────────────────────────────

/// Runtime data supplied when rendering a [`UiTemplate`].
pub struct UiContext<'a> {
    pub player: &'a crate::player::Player,
    /// Name of the area the player is currently in.
    pub current_area: &'a str,
    /// Pre-rendered content of included templates, keyed by the include name.
    pub includes: HashMap<String, String>,
}

// ── Variable / expression resolution ─────────────────────────────────────────

/// Resolve a `$player.<field>` or `$CURRENT_AREA.<field>` binding.
fn resolve_binding(binding: &str, ctx: &UiContext) -> String {
    let b = binding.trim();
    if let Some(field) = b.strip_prefix("$player.") {
        match field {
            "name" => ctx.player.name.clone(),
            "level" => ctx.player.level.to_string(),
            "hp" | "currentHealth" | "current_health" => ctx.player.hp.to_string(),
            "max_hp" | "maxHp" | "health" => ctx.player.max_hp.to_string(),
            "attack" => ctx.player.attack.to_string(),
            "defense" => ctx.player.defense.to_string(),
            "exp" => ctx.player.exp.to_string(),
            "expToNext" | "exp_to_next" => ctx.player.exp_to_next.to_string(),
            "gold" => ctx.player.gold.to_string(),
            // Mana – not yet in the Player struct; return em-dash placeholder.
            "currentMana" | "current_mana" | "mana" | "maxMana" | "max_mana" => "—".to_string(),
            _ => format!("[?{}]", b),
        }
    } else if let Some(field) = b.strip_prefix("$CURRENT_AREA.") {
        match field {
            "name" => ctx.current_area.to_string(),
            _ => format!("[?{}]", b),
        }
    } else {
        format!("[?{}]", b)
    }
}

/// Resolve an expression found inside `${{ … }}`.
///
/// The expression may be:
/// * `$varname` – look up `varname` in `env`, then resolve the binding.
/// * An include name – return the pre-rendered include string.
/// * A literal string – return as-is.
fn resolve_expr(expr: &str, env: &HashMap<String, String>, ctx: &UiContext) -> String {
    let expr = expr.trim();
    if let Some(var_name) = expr.strip_prefix('$') {
        // Variable reference: first try the env map.
        if let Some(binding) = env.get(var_name) {
            return resolve_binding(binding, ctx);
        }
        // Fall back to direct binding (e.g. `$CURRENT_AREA.name`).
        return resolve_binding(expr, ctx);
    }
    // Include substitution.
    if let Some(included) = ctx.includes.get(expr) {
        return included.clone();
    }
    // Literal.
    expr.to_string()
}

// ── Color helpers ─────────────────────────────────────────────────────────────

/// Wrap `text` with 24-bit ANSI foreground colour escape codes.
fn ansi_color(text: &str, r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, text)
}

// ── Visible-width calculation ─────────────────────────────────────────────────

/// Compute the visible terminal width of `s`, ignoring ANSI escape sequences.
///
/// Uses [`unicode_width`] for per-character column width.  Legacy emoji from the
/// Miscellaneous Symbols (U+2600–U+26FF) and Dingbats (U+2700–U+27BF) blocks
/// are treated as 1 column wide by UAX#11, but modern terminals render them as
/// 2 columns when followed by the emoji variation selector U+FE0F.  We detect
/// this pair and add the extra column.
pub fn visible_width(s: &str) -> usize {
    let mut width = 0usize;
    let mut in_escape = false;
    let mut last_was_emoji_candidate = false;

    for c in s.chars() {
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
            last_was_emoji_candidate = false;
        } else if c == '\u{FE0F}' {
            // Emoji presentation selector: upgrade previous 1-wide symbol to 2.
            if last_was_emoji_candidate {
                width += 1;
            }
            last_was_emoji_candidate = false;
        } else {
            let w = UnicodeWidthChar::width(c).unwrap_or(0);
            // Chars in Misc Symbols / Dingbats that are 1-wide per UAX#11 but
            // displayed as 2 columns when followed by U+FE0F in modern terminals.
            last_was_emoji_candidate =
                w == 1 && matches!(c as u32, 0x2600..=0x27BF | 0x00A9 | 0x00AE | 0x203C
                    | 0x2049 | 0x2122 | 0x2139 | 0x2194..=0x2199 | 0x21A9..=0x21AA
                    | 0x231A..=0x231B | 0x2328 | 0x23CF | 0x23E9..=0x23FA
                    | 0x24C2 | 0x25AA..=0x25AB | 0x25B6 | 0x25C0 | 0x25FB..=0x25FE
                    | 0x2934..=0x2935 | 0x2B05..=0x2B07 | 0x2B1B..=0x2B1C
                    | 0x2B50 | 0x2B55 | 0x3030 | 0x303D | 0x3297 | 0x3299);
            width += w;
        }
    }
    width
}

// ── Padding helpers ───────────────────────────────────────────────────────────

fn pad_right(s: &str, visible: usize, width: usize) -> String {
    if visible >= width {
        return s.to_string();
    }
    format!("{}{}", s, " ".repeat(width - visible))
}

fn pad_left(s: &str, visible: usize, width: usize) -> String {
    if visible >= width {
        return s.to_string();
    }
    format!("{}{}", " ".repeat(width - visible), s)
}

fn pad_center(s: &str, visible: usize, width: usize) -> String {
    if visible >= width {
        return s.to_string();
    }
    let total = width - visible;
    let left = total / 2;
    let right = total - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}

// ── Expression rendering ──────────────────────────────────────────────────────

/// Expand all `${{ expr }}` and `$RRGGBB{{ expr }}` patterns in `text`.
fn render_expressions(text: &str, env: &HashMap<String, String>, ctx: &UiContext) -> String {
    let mut result = String::new();
    let mut rest = text;

    while !rest.is_empty() {
        match rest.find('$') {
            None => {
                result.push_str(rest);
                break;
            }
            Some(pos) => {
                result.push_str(&rest[..pos]);
                rest = &rest[pos..];

                // Try `$RRGGBB{{` (dollar + 6 hex digits + `{{`)
                if rest.len() >= 9 {
                    let maybe_hex = &rest[1..7];
                    if maybe_hex.chars().all(|c| c.is_ascii_hexdigit())
                        && rest[7..].starts_with("{{")
                    {
                        let hex = maybe_hex;
                        let after_open = &rest[9..]; // skip `$RRGGBB{{`
                        if let Some(close) = after_open.find("}}") {
                            let expr_text = &after_open[..close];
                            let value = resolve_expr(expr_text, env, ctx);
                            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
                            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
                            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
                            result.push_str(&ansi_color(&value, r, g, b));
                            rest = &after_open[close + 2..];
                            continue;
                        }
                    }
                }

                // Try `${{`
                if rest.starts_with("${{") {
                    let after_open = &rest[3..]; // skip `${{`
                    if let Some(close) = after_open.find("}}") {
                        let expr_text = &after_open[..close];
                        let value = resolve_expr(expr_text, env, ctx);
                        result.push_str(&value);
                        rest = &after_open[close + 2..];
                        continue;
                    }
                }

                // Not a recognised pattern – emit `$` literally.
                result.push('$');
                rest = &rest[1..];
            }
        }
    }
    result
}

// ── Table row rendering ───────────────────────────────────────────────────────

/// Render a table row such as `| col1 |-col2 |-col3 |`.
///
/// The row is split on `'|'`. Empty leading / trailing parts (from the outer
/// `| … |` delimiters) are discarded.
///
/// Cell alignment is determined by whether the raw cell text (after splitting)
/// starts or ends with `'-'`:
/// * Starts with `'-'` → right-aligned (leading `'-'` stripped)
/// * Ends with `'-'`   → left-aligned  (trailing `'-'` stripped)
/// * Otherwise         → centred
///
/// The total terminal width is distributed evenly, with the last column
/// absorbing any rounding remainder.
fn render_table_row(line: &str, env: &HashMap<String, String>, ctx: &UiContext) -> String {
    let raw_parts: Vec<&str> = line.split('|').collect();

    // Drop the empty leading and trailing fragments produced by the outer `| … |`.
    let cells: Vec<&str> = {
        let mut v = raw_parts.as_slice();
        if v.first().map(|s| s.trim().is_empty()).unwrap_or(false) {
            v = &v[1..];
        }
        if v.last().map(|s| s.trim().is_empty()).unwrap_or(false) {
            v = &v[..v.len() - 1];
        }
        v.to_vec()
    };

    if cells.is_empty() {
        return String::new();
    }

    let n = cells.len();
    let col_width = LINE_WIDTH / n;

    let mut output = String::new();

    for (i, cell) in cells.iter().enumerate() {
        let width = if i == n - 1 {
            LINE_WIDTH.saturating_sub(col_width * i)
        } else {
            col_width
        };

        let (align, raw_content) = if cell.starts_with('-') {
            ('R', &cell[1..])
        } else if cell.ends_with('-') {
            ('L', &cell[..cell.len() - 1])
        } else {
            ('C', *cell)
        };

        let trimmed = raw_content.trim();
        let rendered = render_expressions(trimmed, env, ctx);
        let vw = visible_width(&rendered);

        let padded = match align {
            'L' => pad_right(&rendered, vw, width),
            'R' => pad_left(&rendered, vw, width),
            _   => pad_center(&rendered, vw, width),
        };
        output.push_str(&padded);
    }

    output
}

// ── Main render function ──────────────────────────────────────────────────────

/// Render the full UI template to a `String`, ready to be printed to stdout.
pub fn render_template(template: &UiTemplate, ctx: &UiContext) -> String {
    let separator = "─".repeat(LINE_WIDTH);
    let mut output = String::new();

    for line in template.content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "---" {
            output.push_str(&separator);
            output.push('\n');
        } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Centred full-width line: strip the outer `[` … `]`.
            let inner = trimmed[1..trimmed.len() - 1].trim();
            let rendered = render_expressions(inner, &template.env, ctx);
            let vw = visible_width(&rendered);
            output.push_str(&pad_center(&rendered, vw, LINE_WIDTH));
            output.push('\n');
        } else if trimmed.starts_with('|') {
            // Table row.
            let row = render_table_row(trimmed, &template.env, ctx);
            output.push_str(&row);
            output.push('\n');
        } else {
            // Plain text with expression substitution.
            let rendered = render_expressions(trimmed, &template.env, ctx);
            output.push_str(&rendered);
            output.push('\n');
        }
    }

    output
}

// ── Scope matching ────────────────────────────────────────────────────────────

/// Return `true` when `template` should be rendered for `path`.
///
/// `scope: "/"` matches everything. Any pattern in `exclude` that matches
/// `path` causes the function to return `false`.
pub fn scope_matches(template: &UiTemplate, path: &str) -> bool {
    let scope = template.scope.trim_end_matches('/');
    if !scope.is_empty() && scope != "/" {
        let path_norm = path.trim_end_matches('/');
        if !path_norm.starts_with(scope) {
            return false;
        }
    }
    for pattern in &template.exclude {
        if glob_matches(pattern, path) {
            return false;
        }
    }
    true
}

/// Minimal glob matching: `*` matches any run of characters in a single
/// path segment (i.e. does not cross `/`).
fn glob_matches(pattern: &str, path: &str) -> bool {
    // Split on `*` and match each literal fragment in order.
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() {
        return true;
    }
    let mut remaining = path;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // First part must be a prefix.
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else {
            match remaining.find(part) {
                Some(pos) => remaining = &remaining[pos + part.len()..],
                None => return false,
            }
        }
    }
    true
}

// ── File I/O ──────────────────────────────────────────────────────────────────

/// Parse a [`UiTemplate`] from a YAML file on disk.
pub fn load_ui_template(path: &Path) -> Result<UiTemplate, String> {
    let yaml = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&yaml)
        .map_err(|e| format!("解析 UI 模板 {} 失败: {}", path.display(), e))
}

// ── Default template ──────────────────────────────────────────────────────────

/// The YAML source of the default UI template written to `world/config/ui.yaml`
/// on first launch.
pub const DEFAULT_UI_TEMPLATE: &str = r#"# UI 模板配置文件
# 修改此文件可自定义游戏界面。修改后自动生效。
#
# scope   - 生效路径前缀（"/" 表示所有区域）
# exclude - 不渲染此模板的路径列表（支持 * 通配符）
# env     - 变量绑定：短名称 → $player.<字段> 或 $CURRENT_AREA.<字段>
# include - 引入其他 UI 模板（同目录下的 .yaml 文件名，不含扩展名）
# content - 模板内容（支持下列语法）：
#   ---                  分割线
#   ${{ $var }}          渲染变量
#   $RRGGBB{{ $var }}    渲染变量并着色（十六进制颜色）
#   [ ${{ $var }} ]      居中显示整行
#   | col1 |-col2 |      表格行（-前缀右对齐，-后缀左对齐，否则居中）

scope: /
exclude: []
env:
  name: $player.name
  level: $player.level
  ch: $player.hp
  mh: $player.max_hp
  atk: $player.attack
  def: $player.defense
  exp: $player.exp
  next: $player.exp_to_next
  gold: $player.gold
content: |
  ---
  | ${{ $name }}  Lv.${{ $level }} |-❤️ $00ff00{{ $ch }}/${{ $mh }} |-⚔️${{ $atk }} 🛡️${{ $def }} |
  ---
  | 经验：${{ $exp }}/${{ $next }} |-$ffcc00{{ $gold }}g 💰 |
  ---
  [ ${{ $CURRENT_AREA.name }} ]
  ---
"#;
