use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

// Domain-knowledge: dirs ignored even when they contain `.md` files —
// node_modules and target are noise; .git is repo metadata.
pub const SKIP_DIRS: &[&str] = &[".git", "node_modules", "target"];

const FOLDER_PALETTE: &[Color] = &[
    Color::Rgb(137, 180, 250),
    Color::Rgb(166, 227, 161),
    Color::Rgb(249, 226, 175),
    Color::Rgb(203, 166, 247),
    Color::Rgb(148, 226, 213),
    Color::Rgb(250, 179, 135),
    Color::Rgb(245, 194, 231),
    Color::Rgb(116, 199, 236),
];

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub color: Color,
    pub is_last_sibling: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TreeAction {
    #[default]
    None,
    NewFile,
    NewFolder,
    Rename,
    Delete,
}

#[derive(Debug, Clone)]
pub enum FsOperation {
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Create {
        path: PathBuf,
        is_dir: bool,
    },
    Delete {
        path: PathBuf,
        content: Option<String>,
        is_dir: bool,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub enum HintResult {
    EnteredFolder,
    OpenFile(PathBuf),
}

#[derive(Debug, Default)]
pub struct BrowserState {
    pub entries: Vec<TreeEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub open_dirs: HashSet<PathBuf>,
    pub root: Option<PathBuf>,
    pub action: TreeAction,
    pub input_buf: String,
    pub marked_for_move: Option<PathBuf>,
    pub fs_undo_stack: Vec<FsOperation>,
    pub fs_redo_stack: Vec<FsOperation>,
    pub folder_color_idx: usize,
    pub hint_scope: Option<PathBuf>,
    pub hint_indices: Vec<usize>,
}

impl BrowserState {
    pub fn build(&mut self, root: &Path) {
        self.root = Some(root.to_path_buf());
        self.entries.clear();
        self.folder_color_idx = 0;

        let md_dirs = compute_md_dirs(root);

        let root_name = root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.entries.push(TreeEntry {
            path: root.to_path_buf(),
            name: root_name,
            is_dir: true,
            depth: 0,
            color: FOLDER_PALETTE[0],
            is_last_sibling: false,
        });

        self.build_dir(root, 0, &md_dirs);
        self.compute_hints();

        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
    }

    fn build_dir(&mut self, dir: &Path, depth: usize, md_dirs: &HashSet<PathBuf>) {
        let read = match std::fs::read_dir(dir) {
            Ok(r) => r,
            Err(_) => return,
        };

        let mut dirs: Vec<PathBuf> = Vec::new();
        let mut files: Vec<PathBuf> = Vec::new();
        for entry in read.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_dir() {
                if SKIP_DIRS.iter().any(|s| *s == name) {
                    continue;
                }
                if md_dirs.contains(&path) {
                    dirs.push(path);
                }
            } else if ft.is_file() && is_md_file(&path) {
                files.push(path);
            }
        }

        dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        let parent_color = FOLDER_PALETTE[self.folder_color_idx % FOLDER_PALETTE.len()];
        let total = dirs.len() + files.len();
        let mut idx = 0;

        for child in dirs {
            let is_last = idx + 1 == total;
            self.folder_color_idx += 1;
            let color = FOLDER_PALETTE[self.folder_color_idx % FOLDER_PALETTE.len()];
            let name = child
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let open = self.open_dirs.contains(&child);
            self.entries.push(TreeEntry {
                path: child.clone(),
                name,
                is_dir: true,
                depth,
                color,
                is_last_sibling: is_last,
            });
            if open {
                self.build_dir(&child, depth + 1, md_dirs);
            }
            idx += 1;
        }

        for child in files {
            let is_last = idx + 1 == total;
            let name = child
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            self.entries.push(TreeEntry {
                path: child,
                name,
                is_dir: false,
                depth,
                color: parent_color,
                is_last_sibling: is_last,
            });
            idx += 1;
        }
    }

    pub fn toggle_dir(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if !entry.is_dir {
                return;
            }
            let path = entry.path.clone();
            if self.root.as_ref() == Some(&path) {
                return;
            }
            if self.open_dirs.contains(&path) {
                self.open_dirs.remove(&path);
            } else {
                self.open_dirs.insert(path.clone());
            }
            if let Some(root) = self.root.clone() {
                self.build(&root);
                for (i, e) in self.entries.iter().enumerate() {
                    if e.path == path {
                        self.selected = i;
                        break;
                    }
                }
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self, visible_height: usize) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            if visible_height > 0 && self.selected >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected + 1 - visible_height;
            }
        }
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.entries.get(self.selected).map(|e| &e.path)
    }

    pub fn selected_dir(&self) -> PathBuf {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                entry.path.clone()
            } else {
                entry.path.parent().unwrap_or(Path::new(".")).to_path_buf()
            }
        } else {
            self.root.clone().unwrap_or_else(|| PathBuf::from("."))
        }
    }

    pub fn start_action(&mut self, action: TreeAction) {
        self.action = action;
        self.input_buf.clear();
    }

    pub fn cancel_action(&mut self) {
        self.action = TreeAction::None;
        self.input_buf.clear();
    }

    fn push_fs_op(&mut self, op: FsOperation) {
        self.fs_undo_stack.push(op);
        self.fs_redo_stack.clear();
    }

    pub fn confirm_new_file(&mut self) -> Result<Option<PathBuf>, String> {
        let raw = self.input_buf.trim().to_string();
        if raw.is_empty() {
            self.cancel_action();
            return Ok(None);
        }

        let final_name = match normalize_md_name(&raw) {
            Ok(name) => name,
            Err(e) => {
                self.cancel_action();
                return Err(e);
            }
        };

        let parent = self.selected_dir();
        let new_path = parent.join(&final_name);
        if let Some(dir) = new_path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                self.cancel_action();
                return Err(format!("failed to create dir: {e}"));
            }
        }
        if let Err(e) = std::fs::write(&new_path, "") {
            self.cancel_action();
            return Err(format!("failed to create file: {e}"));
        }
        self.push_fs_op(FsOperation::Create {
            path: new_path.clone(),
            is_dir: false,
        });
        self.cancel_action();
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        Ok(Some(new_path))
    }

    pub fn confirm_new_folder(&mut self) -> Result<Option<PathBuf>, String> {
        let raw = self.input_buf.trim().to_string();
        if raw.is_empty() {
            self.cancel_action();
            return Ok(None);
        }
        let parent = self.selected_dir();
        let new_path = parent.join(&raw);
        if let Err(e) = std::fs::create_dir_all(&new_path) {
            self.cancel_action();
            return Err(format!("failed to create folder: {e}"));
        }
        self.push_fs_op(FsOperation::Create {
            path: new_path.clone(),
            is_dir: true,
        });
        self.open_dirs.insert(new_path.clone());
        self.cancel_action();
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        Ok(Some(new_path))
    }

    pub fn confirm_rename(&mut self) -> Result<Option<PathBuf>, String> {
        let raw = self.input_buf.trim().to_string();
        if raw.is_empty() {
            self.cancel_action();
            return Ok(None);
        }
        let entry = match self.entries.get(self.selected) {
            Some(e) => e.clone(),
            None => {
                self.cancel_action();
                return Ok(None);
            }
        };
        let final_name = if entry.is_dir {
            raw
        } else {
            match normalize_md_name(&raw) {
                Ok(name) => name,
                Err(e) => {
                    self.cancel_action();
                    return Err(e);
                }
            }
        };
        let old_path = entry.path.clone();
        let new_path = old_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(&final_name);
        if let Err(e) = std::fs::rename(&old_path, &new_path) {
            self.cancel_action();
            return Err(format!("failed to rename: {e}"));
        }
        self.push_fs_op(FsOperation::Rename {
            from: old_path,
            to: new_path.clone(),
        });
        self.cancel_action();
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        Ok(Some(new_path))
    }

    pub fn confirm_delete(&mut self) -> bool {
        let entry = match self.entries.get(self.selected) {
            Some(e) => e.clone(),
            None => {
                self.cancel_action();
                return false;
            }
        };
        let path = entry.path.clone();
        let is_dir = entry.is_dir;
        let content = if !is_dir {
            std::fs::read_to_string(&path).ok()
        } else {
            None
        };
        let result = if is_dir {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        if result.is_err() {
            self.cancel_action();
            return false;
        }
        self.push_fs_op(FsOperation::Delete {
            path: path.clone(),
            content,
            is_dir,
        });
        self.cancel_action();
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        if self.selected >= self.entries.len() && self.selected > 0 {
            self.selected -= 1;
        }
        true
    }

    pub fn mark_for_move(&mut self) {
        if let Some(path) = self.selected_path().cloned() {
            if self.root.as_ref() == Some(&path) {
                return;
            }
            self.marked_for_move = Some(path);
        }
    }

    pub fn confirm_move(&mut self) -> Option<PathBuf> {
        let marked = self.marked_for_move.take()?;
        let dest_dir = self.selected_dir();
        let file_name = marked.file_name()?;
        let new_path = dest_dir.join(file_name);
        if marked == new_path {
            return None;
        }
        if std::fs::rename(&marked, &new_path).is_err() {
            return None;
        }
        self.push_fs_op(FsOperation::Move {
            from: marked,
            to: new_path.clone(),
        });
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        Some(new_path)
    }

    pub fn cancel_move(&mut self) {
        self.marked_for_move = None;
    }

    pub fn undo_last_fs_op(&mut self) -> bool {
        let op = match self.fs_undo_stack.last().cloned() {
            Some(op) => op,
            None => return false,
        };
        if !apply_fs_op(&op, true) {
            return false;
        }
        self.fs_undo_stack.pop();
        self.fs_redo_stack.push(op);
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        true
    }

    pub fn redo_last_fs_op(&mut self) -> bool {
        let op = match self.fs_redo_stack.last().cloned() {
            Some(op) => op,
            None => return false,
        };
        if !apply_fs_op(&op, false) {
            return false;
        }
        self.fs_redo_stack.pop();
        self.fs_undo_stack.push(op);
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        true
    }

    pub fn init_hint_scope(&mut self) {
        self.hint_scope = self.root.clone();
        self.compute_hints();
    }

    pub fn compute_hints(&mut self) {
        self.hint_indices.clear();
        let scope = match self.hint_scope.clone().or_else(|| self.root.clone()) {
            Some(s) => s,
            None => return,
        };
        for (i, entry) in self.entries.iter().enumerate() {
            if i == 0 && entry.path == scope {
                continue;
            }
            if entry.path.parent() == Some(&scope) {
                self.hint_indices.push(i);
                if self.hint_indices.len() >= 9 {
                    break;
                }
            }
        }
    }

    pub fn hint_enter(&mut self, n: usize) -> Option<HintResult> {
        let &entry_idx = self.hint_indices.get(n)?;
        let entry = self.entries.get(entry_idx)?;
        let path = entry.path.clone();
        let is_dir = entry.is_dir;

        if is_dir {
            if !self.open_dirs.contains(&path) {
                self.open_dirs.insert(path.clone());
                if let Some(root) = self.root.clone() {
                    self.build(&root);
                }
            }
            for (i, e) in self.entries.iter().enumerate() {
                if e.path == path {
                    self.selected = i;
                    break;
                }
            }
            self.hint_scope = Some(path);
            self.compute_hints();
            Some(HintResult::EnteredFolder)
        } else {
            self.selected = entry_idx;
            Some(HintResult::OpenFile(path))
        }
    }

    pub fn hint_back(&mut self) {
        let scope = match &self.hint_scope {
            Some(s) => s.clone(),
            None => return,
        };
        let root = match &self.root {
            Some(r) => r.clone(),
            None => return,
        };
        if scope == root {
            return;
        }
        if let Some(parent) = scope.parent() {
            self.hint_scope = Some(parent.to_path_buf());
            for (i, entry) in self.entries.iter().enumerate() {
                if entry.path == scope {
                    self.selected = i;
                    break;
                }
            }
            self.compute_hints();
        }
    }

    pub fn hint_index_for_entry(&self, entry_idx: usize) -> Option<usize> {
        self.hint_indices.iter().position(|&i| i == entry_idx)
    }
}

