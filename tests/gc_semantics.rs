// Pins the iroh-blobs GC semantics the fetch handler's hold-tagging relies
// on (see the `hold` field on Request in src/main.rs). The GC's mark phase
// roots ONLY named tags + temp tags (+ the add_protected callback, which
// this sidecar doesn't use) and the live-set is rebuilt from scratch every
// cycle — there is no grace period and no memory of past roots. So:
//
//   - a blob imported WITHOUT a tag (exactly what a bare download used to
//     leave behind) is deleted on the next sweep;
//   - a named tag keeps a blob alive across sweeps — publish's auto tag and
//     fetch's `held-<hash>` tag both rely on this;
//   - deleting the tag (what `forget` does, by hash value) releases the
//     blob to the very next sweep.
//
// If an iroh-blobs bump ever changes any of these three facts, this test
// fails and the hold/forget design needs re-checking before the bump lands.

use std::time::Duration;

use iroh_blobs::store::fs::{options::Options as StoreOptions, FsStore};
use iroh_blobs::store::GcConfig;

// GC "sleeps interval, then marks+sweeps" in a loop; three cycles of margin
// keeps this robust on slow CI runners without dragging the test out.
const GC_INTERVAL: Duration = Duration::from_millis(250);
const SETTLE: Duration = Duration::from_millis(1500);

#[tokio::test]
async fn gc_reclaims_untagged_and_keeps_tagged() -> anyhow::Result<()> {
    let dir = std::env::temp_dir().join(format!("p2p-sidecar-gc-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir); // stale state from a crashed prior run
    std::fs::create_dir_all(&dir)?;

    // Same store construction as src/main.rs, faster GC clock.
    let blobs_root = dir.join("blobs");
    let mut opts = StoreOptions::new(&blobs_root);
    opts.gc = Some(GcConfig { interval: GC_INTERVAL, add_protected: None });
    let store = FsStore::load_with_opts(blobs_root.join("blobs.db"), opts).await?;

    // 1. Untagged: import through a temp tag and drop it — the exact
    //    rootless state a pre-`hold` download left behind.
    let tt = store.blobs().add_slice(b"fetched but never rooted").temp_tag().await?;
    let untagged = tt.hash();
    drop(tt);

    // 2. Publish-style: add_slice's plain await creates an auto named tag.
    let published = store.blobs().add_slice(b"published with an auto tag").await?;

    // 3. Fetch-with-hold-style: rootless import, then the explicit
    //    `held-<hash>` tag the fetch handler sets.
    let tt = store.blobs().add_slice(b"fetched then held").temp_tag().await?;
    let held = tt.hash();
    store.tags().set(format!("held-{held}"), held).await?;
    drop(tt);

    assert!(store.blobs().has(untagged).await?, "imported blob starts present");

    tokio::time::sleep(SETTLE).await;
    assert!(
        !store.blobs().has(untagged).await?,
        "an untagged blob must be swept by the next GC cycles"
    );
    assert!(
        store.blobs().has(published.hash).await?,
        "an auto-tagged (publish) blob must survive GC"
    );
    assert!(
        store.blobs().has(held).await?,
        "a held-tagged (fetch hold:true) blob must survive GC"
    );

    // `forget` deletes tags by hash value; deleting the hold tag must
    // release the blob to the sweeper.
    store.tags().delete(format!("held-{held}")).await?;
    tokio::time::sleep(SETTLE).await;
    assert!(
        !store.blobs().has(held).await?,
        "deleting the hold tag must release the blob to GC"
    );

    store.shutdown().await?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
