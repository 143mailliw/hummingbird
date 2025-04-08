// `scan2` is/will be a work-in-progress asynchronous thread-safe library scanner designed to
// eliminate I/O bottlenecks while reducing per-thread memory usage by taking advantage of GPUI's
// built-in async runtime.
//
// Eventually, this implementation will be made available through an environment variable once it
// lands in master, then (once it is proven functional), replace the `scan` module completely.
//
// !> In order for `scan2` to be thread safe, the SQLite database must be opened in "serialized"
// !> mode (see https://www.sqlite.org/threadsafe.html) - the implementation will assume that
// !> queries are executed in seqeuential order in order to reduce synchronization complexity.
//
// `scan2` will also solve the data deduplication problem by locking albums behind folder/disc
// pairs, and optionally using MBIDs if available.
use async_std::fs::read_dir;
use async_std::path::PathBuf;
use async_std::stream::StreamExt;
use futures::future::join_all;
use tracing::warn;

async fn scan(path: PathBuf) -> anyhow::Result<()> {
    let mut futures: Vec<_> = Vec::new();

    if path.exists().await {
        let mut children = read_dir(&path).await?;

        while let Some(child) = children.next().await {
            let considered_path = child?.path();

            if considered_path.is_dir().await {
                let scan = scan(considered_path);

                futures.push(Box::pin(scan));
            }
        }
    }

    let results = join_all(futures).await;

    for result in results {
        if let anyhow::Result::Err(e) = result {
            warn!("Error occured during scanning directory {:?}: {}", path, e);
        }
    }

    Ok(())
}
