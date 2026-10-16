//! A tool to help understand XAPI logs.
//!
//! This project explains how XAPI writes its logs, and builds a tool to
//! read them. The documentation is written like a small book. Each chapter
//! explains one idea, with real lines from XAPI logs.
//!
//! The examples in the chapters are tests. When the code changes, they are
//! run again, so the book stays correct.
//!
//! # Chapters
//!
//! 1. [Refs in XAPI logs](crate::chapter_01_refs): what a ref is, and how a
//!    short ref like `R:5fe71a7a2d59` links *xensource.log* and *SMlog*.
//!
//! # Code
//!
//! - [`refs`]: converts short refs, full refs and uuids into keys.
//! - [`mod@line`]: splits a log line into its syslog part and its body.
//!
//! The XAPI source code is here: <https://github.com/xapi-project/xen-api>

#[doc = include_str!("../doc/01-refs.md")]
pub mod chapter_01_refs {}

pub mod line;
pub mod refs;
