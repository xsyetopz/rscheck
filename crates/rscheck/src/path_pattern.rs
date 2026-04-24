pub fn matches_path_prefix(candidate: &str, prefix: &str) -> bool {
    if !candidate.starts_with(prefix) {
        return false;
    }

    if candidate.len() == prefix.len() {
        return true;
    }

    candidate[prefix.len()..]
        .chars()
        .next()
        .is_some_and(is_path_boundary)
}

fn is_path_boundary(ch: char) -> bool {
    matches!(ch, ':' | '<' | '(' | '!' | '[' | ',' | ' ' | '&')
}

#[cfg(test)]
mod tests;
