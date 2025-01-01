use std::io::Cursor;

use bao_tree::{
    io::{
        fsm::{encode_ranges_validated, outboard_post_order},
        outboard::PreOrderOutboard,
        round_up_to_chunks,
    },
    BaoTree, BlockSize, ByteRanges,
};
use clap::Parser;
use iroh_io::{AsyncSliceReader, HttpAdapter};
use tokio_util::either::Either;
mod args;
mod outboard;
use args::{GenerateArgs, PathOrUrl, SubCommand, ValidateArgs};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::args::Args;

type Reader = Either<iroh_io::HttpAdapter, iroh_io::File>;

async fn open(path: PathOrUrl) -> anyhow::Result<Reader> {
    Ok(match path {
        PathOrUrl::Url(url) => Either::Left(iroh_io::HttpAdapter::new(url)),
        PathOrUrl::Path(path) => Either::Right(iroh_io::File::open(path).await?),
    })
}

async fn generate(args: GenerateArgs) -> anyhow::Result<()> {
    let mut outboard = Vec::new();
    let mut data = HttpAdapter::new(args.data.clone());
    let block_size = BlockSize::from_chunk_log(args.block_size_log);
    let size = data.size().await?;
    let tree = BaoTree::new(size, block_size);
    println!(
        "Computing outboard for {} of size {} with block log {}, size {}",
        args.data,
        size,
        block_size.chunk_log(),
        block_size.bytes()
    );
    let hash = outboard_post_order(Cursor::new(data), tree, &mut outboard).await?;
    println!("Computed hash: {}", hash.to_hex());
    let filename = args
        .target
        .unwrap_or_else(|| format!("{}.obao{}", hash.to_hex(), block_size.chunk_log()).into());
    println!("Writing outboard to {}", filename.display());
    tokio::fs::write(filename, outboard).await?;
    Ok(())
}

async fn validate(args: ValidateArgs) -> anyhow::Result<()> {
    let mut data = open(args.data.clone()).await?;
    let mut outboard = open(args.outboard.clone()).await?;
    let size = data.size().await?;
    let outboard_size = outboard.size().await?;
    let byte_ranges = if let Some(range) = args
        .range
        .as_ref()
        .map(|x| x.split("..").collect::<Vec<_>>())
    {
        if range.len() != 2 {
            anyhow::bail!("Invalid range");
        }
        let start: u64 = range[0].parse()?;
        let end: u64 = range[1].parse()?;
        ByteRanges::from(start..end)
    } else {
        ByteRanges::all()
    };
    let chunk_ranges = round_up_to_chunks(&byte_ranges);
    // let size = outboard.read_at(0, 8).await?;
    // let size = u64::from_le_bytes(size.as_ref().try_into()?);
    let block_size = BlockSize::from_chunk_log(args.block_size_log);
    println!("Size: {}", size);
    println!("Outboard: {}", outboard_size);
    println!("Hash: {}", args.hash.to_hex());
    println!("Block size: {}", block_size);
    println!("Byte ranges: {:?}", byte_ranges);
    println!("Chunk ranges: {:?}", chunk_ranges);
    let tree = BaoTree::new(size, block_size);
    let outboard = PreOrderOutboard {
        root: args.hash,
        data: outboard,
        tree,
    };
    let encoded = Vec::new();
    encode_ranges_validated(data, outboard, &chunk_ranges, encoded).await?;
    println!(
        "confirmed that range {:?} of {} matches {}",
        byte_ranges,
        args.data,
        args.hash.to_hex()
    );
    Ok(())
}

fn setup_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();
    let args = Args::parse();
    match args.subcommand {
        SubCommand::Validate(args) => validate(args).await?,
        SubCommand::Generate(args) => generate(args).await?,
    }
    Ok(())
}
