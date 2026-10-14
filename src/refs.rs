/// Converts a short ref from the XAPI logs into a number
///
/// In the logs, XAPI does not write the full ref of a task. It writes a short
/// version in the task part of the line:
/// ```text
/// [debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit]
///                                               ^^^^^^^^^^^^^^
/// ```
///
/// A short ref starts with a R:, D: or O: and is followed by 12 hex digits.
/// - R: is a real ref. The task is in the XAPI database.
/// - D: is a dummy ref. The task is not in the XAPI database.
/// - O: is another kind of ref. It contains text, not hex, so we return None.
///
/// The 12 hex digits come from the uuid of the full ref. We take the first
/// 8 digits and the 4 digits after the first dash:
///
/// ```text
/// OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2
///           ^^^^^^^^ ^^^^
/// R:5fe71a7a2d59
/// ```
///
/// We return the 12 digits as a number because it is fast to compare and
/// easy to use as a key.
///
/// See `really_pretty_and_small` in ocaml/xapi-types/ref.ml.
pub fn short_key(s: &str) -> Option<u64> {
    let chars = s.as_bytes(); // gives a slice

    if chars.len() != 14 {
        return None;
    };

    if !matches!(chars[0], b'R' | b'D' | b'O') || chars[1] != b':' {
        return None;
    }

    chars[2..].iter().try_fold(0u64, |acc, &b| {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as u64,
            b'a'..=b'f' => (b - b'a' + 10) as u64,
            b'A'..=b'F' => (b - b'A' + 10) as u64,
            _ => return None,
        };
        acc.checked_mul(16)?.checked_add(digit)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // helper: prints values in hex when an assertion fails
    fn check(input: &str, expected: Option<u64>) {
        let got = short_key(input);
        assert_eq!(
            got, expected,
            "short_key({input:?}) = {got:x?}, expected {expected:x?}"
        );
    }

    // --- valid refs, taken from the real logs ---

    #[test]
    fn real_ref() {
        check("R:5fe71a7a2d59", Some(0x5fe71a7a2d59));
    }

    #[test]
    fn dummy_ref() {
        check("D:c1baed25a658", Some(0xc1baed25a658));
    }

    #[test]
    fn leading_zeros() {
        check("D:000000000001", Some(1));
    }

    #[test]
    fn all_f() {
        check("R:ffffffffffff", Some(0xffff_ffff_ffff));
    }

    // we chose to accept uppercase: this test makes that choice explicit
    #[test]
    fn uppercase_accepted() {
        check("R:5FE71A7A2D59", Some(0x5fe71a7a2d59));
    }

    // --- prefix ---

    #[test]
    fn bad_letter() {
        check("X:5fe71a7a2d59", None);
    }

    #[test]
    fn bad_separator() {
        check("R-5fe71a7a2d59", None);
    }

    #[test]
    fn lowercase_prefix() {
        check("r:5fe71a7a2d59", None);
    }

    // --- length ---

    #[test]
    fn too_short() {
        check("R:5fe71a7a2d5", None);
    }

    #[test]
    fn too_long() {
        check("R:5fe71a7a2d590", None);
    }

    #[test]
    fn empty() {
        check("", None);
    }

    // --- content ---

    #[test]
    fn null_ref() {
        check("NULL", None);
    }

    // O: refs carry text (a truncated dbg string), not hex
    #[test]
    fn other_ref_with_text() {
        check("O:VM.migrate_s", None);
    }

    #[test]
    fn plus_sign() {
        check("R:+fe71a7a2d59", None);
    }

    #[test]
    fn non_hex_letter() {
        check("R:5fe71a7a2d5g", None);
    }

    #[test]
    fn space_inside() {
        check("R:5fe71a7 2d59", None);
    }

    // 14 bytes but not 14 characters: 'é' is 2 bytes in UTF-8
    #[test]
    fn non_ascii() {
        check("R:5fe71a7a2dé", None);
    }
}
