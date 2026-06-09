//! reedline glue for the REPL: prompt, tab-completion, syntax highlighting, and
//! multi-line validation. The session's bound names are shared in so completion
//! and highlighting know about user-defined values, not just builtins.

use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use nu_ansi_term::{Color, Style};
use reedline::{
    Completer, Highlighter, Prompt, PromptEditMode, PromptHistorySearch,
    PromptHistorySearchStatus, Span, StyledText, Suggestion, ValidationResult, Validator,
};

/// Built-in functions — prysm constructors, acts, and field arithmetic.
pub const BUILTINS: &[&str] = &[
    "text", "anno", "error", "log", "button", "col", // prysm
    "emit", "query", "link", "seal", "subscribe", // acts
    "add", "sub", "mul", "lt", "gt", // arithmetic
];

/// Register keywords (classic).
pub const KEYWORDS: &[&str] = &["let", "fn", "if", "else", "loop"];

/// REPL meta-commands.
pub const METAS: &[&str] = &[
    ":help", ":env", ":reset", ":nox", ":type", ":ast", ":load", ":quit",
];

/// Bindings shared between the loop and the completer/highlighter.
pub type Binds = Arc<Mutex<Vec<String>>>;

/// Extract bound names from accumulated `let NAME …` source lines.
pub fn bound_names(binds: &Binds) -> Vec<String> {
    binds
        .lock()
        .unwrap()
        .iter()
        .filter_map(|l| {
            l.strip_prefix("let ")
                .and_then(|r| r.split(['=', ':', ' ']).next())
                .map(|n| n.trim().to_string())
        })
        .filter(|n| !n.is_empty())
        .collect()
}

// ── prompt ───────────────────────────────────────────────────────────────────

pub struct RunePrompt;

impl Prompt for RunePrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[35mrune\x1b[0m")
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[90m ▸ \x1b[0m")
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("\x1b[90m … \x1b[0m")
    }
    fn render_prompt_history_search_indicator(&self, h: PromptHistorySearch) -> Cow<'_, str> {
        let pre = match h.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!("({pre}search: {}) ", h.term))
    }
}

// ── completion ───────────────────────────────────────────────────────────────

pub struct RuneCompleter {
    pub binds: Binds,
}

impl Completer for RuneCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        // The word under the cursor: back to the last separator.
        let start = line[..pos]
            .rfind(|c: char| c.is_whitespace() || "([,".contains(c))
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &line[start..pos];

        let pool: Vec<String> = if word.starts_with(':') || line.starts_with(':') {
            METAS.iter().map(|s| s.to_string()).collect()
        } else {
            BUILTINS
                .iter()
                .chain(KEYWORDS.iter())
                .map(|s| s.to_string())
                .chain(bound_names(&self.binds))
                .collect()
        };

        pool.into_iter()
            .filter(|c| c.starts_with(word) && c.as_str() != word)
            .map(|value| Suggestion {
                value,
                description: None,
                style: None,
                extra: None,
                span: Span::new(start, pos),
                append_whitespace: false,
                match_indices: None,
            })
            .collect()
    }
}

// ── highlighting ─────────────────────────────────────────────────────────────

pub struct RuneHighlighter {
    pub binds: Binds,
}

impl Highlighter for RuneHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let names = bound_names(&self.binds);
        let mut st = StyledText::new();

        // A whole-line meta-command renders blue.
        if line.starts_with(':') {
            st.push((Style::new().fg(Color::Blue), line.to_string()));
            return st;
        }

        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '"' {
                // string literal up to the next quote
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '"' {
                    j += 1;
                }
                let end = (j + 1).min(chars.len());
                let s: String = chars[i..end].iter().collect();
                st.push((Style::new().fg(Color::Green), s));
                i = end;
            } else if c.is_alphabetic() || c == '_' {
                let mut j = i;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let w: String = chars[i..j].iter().collect();
                let style = if BUILTINS.contains(&w.as_str()) {
                    Style::new().fg(Color::Cyan)
                } else if KEYWORDS.contains(&w.as_str()) {
                    Style::new().fg(Color::Magenta)
                } else if names.contains(&w) {
                    Style::new().fg(Color::LightYellow)
                } else {
                    Style::default()
                };
                st.push((style, w));
                i = j;
            } else if c.is_ascii_digit() {
                let mut j = i;
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                    j += 1;
                }
                st.push((Style::new().fg(Color::Yellow), chars[i..j].iter().collect()));
                i = j;
            } else {
                st.push((Style::default(), c.to_string()));
                i += 1;
            }
        }
        st
    }
}

// ── multi-line validation ────────────────────────────────────────────────────

pub struct RuneValidator;

impl Validator for RuneValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        // Continue onto another line while brackets or a string are still open.
        let mut depth = 0i32;
        let mut in_str = false;
        for c in line.chars() {
            match c {
                '"' => in_str = !in_str,
                '(' | '[' if !in_str => depth += 1,
                ')' | ']' if !in_str => depth -= 1,
                _ => {}
            }
        }
        if depth > 0 || in_str {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}
