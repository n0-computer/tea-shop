use std::str::FromStr;

use anyhow::Result;
use iroh::docs::DocTicket;

mod sync;

use crate::sync::async_watch;

/// Async, futures channel based event watching
#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("Argument 1 needs to be a path");

    let mut doc_ticket = None;
    if let Some(ticket) = std::env::args().nth(2) {
        let ticket = iroh::docs::DocTicket::from_str(ticket.as_str())?;
        doc_ticket = Some(ticket);
    }
    println!("watching {}", path);

    let path = std::path::PathBuf::from(&path);
    let iroh_path = path.join(".iroh");
    let node = iroh::node::Node::persistent(iroh_path)
        .await?
        .spawn()
        .await?;

    let doc = open_or_create_document(&node, doc_ticket).await?;
    let author = node.authors.create().await?;

    tokio::spawn(async_watch(path, doc, author)).await??;

    Ok(())
}

async fn open_or_create_document(
    node: &iroh::client::MemIroh,
    ticket: Option<DocTicket>,
) -> anyhow::Result<iroh::client::MemDoc> {
    let doc = match ticket {
        Some(ticket) => {
            println!("importing ticket: {:?}", ticket);
            node.docs.import(ticket).await?
        }
        None => {
            let doc = node.docs.create().await?;
            let ticket = doc
                .share(
                    iroh::client::docs::ShareMode::Write,
                    iroh::base::node_addr::AddrInfoOptions::Relay,
                )
                .await?;
            println!(
                "created new doc {:?} write ticket:\n{:?}",
                doc.id(),
                ticket.to_string()
            );
            doc
        }
    };
    Ok(doc)
}
