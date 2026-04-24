use super::matches_path_prefix;

#[test]
fn respects_segment_boundaries() {
    assert!(matches_path_prefix("crate::Error", "crate::Error"));
    assert!(matches_path_prefix("crate::Error<T>", "crate::Error"));
    assert!(!matches_path_prefix("crate::Errorish", "crate::Error"));
    assert!(!matches_path_prefix("crate::Errors::Thing", "crate::Error"));
}
