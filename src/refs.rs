fn parse_hex(bytes: &[u8]) -> Option<u64> {
    // Caller checks the length, empty gives Some(0).
    bytes.iter().try_fold(0u64, |acc, &b| {
        let digit = match b {
            b'0'..=b'9' => (b - b'0') as u64,
            b'a'..=b'f' => (b - b'a' + 10) as u64,
            b'A'..=b'F' => (b - b'A' + 10) as u64,
            _ => return None,
        };
        acc.checked_mul(16)?.checked_add(digit)
    })
}

/// Converts a short ref from the XAPI logs into a number
///
/// In the logs, XAPI does not write the full ref of a task. It writes a short
/// version in the task part of the line:
/// ```text
/// [debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit]
///                                               ^^^^^^^^^^^^^^
/// ```
///
/// A short ref starts with R:, D: or O: and is followed by 12 hex digits.
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
    }

    if !matches!(chars[0], b'R' | b'D' | b'O') || chars[1] != b':' {
        return None;
    }

    parse_hex(&chars[2..])
}

/// Converts a full uuid into the same key as [`short_key`]
///
/// XAPI uses uuids in full refs:
/// - `OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2` in the logs and the database
/// - `DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan` in SMlog
///
/// This function takes only the uuid part, not the full ref.
///
/// A uuid is 36 bytes: 32 hex digits plus 4 dashes, always at the same positions.
///
/// ```text
/// c1baed25-a658-9de0-b212-200d2d2d9c9f
/// 0       8    13   18   23          35 <- dashes positions are 8, 13, 18 and 23
/// ^^^^^^^^ ^^^^
///    |       |
///    |       -- 9..13
///    -- 0..8
/// key = (first 8 hex) followed by (next 4 hex) = 48 bits
/// ```
///
/// The full uuid must be valid. If one digit or one dash is wrong, even in the part
/// we don't use, we return None.
///
/// # Examples
///
/// ```
/// use xapi_logs::refs::{key_of_uuid, short_key};
///
/// // A uuid from SMlog gives the same key as the short ref in xensource.log
/// let from_smlog = key_of_uuid("c1baed25-a658-9de0-b212-200d2d2d9c9f");
/// assert_eq!(from_smlog, short_key("D:c1baed25a658"));
///
/// // A full ref is not a uuid
/// assert_eq!(key_of_uuid("OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2"), None);
/// ```
pub fn key_of_uuid(uuid: &str) -> Option<u64> {
    // We are expecting 32 digits + 4 dashes
    if uuid.len() != 36 {
        return None;
    }

    // Split parts
    let parts = uuid.split('-');

    // Check that we have the expected repartition.
    if !parts.clone().map(|s| s.len()).eq([8, 4, 4, 4, 12]) {
        return None;
    }

    // First we extract the first two values that we will combine later.
    let mut hex_values = parts.map(|s| parse_hex(s.as_bytes()));
    let v1 = hex_values.next()??;
    let v2 = hex_values.next()??;

    // Ensure that all remaining values are valid hexadecimal.
    if hex_values.any(|v| v.is_none()) {
        return None;
    }

    // uuid is valid so return its key
    Some(v1 << 16 | v2)
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

    fn check_uuid(input: &str, expected: Option<u64>) {
        let got = key_of_uuid(input);
        assert_eq!(
            got, expected,
            "key_of_uuid({input:?} = {got:x?}), expected {expected:x?}"
        );
    }
    // --- valid refs, taken from the real logs ---

    #[test]
    fn real_ref() {
        check("R:5fe71a7a2d59", Some(0x5fe71a7a2d59));
    }

    #[test]
    fn empty_ref() {
        check("", None);
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

    #[test]
    fn uuid_basic() {
        check_uuid("c1baed25-a658-9de0-b212-200d2d2d9c9f", Some(0xc1baed25a658));
    }

    #[test]
    fn uuid_leading_zeros() {
        // real: R:0001943f66a7 in xensource.log.3
        check_uuid("0001943f-66a7-bd78-a605-90682c16f252", Some(0x0001943f66a7));
    }

    // SMlog: 'subtask_of': 'DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan'
    // xensource.log: task SR.scan D:c1baed25a658
    #[test]
    fn smlog_dummy_ref_matches_xapi_short_ref() {
        assert_eq!(
            key_of_uuid("c1baed25-a658-9de0-b212-200d2d2d9c9f"),
            short_key("D:c1baed25a658"),
        );
    }

    // the migration task: OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2 <-> R:5fe71a7a2d59
    #[test]
    fn opaque_ref_matches_xapi_short_ref() {
        assert_eq!(
            key_of_uuid("5fe71a7a-2d59-af9c-0ddb-89f1324b52d2"),
            short_key("R:5fe71a7a2d59"),
        );
    }

    // step 1c: a task's "(uuid:...)" is its uuid FIELD, not its OpaqueRef.
    // xensource.log.3:
    //   task Async.VDI.snapshot R:6cdc3a335e85 (uuid:8b98b2c1-9519-6bd8-c3b2-5dd61df04f3d) created ...
    #[test]
    fn object_uuid_is_not_its_ref() {
        let from_uuid_field = key_of_uuid("8b98b2c1-9519-6bd8-c3b2-5dd61df04f3d");
        assert!(from_uuid_field.is_some(), "it is a valid uuid...");
        assert_ne!(
            from_uuid_field,
            short_key("R:6cdc3a335e85"),
            "...but not the task's ref"
        );
    }

    // --- invalid ---

    #[test]
    fn uuid_empty() {
        check_uuid("", None);
    }

    #[test]
    fn uuid_too_short() {
        check_uuid("c1baed25-a658-9de0-b212-200d2d2d9c9", None);
    }

    #[test]
    fn uuid_too_long() {
        check_uuid("c1baed25-a658-9de0-b212-200d2d2d9c9f0", None);
    }

    // a short ref is not a uuid
    #[test]
    fn uuid_given_a_short_ref() {
        check_uuid("D:c1baed25a658", None);
    }

    // a whole ref is not a bare uuid (extracting the uuid from refs is the next step)
    #[test]
    fn uuid_given_a_full_opaque_ref() {
        check_uuid("OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2", None);
    }

    #[test]
    fn uuid_null_ref() {
        check_uuid("OpaqueRef:NULL", None);
    }

    // --- strict validation (drop these if you choose lenient) ---

    #[test]
    fn uuid_no_dashes() {
        // 32 hex chars, but the wrong shape (and the wrong length)
        check_uuid("c1baed25a6589de0b212200d2d2d9c9f", None);
    }

    #[test]
    fn uuid_dash_misplaced() {
        check_uuid("c1baed2-5a658-9de0-b212-200d2d2d9c9f", None);
    }

    #[test]
    fn uuid_non_hex_in_used_part() {
        check_uuid("c1baed25-a65g-9de0-b212-200d2d2d9c9f", None);
    }

    #[test]
    fn uuid_non_hex_in_unused_part() {
        // only fails if you validate the whole uuid
        check_uuid("c1baed25-a658-9de0-b212-200d2d2d9c9z", None);
    }
}
