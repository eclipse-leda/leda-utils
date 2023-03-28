use notify::{Config, PollWatcher, RecursiveMode, Watcher};
use std::future::Future;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub use notify::Event;
use tokio::sync::mpsc::{channel, Receiver};

const POLL_SECONDS: f64 = 2.0;

/// Directly taken from the notify async watcher example with slight modifications
fn async_watcher() -> notify::Result<(PollWatcher, Receiver<notify::Result<Event>>)> {
    let (tx, rx) = channel(1);

    let config = Config::default()
        .with_poll_interval(Duration::from_secs_f64(POLL_SECONDS))
        .with_compare_contents(true);

    let rt = tokio::runtime::Runtime::new().unwrap();

    let watcher = PollWatcher::new(
        move |res| {
            rt.block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        config,
    )?;
    Ok((watcher, rx))
}

pub async fn async_watch<'a, P, F, Fut>(path: P, callback: F) -> notify::Result<()>
where
    P: AsRef<Path>,
    F: Fn(Event) -> Fut,
    Fut: Future<Output = ()>,
{
    let (mut watcher, mut rx) = async_watcher()?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    while let Some(event) = rx.recv().await {
        callback(event?).await
    }

    Ok(())
}

pub fn is_filetype(path: &PathBuf, extension: &str) -> bool {
    if path.extension().is_none() {
        return false;
    }

    if path.extension().unwrap() == extension {
        return true;
    }

    return false;
}
