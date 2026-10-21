use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTreeItem {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: i32,
    pub is_active: bool,
}

/// Returns the user-friendly folder name for a given directory path.
pub fn get_directory_name(dir: &Path) -> String {
    if let Some(name) = dir.file_name() {
        name.to_string_lossy().to_string()
    } else {
        dir.to_string_lossy().to_string()
    }
}

/// Attempts to find the parent directory of a file, falling back to current working dir.
pub fn get_parent_or_current(file_path: &Path) -> PathBuf {
    if let Some(parent) = file_path.parent()
        && !parent.as_os_str().is_empty()
    {
        return parent.to_path_buf();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Checks whether a directory or file should be skipped from scanning.
fn should_ignore_entry(name: &str) -> bool {
    name.starts_with('.') || name == "target" || name == "node_modules"
}

/// Checks if an entry is a Markdown file by extension (.md, .markdown).
fn is_markdown_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
    } else {
        false
    }
}

/// Recursively checks if a directory contains at least one Markdown file.
pub fn has_markdown_files(dir: &Path) -> bool {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_ignore_entry(&name) {
            continue;
        }

        if (path.is_file() && is_markdown_file(&path))
            || (path.is_dir() && has_markdown_files(&path))
        {
            return true;
        }
    }

    false
}

/// Recursively counts all Markdown files in a directory tree.
pub fn count_markdown_files(dir: &Path) -> i32 {
    let mut count = 0;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_ignore_entry(&name) {
            continue;
        }

        if path.is_file() && is_markdown_file(&path) {
            count += 1;
        } else if path.is_dir() {
            count += count_markdown_files(&path);
        }
    }

    count
}

/// Ensures all ancestor directories of `file_path` up to `root` are present in `expanded_paths`.
pub fn expand_ancestors_of(root: &Path, file_path: &Path, expanded_paths: &mut HashSet<PathBuf>) {
    let canonical_root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let canonical_file = fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());

    let mut current = canonical_file.parent();
    while let Some(parent) = current {
        if parent == canonical_root {
            break;
        }
        if parent.starts_with(&canonical_root) {
            expanded_paths.insert(parent.to_path_buf());
            current = parent.parent();
        } else {
            break;
        }
    }
}

/// Recursively builds the visible file tree.
fn populate_tree(
    current_dir: &Path,
    depth: i32,
    canonical_active: Option<&PathBuf>,
    expanded_paths: &HashSet<PathBuf>,
    items: &mut Vec<FileTreeItem>,
) {
    let entries = match fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut subdirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if should_ignore_entry(&name) {
            continue;
        }

        if path.is_dir() {
            if has_markdown_files(&path) {
                subdirs.push((name, path));
            }
        } else if path.is_file() && is_markdown_file(&path) {
            files.push((name, path));
        }
    }

    // Sort case-insensitively
    subdirs.sort_by_key(|(name, _)| name.to_lowercase());
    files.sort_by_key(|(name, _)| name.to_lowercase());

    // Subdirectories first
    for (name, path) in subdirs {
        let canonical_dir = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let is_expanded = expanded_paths.contains(&canonical_dir) || expanded_paths.contains(&path);

        items.push(FileTreeItem {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir: true,
            is_expanded,
            depth,
            is_active: false,
        });

        if is_expanded {
            populate_tree(&path, depth + 1, canonical_active, expanded_paths, items);
        }
    }

    // Then Markdown files
    for (name, path) in files {
        let canonical_current = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let is_active = match canonical_active {
            Some(act) => act == &canonical_current,
            None => false,
        };

        items.push(FileTreeItem {
            name,
            path: path.to_string_lossy().to_string(),
            is_dir: false,
            is_expanded: false,
            depth,
            is_active,
        });
    }
}

