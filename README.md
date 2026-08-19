# mstream-p2p-sidecar

The iroh networking companion process for [mStream](https://github.com/IrosTheBeggar/mStream)'s
music-discovery network. mStream is Node, but the rich iroh stack
(iroh-blobs verified transfers, iroh-gossip for the catalog) only exists in
Rust — this binary is the application-specific wrapper n0 recommends,
spawned as a long-running child by the server
(`src/state/discovery-p2p.js` in mStream) and speaking line-delimited
JSON-RPC on stdin/stdout. It exits when stdin closes, so a dying parent can
never leave an orphan. The full protocol is documented at the top of
[`src/main.rs`](src/main.rs).

## Provenance

Extracted 2026-08-19 from the mStream repo at commit
`61eca273845f0fcbf431ef2089f2647ecd20cfd8` (path `p2p-sidecar/`, tree
`8080df40efa897a65991f6f249471f46c288b195`). This repo starts with fresh
history; everything before the extraction lives in
[mStream's git history](https://github.com/IrosTheBeggar/mStream/commits/master/p2p-sidecar).

## Releases and how mStream consumes them

- **No binaries live in git — here or in mStream.** Pushing a `v*` tag makes
  CI build all nine platform binaries (glibc/darwin/windows +
  statically-linked musl for Alpine), self-test each (identity round-trip;
  qemu for the ARM musl legs; a real-run socket smoke on Linux), and attach
  them to a **draft release** for that tag, together with two machine-readable
  manifest fragments (`manifest-fragment.json`, `manifest-fragment-musl.json`)
  listing `{file, sha256, size}` per binary. A maintainer reviews and
  publishes the draft — CI never publishes.
- **Published release assets are immutable.** Never replace or delete one;
  fix problems by cutting a new version. mStream's committed manifests
  (`bin/p2p-sidecar/manifest*.json` there) pin `{repo, tag, file, sha256,
  size}` and the server verifies every downloaded byte against them, so a
  swapped asset fails closed.
- Assets are **not code-signed**. The signing-sensitive path (Windows
  bundles, Smart App Control) is covered by mStream's bundle build, which
  stages the sidecar and signs it along with everything else at bundle time;
  the runtime fetch is only used by npm/source/Docker installs, where the
  sha256 pins are the integrity story.

## Versioning policy

`MAJOR.MINOR` mirrors the pinned iroh line; `PATCH` is this sidecar's own.
So `v1.0.x` releases pin iroh `1.0.*` (see `Cargo.toml` — the iroh crates
are pinned exactly and bumped together, deliberately). A new iroh line means
at least a MINOR bump here.

## Compatibility contract (server ↔ sidecar)

- The server **capability-probes before using newer RPCs**: it issues the
  new command once and treats an `{"ok":false,"error":"unknown command: …"}`
  reply as "this sidecar predates the feature", degrading that feature only.
  Therefore: unknown commands MUST keep answering with an error response —
  never a crash, never silence.
- New unsolicited events are additive; consumers ignore events they don't
  know.
- A change that breaks existing commands, events, or the signed wire formats
  (announcements, holds beacons) requires at least a MINOR bump and a loud
  release note. The wire formats carry their own `v` field and reject
  unknown versions.

## Developing

```
cargo build --release
cargo test
```

To run your local build under a development mStream checkout, clone this
repo into it as `p2p-sidecar/` (the directory is gitignored there) and build
— mStream's binary resolution prefers `p2p-sidecar/target/release/` over
anything fetched or prebuilt:

```
cd <mstream-checkout>
git clone https://github.com/IrosTheBeggar/mstream-p2p-sidecar p2p-sidecar
cd p2p-sidecar && cargo build --release
```

`--print-id --data-dir <dir>` is the one-shot mode CI uses to self-test
(creates/loads an identity, prints the endpoint id, exits — no sockets).

## License

GPL-3.0, same as mStream.