fn is_md_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

// Single-pass collector: for every `.md` file found, mark every ancestor up to
// (and including) `root` as an "md-bearing" directory. Used during build to
// hide dirs that don't lead to any `.md`.
fn compute_md_dirs(root: &Path) -> HashSet<PathBuf> {
    let mut out = HashSet::new();
    out.insert(root.to_path_buf());
    walk_collect(root, root, &mut out);
    out
}

fn walk_collect(root: &Path, dir: &Path, out: &mut HashSet<PathBuf>) {
    let read = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    for entry in read.flatten() {
        let path = entry.path();
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if ft.is_dir() {
            if SKIP_DIRS.iter().any(|s| *s == name) {
                continue;
            }
            walk_collect(root, &path, out);
        } else if ft.is_file() && is_md_file(&path) {
            let mut cur = path.parent();
            while let Some(p) = cur {
                if out.insert(p.to_path_buf()) && p == root {
                    break;
                }
                if p == root {
                    break;
                }
                cur = p.parent();
            }
        }
    }
}

// Returns the on-disk filename to use, or rejects with L16 error.
fn normalize_md_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("name is empty".to_string());
    }
    // No dot at all → append `.md`. Multi-dot names (e.g. `foo.bar.md`) keep
    // their full dotted name; only the LAST extension is validated.
    match trimmed.rsplit_once('.') {
        None => Ok(format!("{trimmed}.md")),
        Some((stem, ext)) => {
            if stem.is_empty() {
                return Err("Veol only creates .md files".to_string());
            }
            if ext.eq_ignore_ascii_case("md") {
                Ok(trimmed.to_string())
            } else {
                Err("Veol only creates .md files".to_string())
            }
        }
    }
}

