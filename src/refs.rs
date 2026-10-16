//! Short refs, full refs and keys.
//!
//! In the logs, XAPI writes refs in two ways: short (`R:5fe71a7a2d59`)
//! and full (`OpaqueRef:5fe71a7a-2d59-...`). This module converts both
//! into the same number, a key, so we can link them.
//!
//! See [chapter 1](crate::chapter_01_refs) for how XAPI uses refs.

const REF_PREFIX: &str = "OpaqueRef:";
const DUMMY_PREFIX: &str = "DummyRef:";
const DUMMY_SEP: &str = "|";

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

/// Converts a short ref from the XAPI logs into a number.
///
/// A short ref is what XAPI writes in the task part of a log line.
/// See [chapter 1](crate::chapter_01_refs#short-refs-in-the-task-column).
///
/// The format is `R:`, `D:` or `O:` followed by 12 hex digits. We return
/// the 12 digits as a number because it is fast to compare and
/// easy to use as a key.
///
/// Returns None if:
/// - the prefix is not `R:`, `D:` or `O:`,
/// - there are not exactly 12 hex digits after it. An `O:` ref usually contains
///   text, like `O:VM.migrate_s`, so it gives None.
///
/// See [`really_pretty_and_small`](https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/xapi-types/ref.ml#L137)
/// in ref.ml.
///
/// # Examples
///
/// ```
/// use xapi_logs::refs::short_key;
///
/// assert_eq!(short_key("R:5fe71a7a2d59"), Some(0x5fe71a7a2d59));
/// assert_eq!(short_key("O:VM.migrate_s"), None);
/// ```
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

/// Converts a full uuid into the same key as [`short_key`].
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
/// Be careful: use the uuid inside a ref, not the `uuid` field of an object.
/// See [the trap](crate::chapter_01_refs#a-trap-a-ref-is-not-a-uuid).
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

