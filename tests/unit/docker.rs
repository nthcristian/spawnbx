use super::*;

#[test]
fn identity_hash_preserves_original_algorithm_and_decimal_encoding() {
    for (source, pinned) in [
        ("", "49651267"),
        ("/", "10306387"),
        ("/tmp/workspace", "26388987"),
        ("/tmp/workspace with spaces", "87362416"),
        ("relative/path", "47493312"),
    ] {
        let mut hasher = DefaultHasher::new();
        source.to_owned().hash(&mut hasher);
        let expected: String = hasher.finish().to_be_bytes()[..4]
            .iter()
            .map(|byte| format!("{:02}", byte % 99 + 1))
            .collect();
        let actual = workspace_hash(source);
        assert_eq!(actual, expected);
        assert_eq!(actual, pinned);
        assert_eq!(actual.len(), 8);
        assert!(actual.bytes().all(|byte| byte.is_ascii_digit()));
    }
}