/// Scans a directory and returns the flattened visible tree according to `expanded_paths`,
/// along with the total count of Markdown files in the whole subtree.
pub fn scan_directory_tree(
    root: &Path,
    active_path: Option<&Path>,
    expanded_paths: &HashSet<PathBuf>,
) -> (Vec<FileTreeItem>, i32) {
    let mut items = Vec::new();
    let canonical_active = active_path.and_then(|p| fs::canonicalize(p).ok().or_else(|| Some(p.to_path_buf())));

    populate_tree(root, 0, canonical_active.as_ref(), expanded_paths, &mut items);
    let total_count = count_markdown_files(root);

    (items, total_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_tree_with_subdirectories_and_expansion() {
        let temp_dir = std::env::temp_dir().join("markdown_editor_test_tree");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let sub_docs = temp_dir.join("docs");
        let sub_notes = temp_dir.join("notes");
        let sub_empty = temp_dir.join("empty_folder");
        let sub_hidden = temp_dir.join(".hidden");
        let sub_target = temp_dir.join("target");

        fs::create_dir_all(&sub_docs).unwrap();
        fs::create_dir_all(&sub_notes).unwrap();
        fs::create_dir_all(&sub_empty).unwrap();
        fs::create_dir_all(&sub_hidden).unwrap();
        fs::create_dir_all(&sub_target).unwrap();

        // Files
        let root_readme = temp_dir.join("README.md");
        let doc_guide = sub_docs.join("guide.md");
        let notes_todo = sub_notes.join("todo.markdown");
        let hidden_file = sub_hidden.join("secret.md");
        let target_file = sub_target.join("target.md");
        let empty_txt = sub_empty.join("hello.txt");

        File::create(&root_readme).unwrap().write_all(b"# Root").unwrap();
        File::create(&doc_guide).unwrap().write_all(b"# Guide").unwrap();
        File::create(&notes_todo).unwrap().write_all(b"# Todo").unwrap();
        File::create(&hidden_file).unwrap().write_all(b"# Secret").unwrap();
        File::create(&target_file).unwrap().write_all(b"# Target").unwrap();
        File::create(&empty_txt).unwrap().write_all(b"not md").unwrap();

        // 1. Initially without any expanded subdirectories
        let mut expanded = HashSet::new();
        let (tree_collapsed, count) = scan_directory_tree(&temp_dir, Some(&root_readme), &expanded);

        assert_eq!(count, 3); // README.md, guide.md, todo.markdown (ignores .hidden and target)

        // Tree when collapsed: docs (dir), notes (dir), README.md (file). empty_folder is omitted because it has no md!
        assert_eq!(tree_collapsed.len(), 3);
        assert_eq!(tree_collapsed[0].name, "docs");
        assert!(tree_collapsed[0].is_dir);
        assert!(!tree_collapsed[0].is_expanded);

        assert_eq!(tree_collapsed[1].name, "notes");
        assert!(tree_collapsed[1].is_dir);
        assert!(!tree_collapsed[1].is_expanded);

        assert_eq!(tree_collapsed[2].name, "README.md");
        assert!(!tree_collapsed[2].is_dir);
        assert!(tree_collapsed[2].is_active);

        // 2. Expand 'docs'
        expanded.insert(fs::canonicalize(&sub_docs).unwrap());
        let (tree_expanded, _) = scan_directory_tree(&temp_dir, Some(&doc_guide), &expanded);

        // Tree now: docs (expanded), guide.md (depth 1, active), notes (dir), README.md
        assert_eq!(tree_expanded.len(), 4);
        assert_eq!(tree_expanded[0].name, "docs");
        assert!(tree_expanded[0].is_expanded);

        assert_eq!(tree_expanded[1].name, "guide.md");
        assert_eq!(tree_expanded[1].depth, 1);
        assert!(tree_expanded[1].is_active);

        assert_eq!(tree_expanded[2].name, "notes");
        assert_eq!(tree_expanded[2].depth, 0);

        assert_eq!(tree_expanded[3].name, "README.md");
        assert_eq!(tree_expanded[3].depth, 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_expand_ancestors_of() {
        let temp_dir = std::env::temp_dir().join("markdown_editor_test_ancestors");
        let _ = fs::remove_dir_all(&temp_dir);
        let nested = temp_dir.join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        let file = nested.join("deep.md");
        File::create(&file).unwrap().write_all(b"# Deep").unwrap();

        let mut expanded = HashSet::new();
        expand_ancestors_of(&temp_dir, &file, &mut expanded);

        let can_a = fs::canonicalize(temp_dir.join("a")).unwrap();
        let can_b = fs::canonicalize(temp_dir.join("a").join("b")).unwrap();
        let can_c = fs::canonicalize(temp_dir.join("a").join("b").join("c")).unwrap();

        assert!(expanded.contains(&can_a));
        assert!(expanded.contains(&can_b));
        assert!(expanded.contains(&can_c));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_directory_name() {
        let p = Path::new("/home/user/my-notes");
        assert_eq!(get_directory_name(p), "my-notes");
    }
}