/// Converts a full ref into the same key as [`short_key`].
///
/// XAPI writes full refs in log messages, in *SMlog*, and sometimes in the
/// task part of the line. There are two kinds of full ref:
/// ```text
/// OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2          <- Real ref
/// DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan  <- Dummy ref
///           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ ^^^^^^^
///                       uuid                        name
/// ```
///
/// We take the uuid and we use [`key_of_uuid`]. The name of a dummy ref is
/// not used: it can be empty, or contain `|` or `:`.
///
/// Returns None if:
/// - the input is `OpaqueRef:NULL`, the null ref,
/// - the uuid is invalid,
/// - there is extra text after the ref, like `OpaqueRef:<uuid>,`. The input
///   must be one ref only. Finding refs inside a message is another job.
///
/// See [`of_string`](https://github.com/xapi-project/xen-api/blob/26.1-lcm/ocaml/xapi-types/ref.ml#L109)
/// in ref.ml for the formats.
///
/// # Examples
///
/// ```
/// use xapi_logs::refs::{key_of_ref, short_key};
///
/// // A full ref and its short ref give the same key
/// let full = key_of_ref("OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2");
/// assert_eq!(full, short_key("R:5fe71a7a2d59"));
///
/// // A dummy ref from SMlog and its short ref in xensource.log
/// let dummy = key_of_ref("DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan");
/// assert_eq!(dummy, short_key("D:c1baed25a658"));
///
/// assert_eq!(key_of_ref("OpaqueRef:NULL"), None);
/// ```
pub fn key_of_ref(input: &str) -> Option<u64> {
    if let Some(uuid) = input.strip_prefix(REF_PREFIX) {
        key_of_uuid(uuid)
    } else if let Some(dummy_parts) = input.strip_prefix(DUMMY_PREFIX) {
        let mut subparts = dummy_parts.split(DUMMY_SEP);
        // We are expecting an empty string and then the UUID
        if !subparts.next()?.is_empty() {
            return None;
        }
        let uuid = subparts.next()?;
        key_of_uuid(uuid)
    } else {
        None
    }
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

    fn check_ref(input: &str, expected: Option<u64>) {
        let got = key_of_ref(input);
        assert_eq!(
            got, expected,
            "key_of_ref({input:?}) = {got:x?}, expected {expected:x?}"
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

    // --- real refs ---

    #[test]
    fn ref_opaque() {
        check_ref(
            "OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2",
            Some(0x5fe71a7a2d59),
        );
    }

    // the migration task: full ref <-> R: tag in the task column
    #[test]
    fn ref_opaque_matches_short_ref() {
        assert_eq!(
            key_of_ref("OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2"),
            short_key("R:5fe71a7a2d59"),
        );
    }

    // xensource.log.3: a full ref used as the task column itself
    //   [ info||262544 |OpaqueRef:84efd69f-050a-c857-6d69-ca645f4faf24|Storage_smapiv1_wrapper]
    #[test]
    fn ref_opaque_from_task_column() {
        check_ref(
            "OpaqueRef:84efd69f-050a-c857-6d69-ca645f4faf24",
            Some(0x84efd69f050a),
        );
    }

    // --- dummy refs ---

    // SMlog: 'subtask_of': 'DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan'
    #[test]
    fn ref_dummy() {
        check_ref(
            "DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan",
            Some(0xc1baed25a658),
        );
    }

    // xensource-ip11.log (http request): subtask_of = DummyRef:|d208e7e2-...|VM.assert_can_migrate
    // and in the task column of the migration: VM.assert_can_migrate D:d208e7e26ae6
    #[test]
    fn ref_dummy_matches_short_ref() {
        assert_eq!(
            key_of_ref("DummyRef:|d208e7e2-6ae6-0dbf-e3f0-1b75437f516e|VM.assert_can_migrate"),
            short_key("D:d208e7e26ae6"),
        );
    }

    // xapi joins the rest back as the name: a name can contain '|'
    #[test]
    fn ref_dummy_name_with_pipe() {
        check_ref(
            "DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|a|b",
            Some(0xc1baed25a658),
        );
    }

    #[test]
    fn ref_dummy_empty_name() {
        check_ref(
            "DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|",
            Some(0xc1baed25a658),
        );
    }

    // valid for xapi's Ref.of_string too: no name at all
    #[test]
    fn ref_dummy_no_name() {
        check_ref(
            "DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f",
            Some(0xc1baed25a658),
        );
    }

    // dummy names are task names, and many contain ':' (dispatch:event.from)
    #[test]
    fn ref_dummy_name_with_colon() {
        check_ref(
            "DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f|dispatch:event.from",
            Some(0xc1baed25a658),
        );
    }

    // the uuid must come right after "DummyRef:|"
    #[test]
    fn ref_dummy_text_before_pipe() {
        check_ref(
            "DummyRef:x|c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan",
            None,
        );
    }

    // the uuid must be followed by '|' or the end, not ':'
    #[test]
    fn ref_dummy_colon_after_uuid() {
        check_ref(
            "DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f:SR.scan",
            None,
        );
    }

    // --- null and other ---

    #[test]
    fn ref_null() {
        check_ref("OpaqueRef:NULL", None);
    }

    #[test]
    fn ref_prefix_only() {
        check_ref("OpaqueRef:", None);
    }

    #[test]
    fn ref_dummy_prefix_only() {
        check_ref("DummyRef:|", None);
    }

    #[test]
    fn ref_empty() {
        check_ref("", None);
    }

    // not a full ref: those have their own functions
    #[test]
    fn ref_given_a_bare_uuid() {
        check_ref("5fe71a7a-2d59-af9c-0ddb-89f1324b52d2", None);
    }

    #[test]
    fn ref_given_a_short_ref() {
        check_ref("R:5fe71a7a2d59", None);
    }

    #[test]
    fn ref_lowercase_prefix() {
        check_ref("opaqueref:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2", None);
    }

    // --- malformed ---

    #[test]
    fn ref_opaque_bad_uuid() {
        check_ref("OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52dz", None);
    }

    // missing the '|' right after "DummyRef:"
    #[test]
    fn ref_dummy_no_leading_pipe() {
        check_ref(
            "DummyRef:c1baed25-a658-9de0-b212-200d2d2d9c9f|SR.scan",
            None,
        );
    }

    // the uuid must be followed by '|' or the end
    #[test]
    fn ref_dummy_uuid_too_long() {
        check_ref(
            "DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9f0|SR.scan",
            None,
        );
    }

    // must return None, and must not panic: 'é' is 2 bytes and sits across byte 36
    #[test]
    fn ref_dummy_non_ascii_at_uuid_end() {
        check_ref(
            "DummyRef:|c1baed25-a658-9de0-b212-200d2d2d9c9é|SR.scan",
            None,
        );
    }

    // --- exact token only: finding refs inside text is a later step ---

    // real: "Failed to synchronise DB with host OpaqueRef:6fd1bdb7-...:"
    #[test]
    fn ref_with_trailing_colon() {
        check_ref("OpaqueRef:6fd1bdb7-8df9-4b87-a280-2e072236498b:", None);
    }

    #[test]
    fn ref_with_trailing_semicolon() {
        check_ref("OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2;", None);
    }

    #[test]
    fn ref_with_trailing_space() {
        check_ref("OpaqueRef:5fe71a7a-2d59-af9c-0ddb-89f1324b52d2 ", None);
    }
}
