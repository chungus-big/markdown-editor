//! Native file selection, save, and help dialogs via Zenity.

use std::path::PathBuf;
use std::process::Child;
use std::sync::Mutex;

/// Native help dialog manager ensuring single-instance execution for About and Licenses dialogs.
#[derive(Default)]
pub struct HelpDialogManager {
    about_process: Mutex<Option<Child>>,
    licenses_process: Mutex<Option<Child>>,
}

impl HelpDialogManager {
    /// Creates a new HelpDialogManager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a child process is still running. If it exited, reaps the process and clears the slot.
    fn is_running(process: &mut Option<Child>) -> bool {
        match process.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => true, // Process is still running
                Ok(Some(_)) => {
                    *process = None; // Process exited
                    false
                }
                Err(_) => {
                    *process = None; // Error querying status, assume closed
                    false
                }
            },
            None => false,
        }
    }

    /// Opens the native About dialog. If an instance is already running, does nothing.
    pub fn show_about(&self, is_ru: bool) {
        let mut lock = match self.about_process.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if Self::is_running(&mut lock) {
            return;
        }

        let title = if is_ru {
            "О программе Markdown Editor"
        } else {
            "About Markdown Editor"
        };
        let text = if is_ru {
            "<b>Markdown Editor</b> (CommonMark + GFM)\n\
            Версия: <b>0.2.0</b>\n\
            Лицензия: <b>GNU General Public License v3.0 (GPL-3.0-or-later)</b>\n\n\
            Быстрый и легковесный десктопный редактор Markdown на Rust и Slint.\n\n\
            <b>Возможности:</b>\n\
            • CommonMark 0.31: заголовки, списки, цитаты\n\
            • GFM Таблицы: адаптивная ширина, выравнивание, перенос\n\
            • GFM Задачи (- [ ], - [x]), сноски ([^note]) и зачеркивание\n\
            • Блоки кода: выделение, копирование, моноширинный шрифт\n\
            • Режимы: сплит, только редактор, только превью\n\
            • Дерево файлов, статистика слов/символов, поиск\n\
            • Светлая и тёмная темы, автоопределение темы ОС\n\
            • Полная поддержка русского и английского языков"
        } else {
            "<b>Markdown Editor</b> (CommonMark + GFM)\n\
            Version: <b>0.2.0</b>\n\
            License: <b>GNU General Public License v3.0 (GPL-3.0-or-later)</b>\n\n\
            Fast and lightweight desktop Markdown editor built with Rust and Slint.\n\n\
            <b>Features:</b>\n\
            • CommonMark 0.31: headings, lists, blockquotes\n\
            • GFM Tables: auto-width, alignments, responsive wrapping\n\
            • GFM Task Lists (- [ ], - [x]), footnotes ([^note]), strikethrough\n\
            • Code blocks: selection, one-click copy, monospaced layout\n\
            • View modes: Split, Editor Only, Preview Only\n\
            • File tree sidebar, live word/character statistics, search\n\
            • Light and Dark themes with automatic OS scheme detection\n\
            • Full English and Russian localization"
        };

        if let Some(child) = spawn_dialog(title, text, 440) {
            *lock = Some(child);
        }
    }

    /// Opens the native Licenses dialog. If an instance is already running, does nothing.
    pub fn show_licenses(&self, is_ru: bool) {
        let mut lock = match self.licenses_process.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if Self::is_running(&mut lock) {
            return;
        }

        let title = if is_ru {
            "Сведения о лицензиях — Markdown Editor"
        } else {
            "License Information — Markdown Editor"
        };
        let text = if is_ru {
            "<b>Сведения о лицензиях</b>\n\n\
            <b>Markdown Editor</b>\n\
            Лицензия: GNU General Public License v3.0 (GPL-3.0-or-later)\n\
            Свободный редактор Markdown с открытым исходным кодом.\n\n\
            <b>Шрифты:</b>\n\
            • Hack Nerd Font: SIL Open Font License 1.1\n\
            • Ubuntu Fonts: Ubuntu Font License 1.0\n\
            • Symbols Nerd Font: MIT License\n\n\
            <b>Иконки:</b>\n\
            • Octicons: MIT License (GitHub, Inc.)\n\
            • Devicons: MIT License (Theodore Vorillas)\n\
            • Font Awesome (Free): SIL OFL 1.1 / MIT License\n\n\
            <b>Основные библиотеки Rust:</b>\n\
            • Slint GUI: GNU GPL v3.0 / Slint Free Desktop License\n\
            • pulldown-cmark: MIT License (парсер CommonMark + GFM)\n\
            • dark-light: MIT / Apache 2.0 (определение темы ОС)\n\n\
            Подробные тексты лицензий доступны в файле THIRD_PARTY_LICENSES.md."
        } else {
            "<b>License Information</b>\n\n\
            <b>Markdown Editor</b>\n\
            License: GNU General Public License v3.0 (GPL-3.0-or-later)\n\
            Free and open-source desktop Markdown editor.\n\n\
            <b>Included Fonts:</b>\n\
            • Hack Nerd Font: SIL Open Font License 1.1\n\
            • Ubuntu Fonts: Ubuntu Font License 1.0\n\
            • Symbols Nerd Font: MIT License\n\n\
            <b>Icon Sets:</b>\n\
            • Octicons: MIT License (GitHub, Inc.)\n\
            • Devicons: MIT License (Theodore Vorillas)\n\
            • Font Awesome (Free): SIL OFL 1.1 / MIT License\n\n\
            <b>Core Rust Libraries:</b>\n\
            • Slint GUI: GNU GPL v3.0 / Slint Free Desktop License\n\
            • pulldown-cmark: MIT License (CommonMark + GFM parser)\n\
            • dark-light: MIT / Apache 2.0 (OS theme detection)\n\n\
            Detailed license texts are provided in the THIRD_PARTY_LICENSES.md file."
        };

        if let Some(child) = spawn_dialog(title, text, 520) {
            *lock = Some(child);
        }
    }
}

