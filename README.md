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
So `v1.0.x` releases pin iroh `1.0.*` and `v1.4.x` releases pin iroh
`1.4.*` (see `Cargo.toml` — the iroh crates are pinned exactly and bumped
together, deliberately). A new iroh line means at least a MINOR bump here.

`iroh-gossip` is the exception to "crates.io, pinned exactly": since v1.4.0
it comes from a tag on the `IrosTheBeggar/iroh-gossip` fork that is upstream
`main` plus upstream PR #185, because the 0.101.0 release wedges the whole
network under a slow peer (the 2026-10-22 outage — the comment in
`Cargo.toml` has the full story). The tag is pinned like a version: never
track a branch. Re-pin to crates.io the moment a release contains #185.

## Liveness (v1.4.0+)

The gossip actor is a single task. If it stops turning, every topic on the
node goes silent while the QUIC links — and so the neighbor lists on both
sides — stay up, which is exactly what the 2026-10-22 outage looked like:
`status` kept reporting the last neighbor count for 22 hours and nothing
noticed. Three guards now:

- `status` reports the **live** neighbor set (`neighbors`, from events)
  plus `lastGossipAgeSecs` / `lastEventAgeSecs` (seconds since the last
  received message / any gossip event, `null` before the first),
  `subscriberRestarts` and `actorAlive`. A server can treat "joined,
  neighbors > 0, lastGossipAgeSecs in the tens of minutes" as a dead mesh.
- A dead subscriber is re-created. The gossip API *closes* a lagged
  subscriber; the old loop logged "lagged" and died with it. On
  re-subscribe the sidecar emits `{"event":"neighbors","reset":true}` and
  the actor replays a `neighbor` up event per current neighbor.
- An actor probe: while there are neighbors, a fresh subscribe must yield
  the actor's NeighborUp replay within 20 s. Two misses in a row and the
  process exits with **code 3** so the parent respawns it (mStream's crash
  recovery replays the whole stack; a seed's container restart policy does
  the same). Exit code 0 stays "clean" (stdin EOF / `shutdown`).

## Compatibility contract (server ↔ sidecar)

- The server **capability-probes before using newer RPCs**: it issues the
  new command once and treats an `{"ok":false,"error":"unknown command: …"}`
  reply as "this sidecar predates the feature", degrading that feature only.
  Therefore: unknown commands MUST keep answering with an error response —
  never a crash, never silence.
- New unsolicited events are additive; consumers ignore events they don't
  know.
- New OPTIONAL request params are additive too: the request parser tolerates
  unknown keys, so an older sidecar silently ignores a param it predates.
  That means a param can never be the only line of defense — the server
  keeps its own fallback until the pinned sidecar is new enough (e.g.
  `fetch.maxBytes`, added in v1.0.4: pre-1.0.4 sidecars download uncapped
  and the server's post-download size check does the rejecting).
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
