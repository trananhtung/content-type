# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-22

### Added

- Initial release.
- `parse` / `parse_with` — parse a `Content-Type` header into a `ContentType`
  (lenient; type and parameter names lower-cased, values case-preserved, first
  occurrence wins, quoted strings unescaped).
- `format` — serialize a `ContentType` to a header string, validating the media type
  and parameters and quoting values when needed.
- `ContentType` with `new`, `with_parameter`, and `get_parameter`, plus a `FormatError`.
- Faithful to the `content-type` npm package v2.0.0. Zero dependencies; `#![no_std]`.

[0.1.0]: https://github.com/trananhtung/content-type/releases/tag/v0.1.0
