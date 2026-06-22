//! # content-type — parse and format `Content-Type` headers
//!
//! Parse an HTTP `Content-Type` / media-type header value into its type and
//! parameters, and format one back out, following RFC 9110. A faithful Rust port of
//! the [`content-type`](https://www.npmjs.com/package/content-type) npm package
//! (v2.0.0): a lenient [`parse`] and a strict, validating [`format`]. Zero
//! dependencies and `#![no_std]`.
//!
//! ```
//! use content_type::{parse, format, ContentType};
//!
//! let ct = parse("text/html; charset=utf-8");
//! assert_eq!(ct.type_, "text/html");
//! assert_eq!(ct.get_parameter("charset"), Some("utf-8"));
//!
//! let ct = ContentType::new("application/json").with_parameter("charset", "utf-8");
//! assert_eq!(format(&ct).unwrap(), "application/json; charset=utf-8");
//! ```

#![no_std]
#![doc(html_root_url = "https://docs.rs/content-type/0.1.0")]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

// Compile-test the README's examples as part of `cargo test`.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// A parsed `Content-Type`: a media type plus its parameters.
///
/// `type_` is the lower-cased media type (e.g. `text/html`). `parameters` are
/// `(lower-cased name, value)` pairs in header order; values keep their original case.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContentType {
    /// The media type, lower-cased (e.g. `text/html`).
    pub type_: String,
    /// The parameters as `(lower-cased name, value)` pairs, in order.
    pub parameters: Vec<(String, String)>,
}

impl ContentType {
    /// Create a `ContentType` with the given media type and no parameters.
    #[must_use]
    pub fn new(type_: impl Into<String>) -> Self {
        Self {
            type_: type_.into(),
            parameters: Vec::new(),
        }
    }

    /// Builder: add a parameter (consuming and returning `self`).
    #[must_use]
    pub fn with_parameter(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.push((name.into(), value.into()));
        self
    }

    /// Look up a parameter value by name, case-insensitively.
    ///
    /// Matches regardless of how the [`ContentType`] was built — [`parse`] stores
    /// lower-cased names, but a name added via [`with_parameter`](Self::with_parameter)
    /// keeps its case, so both the query and the stored name are lower-cased here.
    #[must_use]
    pub fn get_parameter(&self, name: &str) -> Option<&str> {
        let lname = lowercase(name);
        self.parameters
            .iter()
            .find(|(k, _)| lowercase(k) == lname)
            .map(|(_, v)| v.as_str())
    }
}

/// An error from [`format`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// The media type is empty or not a valid `type/subtype`.
    InvalidType(String),
    /// A parameter name is not a valid token.
    InvalidParameterName(String),
    /// A parameter value cannot be represented as a token or quoted string.
    InvalidParameterValue(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::InvalidType(t) => write!(f, "invalid type: {t}"),
            FormatError::InvalidParameterName(n) => write!(f, "invalid parameter name: {n}"),
            FormatError::InvalidParameterValue(v) => write!(f, "invalid parameter value: {v}"),
        }
    }
}

impl core::error::Error for FormatError {}

/// Parse a `Content-Type` header value (lenient: never errors).
///
/// The media type and parameter names are lower-cased; parameter values keep their
/// case. When a parameter name repeats, the first value wins. Quoted-string values
/// are unescaped.
///
/// ```
/// let ct = content_type::parse("Text/HTML; Charset=\"UTF-8\"");
/// assert_eq!(ct.type_, "text/html");
/// assert_eq!(ct.get_parameter("charset"), Some("UTF-8"));
/// ```
#[must_use]
pub fn parse(header: &str) -> ContentType {
    parse_with(header, true)
}

/// Parse a `Content-Type` header value, optionally skipping parameters.
///
/// With `parse_parameters = false`, only the media type is extracted (the reference's
/// `{ parameters: false }` option).
#[must_use]
pub fn parse_with(header: &str, parse_parameters: bool) -> ContentType {
    let chars: Vec<char> = header.chars().collect();
    let len = chars.len();

    let mut index = skip_ows(&chars, 0, len);
    let value_start = index;
    index = skip_value(&chars, index, len);
    let value_end = trailing_ows(&chars, value_start, index);
    let type_ = lowercase_slice(&chars[value_start..value_end]);

    let parameters = if parse_parameters {
        parse_parameters_impl(&chars, index, len)
    } else {
        Vec::new()
    };

    ContentType { type_, parameters }
}

