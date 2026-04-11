use crate::report::TextEdit;

#[derive(Debug, thiserror::Error)]
pub enum FixError {
    #[error("invalid line/column")]
    InvalidLineColumn,
    #[error("invalid utf-8 boundaries for edit")]
    InvalidUtf8Boundary,
    #[error("edit out of bounds")]
    OutOfBounds,
    #[error("overlapping edits")]
    Overlap,
}

pub fn line_col_to_byte_offset(text: &str, line_1: u32, col_1: u32) -> Result<usize, FixError> {
    if line_1 == 0 || col_1 == 0 {
        return Err(FixError::InvalidLineColumn);
    }

    let mut current_line: u32 = 1;
    let mut idx: usize = 0;
    while current_line < line_1 {
        let Some(pos) = text[idx..].find('\n') else {
            return Err(FixError::InvalidLineColumn);
        };
        idx += pos + 1;
        current_line += 1;
    }

    let line_end = text[idx..].find('\n').map_or(text.len(), |p| idx + p);
    let line_slice = &text[idx..line_end];

    let target_col = (col_1 - 1) as usize;
    let mut col: usize = 0;
    let mut byte_in_line: usize = 0;
    for (bidx, ch) in line_slice.char_indices() {
        if col == target_col {
            byte_in_line = bidx;
            break;
        }
        col += 1;
        byte_in_line = bidx + ch.len_utf8();
    }
    if col < target_col {
        if target_col == col {
            // ok
        } else {
            return Err(FixError::InvalidLineColumn);
        }
    }

    Ok(idx + byte_in_line)
}

pub fn apply_text_edits(text: &str, edits: &[TextEdit]) -> Result<String, FixError> {
    let mut out = text.to_string();
    let mut ordered = edits.to_vec();
    ordered.sort_by(|a, b| {
        b.byte_start
            .cmp(&a.byte_start)
            .then(b.byte_end.cmp(&a.byte_end))
    });

    let mut last_start: Option<u32> = None;
    for e in &ordered {
        if e.byte_start > e.byte_end {
            return Err(FixError::OutOfBounds);
        }
        if let Some(last) = last_start {
            if e.byte_end > last {
                return Err(FixError::Overlap);
            }
        }
        last_start = Some(e.byte_start);

        let start = e.byte_start as usize;
        let end = e.byte_end as usize;
        if end > out.len() {
            return Err(FixError::OutOfBounds);
        }
        if !out.is_char_boundary(start) || !out.is_char_boundary(end) {
            return Err(FixError::InvalidUtf8Boundary);
        }
        out.replace_range(start..end, &e.replacement);
    }

    Ok(out)
}

pub fn find_use_insertion_offset(text: &str) -> usize {
    let mut offset: usize = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let is_inner_attr = trimmed.starts_with("#![");
        let is_inner_doc = trimmed.starts_with("//!") || trimmed.starts_with("/*!");
        if is_inner_attr || is_inner_doc {
            offset += line.len();
            continue;
        }
        break;
    }
    offset
}