fn apply_fs_op(op: &FsOperation, reverse: bool) -> bool {
    match op {
        FsOperation::Move { from, to } | FsOperation::Rename { from, to } => {
            let (src, dest) = if reverse { (to, from) } else { (from, to) };
            std::fs::rename(src, dest).is_ok()
        }
        FsOperation::Create { path, is_dir } => {
            if reverse {
                if *is_dir {
                    std::fs::remove_dir_all(path).is_ok()
                } else {
                    std::fs::remove_file(path).is_ok()
                }
            } else if *is_dir {
                std::fs::create_dir_all(path).is_ok()
            } else {
                std::fs::write(path, "").is_ok()
            }
        }
        FsOperation::Delete {
            path,
            content,
            is_dir,
        } => {
            if reverse {
                if *is_dir {
                    std::fs::create_dir_all(path).is_ok()
                } else {
                    let data = content.as_deref().unwrap_or("");
                    std::fs::write(path, data).is_ok()
                }
            } else if *is_dir {
                std::fs::remove_dir_all(path).is_ok()
            } else {
                std::fs::remove_file(path).is_ok()
            }
        }
    }
}

// Tree-guide chars adapted from reedo: `├ ` for non-last sibling, `└ ` for
// last sibling, `│ ` for continuation through ancestor levels.
pub fn tree_guide_prefix(entries: &[TreeEntry], idx: usize) -> String {
    let entry = &entries[idx];
    if entry.depth == 0 {
        return String::new();
    }
    let mut prefix = String::new();
    for d in 0..entry.depth.saturating_sub(1) {
        let target_depth = d;
        let mut ancestor_is_last = false;
        for j in (1..idx).rev() {
            if entries[j].depth == target_depth {
                ancestor_is_last = entries[j].is_last_sibling;
                break;
            }
            if entries[j].depth < target_depth {
                break;
            }
        }
        if ancestor_is_last {
            prefix.push_str("  ");
        } else {
            prefix.push_str("│ ");
        }
    }
    if entry.is_last_sibling {
        prefix.push_str("└ ");
    } else {
        prefix.push_str("├ ");
    }
    prefix
}

