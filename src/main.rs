// Prevent console window in addition to Slint window in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod dialogs;
mod file_browser;
mod formatting;
mod markdown;
mod preview;
mod shortcuts;
mod system;

use shortcuts::{AppCommand, CommandRegistry};

use std::cell::RefCell;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use app_state::{execute_pending_action, load_file, request_action, save_file, AppState, PendingAction};
use dialogs::{save_file_dialog, HelpDialogManager};
use formatting::{apply_formatting, UndoAction};
use preview::{update_file_tree, update_preview};
use system::{
    detect_system_dark, detect_system_language, get_document_fallback_name, get_sample_markdown,
    is_russian,
};

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;

    // Initialize Theme: 0 = system, 1 = light, 2 = dark
    let initial_dark = detect_system_dark();
    ui.invoke_set_theme_mode(0);
    ui.set_is_dark(initial_dark);

    // Initialize Language: 0 = system, 1 = english, 2 = russian
    let language_mode_atomic = Arc::new(AtomicU8::new(0));
    ui.invoke_set_language_mode(0);
    ui.invoke_set_editor_font_mode(1);
    ui.invoke_set_view_mode(0);
    let initial_lang = detect_system_language();
    let _ = slint::select_bundled_translation(initial_lang);
    let is_ru_initial = initial_lang == "ru";

    let initial_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let state = Rc::new(RefCell::new(AppState {
        current_file: None,
        current_dir: initial_dir.clone(),
        expanded_dirs: HashSet::new(),
        footnote_nav: preview::FootnoteNavigationState::default(),
        preview_blocks: Vec::new(),
    }));
    let pending_action = Rc::new(RefCell::new(PendingAction::None));

    let state_height = Rc::clone(&state);
    ui.on_compute_preview_height(move |width, _version| {
        if let Ok(st) = state_height.try_borrow() {
            markdown::layout::estimate_preview_height_for_width(&st.preview_blocks, true, width)
        } else {
            0.0
        }
    });

    // Handle window close request ('X' button in titlebar)
    let ui_weak = ui.as_weak();
    let pending_action_close = Rc::clone(&pending_action);
    ui.window().on_close_requested(move || {
        if let Some(ui) = ui_weak.upgrade()
            && ui.get_is_modified()
        {
            *pending_action_close.borrow_mut() = PendingAction::Exit;
            ui.set_show_unsaved_dialog(true);
            return slint::CloseRequestResponse::KeepWindowShown;
        }
        slint::CloseRequestResponse::HideWindow
    });

    let theme_mode_atomic = Arc::new(AtomicU8::new(0));
    let help_dialog_manager = Arc::new(HelpDialogManager::new());

    // Check CLI argument for opening a specific file or folder
    let cli_arg = std::env::args().nth(1);
    if let Some(arg) = cli_arg {
        let p = PathBuf::from(arg);
        if p.is_file() {
            load_file(&ui, &p, &state, is_ru_initial);
        } else if p.is_dir() {
            let mut st = state.borrow_mut();
            st.current_dir = p.clone();
            drop(st);
            let (files, _) =
                file_browser::scan_directory_tree(&p, None, &HashSet::new());
            if let Some(first_file) = files.iter().find(|item| !item.is_dir) {
                load_file(&ui, &PathBuf::from(&first_file.path), &state, is_ru_initial);
            } else {
                let sample = get_sample_markdown(is_ru_initial);
                ui.set_markdown_text(sample.into());
                update_preview(&ui, sample, &mut state.borrow_mut());
                ui.set_current_file_name(if is_ru_initial { "Пример GFM".into() } else { "GFM Sample".into() });
                let st = state.borrow();
                update_file_tree(&ui, &p, None, &st.expanded_dirs);
            }
        } else {
            // Specified path not found, open sample in default dir
            let sample = get_sample_markdown(is_ru_initial);
            ui.set_markdown_text(sample.into());
            update_preview(&ui, sample, &mut state.borrow_mut());
            ui.set_current_file_name(if is_ru_initial { "Пример GFM".into() } else { "GFM Sample".into() });
            let st = state.borrow();
            update_file_tree(&ui, &initial_dir, None, &st.expanded_dirs);
        }
    } else {
        // Automatically discover markdown files in the current working directory
        let (files, _) =
            file_browser::scan_directory_tree(&initial_dir, None, &HashSet::new());
        let readme = files
            .iter()
            .find(|f| !f.is_dir && f.name.eq_ignore_ascii_case("readme.md"));
        if let Some(target) = readme.or_else(|| files.iter().find(|f| !f.is_dir)) {
            load_file(&ui, &PathBuf::from(&target.path), &state, is_ru_initial);
        } else {
            let sample = get_sample_markdown(is_ru_initial);
            ui.set_markdown_text(sample.into());
            update_preview(&ui, sample, &mut state.borrow_mut());
            ui.set_current_file_name(if is_ru_initial { "Пример GFM".into() } else { "GFM Sample".into() });
            let st = state.borrow();
            update_file_tree(&ui, &initial_dir, None, &st.expanded_dirs);
        }
    }

    let undo_stack: Arc<Mutex<Vec<UndoAction>>> = Arc::new(Mutex::new(Vec::new()));
    let redo_stack: Arc<Mutex<Vec<UndoAction>>> = Arc::new(Mutex::new(Vec::new()));
    let is_programmatic = Arc::new(AtomicBool::new(false));

    // Callback on text edit
    let ui_weak = ui.as_weak();
    let undo_stack_edit = Arc::clone(&undo_stack);
    let redo_stack_edit = Arc::clone(&redo_stack);
    let is_prog_edit = Arc::clone(&is_programmatic);
    let state_text = Rc::clone(&state);
    ui.on_text_edited(move |new_text| {
        if let Some(ui) = ui_weak.upgrade() {
            if !is_prog_edit.load(Ordering::SeqCst) {
                if let Ok(mut stack) = undo_stack_edit.lock() {
                    stack.clear();
                }
                if let Ok(mut rstack) = redo_stack_edit.lock() {
                    rstack.clear();
                }
            }
            if new_text.contains('\t') {
                let clean = markdown::expand_tabs(&new_text, 4);
                ui.set_markdown_text(clean.as_str().into());
                ui.set_is_modified(true);
                update_preview(&ui, &clean, &mut state_text.borrow_mut());
                return;
            }
            ui.set_is_modified(true);
            update_preview(&ui, &new_text, &mut state_text.borrow_mut());
        }
    });

    // Callback on file selected in the tree
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let pending_clone = Rc::clone(&pending_action);
    let lang_mode_file = Arc::clone(&language_mode_atomic);
    ui.on_file_selected(move |path_str| {
        if let Some(ui) = ui_weak.upgrade() {
            let path = PathBuf::from(path_str.as_str());
            let is_ru = is_russian(lang_mode_file.load(Ordering::Relaxed));
            request_action(&ui, &state_clone, &pending_clone, PendingAction::SelectFile(path), is_ru);
        }
    });

    // Callback on directory expand/collapse toggle
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    ui.on_folder_toggled(move |path_str| {
        if let Some(ui) = ui_weak.upgrade() {
            let path = PathBuf::from(path_str.as_str());
            let can_path = std::fs::canonicalize(&path).unwrap_or(path);

            let mut st = state_clone.borrow_mut();
            if st.expanded_dirs.contains(&can_path) {
                st.expanded_dirs.remove(&can_path);
            } else {
                st.expanded_dirs.insert(can_path);
            }

            let active = st.current_file.clone();
            let dir = st.current_dir.clone();
            let expanded = st.expanded_dirs.clone();
            drop(st);

            update_file_tree(&ui, &dir, active.as_deref(), &expanded);
        }
    });

    // Callback on refresh file tree
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    ui.on_refresh_files_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            perform_refresh_files(&ui, &state_clone);
        }
    });

    // Callback on open file dialog
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let pending_clone = Rc::clone(&pending_action);
    let lang_mode_open = Arc::clone(&language_mode_atomic);
    ui.on_open_file_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_open.load(Ordering::Relaxed));
            request_action(&ui, &state_clone, &pending_clone, PendingAction::OpenFile, is_ru);
        }
    });

    // Callback on save file
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let lang_mode_save = Arc::clone(&language_mode_atomic);
    ui.on_save_file_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_save.load(Ordering::Relaxed));
            save_file(&ui, &state_clone, is_ru);
        }
    });

    // Callback on load sample
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let pending_clone = Rc::clone(&pending_action);
    let lang_mode_load = Arc::clone(&language_mode_atomic);
    ui.on_load_sample_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_load.load(Ordering::Relaxed));
            request_action(&ui, &state_clone, &pending_clone, PendingAction::LoadSample, is_ru);
        }
    });

    // Callback on clear
    let ui_weak = ui.as_weak();
    let undo_stack_clear = Arc::clone(&undo_stack);
    let redo_stack_clear = Arc::clone(&redo_stack);
    let state_clear = Rc::clone(&state);
    ui.on_clear_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            if let Ok(mut stack) = undo_stack_clear.lock() {
                stack.clear();
            }
            if let Ok(mut rstack) = redo_stack_clear.lock() {
                rstack.clear();
            }
            ui.set_markdown_text("".into());
            {
                let mut st = state_clear.borrow_mut();
                st.footnote_nav.clear_recorded();
                update_preview(&ui, "", &mut st);
            }
            ui.set_is_modified(true);
            ui.invoke_focus_editor();
        }
    });

    // Callback on new file
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let pending_clone = Rc::clone(&pending_action);
    let lang_mode_new = Arc::clone(&language_mode_atomic);
    ui.on_new_file_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_new.load(Ordering::Relaxed));
            request_action(&ui, &state_clone, &pending_clone, PendingAction::NewFile, is_ru);
        }
    });

    // Callback on save as
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let lang_mode_save_as = Arc::clone(&language_mode_atomic);
    ui.on_save_as_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_save_as.load(Ordering::Relaxed));
            perform_save_as(&ui, &state_clone, is_ru);
        }
    });

    // Callback on exit
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let pending_clone = Rc::clone(&pending_action);
    let lang_mode_exit = Arc::clone(&language_mode_atomic);
    ui.on_exit_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_exit.load(Ordering::Relaxed));
            request_action(&ui, &state_clone, &pending_clone, PendingAction::Exit, is_ru);
        }
    });

    // Callback on insert formatting (bold, italic, code block, headings, etc.)
    let ui_weak = ui.as_weak();
    let undo_stack_fmt = Arc::clone(&undo_stack);
    let redo_stack_fmt = Arc::clone(&redo_stack);
    let is_prog_fmt = Arc::clone(&is_programmatic);
    ui.on_insert_formatting(move |kind| {
        if let Some(ui) = ui_weak.upgrade() {
            apply_formatting(&ui, &kind, &undo_stack_fmt, &redo_stack_fmt, &is_prog_fmt);
        }
    });

    // Callback on insert custom table (columns and rows)
    let ui_weak = ui.as_weak();
    let undo_stack_tbl = Arc::clone(&undo_stack);
    let redo_stack_tbl = Arc::clone(&redo_stack);
    let is_prog_tbl = Arc::clone(&is_programmatic);
    ui.on_insert_custom_table(move |cols, rows| {
        if let Some(ui) = ui_weak.upgrade() {
            let cols = (cols as usize).clamp(1, 50);
            let rows = (rows as usize).clamp(1, 100);
            let kind = format!("table:{}:{}", cols, rows);
            apply_formatting(&ui, &kind, &undo_stack_tbl, &redo_stack_tbl, &is_prog_tbl);
        }
    });

    // Callback on insert markdown snippet (fallback)
    let ui_weak = ui.as_weak();
    let undo_stack_snp = Arc::clone(&undo_stack);
    let redo_stack_snp = Arc::clone(&redo_stack);
    let is_prog_snp = Arc::clone(&is_programmatic);
    ui.on_insert_snippet(move |snippet| {
        if let Some(ui) = ui_weak.upgrade() {
            apply_formatting(&ui, &snippet, &undo_stack_snp, &redo_stack_snp, &is_prog_snp);
        }
    });

    // Callback on undo requested
    let ui_weak = ui.as_weak();
    let undo_stack_undo = Arc::clone(&undo_stack);
    let redo_stack_undo = Arc::clone(&redo_stack);
    let is_prog_undo = Arc::clone(&is_programmatic);
    ui.on_undo_requested(move || {
        if let Some(ui) = ui_weak.upgrade() {
            perform_undo(&ui, &undo_stack_undo, &redo_stack_undo, &is_prog_undo);
        }
    });

    // Callback on redo requested
    let ui_weak = ui.as_weak();
    let undo_stack_redo = Arc::clone(&undo_stack);
    let redo_stack_redo = Arc::clone(&redo_stack);
    let is_prog_redo = Arc::clone(&is_programmatic);
    ui.on_redo_requested(move || {
        if let Some(ui) = ui_weak.upgrade() {
            perform_redo(&ui, &undo_stack_redo, &redo_stack_redo, &is_prog_redo);
        }
    });

    // Callback on tab pressed in editor: insert 4 spaces with selection safety guard
    let ui_weak = ui.as_weak();
    ui.on_tab_pressed(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let ui_weak_ev = ui.as_weak();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak_ev.upgrade() {
                    let text = ui.get_markdown_text();
                    let text_len = text.len() as i32;
                    let anchor = ui.get_editor_anchor_pos();
                    let cursor = ui.get_editor_cursor_pos();

                    // Safety guard: If the entire text was selected (e.g. by Slint's TabNavigation),
                    // clear the selection instead of replacing the whole document with 4 spaces!
                    if text_len > 0
                        && ((anchor == 0 && cursor == text_len)
                            || (cursor == 0 && anchor == text_len))
                    {
                        ui.invoke_set_editor_selection(cursor, cursor);
                        return;
                    }

                    ui.window().dispatch_event(slint::platform::WindowEvent::KeyPressed {
                        text: "    ".into(),
                    });
                    ui.window().dispatch_event(slint::platform::WindowEvent::KeyReleased {
                        text: "    ".into(),
                    });
                }
            });
        }
    });

    // Callback on shift+tab (backtab) in editor: navigate to previous pane
    let ui_weak_bt = ui.as_weak();
    ui.on_backtab_pressed(move || {
        if let Some(ui) = ui_weak_bt.upgrade() {
            ui.invoke_focus_prev_pane();
        }
    });

    // Callback on about dialog (native Zenity dialog with single-instance guarantee)
    let help_dialog_about = Arc::clone(&help_dialog_manager);
    let lang_mode_about = Arc::clone(&language_mode_atomic);
    ui.on_about_clicked(move || {
        let is_ru = is_russian(lang_mode_about.load(Ordering::Relaxed));
        help_dialog_about.show_about(is_ru);
    });

    // Callback on licenses clicked (native Zenity dialog with single-instance guarantee)
    let help_dialog_lic = Arc::clone(&help_dialog_manager);
    let lang_mode_lic = Arc::clone(&language_mode_atomic);
    ui.on_licenses_clicked(move || {
        let is_ru = is_russian(lang_mode_lic.load(Ordering::Relaxed));
        help_dialog_lic.show_licenses(is_ru);
    });

    // Callback on help guide
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let pending_clone = Rc::clone(&pending_action);
    let lang_mode_help = Arc::clone(&language_mode_atomic);
    ui.on_help_guide_clicked(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_help.load(Ordering::Relaxed));
            request_action(&ui, &state_clone, &pending_clone, PendingAction::HelpGuide, is_ru);
        }
    });

    // Dialog callbacks for unsaved changes confirmation
    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let pending_clone = Rc::clone(&pending_action);
    let lang_mode_dialog = Arc::clone(&language_mode_atomic);
    ui.on_unsaved_dialog_save(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_dialog.load(Ordering::Relaxed));
            if save_file(&ui, &state_clone, is_ru) {
                ui.set_show_unsaved_dialog(false);
                let action = std::mem::replace(&mut *pending_clone.borrow_mut(), PendingAction::None);
                execute_pending_action(&ui, &state_clone, action, is_ru);
            }
        }
    });

    let ui_weak = ui.as_weak();
    let state_clone = Rc::clone(&state);
    let pending_clone = Rc::clone(&pending_action);
    let lang_mode_dialog = Arc::clone(&language_mode_atomic);
    ui.on_unsaved_dialog_discard(move || {
        if let Some(ui) = ui_weak.upgrade() {
            let is_ru = is_russian(lang_mode_dialog.load(Ordering::Relaxed));
            ui.set_show_unsaved_dialog(false);
            ui.set_is_modified(false);
            let action = std::mem::replace(&mut *pending_clone.borrow_mut(), PendingAction::None);
            execute_pending_action(&ui, &state_clone, action, is_ru);
        }
    });

    let ui_weak = ui.as_weak();
    let pending_clone = Rc::clone(&pending_action);
    ui.on_unsaved_dialog_cancel(move || {
        if let Some(ui) = ui_weak.upgrade() {
            *pending_clone.borrow_mut() = PendingAction::None;
            ui.set_show_unsaved_dialog(false);
            ui.invoke_focus_editor();
        }
    });

    // Callback on theme mode changed (0 = system auto, 1 = light, 2 = dark)
    let ui_weak = ui.as_weak();
    let theme_mode_atomic_clone = Arc::clone(&theme_mode_atomic);
    ui.on_theme_changed(move |mode| {
        if let Some(ui) = ui_weak.upgrade() {
            let mode_u8 = mode.clamp(0, 2) as u8;
            theme_mode_atomic_clone.store(mode_u8, Ordering::Relaxed);
            let dark = match mode {
                0 => detect_system_dark(),
                1 => false,
                2 => true,
                _ => false,
            };
            ui.invoke_set_theme_mode(mode);
            ui.set_is_dark(dark);
        }
    });

    // Callback on language changed (0 = system auto, 1 = English, 2 = Russian)
    let ui_weak_lang = ui.as_weak();
    let language_mode_clone = Arc::clone(&language_mode_atomic);
    ui.on_language_changed(move |mode| {
        let mode_u8 = mode.clamp(0, 2) as u8;
        language_mode_clone.store(mode_u8, Ordering::Relaxed);
        let lang = match mode {
            1 => "en",
            2 => "ru",
            _ => detect_system_language(),
        };
        let _ = slint::select_bundled_translation(lang);
        if let Some(ui) = ui_weak_lang.upgrade() {
            ui.invoke_set_language_mode(mode);
        }
    });

    // Callback to open external hyperlinks in system browser (xdg-open)
    ui.on_open_external_url(move |url| {
        let u = url.to_string();
        if u.starts_with("http://") || u.starts_with("https://") || u.starts_with("mailto:") {
            let _ = std::process::Command::new("xdg-open").arg(&u).spawn();
        }
    });

    // Callbacks for bidirectional footnote navigation
    let state_save_fn = Rc::clone(&state);
    ui.on_save_footnote_pos(move |url, y| {
        state_save_fn
            .borrow_mut()
            .footnote_nav
            .save_pos(url.as_str(), y);
    });

    let state_get_fn = Rc::clone(&state);
    let ui_weak_fn = ui.as_weak();
    ui.on_get_footnote_pos(move |url| {
        let word_wrap = ui_weak_fn.upgrade().map(|u| u.get_word_wrap()).unwrap_or(true);
        let st = state_get_fn.borrow();
        if let Some(y) = st.footnote_nav.get_pos(url.as_str(), word_wrap) {
            ScrollTarget {
                found: true,
                y,
            }
        } else {
            ScrollTarget {
                found: false,
                y: 0.0,
            }
        }
    });

    // Preview copy callbacks
    let ui_weak_code = ui.as_weak();
    ui.on_copy_code_block(move |content, idx| {
        system::copy_text_to_clipboard(content.as_str());
        if let Some(ui) = ui_weak_code.upgrade() {
            ui.set_copied_code_block_index(idx);
            let ui_weak_reset = ui.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                if let Some(ui) = ui_weak_reset.upgrade() {
                    ui.set_copied_code_block_index(-1);
                }
            });
        }
    });

    let ui_weak_code_sel = ui.as_weak();
    ui.on_copy_code_block_selection(move |content, anchor, cursor, idx| {
        let text = content.as_str();
        let to_copy = system::extract_text_selection(text, anchor, cursor);
        system::copy_text_to_clipboard(to_copy);
        if let Some(ui) = ui_weak_code_sel.upgrade() {
            ui.set_copied_code_block_index(idx);
            let ui_weak_reset = ui.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                if let Some(ui) = ui_weak_reset.upgrade() {
                    ui.set_copied_code_block_index(-1);
                }
            });
        }
    });

    let ui_weak_preview_block = ui.as_weak();
    ui.on_copy_preview_block(move |content, idx| {
        let plain = markdown::markdown_to_plain_text(content.as_str());
        let to_copy = if plain.is_empty() {
            content.as_str()
        } else {
            plain.as_str()
        };
        system::copy_text_to_clipboard(to_copy);
        if let Some(ui) = ui_weak_preview_block.upgrade() {
            ui.set_copied_preview_block_index(idx);
            let ui_weak_reset = ui.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
                if let Some(ui) = ui_weak_reset.upgrade()
                    && ui.get_copied_preview_block_index() == idx
                {
                    ui.set_copied_preview_block_index(-1);
                }
            });
        }
    });

    let ui_weak_html = ui.as_weak();
    ui.on_copy_preview_html_clicked(move || {
        if let Some(ui) = ui_weak_html.upgrade() {
            handle_copy_html(&ui);
        }
    });

    let ui_weak_txt = ui.as_weak();
    ui.on_copy_preview_text_clicked(move || {
        if let Some(ui) = ui_weak_txt.upgrade() {
            handle_copy_text(&ui);
        }
    });

    let ui_weak_md = ui.as_weak();
    ui.on_copy_markdown_clicked(move || {
        if let Some(ui) = ui_weak_md.upgrade() {
            let md = ui.get_markdown_text();
            system::copy_text_to_clipboard(md.as_str());
        }
    });

    // Unified keyboard shortcuts dispatcher via Command Registry
    let command_registry = Arc::new(CommandRegistry::new());
    let reg_clone = Arc::clone(&command_registry);
    let ui_weak_sc = ui.as_weak();
    let state_shortcut = Rc::clone(&state);
    let pending_shortcut = Rc::clone(&pending_action);
    let lang_mode_shortcut = Arc::clone(&language_mode_atomic);
    let help_dialog_shortcut = Arc::clone(&help_dialog_manager);
    let undo_stack_shortcut = Arc::clone(&undo_stack);
    let redo_stack_shortcut = Arc::clone(&redo_stack);
    let is_prog_shortcut = Arc::clone(&is_programmatic);

    ui.on_shortcut_requested(move |raw_text, control, alt, shift, meta| {
        let Some(ui) = ui_weak_sc.upgrade() else {
            return false;
        };

        let cmd = reg_clone.resolve(raw_text.as_str(), control, alt, shift, meta);

        // Modal dialog handling takes priority:
        if ui.get_show_unsaved_dialog() {
            match cmd {
                Some(AppCommand::DialogCancel) => {
                    *pending_shortcut.borrow_mut() = PendingAction::None;
                    ui.set_show_unsaved_dialog(false);
                    ui.invoke_focus_editor();
                    return true;
                }
                Some(AppCommand::DialogConfirm) => {
                    let is_ru = is_russian(lang_mode_shortcut.load(Ordering::Relaxed));
                    if save_file(&ui, &state_shortcut, is_ru) {
                        ui.set_show_unsaved_dialog(false);
                        let action = std::mem::replace(&mut *pending_shortcut.borrow_mut(), PendingAction::None);
                        execute_pending_action(&ui, &state_shortcut, action, is_ru);
                    }
                    return true;
                }
                _ => return true, // swallow other shortcuts while unsaved changes dialog is open
            }
        }

        if ui.get_show_table_dialog() {
            match cmd {
                Some(AppCommand::DialogCancel) => {
                    ui.set_show_table_dialog(false);
                    ui.invoke_focus_editor();
                    return true;
                }
                Some(AppCommand::DialogConfirm) => {
                    ui.set_show_table_dialog(false);
                    let cols = (ui.get_table_dialog_cols() as usize).clamp(1, 50);
                    let rows = (ui.get_table_dialog_rows() as usize).clamp(1, 100);
                    let kind = format!("table:{}:{}", cols, rows);
                    apply_formatting(&ui, &kind, &undo_stack_shortcut, &redo_stack_shortcut, &is_prog_shortcut);
                    ui.invoke_focus_editor();
                    return true;
                }
                _ => return true, // swallow other shortcuts while table dialog is open
            }
        }

        let Some(cmd) = cmd else {
            return false;
        };

        let is_ru = is_russian(lang_mode_shortcut.load(Ordering::Relaxed));
        match cmd {
            AppCommand::NewFile => {
                request_action(&ui, &state_shortcut, &pending_shortcut, PendingAction::NewFile, is_ru);
                true
            }
            AppCommand::OpenFile => {
                request_action(&ui, &state_shortcut, &pending_shortcut, PendingAction::OpenFile, is_ru);
                true
            }
            AppCommand::SaveFile => {
                save_file(&ui, &state_shortcut, is_ru);
                true
            }
            AppCommand::SaveAs => {
                perform_save_as(&ui, &state_shortcut, is_ru);
                true
            }
            AppCommand::Exit => {
                request_action(&ui, &state_shortcut, &pending_shortcut, PendingAction::Exit, is_ru);
                true
            }
            AppCommand::ReloadDirectory => {
                perform_refresh_files(&ui, &state_shortcut);
                true
            }
            AppCommand::Undo => {
                perform_undo(&ui, &undo_stack_shortcut, &redo_stack_shortcut, &is_prog_shortcut);
                true
            }
            AppCommand::Redo => {
                perform_redo(&ui, &undo_stack_shortcut, &redo_stack_shortcut, &is_prog_shortcut);
                true
            }
            AppCommand::Cut => {
                ui.invoke_cut();
                true
            }
            AppCommand::Copy => {
                if ui.get_active_code_block_index() >= 0 && ui.get_active_code_block_has_selection() {
                    ui.invoke_copy_active_code_block_selection();
                } else if ui.get_active_preview_block_index() >= 0 {
                    ui.invoke_copy_active_preview_block();
                } else if ui.get_active_code_block_index() >= 0 {
                    ui.invoke_copy_active_code_block_selection();
                } else {
                    ui.invoke_copy();
                    if ui.get_editor_has_focus() && ui.get_editor_has_selection() {
                        let text = ui.get_markdown_text();
                        let anchor = ui.get_editor_anchor_pos();
                        let cursor = ui.get_editor_cursor_pos();
                        let selected = system::extract_text_selection(text.as_str(), anchor, cursor);
                        if !selected.is_empty() && selected != text.as_str() {
                            system::copy_text_to_clipboard(selected);
                        }
                    }
                }
                true
            }
            AppCommand::Paste => {
                ui.invoke_paste();
                true
            }
            AppCommand::SelectAll => {
                ui.invoke_select_all();
                true
            }
            AppCommand::CopyPreviewHtml => {
                handle_copy_html(&ui);
                true
            }
            AppCommand::CopyPreviewText => {
                handle_copy_text(&ui);
                true
            }
            AppCommand::FormatHeading1 => {
                apply_formatting(&ui, "h1", &undo_stack_shortcut, &redo_stack_shortcut, &is_prog_shortcut);
                true
            }
            AppCommand::FormatHeading2 => {
                apply_formatting(&ui, "h2", &undo_stack_shortcut, &redo_stack_shortcut, &is_prog_shortcut);
                true
            }
            AppCommand::FormatBold => {
                apply_formatting(&ui, "bold", &undo_stack_shortcut, &redo_stack_shortcut, &is_prog_shortcut);
                true
            }
            AppCommand::FormatItalic => {
                apply_formatting(&ui, "italic", &undo_stack_shortcut, &redo_stack_shortcut, &is_prog_shortcut);
                true
            }
            AppCommand::FormatCodeBlock => {
                apply_formatting(&ui, "code_block", &undo_stack_shortcut, &redo_stack_shortcut, &is_prog_shortcut);
                true
            }
            AppCommand::FormatTable => {
                ui.set_table_dialog_cols(2);
                ui.set_table_dialog_rows(2);
                ui.set_show_table_dialog(true);
                true
            }
            AppCommand::ToggleSidebar => {
                let cur = ui.get_show_files_sidebar();
                ui.set_show_files_sidebar(!cur);
                true
            }
            AppCommand::ViewSplit => {
                ui.invoke_set_view_mode(0);
                true
            }
            AppCommand::ViewEditor => {
                ui.invoke_set_view_mode(1);
                true
            }
            AppCommand::ViewPreview => {
                ui.invoke_set_view_mode(2);
                true
            }
            AppCommand::FocusNextPane => {
                ui.invoke_focus_next_pane();
                true
            }
            AppCommand::FocusPrevPane => {
                ui.invoke_focus_prev_pane();
                true
            }
            AppCommand::HelpGuide => {
                request_action(&ui, &state_shortcut, &pending_shortcut, PendingAction::HelpGuide, is_ru);
                true
            }
            AppCommand::Licenses => {
                let is_ru = is_russian(lang_mode_shortcut.load(Ordering::Relaxed));
                help_dialog_shortcut.show_licenses(is_ru);
                true
            }
            AppCommand::About => {
                let is_ru = is_russian(lang_mode_shortcut.load(Ordering::Relaxed));
                help_dialog_shortcut.show_about(is_ru);
                true
            }
            AppCommand::DialogCancel | AppCommand::DialogConfirm => false,
        }
    });

    // Background watcher for OS dark mode changes (Freedesktop portal / Windows / macOS)
    let theme_mode_atomic_bg = Arc::clone(&theme_mode_atomic);
    let ui_weak_bg = ui.as_weak();
    std::thread::spawn(move || {
        if let Ok(watcher) = dark_light::subscribe() {
            while let Ok(mode) = watcher.recv() {
                if theme_mode_atomic_bg.load(Ordering::Relaxed) == 0 {
                    let is_dark = matches!(mode, dark_light::Mode::Dark);
                    let _ = slint::invoke_from_event_loop({
                        let ui_weak = ui_weak_bg.clone();
                        move || {
                            if let Some(ui) = ui_weak.upgrade()
                                && ui.get_theme_mode() == 0
                            {
                                ui.set_is_dark(is_dark);
                            }
                        }
                    });
                }
            }
        }
    });

    ui.run()?;
    Ok(())
}

