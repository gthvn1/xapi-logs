//! The syslog layer of a log line.
//!
//! XAPI does not write its log files itself. It sends each message to
//! syslog, and syslog writes the file (xensource.log, SMlog, ...).
//! So a line has two layers:
//!
//! ```text
//! Mar 23 18:31:11 xcp-gtn-ip11 xapi: [debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit] ...
//! ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//!          syslog                                 body, written by the daemon
//! ```
//! - Syslog adds the time, the host and the daemon name.
//! - The daemon writes the body. The XAPI daemons (xapi, xenopsd-xc, xcp-networkd, ...)
//!   all use the same body format. SM is written in Python and uses another one.
//!
//! In ocaml/libs/log/debug.ml, `output_log` builds the body with `format`
//! and gives it to `Syslog.log`. Three things to know:
//!
//! - It calls `format false`, so the body has no time. With the default syslog format,
//!   it has no year, only seconds.
//! - It calls `escape` on the body. New lines and non-ASCII characters are escaped,
//!   so one message is always one line.
//! - XAPI does not call `openlog` (see ocaml/libs/log/syslog.ml), so syslog uses
//!   the program name as the daemon name.

#[allow(unused)]
#[derive(Debug, PartialEq)]
pub struct XapiLog<'a> {
    priority: &'a str,
    remote_ctx: &'a str,
    id: u64,
    name: &'a str,
    task: &'a str,
    brand: &'a str,
    message: &'a str,
}

/// A log line split into its syslog part and its body.
///
/// All fields point into the original line. Nothing is copied.
#[derive(Debug, PartialEq)]
pub struct Line<'a> {
    /// `Mar 23 18:31:11`. Default is 15 characters, with no year.
    pub time: &'a str,
    /// The host that wrote the line.
    pub host: &'a str,
    /// The program name, without the ':'. For example `xapi`, `SM`.
    pub daemon: &'a str,
    /// Everything after `daemon: `. Its format depends on the daemon.
    pub body: &'a str,
}