fn file_icon(is_dir: bool, is_open: bool) -> &'static str {
    if is_dir {
        if is_open {
            "\u{f07c}  "
        } else {
            "\u{f07b}  "
        }
    } else {
        "\u{e73e}  "
    }
}

pub struct BrowserWidget<'a> {
    pub state: &'a BrowserState,
    pub theme: &'a crate::render::theme::Theme,
}

fn inner_area(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

pub fn list_height(area: Rect) -> usize {
    let inner = inner_area(area);
    (inner.height as usize).saturating_sub(1).saturating_sub(1)
}

impl<'a> Widget for BrowserWidget<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        if area.width < 3 || area.height < 3 {
            return;
        }

        let bg = self.theme.colors.popup_bg.to_ratatui();
        let border = self.theme.colors.popup_border.to_ratatui();
        let selected_bg = self.theme.colors.popup_selected.to_ratatui();
        let dim = self.theme.colors.popup_dim.to_ratatui();
        let accent = self.theme.colors.popup_accent.to_ratatui();
        let fg = self.theme.colors.fg.to_ratatui();

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                }
            }
        }

        let right_x = area.x + area.width - 1;
        let bottom_y = area.y + area.height - 1;
        for x in area.x..=right_x {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char('─');
                cell.set_style(Style::default().fg(border).bg(bg));
            }
            if let Some(cell) = buf.cell_mut((x, bottom_y)) {
                cell.set_char('─');
                cell.set_style(Style::default().fg(border).bg(bg));
            }
        }
        for y in area.y..=bottom_y {
            if let Some(cell) = buf.cell_mut((area.x, y)) {
                cell.set_char('│');
                cell.set_style(Style::default().fg(border).bg(bg));
            }
            if let Some(cell) = buf.cell_mut((right_x, y)) {
                cell.set_char('│');
                cell.set_style(Style::default().fg(border).bg(bg));
            }
        }
        for (cx, cy, ch) in [
            (area.x, area.y, '╭'),
            (right_x, area.y, '╮'),
            (area.x, bottom_y, '╰'),
            (right_x, bottom_y, '╯'),
        ] {
            if let Some(cell) = buf.cell_mut((cx, cy)) {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(border).bg(bg));
            }
        }

        let inner = inner_area(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let title_y = inner.y;

        let root_name = self
            .state
            .entries
            .first()
            .map(|e| e.name.as_str())
            .unwrap_or("Markdown");
        let title = format!(" \u{f015}  {} - Markdown ", root_name);
        let is_root_selected = self.state.selected == 0;
        let title_bg = if is_root_selected { selected_bg } else { bg };

        for lx in inner.x..inner.x + inner.width {
            if let Some(cell) = buf.cell_mut((lx, title_y)) {
                cell.set_style(Style::default().bg(title_bg));
            }
        }
        let mut x = inner.x;
        for ch in title.chars() {
            if x >= inner.x + inner.width {
                break;
            }
            if let Some(cell) = buf.cell_mut((x, title_y)) {
                cell.set_char(ch);
                cell.set_style(
                    Style::default()
                        .fg(accent)
                        .bg(title_bg)
                        .add_modifier(Modifier::BOLD),
                );
            }
            x += 1;
        }

        let entries_start = 1usize;
        let visible = list_height(area);
        for i in 0..visible {
            let entry_idx = self.state.scroll_offset + i + 1;
            let y = inner.y + (entries_start + i) as u16;
            if y >= inner.y + inner.height {
                break;
            }
            let entry = match self.state.entries.get(entry_idx) {
                Some(e) => e,
                None => continue,
            };
            let is_selected = entry_idx == self.state.selected;
            let line_bg = if is_selected { selected_bg } else { bg };

            for lx in inner.x..inner.x + inner.width {
                if let Some(cell) = buf.cell_mut((lx, y)) {
                    cell.set_style(Style::default().bg(line_bg));
                }
            }

            let guide = tree_guide_prefix(&self.state.entries, entry_idx);
            let is_open = entry.is_dir && self.state.open_dirs.contains(&entry.path);
            let icon = file_icon(entry.is_dir, is_open);
            let move_indicator = if self.state.marked_for_move.as_ref() == Some(&entry.path) {
                " [moving]"
            } else {
                ""
            };
            let display = format!(" {}{}{}{}", guide, icon, entry.name, move_indicator);

            let guide_end = 1 + guide.chars().count();
            let icon_start = guide_end;
            let icon_end = icon_start + icon.chars().count();
            let name_start = icon_end;
            let name_end = name_start + entry.name.chars().count();

            let mut cx = inner.x;
            for (ci, ch) in display.chars().enumerate() {
                if cx >= inner.x + inner.width {
                    break;
                }
                let style = if ci > 0 && ci < guide_end {
                    Style::default().fg(dim).bg(line_bg)
                } else if ci >= icon_start && ci < name_end {
                    let mut s = Style::default().fg(entry.color).bg(line_bg);
                    if entry.is_dir {
                        s = s.add_modifier(Modifier::BOLD);
                    }
                    s
                } else {
                    Style::default().fg(dim).bg(line_bg)
                };
                if let Some(cell) = buf.cell_mut((cx, y)) {
                    cell.set_char(ch);
                    cell.set_style(style);
                }
                cx += 1;
            }

            if let Some(n) = self.state.hint_index_for_entry(entry_idx) {
                let label = format!("{}", n + 1);
                let label_x = inner.x + inner.width - label.chars().count() as u16 - 1;
                if label_x > cx {
                    let mut sx = label_x;
                    for ch in label.chars() {
                        if sx >= inner.x + inner.width {
                            break;
                        }
                        if let Some(cell) = buf.cell_mut((sx, y)) {
                            cell.set_char(ch);
                            cell.set_style(Style::default().fg(dim).bg(line_bg));
                        }
                        sx += 1;
                    }
                }
            }
        }

        let bar_y = inner.y + inner.height - 1;
        if self.state.action == TreeAction::None {
            for lx in inner.x..inner.x + inner.width {
                if let Some(cell) = buf.cell_mut((lx, bar_y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                }
            }
            let max_n = self.state.hint_indices.len();
            let jump = if max_n == 0 {
                String::new()
            } else if max_n == 1 {
                "1 jump  ".to_string()
            } else {
                format!("1-{max_n} jump  ")
            };
            let line = format!("{}\u{232b} back  n new  r rename  d del  z collapse", jump);
            let mut cx = inner.x + 1;
            for ch in line.chars() {
                if cx >= inner.x + inner.width - 1 {
                    break;
                }
                if let Some(cell) = buf.cell_mut((cx, bar_y)) {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(dim).bg(bg));
                }
                cx += 1;
            }
        } else {
            for lx in inner.x..inner.x + inner.width {
                if let Some(cell) = buf.cell_mut((lx, bar_y)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(selected_bg));
                }
            }
            let label = match self.state.action {
                TreeAction::NewFile => " new file: ",
                TreeAction::NewFolder => " new folder: ",
                TreeAction::Rename => " rename: ",
                TreeAction::Delete => " delete? (y/n) ",
                TreeAction::None => "",
            };
            let display = format!("{}{}", label, self.state.input_buf);
            let mut cx = inner.x;
            for (ci, ch) in display.chars().enumerate() {
                if cx >= inner.x + inner.width {
                    break;
                }
                let style = if ci < label.chars().count() {
                    Style::default().fg(accent).bg(selected_bg)
                } else {
                    Style::default().fg(fg).bg(selected_bg)
                };
                if let Some(cell) = buf.cell_mut((cx, bar_y)) {
                    cell.set_char(ch);
                    cell.set_style(style);
                }
                cx += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn paths(state: &BrowserState) -> Vec<String> {
        state
            .entries
            .iter()
            .map(|e| e.path.display().to_string())
            .collect()
    }

    fn names(state: &BrowserState) -> Vec<String> {
        state.entries.iter().map(|e| e.name.clone()).collect()
    }

    #[test]
    fn build_filters_to_md_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("dir")).unwrap();
        fs::write(root.join("dir/a.md"), "").unwrap();
        fs::write(root.join("dir/b.txt"), "").unwrap();
        fs::create_dir(root.join("empty_dir")).unwrap();
        fs::write(root.join("top.md"), "").unwrap();

        let mut state = BrowserState::default();
        state.open_dirs.insert(root.join("dir"));
        state.build(root);

        let ps = paths(&state);
        let has = |s: &str| ps.iter().any(|p| p.ends_with(s));
        assert!(has("dir"), "dir should appear");
        assert!(has("dir/a.md"), "dir/a.md should appear");
        assert!(has("top.md"), "top.md should appear");
        assert!(!has("dir/b.txt"), "b.txt must be filtered out");
        assert!(
            !ps.iter().any(|p| p.ends_with("empty_dir")),
            "empty_dir must be excluded"
        );
    }

    #[test]
    fn build_skips_dot_git_and_node_modules() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join(".git/inside.md"), "").unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::write(root.join("node_modules/pkg.md"), "").unwrap();
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("target/build.md"), "").unwrap();
        fs::write(root.join("README.md"), "").unwrap();

        let mut state = BrowserState::default();
        state.build(root);

        let ns = names(&state);
        assert!(!ns.iter().any(|n| n == ".git"), "git must be hidden");
        assert!(
            !ns.iter().any(|n| n == "node_modules"),
            "node_modules must be hidden"
        );
        assert!(!ns.iter().any(|n| n == "target"), "target must be hidden");
        assert!(ns.iter().any(|n| n == "README.md"));
    }

    #[test]
    fn build_excludes_empty_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("nope")).unwrap();
        fs::write(root.join("nope/code.rs"), "").unwrap();
        fs::write(root.join("yes.md"), "").unwrap();

        let mut state = BrowserState::default();
        state.build(root);

        assert!(!names(&state).iter().any(|n| n == "nope"));
        assert!(names(&state).iter().any(|n| n == "yes.md"));
    }

    #[test]
    fn confirm_new_file_appends_md_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mut state = BrowserState::default();
        state.build(root);
        state.selected = 0;
        state.start_action(TreeAction::NewFile);
        state.input_buf = "notes".to_string();
        let created = state.confirm_new_file().expect("ok").expect("path");
        assert!(created.ends_with("notes.md"));
        assert!(created.exists());
    }

    #[test]
    fn confirm_new_file_accepts_explicit_md() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mut state = BrowserState::default();
        state.build(root);
        state.selected = 0;
        state.start_action(TreeAction::NewFile);
        state.input_buf = "notes.md".to_string();
        let created = state.confirm_new_file().expect("ok").expect("path");
        assert!(created.ends_with("notes.md"));
    }

    #[test]
    fn confirm_new_file_rejects_other_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mut state = BrowserState::default();
        state.build(root);
        state.selected = 0;
        state.start_action(TreeAction::NewFile);
        state.input_buf = "notes.txt".to_string();
        let err = state.confirm_new_file().unwrap_err();
        assert!(err.contains(".md"));
        assert!(!root.join("notes.txt").exists());
    }

    #[test]
    fn confirm_new_file_rejects_dot_path_with_no_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mut state = BrowserState::default();
        state.build(root);
        state.selected = 0;
        state.start_action(TreeAction::NewFile);
        state.input_buf = "foo.".to_string();
        let err = state.confirm_new_file().unwrap_err();
        assert!(err.contains(".md"));
        assert!(!root.join("foo.").exists());
    }

    #[test]
    fn confirm_new_folder_accepts_any_name() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mut state = BrowserState::default();
        state.build(root);
        state.selected = 0;
        state.start_action(TreeAction::NewFolder);
        state.input_buf = "weird.name".to_string();
        let created = state.confirm_new_folder().expect("ok").expect("path");
        assert!(created.ends_with("weird.name"));
        assert!(created.is_dir());
    }

    #[test]
    fn rename_then_undo_restores_old_name() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("a.md"), "body").unwrap();
        let mut state = BrowserState::default();
        state.build(root);
        let idx = state
            .entries
            .iter()
            .position(|e| e.name == "a.md")
            .expect("a.md");
        state.selected = idx;
        state.start_action(TreeAction::Rename);
        state.input_buf = "b.md".to_string();
        let new = state.confirm_rename().expect("ok").expect("path");
        assert!(new.ends_with("b.md"));
        assert!(!root.join("a.md").exists());
        assert!(root.join("b.md").exists());

        assert!(state.undo_last_fs_op());
        assert!(root.join("a.md").exists());
        assert!(!root.join("b.md").exists());
    }

    #[test]
    fn delete_file_then_undo_restores_content() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("a.md"), "hello world").unwrap();
        let mut state = BrowserState::default();
        state.build(root);
        let idx = state
            .entries
            .iter()
            .position(|e| e.name == "a.md")
            .expect("a.md");
        state.selected = idx;
        assert!(state.confirm_delete());
        assert!(!root.join("a.md").exists());
        assert!(state.undo_last_fs_op());
        assert!(root.join("a.md").exists());
        assert_eq!(
            fs::read_to_string(root.join("a.md")).unwrap(),
            "hello world"
        );
    }

    #[test]
    fn move_marks_then_confirms_to_new_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("dest")).unwrap();
        fs::write(root.join("dest/keep.md"), "").unwrap();
        fs::write(root.join("a.md"), "x").unwrap();

        let mut state = BrowserState::default();
        state.open_dirs.insert(root.join("dest"));
        state.build(root);
        let src_idx = state
            .entries
            .iter()
            .position(|e| e.name == "a.md" && e.depth == 0)
            .expect("a.md");
        state.selected = src_idx;
        state.mark_for_move();
        assert!(state.marked_for_move.is_some());

        let dest_idx = state
            .entries
            .iter()
            .position(|e| e.name == "dest")
            .expect("dest");
        state.selected = dest_idx;
        let new = state.confirm_move().expect("moved");
        assert!(new.ends_with("dest/a.md") || new.ends_with("dest\\a.md"));
        assert!(!root.join("a.md").exists());
        assert!(root.join("dest/a.md").exists());
    }

    #[test]
    fn cancel_move_clears_marker() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("a.md"), "").unwrap();
        let mut state = BrowserState::default();
        state.build(root);
        let idx = state
            .entries
            .iter()
            .position(|e| e.name == "a.md")
            .expect("a.md");
        state.selected = idx;
        state.mark_for_move();
        assert!(state.marked_for_move.is_some());
        state.cancel_move();
        assert!(state.marked_for_move.is_none());
    }

    #[test]
    fn hint_navigation_enters_folder_then_back() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub/inner.md"), "").unwrap();
        fs::write(root.join("top.md"), "").unwrap();

        let mut state = BrowserState::default();
        state.build(root);
        state.init_hint_scope();
        let sub_hint = state
            .hint_indices
            .iter()
            .position(|&i| state.entries[i].name == "sub")
            .expect("sub in hints");
        let result = state.hint_enter(sub_hint).expect("entered");
        assert!(matches!(result, HintResult::EnteredFolder));
        assert_eq!(
            state.hint_scope.as_deref(),
            Some(root.join("sub").as_path())
        );
        assert!(state
            .hint_indices
            .iter()
            .any(|&i| state.entries[i].name == "inner.md"));

        state.hint_back();
        assert_eq!(state.hint_scope.as_deref(), Some(root));
    }

    #[test]
    fn hint_navigation_opens_md_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("readme.md"), "").unwrap();
        let mut state = BrowserState::default();
        state.build(root);
        state.init_hint_scope();
        let pos = state
            .hint_indices
            .iter()
            .position(|&i| state.entries[i].name == "readme.md")
            .expect("readme.md");
        let r = state.hint_enter(pos).expect("opened");
        match r {
            HintResult::OpenFile(p) => assert!(p.ends_with("readme.md")),
            _ => panic!("expected OpenFile"),
        }
    }

    #[test]
    fn redo_after_undo_restores() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let mut state = BrowserState::default();
        state.build(root);
        state.selected = 0;
        state.start_action(TreeAction::NewFile);
        state.input_buf = "x".to_string();
        let created = state.confirm_new_file().unwrap().unwrap();
        assert!(created.exists());

        assert!(state.undo_last_fs_op());
        assert!(!created.exists());

        assert!(state.redo_last_fs_op());
        assert!(created.exists());
    }
}
