use rscheck::fix::apply_text_edits;
use rscheck::report::{FixSafety, Report, TextEdit};
use similar::TextDiff;
use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, io};

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("failed to read file: {path}")]
    Read { path: String, source: io::Error },
    #[error("failed to write file: {path}")]
    Write { path: String, source: io::Error },
    #[error("failed to apply edits for file: {path}")]
    Apply {
        path: String,
        source: rscheck::fix::FixError,
    },
}

#[derive(Debug, Clone)]
pub struct PlannedEdits {
    pub by_file: BTreeMap<String, Vec<TextEdit>>,
}

impl PlannedEdits {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_file.values().all(Vec::is_empty)
    }
}

#[derive(Clone)]
struct EditWithMeta {
    safety: FixSafety,
    fix_id: String,
    edit: TextEdit,
}

pub fn plan_edits(report: &Report, include_unsafe: bool) -> PlannedEdits {
    let mut by_file: BTreeMap<String, Vec<EditWithMeta>> = BTreeMap::new();
    for finding in &report.findings {
        for fix in finding.fixes() {
            if fix.safety == FixSafety::Unsafe && !include_unsafe {
                continue;
            }
            for edit in &fix.edits {
                insert_edit_with_meta(&mut by_file, fix, edit);
            }
        }
    }

    let mut planned: BTreeMap<String, Vec<TextEdit>> = BTreeMap::new();
    for (file, mut edits) in by_file {
        edits.sort_by(|a, b| {
            let sa = safety_rank(a.safety);
            let sb = safety_rank(b.safety);
            sb.cmp(&sa)
                .then(a.edit.byte_start.cmp(&b.edit.byte_start))
                .then(a.edit.byte_end.cmp(&b.edit.byte_end))
                .then(a.fix_id.cmp(&b.fix_id))
        });

        let mut chosen: Vec<EditWithMeta> = Vec::new();
        'next: for e in edits {
            for c in &chosen {
                if overlaps(&e.edit, &c.edit) {
                    continue 'next;
                }
            }
            chosen.push(e);
        }

        planned.insert(file, chosen_edits(chosen));
    }

    PlannedEdits { by_file: planned }
}

fn insert_edit_with_meta(
    by_file: &mut BTreeMap<String, Vec<EditWithMeta>>,
    fix: &rscheck::report::Fix,
    edit: &TextEdit,
) {
    by_file
        .entry(Clone::clone(&edit.file))
        .or_default()
        .push(EditWithMeta {
            safety: fix.safety,
            fix_id: Clone::clone(&fix.id),
            edit: Clone::clone(edit),
        });
}

fn chosen_edits(chosen: Vec<EditWithMeta>) -> Vec<TextEdit> {
    Vec::from_iter(chosen.into_iter().map(|edit| edit.edit))
}

fn safety_rank(s: FixSafety) -> u8 {
    match s {
        FixSafety::Safe => 2,
        FixSafety::Unsafe => 1,
    }
}

fn overlaps(a: &TextEdit, b: &TextEdit) -> bool {
    let a0 = a.byte_start;
    let a1 = a.byte_end;
    let b0 = b.byte_start;
    let b1 = b.byte_end;
    !(a1 <= b0 || b1 <= a0)
}

pub fn apply_planned_edits(planned: &PlannedEdits) -> Result<bool, ApplyError> {
    let mut changed = false;
    for (file, edits) in &planned.by_file {
        if edits.is_empty() {
            continue;
        }
        let old = read_edit_file(file)?;
        let new = apply_file_edits(file, &old, edits)?;
        if new != old {
            write_edit_file(file, new)?;
            changed = true;
        }
    }
    Ok(changed)
}

pub fn print_dry_run(planned: &PlannedEdits) -> Result<bool, ApplyError> {
    let mut would_change = false;
    for (file, edits) in &planned.by_file {
        if edits.is_empty() {
            continue;
        }
        let old = read_edit_file(file)?;
        let new = apply_file_edits(file, &old, edits)?;
        if new == old {
            continue;
        }
        would_change = true;
        let diff = unified_diff(file, &old, &new);
        print!("{diff}");
    }
    Ok(would_change)
}

fn read_edit_file(file: &str) -> Result<String, ApplyError> {
    fs::read_to_string(file).map_err(|source| ApplyError::Read {
        path: file.to_string(),
        source,
    })
}

fn apply_file_edits(file: &str, old: &str, edits: &[TextEdit]) -> Result<String, ApplyError> {
    apply_text_edits(old, edits).map_err(|source| ApplyError::Apply {
        path: file.to_string(),
        source,
    })
}

fn write_edit_file(file: &str, new: String) -> Result<(), ApplyError> {
    fs::write(file, new).map_err(|source| ApplyError::Write {
        path: file.to_string(),
        source,
    })
}

fn unified_diff(file: &str, old: &str, new: &str) -> String {
    TextDiff::from_lines(old, new)
        .unified_diff()
        .header(&diff_header("a", file), &diff_header("b", file))
        .to_string()
}

fn diff_header(prefix: &str, file: &str) -> String {
    format!("{prefix}/{}", display_path(file))
}

fn display_path(path: &str) -> String {
    Path::new(path).to_string_lossy().to_string()
}