impl Drop for HelpDialogManager {
    fn drop(&mut self) {
        if let Ok(mut lock) = self.about_process.lock()
            && let Some(mut child) = lock.take()
        {
            let _ = child.kill();
        }
        if let Ok(mut lock) = self.licenses_process.lock()
            && let Some(mut child) = lock.take()
        {
            let _ = child.kill();
        }
    }
}

fn spawn_dialog(title: &str, text: &str, width: u32) -> Option<Child> {
    if let Ok(child) = std::process::Command::new("zenity")
        .args([
            "--info",
            &format!("--title={}", title),
            &format!("--text={}", text),
            &format!("--width={}", width),
        ])
        .spawn()
    {
        return Some(child);
    }

    let plain = text
        .replace("<b>", "")
        .replace("</b>", "")
        .replace("<i>", "")
        .replace("</i>", "");

    if let Ok(child) = std::process::Command::new("kdialog")
        .args(["--title", title, "--msgbox", &plain])
        .spawn()
    {
        return Some(child);
    }

    None
}

/// Opens a Zenity native file picker dialog for opening Markdown files.
pub fn pick_file_dialog(is_ru: bool) -> Option<PathBuf> {
    let title = if is_ru {
        "Открыть Markdown файл"
    } else {
        "Open Markdown File"
    };
    let all_files = if is_ru {
        "Все файлы | *"
    } else {
        "All Files | *"
    };
    if let Ok(output) = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            &format!("--title={}", title),
            "--file-filter=Markdown (*.md *.markdown) | *.md *.markdown",
            &format!("--file-filter={}", all_files),
        ])
        .output()
        && output.status.success()
    {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            return Some(PathBuf::from(path_str));
        }
    }
    None
}

/// Opens a Zenity native file dialog for saving Markdown files.
pub fn save_file_dialog(default_name: &str, is_ru: bool) -> Option<PathBuf> {
    let title = if is_ru {
        "Сохранить Markdown файл"
    } else {
        "Save Markdown File"
    };
    if let Ok(output) = std::process::Command::new("zenity")
        .args([
            "--file-selection",
            "--save",
            "--confirm-overwrite",
            &format!("--title={}", title),
            &format!("--filename={}", default_name),
            "--file-filter=Markdown (*.md) | *.md",
        ])
        .output()
        && output.status.success()
    {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            return Some(PathBuf::from(path_str));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_dialog_manager_initial_state() {
        let manager = HelpDialogManager::new();
        let mut lock = manager.about_process.lock().unwrap();
        assert!(!HelpDialogManager::is_running(&mut lock));
        let mut lock_lic = manager.licenses_process.lock().unwrap();
        assert!(!HelpDialogManager::is_running(&mut lock_lic));
    }
}
