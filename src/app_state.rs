//! Application state management, document operations, and deferred pending actions.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::dialogs::{pick_file_dialog, save_file_dialog};
use crate::file_browser;
use crate::markdown;
use crate::preview::{update_file_tree, update_preview, FootnoteNavigationState};
use crate::system::{get_document_fallback_name, get_sample_markdown};
use crate::AppWindow;
use slint::ComponentHandle;

/// Central runtime state of the application.
pub struct AppState {
    pub current_file: Option<PathBuf>,
    pub current_dir: PathBuf,
    pub expanded_dirs: HashSet<PathBuf>,
    pub footnote_nav: FootnoteNavigationState,
}

/// Actions requested by the user that may be deferred if there are unsaved changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingAction {
    None,
    Exit,
    NewFile,
    OpenFile,
    SelectFile(PathBuf),
    LoadSample,
    HelpGuide,
}

/// Loads a Markdown file from disk into the editor and preview.
pub fn load_file(ui: &AppWindow, path: &Path, state: &Rc<RefCell<AppState>>, is_ru: bool) {
    if let Ok(content) = std::fs::read_to_string(path) {
        let clean_content = markdown::expand_tabs(&content, 4);
        ui.set_markdown_text(clean_content.as_str().into());
        {
            let mut st = state.borrow_mut();
            st.footnote_nav.clear_recorded();
            update_preview(ui, &clean_content, &mut st.footnote_nav);
        }

        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| get_document_fallback_name(is_ru).to_string());

        let dir = file_browser::get_parent_or_current(path);

        ui.set_current_file_name(file_name.into());
        ui.set_current_file_path(path.to_string_lossy().to_string().into());
        ui.set_is_modified(false);

        let mut st = state.borrow_mut();
        st.current_file = Some(path.to_path_buf());
        st.current_dir = dir.clone();
        file_browser::expand_ancestors_of(&dir, path, &mut st.expanded_dirs);

        update_file_tree(ui, &dir, Some(path), &st.expanded_dirs);
        ui.invoke_focus_editor();
    }
}

/// Saves the current editor content to disk. Prompts for a file name if untitled.
pub fn save_file(ui: &AppWindow, state: &Rc<RefCell<AppState>>, is_ru: bool) -> bool {
    let target_path = {
        let st = state.borrow();
        match &st.current_file {
            Some(p) => p.clone(),
            None => {
                let default_name = st.current_dir.join("document.md");
                if let Some(chosen) = save_file_dialog(&default_name.to_string_lossy(), is_ru) {
                    chosen
                } else {
                    return false;
                }
            }
        }
    };

    let text = ui.get_markdown_text();
    if std::fs::write(&target_path, text.as_str()).is_ok() {
        let file_name = target_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| get_document_fallback_name(is_ru).to_string());
        let dir = file_browser::get_parent_or_current(&target_path);

        ui.set_current_file_name(file_name.into());
        ui.set_current_file_path(target_path.to_string_lossy().to_string().into());
        ui.set_is_modified(false);

        let mut st = state.borrow_mut();
        st.current_file = Some(target_path.clone());
        st.current_dir = dir.clone();
        file_browser::expand_ancestors_of(&dir, &target_path, &mut st.expanded_dirs);

        update_file_tree(ui, &dir, Some(&target_path), &st.expanded_dirs);
        true
    } else {
        false
    }
}

/// Executes a requested pending action without prompting.
pub fn execute_pending_action(
    ui: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    action: PendingAction,
    is_ru: bool,
) {
    match action {
        PendingAction::None => {}
        PendingAction::Exit => {
            let _ = ui.hide();
        }
        PendingAction::NewFile => {
            ui.set_markdown_text("".into());
            let (dir, expanded) = {
                let mut st = state.borrow_mut();
                st.footnote_nav.clear_recorded();
                update_preview(ui, "", &mut st.footnote_nav);
                st.current_file = None;
                (st.current_dir.clone(), st.expanded_dirs.clone())
            };
            ui.set_current_file_name(if is_ru {
                "Безымянный.md".into()
            } else {
                "Untitled.md".into()
            });
            ui.set_current_file_path("".into());
            ui.set_is_modified(false);

            update_file_tree(ui, &dir, None, &expanded);
            ui.invoke_focus_editor();
        }
        PendingAction::OpenFile => {
            if let Some(chosen_path) = pick_file_dialog(is_ru) {
                load_file(ui, &chosen_path, state, is_ru);
            }
        }
        PendingAction::SelectFile(path) => {
            load_file(ui, &path, state, is_ru);
        }
        PendingAction::LoadSample => {
            let sample = get_sample_markdown(is_ru);
            ui.set_markdown_text(sample.into());
            let (dir, expanded) = {
                let mut st = state.borrow_mut();
                st.footnote_nav.clear_recorded();
                update_preview(ui, sample, &mut st.footnote_nav);
                st.current_file = None;
                (st.current_dir.clone(), st.expanded_dirs.clone())
            };
            ui.set_current_file_name(if is_ru {
                "Пример GFM".into()
            } else {
                "GFM Sample".into()
            });
            ui.set_current_file_path("".into());
            ui.set_is_modified(false);

            update_file_tree(ui, &dir, None, &expanded);
            ui.invoke_focus_editor();
        }
        PendingAction::HelpGuide => {
            let sample = get_sample_markdown(is_ru);
            ui.set_markdown_text(sample.into());
            let (dir, expanded) = {
                let mut st = state.borrow_mut();
                st.footnote_nav.clear_recorded();
                update_preview(ui, sample, &mut st.footnote_nav);
                st.current_file = None;
                (st.current_dir.clone(), st.expanded_dirs.clone())
            };
            ui.set_current_file_name(if is_ru {
                "Справка CommonMark + GFM".into()
            } else {
                "CommonMark + GFM Guide".into()
            });
            ui.set_current_file_path("".into());
            ui.set_is_modified(false);

            update_file_tree(ui, &dir, None, &expanded);
        }
    }
}

/// Checks if document is modified before performing an action; if so, displays confirmation dialog.
pub fn request_action(
    ui: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    pending: &Rc<RefCell<PendingAction>>,
    action: PendingAction,
    is_ru: bool,
) {
    if ui.get_is_modified() {
        *pending.borrow_mut() = action;
        ui.set_show_unsaved_dialog(true);
    } else {
        execute_pending_action(ui, state, action, is_ru);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_action_variants() {
        let action = PendingAction::Exit;
        assert_eq!(action, PendingAction::Exit);

        let select = PendingAction::SelectFile(PathBuf::from("test.md"));
        assert_eq!(select, PendingAction::SelectFile(PathBuf::from("test.md")));
        assert_ne!(select, PendingAction::None);

        let new_file = PendingAction::NewFile;
        assert_eq!(new_file, PendingAction::NewFile);

        let open_file = PendingAction::OpenFile;
        assert_eq!(open_file, PendingAction::OpenFile);
    }
}
