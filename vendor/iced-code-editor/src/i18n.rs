//! Internationalization support for the code editor.
//!
//! This module provides translation support for UI text in the search dialog.
//!
//! # Using rust-i18n
//!
//! The translations are available in YAML files in the `locales` directory.
//! The `rust-i18n` crate is integrated and can be used directly via the `t!` macro:
//!
//! ```ignore
//! use iced_code_editor::t;
//!
//! // Use translations directly
//! let text = t!("search.placeholder");
//! ```

/// Supported languages for the code editor UI.
///
/// # Examples
///
/// ```
/// use iced_code_editor::Language;
///
/// let lang = Language::English;
/// assert_eq!(lang, Language::default());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// English language
    #[default]
    English,
    /// French language
    French,
    /// Spanish language
    Spanish,
    /// German language
    German,
    /// Italian language
    Italian,
    /// Portuguese (Brazilian) language
    PortugueseBR,
    /// Portuguese (European) language
    PortuguesePT,
    /// Simplified Chinese language
    ChineseSimplified,
}

impl Language {
    /// Returns the locale code for this language.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::Language;
    ///
    /// assert_eq!(Language::English.to_locale(), "en");
    /// assert_eq!(Language::French.to_locale(), "fr");
    /// assert_eq!(Language::Spanish.to_locale(), "es");
    /// assert_eq!(Language::German.to_locale(), "de");
    /// assert_eq!(Language::Italian.to_locale(), "it");
    /// assert_eq!(Language::PortugueseBR.to_locale(), "pt-BR");
    /// assert_eq!(Language::PortuguesePT.to_locale(), "pt-PT");
    /// ```
    #[must_use]
    pub const fn to_locale(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
            Self::Spanish => "es",
            Self::German => "de",
            Self::Italian => "it",
            Self::PortugueseBR => "pt-BR",
            Self::PortuguesePT => "pt-PT",
            Self::ChineseSimplified => "zh-CN",
        }
    }
}

/// Provides translated text strings for UI elements.
///
/// This struct contains all UI text translations used in the search dialog,
/// including placeholders, tooltips, and labels.
///
/// # Examples
///
/// ```
/// use iced_code_editor::{Language, Translations};
///
/// let translations = Translations::new(Language::French);
/// assert_eq!(translations.search_placeholder(), "Rechercher...");
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Translations {
    language: Language,
}

impl Translations {
    /// Creates a new `Translations` instance with the specified language.
    ///
    /// This sets the global rust-i18n locale to the specified language.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let translations = Translations::new(Language::Spanish);
    /// assert_eq!(translations.language(), Language::Spanish);
    /// ```
    #[must_use]
    pub fn new(language: Language) -> Self {
        rust_i18n::set_locale(language.to_locale());
        Self { language }
    }