fn perform_save_as(ui: &AppWindow, state: &Rc<RefCell<AppState>>, is_ru: bool) {
    let default_name = {
        let st = state.borrow();
        st.current_file
            .as_ref()
            .cloned()
            .unwrap_or_else(|| st.current_dir.join("document.md"))
    };
    if let Some(target_path) = save_file_dialog(&default_name.to_string_lossy(), is_ru) {
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
        }
    }
}

fn perform_refresh_files(ui: &AppWindow, state: &Rc<RefCell<AppState>>) {
    let st = state.borrow();
    let active = st.current_file.as_deref();
    update_file_tree(ui, &st.current_dir, active, &st.expanded_dirs);
}

fn perform_undo(
    ui: &AppWindow,
    undo_stack: &Arc<Mutex<Vec<UndoAction>>>,
    redo_stack: &Arc<Mutex<Vec<UndoAction>>>,
    is_programmatic: &Arc<AtomicBool>,
) {
    let action = if let Ok(mut stack) = undo_stack.lock() {
        stack.pop()
    } else {
        None
    };

    if let Some(action) = action {
        is_programmatic.store(true, Ordering::SeqCst);
        for _ in 0..action.steps {
            ui.invoke_raw_undo();
        }
        ui.invoke_set_editor_selection(
            action.restore_anchor as i32,
            action.restore_cursor as i32,
        );
        is_programmatic.store(false, Ordering::SeqCst);

        if let Ok(mut rstack) = redo_stack.lock() {
            rstack.push(action);
        }
    } else {
        ui.invoke_raw_undo();
    }
}

