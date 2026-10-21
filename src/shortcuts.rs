use std::collections::HashMap;

/// Application commands that can be bound to keyboard shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppCommand {
    // File operations
    NewFile,
    OpenFile,
    SaveFile,
    SaveAs,
    Exit,
    ReloadDirectory,

    // Edit operations
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    CopyPreviewHtml,
    CopyPreviewText,

    // Markdown formatting
    FormatHeading1,
    FormatHeading2,
    FormatBold,
    FormatItalic,
    FormatCodeBlock,
    FormatTable,

    // View layout
    ToggleSidebar,
    ViewSplit,
    ViewEditor,
    ViewPreview,
    FocusNextPane,
    FocusPrevPane,

    // Help & dialogs
    Licenses,
    About,
    HelpGuide,
    DialogCancel,
    DialogConfirm,
}

#[allow(dead_code)]
impl AppCommand {
    /// Returns the unique canonical identifier of the command.
    pub fn id(&self) -> &'static str {
        match self {
            Self::NewFile => "file.new",
            Self::OpenFile => "file.open",
            Self::SaveFile => "file.save",
            Self::SaveAs => "file.save_as",
            Self::Exit => "file.exit",
            Self::ReloadDirectory => "file.reload_dir",

            Self::Undo => "edit.undo",
            Self::Redo => "edit.redo",
            Self::Cut => "edit.cut",
            Self::Copy => "edit.copy",
            Self::Paste => "edit.paste",
            Self::SelectAll => "edit.select_all",
            Self::CopyPreviewHtml => "edit.copy_preview_html",
            Self::CopyPreviewText => "edit.copy_preview_text",

            Self::FormatHeading1 => "format.h1",
            Self::FormatHeading2 => "format.h2",
            Self::FormatBold => "format.bold",
            Self::FormatItalic => "format.italic",
            Self::FormatCodeBlock => "format.code_block",
            Self::FormatTable => "format.table",

            Self::ToggleSidebar => "view.toggle_sidebar",
            Self::ViewSplit => "view.split",
            Self::ViewEditor => "view.editor",
            Self::ViewPreview => "view.preview",
            Self::FocusNextPane => "view.focus_next_pane",
            Self::FocusPrevPane => "view.focus_prev_pane",

            Self::Licenses => "help.licenses",
            Self::About => "help.about",
            Self::HelpGuide => "help.guide",
            Self::DialogCancel => "dialog.cancel",
            Self::DialogConfirm => "dialog.confirm",
        }
    }

    /// Resolves a command from its string identifier.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "file.new" => Some(Self::NewFile),
            "file.open" => Some(Self::OpenFile),
            "file.save" => Some(Self::SaveFile),
            "file.save_as" => Some(Self::SaveAs),
            "file.exit" => Some(Self::Exit),
            "file.reload_dir" => Some(Self::ReloadDirectory),

            "edit.undo" => Some(Self::Undo),
            "edit.redo" => Some(Self::Redo),
            "edit.cut" => Some(Self::Cut),
            "edit.copy" => Some(Self::Copy),
            "edit.paste" => Some(Self::Paste),
            "edit.select_all" => Some(Self::SelectAll),
            "edit.copy_preview_html" => Some(Self::CopyPreviewHtml),
            "edit.copy_preview_text" => Some(Self::CopyPreviewText),

            "format.h1" => Some(Self::FormatHeading1),
            "format.h2" => Some(Self::FormatHeading2),
            "format.bold" => Some(Self::FormatBold),
            "format.italic" => Some(Self::FormatItalic),
            "format.code_block" => Some(Self::FormatCodeBlock),
            "format.table" => Some(Self::FormatTable),

            "view.toggle_sidebar" => Some(Self::ToggleSidebar),
            "view.split" => Some(Self::ViewSplit),
            "view.editor" => Some(Self::ViewEditor),
            "view.preview" => Some(Self::ViewPreview),
            "view.focus_next_pane" => Some(Self::FocusNextPane),
            "view.focus_prev_pane" => Some(Self::FocusPrevPane),

            "help.licenses" => Some(Self::Licenses),
            "help.about" => Some(Self::About),
            "help.guide" => Some(Self::HelpGuide),
            "dialog.cancel" => Some(Self::DialogCancel),
            "dialog.confirm" => Some(Self::DialogConfirm),
            _ => None,
        }
    }
}

