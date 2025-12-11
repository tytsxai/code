#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Chinese,
}

impl Language {
    #[must_use]
    pub fn detect() -> Self {
        let codex_lang = std::env::var("CODEX_LANG").ok();
        let lang = std::env::var("LANG").ok();
        Self::from_env(codex_lang, lang)
    }

    fn from_env(codex_lang: Option<String>, lang: Option<String>) -> Self {
        if let Some(lang) = codex_lang
            && Self::is_zh(&lang) {
                return Self::Chinese;
            }

        if let Some(lang) = lang
            && Self::is_zh(&lang) {
                return Self::Chinese;
            }

        Self::English
    }

    fn is_zh(lang: &str) -> bool {
        let lower = lang.to_ascii_lowercase();
        lower.starts_with("zh")
    }

    #[must_use]
    pub fn is_chinese(self) -> bool {
        matches!(self, Self::Chinese)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn detect_prefers_codex_lang() {
        assert_eq!(
            Language::from_env(Some("zh_CN.UTF-8".to_string()), Some("en_US.UTF-8".to_string())),
            Language::Chinese
        );
    }

    #[test]
    fn detect_falls_back_to_lang() {
        assert_eq!(
            Language::from_env(None, Some("zh_TW.UTF-8".to_string())),
            Language::Chinese
        );
    }

    #[test]
    fn detect_defaults_to_english() {
        assert_eq!(
            Language::from_env(None, Some("en_US.UTF-8".to_string())),
            Language::English
        );
    }

    #[test]
    fn empty_inputs_default_to_english() {
        assert_eq!(Language::from_env(None, None), Language::English);
    }
}