/// Format a [`ContentType`] into a header string, validating the type and parameters.
///
/// Returns a [`FormatError`] for an invalid media type, parameter name, or value.
///
/// ```
/// use content_type::ContentType;
/// let ct = ContentType::new("text/plain").with_parameter("name", "two words");
/// assert_eq!(content_type::format(&ct).unwrap(), "text/plain; name=\"two words\"");
/// ```
///
/// # Errors
///
/// Returns [`FormatError`] if `type_` is not a valid `type/subtype`, a parameter name
/// is not a token, or a value cannot be encoded.
pub fn format(content_type: &ContentType) -> Result<String, FormatError> {
    if !is_type(&content_type.type_) {
        return Err(FormatError::InvalidType(content_type.type_.clone()));
    }
    let mut result = content_type.type_.clone();
    for (name, value) in &content_type.parameters {
        if !is_token(name) {
            return Err(FormatError::InvalidParameterName(name.clone()));
        }
        result.push_str("; ");
        result.push_str(name);
        result.push('=');
        result.push_str(&qstring(value)?);
    }
    Ok(result)
}

const SP: char = ' ';
const HTAB: char = '\t';
const SEMI: char = ';';
const EQ: char = '=';
const DQUOTE: char = '"';
const BSLASH: char = '\\';

fn parse_parameters_impl(chars: &[char], mut index: usize, len: usize) -> Vec<(String, String)> {
    let mut parameters: Vec<(String, String)> = Vec::new();

    'parameter: while index < len {
        index = skip_ows(chars, index + 1, len); // skip the ';' then OWS
        let key_start = index;
        while index < len {
            let code = chars[index];
            if code == SEMI {
                continue 'parameter;
            }
            if code == EQ {
                let key_end = trailing_ows(chars, key_start, index);
                let key = lowercase_slice(&chars[key_start..key_end]);
                index = skip_ows(chars, index + 1, len);

                if index < len && chars[index] == DQUOTE {
                    index += 1;
                    let mut value = String::new();
                    let mut closed = false;
                    while index < len {
                        let code = chars[index];
                        index += 1;
                        if code == DQUOTE {
                            index = skip_value(chars, index, len);
                            closed = true;
                            break;
                        }
                        if code == BSLASH && index < len {
                            value.push(chars[index]);
                            index += 1;
                            continue;
                        }
                        value.push(code);
                    }
                    // An unterminated quoted string is dropped (matching the reference).
                    if closed {
                        insert_first(&mut parameters, key, value);
                    }
                    continue 'parameter;
                }

                let value_start = index;
                index = skip_value(chars, index, len);
                let value_end = trailing_ows(chars, value_start, index);
                let value = chars[value_start..value_end].iter().collect();
                insert_first(&mut parameters, key, value);
                continue 'parameter;
            }
            index += 1;
        }
    }

    parameters
}

/// Store `(key, value)` only if `key` is not already present (first occurrence wins).
fn insert_first(parameters: &mut Vec<(String, String)>, key: String, value: String) {
    if !parameters.iter().any(|(k, _)| *k == key) {
        parameters.push((key, value));
    }
}

/// Advance past characters until a `;` or the end.
fn skip_value(chars: &[char], mut index: usize, len: usize) -> usize {
    while index < len && chars[index] != SEMI {
        index += 1;
    }
    index
}

/// Skip optional whitespace (SP / HTAB).
fn skip_ows(chars: &[char], mut index: usize, len: usize) -> usize {
    while index < len && (chars[index] == SP || chars[index] == HTAB) {
        index += 1;
    }
    index
}

/// Trim trailing optional whitespace (SP / HTAB) from `chars[start..end]`.
fn trailing_ows(chars: &[char], start: usize, mut end: usize) -> usize {
    while end > start && (chars[end - 1] == SP || chars[end - 1] == HTAB) {
        end -= 1;
    }
    end
}

fn lowercase(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

fn lowercase_slice(chars: &[char]) -> String {
    chars.iter().flat_map(|c| c.to_lowercase()).collect()
}

/// A token character per RFC 9110 (ASCII letters, digits, and the `tchar` symbols).
fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            '!' | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '*'
                | '+'
                | '.'
                | '^'
                | '_'
                | '`'
                | '|'
                | '~'
                | '-'
        )
}

/// Whether `s` is a non-empty token.
fn is_token(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_token_char)
}

/// Whether `s` is a valid `type/subtype` (token `/` token).
fn is_type(s: &str) -> bool {
    match s.split_once('/') {
        Some((ty, sub)) => is_token(ty) && is_token(sub),
        None => false,
    }
}

/// Whether `c` is allowed unquoted-or-quoted text per the reference's `TEXT_REGEXP`.
fn is_text_char(c: char) -> bool {
    let n = c as u32;
    n == 0x09 || (0x20..=0x7e).contains(&n) || (0x80..=0xff).contains(&n)
}

/// Serialize a parameter value: a bare token, or an escaped quoted string.
fn qstring(value: &str) -> Result<String, FormatError> {
    if is_token(value) {
        return Ok(value.to_string());
    }
    if value.chars().all(is_text_char) {
        let mut out = String::with_capacity(value.len() + 2);
        out.push(DQUOTE);
        for c in value.chars() {
            if c == BSLASH || c == DQUOTE {
                out.push(BSLASH);
            }
            out.push(c);
        }
        out.push(DQUOTE);
        return Ok(out);
    }
    Err(FormatError::InvalidParameterValue(value.to_string()))
}