/// A combination of keyboard modifiers and a canonical key name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub key: String,
}

impl KeyChord {
    pub fn new(key: impl Into<String>, control: bool, alt: bool, shift: bool, meta: bool) -> Self {
        Self {
            control,
            alt,
            shift,
            meta,
            key: key.into(),
        }
    }

    pub fn ctrl(key: &str) -> Self {
        Self::new(key, true, false, false, false)
    }

    pub fn ctrl_shift(key: &str) -> Self {
        Self::new(key, true, false, true, false)
    }

    pub fn key(key: &str) -> Self {
        Self::new(key, false, false, false, false)
    }
}

impl std::fmt::Display for KeyChord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.control {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.meta {
            parts.push("Meta");
        }
        let key_upper = self.key.to_uppercase();
        parts.push(&key_upper);
        write!(f, "{}", parts.join("+"))
    }
}

/// Normalizes key text from Slint events into a canonical key identifier.
///
/// Handles:
/// 1. Slint named keys (Escape, Return, Tab, Backtab, F1, F5).
/// 2. ASCII control codes (0x01..=0x1A -> 'a'..='z') emitted on Unix/X11 when Ctrl is pressed.
/// 3. Cyrillic characters (Russian, Ukrainian, Belarusian, Bulgarian QWERTY standard mapping).
/// 4. Greek characters.
/// 5. Standard Latin letters and digits (converted to lowercase).
pub fn normalize_key(raw_text: &str, control: bool) -> Option<String> {
    if raw_text.is_empty() {
        return None;
    }

    // 0. Modifier keys alone must never be treated as shortcut trigger keys
    match raw_text {
        // Shift (\u{0010}), Control (\u{0011}), Alt (\u{0012}), AltGr (\u{0013}), CapsLock (\u{0014}), Meta (\u{0017})
        "\u{0010}" | "\u{0011}" | "\u{0012}" | "\u{0013}" | "\u{0014}" | "\u{0017}" => {
            return None;
        }
        _ => {}
    }

    // 1. Slint named keys (special Unicode characters)
    match raw_text {
        // Escape (\u{001b})
        "\u{001b}" => return Some("escape".to_string()),
        // Return (\n or \r)
        "\n" | "\r" => return Some("return".to_string()),
        // Tab (\t)
        "\t" => return Some("tab".to_string()),
        // Backtab (\u{0019} in Slint/Qt)
        "\u{0019}" if !control => return Some("backtab".to_string()),
        // F1 (\u{F704})
        "\u{F704}" => return Some("f1".to_string()),
        // F5 (\u{F708})
        "\u{F708}" => return Some("f5".to_string()),
        // F6 (\u{F709})
        "\u{F709}" => return Some("f6".to_string()),
        _ => {}
    }

    // 2. ASCII control characters when Control modifier is active (only letter codes, excluding modifier keys)
    if control {
        match raw_text {
            "\u{0001}" => return Some("a".to_string()),
            "\u{0002}" => return Some("b".to_string()),
            "\u{0003}" => return Some("c".to_string()),
            "\u{0004}" => return Some("d".to_string()),
            "\u{0005}" => return Some("e".to_string()),
            "\u{0006}" => return Some("f".to_string()),
            "\u{000b}" => return Some("k".to_string()),
            "\u{000c}" => return Some("l".to_string()),
            "\u{000e}" => return Some("n".to_string()),
            "\u{000f}" => return Some("o".to_string()),
            "\u{0016}" => return Some("v".to_string()),
            "\u{0018}" => return Some("x".to_string()),
            "\u{0019}" => return Some("y".to_string()),
            "\u{001a}" => return Some("z".to_string()),
            _ => {}
        }
    }

    // 3. Cyrillic and international layouts -> standard QWERTY key mapping
    let mut chars = raw_text.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        let normalized_char = match first {
            // Cyrillic letters (Russian / Ukrainian / Belarusian / Bulgarian)
            'ф' | 'Ф' => 'a',
            'и' | 'И' => 'b',
            'с' | 'С' => 'c',
            'в' | 'В' => 'd',
            'у' | 'У' => 'e',
            'а' | 'А' => 'f',
            'п' | 'П' => 'g',
            'р' | 'Р' => 'h',
            'ш' | 'Ш' => 'i',
            'о' | 'О' => 'j',
            'л' | 'Л' => 'k',
            'д' | 'Д' => 'l',
            'ь' | 'Ь' => 'm',
            'т' | 'Т' => 'n',
            'щ' | 'Щ' => 'o',
            'з' | 'З' => 'p',
            'й' | 'Й' => 'q',
            'к' | 'К' => 'r',
            'ы' | 'Ы' | 'і' | 'І' => 's',
            'е' | 'Е' => 't',
            'г' | 'Г' => 'u',
            'м' | 'М' => 'v',
            'ц' | 'Ц' => 'w',
            'ч' | 'Ч' => 'x',
            'н' | 'Н' => 'y',
            'я' | 'Я' => 'z',
            'х' | 'Х' => '[',
            'ъ' | 'Ъ' | 'ї' | 'Ї' => ']',
            'ж' | 'Ж' => ';',
            'э' | 'Э' | 'є' | 'Є' => '\'',
            'б' | 'Б' => ',',
            'ю' | 'Ю' => '.',
            'ё' | 'Ё' => '`',

            // Greek letters
            'α' | 'Α' => 'a',
            'β' | 'Β' => 'b',
            'ψ' | 'Ψ' => 'c',
            'δ' | 'Δ' => 'd',
            'ε' | 'Ε' => 'e',
            'φ' | 'Φ' => 'f',
            'γ' | 'Γ' => 'g',
            'η' | 'Η' => 'h',
            'ι' | 'Ι' => 'i',
            'ξ' | 'Ξ' => 'j',
            'κ' | 'Κ' => 'k',
            'λ' | 'Λ' => 'l',
            'μ' | 'Μ' => 'm',
            'ν' | 'Ν' => 'n',
            'ο' | 'Ο' => 'o',
            'π' | 'Π' => 'p',
            'θ' | 'Θ' => 'u',
            'ρ' | 'Ρ' => 'r',
            'σ' | 'ς' | 'Σ' => 's',
            'τ' | 'Τ' => 't',
            'υ' | 'Υ' => 'y',
            'ω' | 'Ω' => 'v',
            'ζ' | 'Ζ' => 'z',
            'χ' | 'Χ' => 'x',

            // Standard Latin and digits
            'a'..='z' | '0'..='9' => first,
            'A'..='Z' => first.to_ascii_lowercase(),

            other => other,
        };

        return Some(normalized_char.to_string());
    }

    // Fallback: lowercase string
    Some(raw_text.to_lowercase())
}

