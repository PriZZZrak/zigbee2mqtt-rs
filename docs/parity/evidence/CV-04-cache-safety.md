# CV-04 — converter cache path safety and download bound

Rust implementation: `src/converters/external.rs` (`cache_path`,
`MAX_FETCH_BYTES`, `load_js`/`fetch_js`). Behavior: CONVERTER-014; plan CV-04.

## Risk

Registry `lookup`/`fingerprints` metadata is fetched from a remote Pages site.
The previous `fetch_js`/`load_js` joined its `file` field directly onto the cache
directory, so a malicious or corrupted registry containing `../../etc/...` or an
absolute path could read or write outside
`/etc/zigbee2mqtt-rs/herdsman-converters`. Downloads were also unbounded.

## Change

- `cache_path` rejects empty names, backslashes, absolute paths, root/prefix
  components and any `..` component.
- `load_js` and `fetch_js` resolve through `cache_path` before reading or
  writing, so traversal is rejected before any network request or filesystem
  operation.
- `fetch_url` refuses a response larger than 8 MiB (per file) instead of writing
  it into the cache. This is a coarse bound; it still buffers the command output.
- Existing on-demand dependency traversal continues to use the same validated
  path function.

## Tests added

- `cache_path_rejects_traversal_and_absolute_paths`: `../`, `devices/../../`,
  absolute path, backslash, empty; a valid nested path stays under the cache.
- `fetch_js_rejects_traversal_before_network`: invalid name returns `None`
  without a fetch attempt.

## Remaining gaps (not verified)

- The 8 MiB bound is not unit-tested; it requires a controlled large response.
- Downloads still use `curl`/`wget`/`uclient-fetch` subprocesses with no explicit
  connect/total timeout, no digest/signature verification and no ETag; versioned
  manifest verification and atomic promotion (CV-04) remain open.
- A malicious registry can still point at an allowed but wrong file name inside
  the cache; content authenticity needs the manifest/digest work.
- No size/closed-cache behavior on the 54 MB router storage has been measured.