/// Splits a log line into its syslog part and its body.
///
/// The format is:
///
/// ```text
/// time host daemon: body
/// ```
///
/// - `time` has two formats. The default syslog format is 15 characters. Syslog pads a
///   single-digit day with a space: `Mar  3 09:03:01`. If syslog is set to high precision
///   (RFC 3339), it looks like `2026-03-23T18:31:11.123456+01:00` and ends at the first
///   space.
/// - `host` has no spaces.
/// - `daemon` has no spaces and ends with ':'.
/// - `body` is everything after `": "`. It is not parsed here.
///
/// It works for every log written by syslog: xensource.log, SMlog and also
/// the "message repeated N times" lines of syslog itself.
///
/// # Examples
///
/// ```
/// use xapi_logs::line::parse_line;
///
/// let line = parse_line("Mar 23 18:30:29 xcp-gtn-ip11 SM: [181066][MainThread] lock: closed").unwrap();
/// assert_eq!(line.daemon, "SM");
/// assert_eq!(line.body, "[181066][MainThread] lock: closed");
/// ```
pub fn parse_line(line: &str) -> Option<Line<'_>> {
    // If the log line starts by a digit (e.g. 2026-...) it is high precision otherwise
    // it is the default.
    let first_char: char = line.chars().next()?;
    let (time, rest) = if first_char.is_ascii_digit() {
        line.split_once(' ')?
    } else {
        // Traditional format is 15 chars
        line.split_at_checked(15)?
    };

    let (host, rest) = rest.trim_start().split_once(' ')?;
    let (daemon, body) = rest.split_once(' ')?;

    Some(Line {
        time,
        host,
        daemon: daemon.strip_suffix(':')?, // Daemon ends with ':' otherwise it is an error
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xapi_line() {
        let l = "Mar 23 18:31:11 xcp-gtn-ip11 xapi: [debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit] VM.migrate_send: VM = '2c422017-4cf9-a73a-dae6-b8475f82eeb8 (gtn alpine 11.1)'";
        assert_eq!(
            parse_line(l),
            Some(Line {
                time: "Mar 23 18:31:11",
                host: "xcp-gtn-ip11",
                daemon: "xapi",
                body: "[debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit] VM.migrate_send: VM = '2c422017-4cf9-a73a-dae6-b8475f82eeb8 (gtn alpine 11.1)'",
            })
        );
    }

    // SMlog: another body format, but the same syslog layer
    #[test]
    fn smlog_line() {
        let l = "Mar 23 18:30:29 xcp-gtn-ip11 SM: [181066][MainThread] lock: opening lock file /var/lock/sm/aa6f46c0-6b17-ac04-bdc2-0bc63589920b/sr";
        assert_eq!(
            parse_line(l),
            Some(Line {
                time: "Mar 23 18:30:29",
                host: "xcp-gtn-ip11",
                daemon: "SM",
                body: "[181066][MainThread] lock: opening lock file /var/lock/sm/aa6f46c0-6b17-ac04-bdc2-0bc63589920b/sr",
            })
        );
    }

    // syslog deduplication: valid syslog, the body is not a xapi log
    // (customer log, anonymized)
    #[test]
    fn message_repeated() {
        let l = "Jun 29 11:30:19 host01 xenopsd-xc: message repeated 5 times: [ [debug||4 ||xenops] Watch on backend domid: 0 kind: vif -> frontend domid: 42 devid: 0]";
        let line = parse_line(l).expect("should parse");
        assert_eq!(line.daemon, "xenopsd-xc");
        assert!(line.body.starts_with("message repeated 5 times: "));
    }

    // syslog pads single-digit days with a space (made up: not in our logs)
    #[test]
    fn single_digit_day() {
        let l = "Mar  3 09:05:01 xcp-gtn-ip11 xapi: [debug||1 ||xapi] hello";
        let line = parse_line(l).expect("should parse");
        assert_eq!(line.time, "Mar  3 09:05:01");
        assert_eq!(line.host, "xcp-gtn-ip11");
    }

    #[test]
    fn empty_line() {
        assert_eq!(parse_line(""), None);
    }

    // a file cut in the middle of the syslog part
    #[test]
    fn truncated_before_daemon() {
        assert_eq!(parse_line("Mar 23 18:29:20 xcp-gtn-ip11"), None);
    }

    // the daemon must end with ':'
    #[test]
    fn daemon_without_colon() {
        let l = "Mar 23 18:29:20 xcp-gtn-ip11 xapi [debug||1 ||xapi] hello";
        assert_eq!(parse_line(l), None);
    }

    // the daemon is the word before ':', not everything up to the first ':'
    #[test]
    fn daemon_with_colon_later_in_message() {
        let l = "Mar 23 18:29:20 xcp-gtn-ip11 xapi [debug||1 ||xapi] VM.migrate_send: x";
        assert_eq!(parse_line(l), None);
    }

    // must return None, and must not panic: 'é' is 2 bytes and sits across byte 15
    #[test]
    fn non_ascii_in_time() {
        let l = "Mar 23 18:29:2é xcp-gtn-ip11 xapi: x";
        assert_eq!(parse_line(l), None);
    }

    // rsyslog high precision format (RSYSLOG_FileFormat): made up, not from a real host
    #[test]
    fn rfc3339_time() {
        let l = "2026-03-23T18:31:11.123456+01:00 xcp-gtn-ip11 xapi: [debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit] VM.migrate_send";
        assert_eq!(
            parse_line(l),
            Some(Line {
                time: "2026-03-23T18:31:11.123456+01:00",
                host: "xcp-gtn-ip11",
                daemon: "xapi",
                body: "[debug||831 /var/lib/xcp/xapi|VM.migrate_send R:5fe71a7a2d59|audit] VM.migrate_send",
            })
        );
    }

    // milliseconds only: the length is not fixed in this format
    #[test]
    fn rfc3339_time_milliseconds() {
        let l = "2026-03-23T18:31:11.123+01:00 xcp-gtn-ip11 xapi: [debug||1 ||xapi] hello";
        let line = parse_line(l).expect("should parse");
        assert_eq!(line.time, "2026-03-23T18:31:11.123+01:00");
        assert_eq!(line.host, "xcp-gtn-ip11");
    }

    // UTC is written with 'Z'
    #[test]
    fn rfc3339_time_utc() {
        let l = "2026-03-23T17:31:11.123456Z xcp-gtn-ip11 xapi: [debug||1 ||xapi] hello";
        let line = parse_line(l).expect("should parse");
        assert_eq!(line.time, "2026-03-23T17:31:11.123456Z");
    }
}