/// Registry storing mappings from key chords to application commands.
#[derive(Debug, Clone)]
pub struct CommandRegistry {
    bindings: HashMap<KeyChord, AppCommand>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        let mut reg = Self {
            bindings: HashMap::new(),
        };
        reg.register_defaults();
        reg
    }
}

impl CommandRegistry {
    /// Creates a new registry with default keybindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the default keyboard shortcuts for all commands.
    pub fn register_defaults(&mut self) {
        // File operations
        self.bind(KeyChord::ctrl("n"), AppCommand::NewFile);
        self.bind(KeyChord::ctrl("o"), AppCommand::OpenFile);
        self.bind(KeyChord::ctrl("s"), AppCommand::SaveFile);
        self.bind(KeyChord::ctrl_shift("s"), AppCommand::SaveAs);
        self.bind(KeyChord::ctrl("q"), AppCommand::Exit);
        self.bind(KeyChord::key("f5"), AppCommand::ReloadDirectory);

        // Edit operations
        self.bind(KeyChord::ctrl("z"), AppCommand::Undo);
        self.bind(KeyChord::ctrl_shift("z"), AppCommand::Redo);
        self.bind(KeyChord::ctrl("y"), AppCommand::Redo);
        self.bind(KeyChord::ctrl("x"), AppCommand::Cut);
        self.bind(KeyChord::ctrl("c"), AppCommand::Copy);
        self.bind(KeyChord::ctrl("v"), AppCommand::Paste);
        self.bind(KeyChord::ctrl("a"), AppCommand::SelectAll);
        self.bind(KeyChord::ctrl_shift("c"), AppCommand::CopyPreviewHtml);

        // Markdown formatting
        self.bind(KeyChord::ctrl("1"), AppCommand::FormatHeading1);
        self.bind(KeyChord::ctrl("2"), AppCommand::FormatHeading2);
        self.bind(KeyChord::ctrl("b"), AppCommand::FormatBold);
        self.bind(KeyChord::ctrl("i"), AppCommand::FormatItalic);
        self.bind(KeyChord::ctrl("k"), AppCommand::FormatCodeBlock);
        self.bind(KeyChord::ctrl_shift("t"), AppCommand::FormatTable);

        // View layout
        self.bind(KeyChord::ctrl_shift("b"), AppCommand::ToggleSidebar);
        self.bind(KeyChord::ctrl_shift("1"), AppCommand::ViewSplit);
        self.bind(KeyChord::ctrl_shift("2"), AppCommand::ViewEditor);
        self.bind(KeyChord::ctrl_shift("3"), AppCommand::ViewPreview);
        self.bind(KeyChord::key("f6"), AppCommand::FocusNextPane);
        self.bind(KeyChord::new("f6", false, false, true, false), AppCommand::FocusPrevPane);
        self.bind(KeyChord::ctrl("tab"), AppCommand::FocusNextPane);
        self.bind(KeyChord::ctrl_shift("tab"), AppCommand::FocusPrevPane);

        // Help & dialogs
        self.bind(KeyChord::key("f1"), AppCommand::HelpGuide);
        self.bind(KeyChord::ctrl("l"), AppCommand::Licenses);
        self.bind(KeyChord::key("escape"), AppCommand::DialogCancel);
        self.bind(KeyChord::key("return"), AppCommand::DialogConfirm);
    }