fn perform_redo(
    ui: &AppWindow,
    undo_stack: &Arc<Mutex<Vec<UndoAction>>>,
    redo_stack: &Arc<Mutex<Vec<UndoAction>>>,
    is_programmatic: &Arc<AtomicBool>,
) {
    let action = if let Ok(mut rstack) = redo_stack.lock() {
        rstack.pop()
    } else {
        None
    };

    if let Some(action) = action {
        is_programmatic.store(true, Ordering::SeqCst);
        for _ in 0..action.steps {
            ui.invoke_raw_redo();
        }
        ui.invoke_set_editor_selection(
            action.redo_anchor as i32,
            action.redo_cursor as i32,
        );
        is_programmatic.store(false, Ordering::SeqCst);

        if let Ok(mut stack) = undo_stack.lock() {
            stack.push(action);
        }
    } else {
        ui.invoke_raw_redo();
    }
}

fn handle_copy_html(ui: &AppWindow) {
    let md = ui.get_markdown_text();
    let html = markdown::markdown_to_html(md.as_str());
    system::copy_html_to_clipboard(&html);
    ui.set_preview_copy_feedback(1);
    let ui_weak = ui.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_preview_copy_feedback(0);
        }
    });
}

fn handle_copy_text(ui: &AppWindow) {
    let md = ui.get_markdown_text();
    let plain = markdown::markdown_to_plain_text(md.as_str());
    system::copy_text_to_clipboard(&plain);
    ui.set_preview_copy_feedback(2);
    let ui_weak = ui.as_weak();
    slint::Timer::single_shot(std::time::Duration::from_millis(1500), move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_preview_copy_feedback(0);
        }
    });
}

