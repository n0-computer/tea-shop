use std::{
    path::{Component, Path, PathBuf},
    sync::mpsc::channel,
};

use anyhow::Result;
use futures::StreamExt;
use iroh::docs::AuthorId;
use iroh::{client::MemDoc as Doc, docs::store::Query};
use jwalk::WalkDir;
use notify::{Config, Event, FsEventWatcher, RecommendedWatcher, RecursiveMode, Watcher};

// reconcile performs a full, one-time sync of the filesystem with the doc
async fn reconcile(doc: Doc, base_path: &str, author: AuthorId) -> Result<()> {
    // walk the filesystem
    for entry in WalkDir::new(base_path).sort(true) {
        let path = entry?.path();
        if path.is_file() {
            let relative = path.strip_prefix(base_path)?;
            let key = canonicalized_path_to_bytes(relative, true)?;

            let query = Query::single_latest_per_key()
                .key_exact(key.clone())
                .build();
            match doc.get_one(query).await? {
                Some(_) => {
                    // doc.update_file(path).await?;
                    // TODO - compare hashes and update if necessary
                }
                None => {
                    // doc.create_file(path).await?;
                    let abs_path = path.canonicalize()?;
                    doc.import_file(author, key, &abs_path, true)
                        .await?
                        .finish()
                        .await?;
                }
            }
        }
    }

    // walk the document
    let query = Query::single_latest_per_key().build();
    let mut res = doc.get_many(query).await?;
    while let Some(entry) = res.next().await {
        let entry = entry?;
        let key = String::from_utf8_lossy(entry.key()).to_string();
        let path = PathBuf::from(base_path).join(key);
        // TODO - add an empty entry check
        if !path.exists() {
            // doc.delete(key).await?;
            doc.export_file(entry, path, iroh::blobs::store::ExportMode::TryReference)
                .await?;
        }
    }

    Ok(())
}

// update applies incremental changes to the doc from the filesystem
pub(crate) async fn update(_doc: Doc, _event: Event) -> Result<()> {
    todo!("update");
    // let mut doc = Doc::from(doc);
    // let mut path = path;
    // let mut events = watchdir::watch(path.clone()).await?;
    // while let Some(event) = events.next().await {
    //     match event {
    //         watchdir::Event::Create(path) => {
    //             let path = path.strip_prefix(&path).unwrap();
    //             let path = path.to_str().unwrap();
    //             doc.create(path).await?;
    //         }
    //         watchdir::Event::Delete(path) => {
    //             let path = path.strip_prefix(&path).unwrap();
    //             let path = path.to_str().unwrap();
    //             doc.delete(path).await?;
    //         }
    //         watchdir::Event::Modify(path) => {
    //             let path = path.strip_prefix(&path).unwrap();
    //             let path = path.to_str().unwrap();
    //             doc.modify(path).await?;
    //         }
    //         watchdir::Event::Rename(old, new) => {
    //             let old = old.strip_prefix(&path).unwrap();
    //             let old = old.to_str().unwrap();
    //             let new = new.strip_prefix(&path).unwrap();
    //             let new = new.to_str().unwrap();
    //             doc.rename(old, new).await?;
    //         }
    //     }
    // }
    // Ok(())
}