    /// Binds a key chord to an application command.
    pub fn bind(&mut self, chord: KeyChord, command: AppCommand) {
        self.bindings.insert(chord, command);
    }

    /// Unbinds a key chord.
    #[allow(dead_code)]
    pub fn unbind(&mut self, chord: &KeyChord) -> Option<AppCommand> {
        self.bindings.remove(chord)
    }

    /// Resolves an input key event to an application command using normalization.
    pub fn resolve(
        &self,
        raw_text: &str,
        control: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    ) -> Option<AppCommand> {
        let key = normalize_key(raw_text, control)?;
        let chord = KeyChord {
            control,
            alt,
            shift,
            meta,
            key,
        };
        self.bindings.get(&chord).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_cyrillic_keys() {
        assert_eq!(normalize_key("ф", true).as_deref(), Some("a"));
        assert_eq!(normalize_key("Ф", true).as_deref(), Some("a"));
        assert_eq!(normalize_key("ы", true).as_deref(), Some("s"));
        assert_eq!(normalize_key("і", true).as_deref(), Some("s")); // Ukrainian
        assert_eq!(normalize_key("с", true).as_deref(), Some("c"));
        assert_eq!(normalize_key("м", true).as_deref(), Some("v"));
        assert_eq!(normalize_key("ч", true).as_deref(), Some("x"));
        assert_eq!(normalize_key("я", true).as_deref(), Some("z"));
        assert_eq!(normalize_key("н", true).as_deref(), Some("y"));
        assert_eq!(normalize_key("и", true).as_deref(), Some("b"));
        assert_eq!(normalize_key("ш", true).as_deref(), Some("i"));
        assert_eq!(normalize_key("л", true).as_deref(), Some("k"));
        assert_eq!(normalize_key("д", true).as_deref(), Some("l"));
    }

    #[test]
    fn test_normalize_ascii_control_codes() {
        assert_eq!(normalize_key("\u{0001}", true).as_deref(), Some("a")); // Ctrl+A
        assert_eq!(normalize_key("\u{0002}", true).as_deref(), Some("b")); // Ctrl+B
        assert_eq!(normalize_key("\u{0003}", true).as_deref(), Some("c")); // Ctrl+C
        assert_eq!(normalize_key("\u{000b}", true).as_deref(), Some("k")); // Ctrl+K
        assert_eq!(normalize_key("\u{0016}", true).as_deref(), Some("v")); // Ctrl+V
        assert_eq!(normalize_key("\u{0018}", true).as_deref(), Some("x")); // Ctrl+X
        assert_eq!(normalize_key("\u{0019}", true).as_deref(), Some("y")); // Ctrl+Y
        assert_eq!(normalize_key("\u{001a}", true).as_deref(), Some("z")); // Ctrl+Z
    }

    #[test]
    fn test_normalize_special_named_keys() {
        assert_eq!(normalize_key("\u{001b}", false).as_deref(), Some("escape"));
        assert_eq!(normalize_key("\n", false).as_deref(), Some("return"));
        assert_eq!(normalize_key("\r", false).as_deref(), Some("return"));
        assert_eq!(normalize_key("\u{F704}", false).as_deref(), Some("f1"));
        assert_eq!(normalize_key("\u{F708}", false).as_deref(), Some("f5"));
    }

    #[test]
    fn test_resolve_default_shortcuts() {
        let registry = CommandRegistry::new();

        // Latin Save
        assert_eq!(
            registry.resolve("s", true, false, false, false),
            Some(AppCommand::SaveFile)
        );
        // Cyrillic Save (Russian 'ы', Ukrainian 'і')
        assert_eq!(
            registry.resolve("ы", true, false, false, false),
            Some(AppCommand::SaveFile)
        );
        assert_eq!(
            registry.resolve("і", true, false, false, false),
            Some(AppCommand::SaveFile)
        );

        // Undo (Latin 'z', Cyrillic 'я', ASCII \u{001a})
        assert_eq!(
            registry.resolve("z", true, false, false, false),
            Some(AppCommand::Undo)
        );
        assert_eq!(
            registry.resolve("я", true, false, false, false),
            Some(AppCommand::Undo)
        );

        // Redo (Ctrl+Shift+Z and Ctrl+Y / Cyrillic 'н')
        assert_eq!(
            registry.resolve("z", true, false, true, false),
            Some(AppCommand::Redo)
        );
        assert_eq!(
            registry.resolve("я", true, false, true, false),
            Some(AppCommand::Redo)
        );
        assert_eq!(
            registry.resolve("y", true, false, false, false),
            Some(AppCommand::Redo)
        );
        assert_eq!(
            registry.resolve("н", true, false, false, false),
            Some(AppCommand::Redo)
        );

        // Formatting
        assert_eq!(
            registry.resolve("b", true, false, false, false),
            Some(AppCommand::FormatBold)
        );
        assert_eq!(
            registry.resolve("и", true, false, false, false),
            Some(AppCommand::FormatBold)
        );
        assert_eq!(
            registry.resolve("i", true, false, false, false),
            Some(AppCommand::FormatItalic)
        );
        assert_eq!(
            registry.resolve("ш", true, false, false, false),
            Some(AppCommand::FormatItalic)
        );
        assert_eq!(
            registry.resolve("k", true, false, false, false),
            Some(AppCommand::FormatCodeBlock)
        );
        assert_eq!(
            registry.resolve("л", true, false, false, false),
            Some(AppCommand::FormatCodeBlock)
        );
        assert_eq!(
            registry.resolve("t", true, false, true, false),
            Some(AppCommand::FormatTable)
        );
        assert_eq!(
            registry.resolve("е", true, false, true, false),
            Some(AppCommand::FormatTable)
        );

        // Clipboard
        assert_eq!(
            registry.resolve("c", true, false, false, false),
            Some(AppCommand::Copy)
        );
        assert_eq!(
            registry.resolve("с", true, false, false, false),
            Some(AppCommand::Copy)
        );
        assert_eq!(
            registry.resolve("v", true, false, false, false),
            Some(AppCommand::Paste)
        );
        assert_eq!(
            registry.resolve("м", true, false, false, false),
            Some(AppCommand::Paste)
        );

        // Function keys
        assert_eq!(
            registry.resolve("\u{F708}", false, false, false, false),
            Some(AppCommand::ReloadDirectory)
        );
        assert_eq!(
            registry.resolve("\u{F704}", false, false, false, false),
            Some(AppCommand::HelpGuide)
        );
    }

    #[test]
    fn test_custom_rebinding() {
        let mut registry = CommandRegistry::new();
        // Bind Ctrl+D to FormatBold
        registry.bind(KeyChord::ctrl("d"), AppCommand::FormatBold);
        assert_eq!(
            registry.resolve("d", true, false, false, false),
            Some(AppCommand::FormatBold)
        );
        assert_eq!(
            registry.resolve("в", true, false, false, false),
            Some(AppCommand::FormatBold)
        );
    }

    #[test]
    fn test_modifier_keys_alone_never_resolve() {
        let registry = CommandRegistry::new();
        let ctrl_char: char = slint::platform::Key::Control.into();
        let shift_char: char = slint::platform::Key::Shift.into();
        let alt_char: char = slint::platform::Key::Alt.into();
        let meta_char: char = slint::platform::Key::Meta.into();
        let altgr_char: char = slint::platform::Key::AltGr.into();

        let ctrl_str = ctrl_char.to_string();
        let shift_str = shift_char.to_string();
        let alt_str = alt_char.to_string();
        let meta_str = meta_char.to_string();
        let altgr_str = altgr_char.to_string();

        assert_eq!(normalize_key(&ctrl_str, true), None);
        assert_eq!(normalize_key(&shift_str, false), None);
        assert_eq!(normalize_key(&alt_str, false), None);
        assert_eq!(normalize_key(&meta_str, false), None);
        assert_eq!(normalize_key(&altgr_str, false), None);

        assert_eq!(registry.resolve(&ctrl_str, true, false, false, false), None);
        assert_eq!(registry.resolve(&shift_str, false, false, true, false), None);
        assert_eq!(registry.resolve(&alt_str, false, true, false, false), None);
    }

    #[test]
    fn test_help_dialog_commands() {
        assert_eq!(AppCommand::Licenses.id(), "help.licenses");
        assert_eq!(AppCommand::About.id(), "help.about");
        assert_eq!(AppCommand::FormatTable.id(), "format.table");
        assert_eq!(AppCommand::FocusNextPane.id(), "view.focus_next_pane");
        assert_eq!(AppCommand::FocusPrevPane.id(), "view.focus_prev_pane");
        assert_eq!(AppCommand::from_id("help.licenses"), Some(AppCommand::Licenses));
        assert_eq!(AppCommand::from_id("help.about"), Some(AppCommand::About));
        assert_eq!(AppCommand::from_id("format.table"), Some(AppCommand::FormatTable));
        assert_eq!(AppCommand::from_id("view.focus_next_pane"), Some(AppCommand::FocusNextPane));
        assert_eq!(AppCommand::from_id("view.focus_prev_pane"), Some(AppCommand::FocusPrevPane));
    }

    #[test]
    fn test_pane_navigation_shortcuts() {
        let registry = CommandRegistry::new();
        // F6
        assert_eq!(
            registry.resolve("\u{F709}", false, false, false, false),
            Some(AppCommand::FocusNextPane)
        );
        // Shift+F6
        assert_eq!(
            registry.resolve("\u{F709}", false, false, true, false),
            Some(AppCommand::FocusPrevPane)
        );
        // Ctrl+Tab
        assert_eq!(
            registry.resolve("\t", true, false, false, false),
            Some(AppCommand::FocusNextPane)
        );
        // Ctrl+Shift+Tab
        assert_eq!(
            registry.resolve("\t", true, false, true, false),
            Some(AppCommand::FocusPrevPane)
        );
    }
}