    /// Returns the current language.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let translations = Translations::new(Language::French);
    /// assert_eq!(translations.language(), Language::French);
    /// ```
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    /// Sets the language for translations.
    ///
    /// This updates the global rust-i18n locale.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let mut translations = Translations::new(Language::English);
    /// translations.set_language(Language::Spanish);
    /// assert_eq!(translations.language(), Language::Spanish);
    /// ```
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
        rust_i18n::set_locale(language.to_locale());
    }

    /// Returns the placeholder text for the search input field.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let en = Translations::new(Language::English);
    /// assert_eq!(en.search_placeholder(), "Search...");
    ///
    /// let fr = Translations::new(Language::French);
    /// assert_eq!(fr.search_placeholder(), "Rechercher...");
    /// ```
    #[must_use]
    pub fn search_placeholder(&self) -> String {
        rust_i18n::t!("search.placeholder", locale = self.language.to_locale())
            .into_owned()
    }

    /// Returns the placeholder text for the replace input field.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let es = Translations::new(Language::Spanish);
    /// assert_eq!(es.replace_placeholder(), "Reemplazar...");
    /// ```
    #[must_use]
    pub fn replace_placeholder(&self) -> String {
        rust_i18n::t!("replace.placeholder", locale = self.language.to_locale())
            .into_owned()
    }

    /// Returns the label text for the case sensitive checkbox.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let fr = Translations::new(Language::French);
    /// assert_eq!(fr.case_sensitive_label(), "Sensible à la casse");
    /// ```
    #[must_use]
    pub fn case_sensitive_label(&self) -> String {
        rust_i18n::t!(
            "settings.case_sensitive_label",
            locale = self.language.to_locale()
        )
        .into_owned()
    }

    /// Returns the tooltip text for the previous match button.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let en = Translations::new(Language::English);
    /// assert_eq!(en.previous_match_tooltip(), "Previous match (Shift+F3)");
    /// ```
    #[must_use]
    pub fn previous_match_tooltip(&self) -> String {
        rust_i18n::t!(
            "search.previous_match_tooltip",
            locale = self.language.to_locale()
        )
        .into_owned()
    }

    /// Returns the tooltip text for the next match button.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let es = Translations::new(Language::Spanish);
    /// assert_eq!(es.next_match_tooltip(), "Siguiente coincidencia (F3 / Enter)");
    /// ```
    #[must_use]
    pub fn next_match_tooltip(&self) -> String {
        rust_i18n::t!(
            "search.next_match_tooltip",
            locale = self.language.to_locale()
        )
        .into_owned()
    }

    /// Returns the tooltip text for the close search dialog button.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let fr = Translations::new(Language::French);
    /// assert_eq!(fr.close_search_tooltip(), "Fermer la recherche (Échap)");
    /// ```
    #[must_use]
    pub fn close_search_tooltip(&self) -> String {
        rust_i18n::t!(
            "search.close_tooltip",
            locale = self.language.to_locale()
        )
        .into_owned()
    }

    /// Returns the tooltip text for the replace current match button.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let en = Translations::new(Language::English);
    /// assert_eq!(en.replace_current_tooltip(), "Replace current match");
    /// ```
    #[must_use]
    pub fn replace_current_tooltip(&self) -> String {
        rust_i18n::t!(
            "replace.current_tooltip",
            locale = self.language.to_locale()
        )
        .into_owned()
    }

    /// Returns the tooltip text for the replace all matches button.
    ///
    /// # Examples
    ///
    /// ```
    /// use iced_code_editor::{Language, Translations};
    ///
    /// let es = Translations::new(Language::Spanish);
    /// assert_eq!(es.replace_all_tooltip(), "Reemplazar todo");
    /// ```
    #[must_use]
    pub fn replace_all_tooltip(&self) -> String {
        rust_i18n::t!("replace.all_tooltip", locale = self.language.to_locale())
            .into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_language() {
        let translations = Translations::default();
        assert_eq!(translations.language(), Language::English);
    }

    #[test]
    fn test_new_with_language() {
        let translations = Translations::new(Language::French);
        assert_eq!(translations.language(), Language::French);
    }

    #[test]
    fn test_set_language() {
        let mut translations = Translations::new(Language::English);
        translations.set_language(Language::Spanish);
        assert_eq!(translations.language(), Language::Spanish);
    }

    #[test]
    fn test_english_translations() {
        let t = Translations::new(Language::English);
        assert_eq!(t.search_placeholder(), "Search...");
        assert_eq!(t.replace_placeholder(), "Replace...");
        assert_eq!(t.case_sensitive_label(), "Case sensitive");
        assert_eq!(t.previous_match_tooltip(), "Previous match (Shift+F3)");
        assert_eq!(t.next_match_tooltip(), "Next match (F3 / Enter)");
        assert_eq!(t.close_search_tooltip(), "Close search dialog (Esc)");
        assert_eq!(t.replace_current_tooltip(), "Replace current match");
        assert_eq!(t.replace_all_tooltip(), "Replace all matches");
    }

    #[test]
    fn test_french_translations() {
        let t = Translations::new(Language::French);
        assert_eq!(t.search_placeholder(), "Rechercher...");
        assert_eq!(t.replace_placeholder(), "Remplacer...");
        assert_eq!(t.case_sensitive_label(), "Sensible à la casse");
        assert_eq!(t.previous_match_tooltip(), "Résultat précédent (Maj+F3)");
        assert_eq!(t.next_match_tooltip(), "Résultat suivant (F3 / Entrée)");
        assert_eq!(t.close_search_tooltip(), "Fermer la recherche (Échap)");
        assert_eq!(
            t.replace_current_tooltip(),
            "Remplacer l'occurrence actuelle"
        );
        assert_eq!(t.replace_all_tooltip(), "Tout remplacer");
    }

    #[test]
    fn test_spanish_translations() {
        let t = Translations::new(Language::Spanish);
        assert_eq!(t.search_placeholder(), "Buscar...");
        assert_eq!(t.replace_placeholder(), "Reemplazar...");
        assert_eq!(t.case_sensitive_label(), "Distinguir mayúsculas");
        assert_eq!(
            t.previous_match_tooltip(),
            "Coincidencia anterior (Mayús+F3)"
        );
        assert_eq!(
            t.next_match_tooltip(),
            "Siguiente coincidencia (F3 / Enter)"
        );
        assert_eq!(t.close_search_tooltip(), "Cerrar búsqueda (Esc)");
        assert_eq!(
            t.replace_current_tooltip(),
            "Reemplazar coincidencia actual"
        );
        assert_eq!(t.replace_all_tooltip(), "Reemplazar todo");
    }

    #[test]
    fn test_german_translations() {
        let t = Translations::new(Language::German);
        assert_eq!(t.search_placeholder(), "Suchen...");
        assert_eq!(t.replace_placeholder(), "Ersetzen...");
        assert_eq!(t.case_sensitive_label(), "Groß-/Kleinschreibung");
        assert_eq!(
            t.previous_match_tooltip(),
            "Vorheriger Treffer (Umschalt+F3)"
        );
        assert_eq!(t.next_match_tooltip(), "Nächster Treffer (F3 / Enter)");
        assert_eq!(t.close_search_tooltip(), "Suchdialog schließen (Esc)");
        assert_eq!(t.replace_current_tooltip(), "Aktuellen Treffer ersetzen");
        assert_eq!(t.replace_all_tooltip(), "Alle ersetzen");
    }

    #[test]
    fn test_italian_translations() {
        let t = Translations::new(Language::Italian);
        assert_eq!(t.search_placeholder(), "Cerca...");
        assert_eq!(t.replace_placeholder(), "Sostituisci...");
        assert_eq!(t.case_sensitive_label(), "Distingui maiuscole");
        assert_eq!(
            t.previous_match_tooltip(),
            "Risultato precedente (Maiusc+F3)"
        );
        assert_eq!(t.next_match_tooltip(), "Risultato successivo (F3 / Invio)");
        assert_eq!(
            t.close_search_tooltip(),
            "Chiudi finestra di ricerca (Esc)"
        );
        assert_eq!(
            t.replace_current_tooltip(),
            "Sostituisci risultato corrente"
        );
        assert_eq!(t.replace_all_tooltip(), "Sostituisci tutto");
    }

    #[test]
    fn test_portuguese_br_translations() {
        let t = Translations::new(Language::PortugueseBR);
        assert_eq!(t.search_placeholder(), "Pesquisar...");
        assert_eq!(t.replace_placeholder(), "Substituir...");
        assert_eq!(t.case_sensitive_label(), "Diferenciar maiúsculas");
        assert_eq!(
            t.previous_match_tooltip(),
            "Correspondência anterior (Shift+F3)"
        );
        assert_eq!(
            t.next_match_tooltip(),
            "Próxima correspondência (F3 / Enter)"
        );
        assert_eq!(
            t.close_search_tooltip(),
            "Fechar diálogo de pesquisa (Esc)"
        );
        assert_eq!(
            t.replace_current_tooltip(),
            "Substituir correspondência atual"
        );
        assert_eq!(t.replace_all_tooltip(), "Substituir tudo");
    }

    #[test]
    fn test_portuguese_pt_translations() {
        let t = Translations::new(Language::PortuguesePT);
        assert_eq!(t.search_placeholder(), "Pesquisar...");
        assert_eq!(t.replace_placeholder(), "Substituir...");
        assert_eq!(t.case_sensitive_label(), "Diferenciar maiúsculas");
        assert_eq!(
            t.previous_match_tooltip(),
            "Correspondência anterior (Shift+F3)"
        );
        assert_eq!(
            t.next_match_tooltip(),
            "Próxima correspondência (F3 / Enter)"
        );
        assert_eq!(
            t.close_search_tooltip(),
            "Fechar diálogo de pesquisa (Esc)"
        );
        assert_eq!(
            t.replace_current_tooltip(),
            "Substituir correspondência actual"
        );
        assert_eq!(t.replace_all_tooltip(), "Substituir tudo");
    }

    #[test]
    fn test_language_switching() {
        let mut t = Translations::new(Language::English);
        assert_eq!(t.search_placeholder(), "Search...");

        t.set_language(Language::French);
        assert_eq!(t.search_placeholder(), "Rechercher...");

        t.set_language(Language::Spanish);
        assert_eq!(t.search_placeholder(), "Buscar...");

        t.set_language(Language::German);
        assert_eq!(t.search_placeholder(), "Suchen...");

        t.set_language(Language::Italian);
        assert_eq!(t.search_placeholder(), "Cerca...");

        t.set_language(Language::PortugueseBR);
        assert_eq!(t.search_placeholder(), "Pesquisar...");

        t.set_language(Language::PortuguesePT);
        assert_eq!(t.search_placeholder(), "Pesquisar...");

        t.set_language(Language::ChineseSimplified);
        assert_eq!(t.search_placeholder(), "搜索...");
    }
}
