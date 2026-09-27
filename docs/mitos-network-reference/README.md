# mitos-network IPC reference

Two files copied verbatim from mitos-network's own `src/ipc/` (as of the
`mitos-network-main.zip` shared during this integration work) so the
actual schema `src/network/client.rs` is built against is sitting next to
the code that depends on it, not just cited from memory:

- `messages.rs` -- the real `Request`/`Response`/`Event`/`ServerMessage`
  definitions. This is what resolved the two mistakes the first
  integration pass made working only from `../network-integration.md`
  (mitos-network's own descriptive guide, still kept for the narrative
  "how to use this" context the raw schema doesn't carry): `BluetoothPower`'s
  field is `on`, not the guessed `powered`, and every reply is wrapped in
  `ServerMessage::Response(..)` rather than sent as a bare `Response`.
- `protocol.rs` -- the wire framing (4-byte little-endian length prefix +
  JSON) `network::json`/`network::client` implement by hand rather than
  pulling in serde_json, to stay consistent with this crate's
  dependency-free design.

These are a snapshot, not a live reference -- if mitos-network's schema
changes, this copy goes stale and `network::client` should be re-checked
against the real file again, the same way this pass re-checked the first
one.