async fn async_watcher() -> Result<(
    FsEventWatcher,
    std::sync::mpsc::Receiver<notify::Result<Event>>,
)> {
    // let (tx, rx) = tokio::sync::mpsc::channel(1000);
    let (tx, rx) = channel();
    let tx_2 = tx.clone();
    // // use the pollwatcher and set a callback for the scanning events
    // let watcher = PollWatcher::with_initial_scan(
    //     move |watch_event| {
    //         let tx_2 = tx_2.clone();
    //         // tx_2.blocking_send(Message::Event(watch_event)).unwrap();
    //         tokio::task::spawn(async move {
    //             println!("watch_event: {:?}", &watch_event);
    //             tx_2.send(Message::Event(watch_event)).unwrap();
    //         });
    //     },
    //     Config::default(),
    //     move |scan_event| {
    //         // tx_3.blocking_send(Message::Scan(scan_event));
    //         let tx_3 = tx_3.clone();
    //         println!("scan_event: {:?}", &scan_event);
    //         tokio::task::spawn(async move {
    //             if let Err(err) = tx_3.send(Message::Scan(scan_event)) {
    //                 println!("send error: {:?}", err);
    //             }
    //         });
    //     },
    // )?;

    // let (tx, rx) = std::sync::mpsc::channel();
    // let (mut tx, rx) = tokio::sync::mpsc::channel(1);

    // let tx_c = tx.clone();
    // // use the pollwatcher and set a callback for the scanning events
    // let mut watcher = PollWatcher::with_initial_scan(
    //     move |watch_event| {
    //         (|| async {
    //             tx_c.send(Message::Event(watch_event)).await.unwrap();
    //         });
    //         // tokio::task::spawn_blocking(move || async {
    //         //     tx_c.send(Message::Event(watch_event)).await.unwrap();
    //         // });
    //     },
    //     Config::default(),
    //     move |scan_event| {
    //         tokio::task::block_in_place(|| async {
    //             tx.send(Message::Scan(scan_event)).await.unwrap();
    //         });
    //     },
    // )?;

    // // Automatically select the best implementation for your platform.
    // // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |res| {
            let tx = tx_2.clone();
            tokio::task::spawn(async move {
                tx.send(res).unwrap();
            });
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

pub async fn async_watch<P: AsRef<Path>>(
    path: P,
    doc: iroh::client::MemDoc,
    author: iroh::docs::AuthorId,
) -> anyhow::Result<()> {
    let (mut watcher, rx) = async_watcher().await?;

    // start with a reconciliation
    reconcile(doc.clone(), path.as_ref().to_str().unwrap(), author).await?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    loop {
        let res = rx.recv()?;
        match res {
            Ok(event) => {
                println!("event: {:?}", event);
                // update(doc.clone(), event).await?;
                println!("event: {:?}", event);
                for path in event.paths {
                    let path = path.canonicalize()?;

                    // skip .iroh directory
                    if path
                        .components()
                        .any(|c| c == Component::Normal(".iroh".as_ref()))
                    {
                        continue;
                    }

                    if path.is_file() {
                        // let key = canonicalized_path_to_bytes(&path, true)?;
                        let key = bytes::Bytes::from(path.display().to_string());
                        doc.import_file(author, key, &path, true)
                            .await?
                            .finish()
                            .await?;
                    }
                }
            }
            Err(e) => {
                println!("watch error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

/// This function converts an already canonicalized path to a string.
///
/// If `must_be_relative` is true, the function will fail if any component of the path is
/// `Component::RootDir`
///
/// This function will also fail if the path is non canonical, i.e. contains
/// `..` or `.`, or if the path components contain any windows or unix path
/// separators.
fn canonicalized_path_to_bytes(
    path: impl AsRef<Path>,
    must_be_relative: bool,
) -> anyhow::Result<bytes::Bytes> {
    let mut path_str = String::new();
    let parts = path
        .as_ref()
        .components()
        .filter_map(|c| match c {
            Component::Normal(x) => {
                let c = match x.to_str() {
                    Some(c) => c,
                    None => return Some(Err(anyhow::anyhow!("invalid character in path"))),
                };

                if !c.contains('/') && !c.contains('\\') {
                    Some(Ok(c))
                } else {
                    Some(Err(anyhow::anyhow!("invalid path component {:?}", c)))
                }
            }
            Component::RootDir => {
                if must_be_relative {
                    Some(Err(anyhow::anyhow!("invalid path component {:?}", c)))
                } else {
                    path_str.push('/');
                    None
                }
            }
            _ => Some(Err(anyhow::anyhow!("invalid path component {:?}", c))),
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let parts = parts.join("/");
    path_str.push_str(&parts);
    let path_bytes = bytes::Bytes::from(path_str);
    Ok(path_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reconcile() -> Result<()> {
        // TODO: Add test cases for the reconcile function
        let dir = tempfile::tempdir()?;
        let node = iroh::node::Node::memory().spawn().await?;
        let doc = node.docs.create().await?;
        let author = node.authors.create().await?;

        doc.set_bytes(author, "test/from/doc.txt", bytes::Bytes::from("hello"))
            .await?;

        tokio::fs::write(dir.path().join("local_path"), b"hello from fs").await?;
        reconcile(doc.clone(), dir.path().to_str().unwrap(), author).await?;

        assert_doc_keys(doc.clone(), vec!["local_path", "test/from/doc.txt"]).await?;

        Ok(())
    }

    async fn assert_doc_keys(doc: Doc, keys: Vec<&str>) -> Result<()> {
        let query = Query::single_latest_per_key().build();
        let mut res = doc.get_many(query).await?;
        let mut doc_keys = vec![];
        while let Some(entry) = res.next().await {
            let entry = entry?;
            let key = String::from_utf8_lossy(entry.key()).to_string();
            doc_keys.push(key);
        }
        assert_eq!(doc_keys, keys);
        Ok(())
    }
}
