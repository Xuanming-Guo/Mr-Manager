use std::sync::LazyLock;

use regex::{Captures, Regex};

const REDACTED: &str = "[REDACTED]";

static PRIVATE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)-----BEGIN [^-\r\n]*PRIVATE KEY-----.*?-----END [^-\r\n]*PRIVATE KEY-----")
        .expect("the hard-coded private-key redaction pattern is valid")
});
static URL_PASSWORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([a-z][a-z0-9+.-]*://[^/\s:@]+:)[^@/\s]+(@)")
        .expect("the hard-coded URL-password redaction pattern is valid")
});
static BEARER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbearer\s+[a-z0-9._~+/=-]+")
        .expect("the hard-coded bearer-token redaction pattern is valid")
});
static SECRET_ASSIGNMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)\b(authorization|api[_-]?key|access[_-]?token|refresh[_-]?token|client[_-]?secret|password|passwd|pwd|secret)(\s*[:=]\s*)(?:"[^"]*"|'[^']*'|[^\s&,;]+)"#,
    )
    .expect("the hard-coded secret-assignment redaction pattern is valid")
});
static SECRET_FLAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(^|\s)(--?(?:password|passwd|token|api[_-]?key|secret)(?:\s+|=))(?:"[^"]*"|'[^']*'|[^\s&,;]+)"#,
    )
    .expect("the hard-coded secret-flag redaction pattern is valid")
});
static JWT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\beyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\b")
        .expect("the hard-coded JWT redaction pattern is valid")
});
static PREFIXED_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:sk-[a-zA-Z0-9_-]{12,}|gh[pousr]_[a-zA-Z0-9]{12,})\b")
        .expect("the hard-coded prefixed-token redaction pattern is valid")
});
static ANSI_ESCAPE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\))")
        .expect("the hard-coded ANSI escape redaction pattern is valid")
});

#[derive(Default)]
pub struct Redactor {
    additional_secrets: Vec<String>,
}

impl Redactor {
    #[allow(dead_code)] // Activated by the managed-process supervisor in Phase 2.
    pub fn with_additional_secrets<I, S>(secrets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            additional_secrets: secrets
                .into_iter()
                .map(Into::into)
                .filter(|secret| !secret.is_empty())
                .collect(),
        }
    }

    pub fn redact(&self, input: &str) -> String {
        let mut output = sanitize_control_characters(input);
        output = PRIVATE_KEY.replace_all(&output, REDACTED).into_owned();
        output = URL_PASSWORD
            .replace_all(&output, |captures: &Captures<'_>| {
                format!("{}{}{}", &captures[1], REDACTED, &captures[2])
            })
            .into_owned();
        output = BEARER
            .replace_all(&output, format!("Bearer {REDACTED}"))
            .into_owned();
        output = SECRET_ASSIGNMENT
            .replace_all(&output, |captures: &Captures<'_>| {
                format!("{}{}{}", &captures[1], &captures[2], REDACTED)
            })
            .into_owned();
        output = SECRET_FLAG
            .replace_all(&output, |captures: &Captures<'_>| {
                format!("{}{}{REDACTED}", &captures[1], &captures[2])
            })
            .into_owned();
        output = JWT.replace_all(&output, REDACTED).into_owned();
        output = PREFIXED_TOKEN.replace_all(&output, REDACTED).into_owned();

        for secret in &self.additional_secrets {
            output = output.replace(secret, REDACTED);
        }
        output
    }
}

pub fn redact(input: &str) -> String {
    Redactor::default().redact(input)
}

fn sanitize_control_characters(input: &str) -> String {
    let without_ansi = ANSI_ESCAPE.replace_all(input, "");
    without_ansi
        .chars()
        .filter_map(|character| match character {
            '\n' | '\r' | '\t' => Some(' '),
            character if character.is_control() => None,
            character => Some(character),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secret_shapes() {
        let input = "Authorization: Bearer abc.def_123 password=hunter2 API_KEY='top-secret'";
        let output = redact(input);

        assert!(!output.contains("abc.def_123"));
        assert!(!output.contains("hunter2"));
        assert!(!output.contains("top-secret"));
        assert!(output.contains(REDACTED));
    }

    #[test]
    fn redacts_url_passwords_and_command_flags() {
        let output =
            redact("tool --token secret-value https://developer:password@example.test/resource");

        assert!(output.contains("--token [REDACTED]"));
        assert!(output.contains("developer:[REDACTED]@example.test"));
        assert!(!output.contains("secret-value"));
    }

    #[test]
    fn strips_ansi_and_known_process_secrets() {
        let redactor = Redactor::with_additional_secrets(["fixture-secret"]);
        let output = redactor.redact("\u{1b}[31mfailed\u{1b}[0m fixture-secret\nnext");

        assert_eq!(output, "failed [REDACTED] next");
    }

    #[test]
    fn redacts_private_key_blocks() {
        let output =
            redact("before -----BEGIN PRIVATE KEY-----\nabc123\n-----END PRIVATE KEY----- after");

        assert_eq!(output, "before [REDACTED] after");
    }
}
