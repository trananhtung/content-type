# content-type

[![Crates.io](https://img.shields.io/crates/v/content-type.svg)](https://crates.io/crates/content-type)
[![Documentation](https://docs.rs/content-type/badge.svg)](https://docs.rs/content-type)
[![CI](https://github.com/trananhtung/content-type/actions/workflows/ci.yml/badge.svg)](https://github.com/trananhtung/content-type/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/content-type.svg)](#license)

**Parse and format HTTP `Content-Type` / media-type headers** (RFC 9110). A faithful
Rust port of the [`content-type`](https://www.npmjs.com/package/content-type) npm
package (v2.0.0): a lenient `parse` and a strict, validating `format`. Zero
dependencies and `#![no_std]`.

```rust
use content_type::{parse, format, ContentType};

let ct = parse("text/html; charset=utf-8");
assert_eq!(ct.type_, "text/html");
assert_eq!(ct.get_parameter("charset"), Some("utf-8"));

let ct = ContentType::new("application/json").with_parameter("charset", "utf-8");
assert_eq!(format(&ct).unwrap(), "application/json; charset=utf-8");
```

## Why content-type?

Reading and writing the `Content-Type` header is a daily HTTP task — pulling out the
media type and `charset`/`boundary`, or building one correctly (quoting parameter
values that need it). This is the lightweight, string-in/string-out port of the
canonical JS implementation. For a richly-typed media-type model, see the `mime`
crate; reach for this when you want the simple `{ type, parameters }` round-trip.

```toml
[dependencies]
content-type = "0.1"
```

## API

| Item | Purpose |
| --- | --- |
| `parse(header)` | Parse into a `ContentType` (lenient, never errors) |
| `parse_with(header, parse_parameters)` | …optionally skipping parameters |
| `format(&content_type)` | Serialize to a header string (validates) |
| `ContentType { type_, parameters }` | `new`, `with_parameter`, `get_parameter` |

## Behavior

- `parse` lower-cases the media type and parameter names; parameter values keep their
  case. When a name repeats, the first value wins. Quoted values are unescaped, and
  text after a closing quote (up to the next `;`) is ignored.
- `format` validates: the type must be `token/token`, parameter names must be tokens,
  and values are emitted bare when they are tokens or quoted (and escaped) otherwise —
  returning a `FormatError` if a value can't be represented.

## Differences from the npm package

Two differences exist only for inputs that don't occur in real `Content-Type` headers,
and both stem from JavaScript runtime behavior rather than the package's intent:

- **Parameter order.** Parameters are kept in header order. The JS package returns a
  plain object, and V8 iterates *integer-like* keys (e.g. `5`, `1`) first in ascending
  order ahead of other keys — so for the (unheard-of) case of numeric parameter names,
  iteration/format order can differ. The set of `(name, value)` pairs is always
  identical (first occurrence wins).
- **Unicode lower-casing.** The media type and parameter names are lower-cased with
  Rust's Unicode lower-casing, which can differ from JS for context-sensitive cases
  (Greek final sigma) in non-ASCII names. Standard ASCII headers are unaffected.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.
