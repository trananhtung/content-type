//! Behavioral spec for `content-type`, cross-checked against the npm package (v2).

use content_type::{format, parse, parse_with, ContentType, FormatError};

fn p(s: &str) -> (String, Vec<(String, String)>) {
    let ct = parse(s);
    (ct.type_, ct.parameters)
}

fn params(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect()
}

#[test]
fn parse_basic() {
    assert_eq!(
        p("text/html; charset=utf-8"),
        ("text/html".into(), params(&[("charset", "utf-8")]))
    );
    assert_eq!(p("image/png"), ("image/png".into(), vec![]));
    // type and parameter names are lower-cased; values keep their case
    assert_eq!(
        p("TEXT/HTML; Charset=UTF-8"),
        ("text/html".into(), params(&[("charset", "UTF-8")]))
    );
}

#[test]
fn parse_whitespace_and_quotes() {
    assert_eq!(
        p("text/html ; charset = utf-8 "),
        ("text/html".into(), params(&[("charset", "utf-8")]))
    );
    assert_eq!(
        p("multipart/form-data; boundary=\"--abc\""),
        (
            "multipart/form-data".into(),
            params(&[("boundary", "--abc")])
        )
    );
    // backslash unescaping inside a quoted string
    assert_eq!(
        p("text/html;x=\"a\\\"b\\\\c\""),
        ("text/html".into(), params(&[("x", "a\"b\\c")]))
    );
    // a quoted value may contain ';'
    assert_eq!(
        p("text/html; key=\"a;b\""),
        ("text/html".into(), params(&[("key", "a;b")]))
    );
    // trailing junk after the closing quote is skipped
    assert_eq!(
        p("a/b; p=\"v\"junk; q=2"),
        ("a/b".into(), params(&[("p", "v"), ("q", "2")]))
    );
}

#[test]
fn parse_edge_cases() {
    // first occurrence of a parameter wins
    assert_eq!(
        p("application/json;charset=utf-8;charset=latin1"),
        ("application/json".into(), params(&[("charset", "utf-8")]))
    );
    // a parameter with no '=' value is ignored; empty segments are ignored
    assert_eq!(p("text/html; charset"), ("text/html".into(), vec![]));
    assert_eq!(p("text/html;"), ("text/html".into(), vec![]));
    assert_eq!(p("x/y;;p=1"), ("x/y".into(), params(&[("p", "1")])));
    // lenient: no validation of the type
    assert_eq!(p("garbage"), ("garbage".into(), vec![]));
    assert_eq!(p(""), (String::new(), vec![]));
}

#[test]
fn parse_without_params() {
    let ct = parse_with("text/html; charset=utf-8", false);
    assert_eq!(ct.type_, "text/html");
    assert!(ct.parameters.is_empty());
}

#[test]
fn get_parameter_is_case_insensitive() {
    let ct = parse("text/html; charset=utf-8");
    assert_eq!(ct.get_parameter("CHARSET"), Some("utf-8"));
    assert_eq!(ct.get_parameter("missing"), None);
    // case-insensitive regardless of how the name was stored (hand-built, mixed case)
    let ct = ContentType::new("text/html").with_parameter("Charset", "utf-8");
    assert_eq!(ct.get_parameter("charset"), Some("utf-8"));
    assert_eq!(ct.get_parameter("CHARSET"), Some("utf-8"));
}

#[test]
fn format_basic() {
    assert_eq!(
        format(&parse("text/html; charset=utf-8")).unwrap(),
        "text/html; charset=utf-8"
    );
    assert_eq!(format(&ContentType::new("image/png")).unwrap(), "image/png");
    let ct = ContentType::new("text/html").with_parameter("x", "a\"b\\c");
    assert_eq!(format(&ct).unwrap(), "text/html; x=\"a\\\"b\\\\c\"");
    let ct = ContentType::new("text/html").with_parameter("boundary", "--ab cd");
    assert_eq!(format(&ct).unwrap(), "text/html; boundary=\"--ab cd\"");
}

#[test]
fn format_errors() {
    assert_eq!(
        format(&ContentType::new("BAD TYPE")),
        Err(FormatError::InvalidType("BAD TYPE".into()))
    );
    assert_eq!(
        format(&ContentType::new("")),
        Err(FormatError::InvalidType(String::new()))
    );
    let ct = ContentType::new("a/b").with_parameter("bad name", "x");
    assert_eq!(
        format(&ct),
        Err(FormatError::InvalidParameterName("bad name".into()))
    );
}

#[test]
fn round_trip() {
    let header = "multipart/form-data; boundary=\"--xyz 123\"; charset=utf-8";
    assert_eq!(format(&parse(header)).unwrap(), header);
}
