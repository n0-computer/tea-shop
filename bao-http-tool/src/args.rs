use std::{fmt::Display, path::PathBuf, str::FromStr};

use bao_tree::blake3::Hash;
use clap::Parser;
use url::Url;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Debug, Parser)]
pub enum SubCommand {
    Validate(ValidateArgs),
    Generate(GenerateArgs),
}

#[derive(Debug, Parser)]
pub struct ValidateArgs {
    #[clap(long, help = "Hash of the data")]
    pub hash: Hash,

    #[clap(long, help = "URL or local path to the data")]
    pub data: PathOrUrl,

    #[clap(long, help = "URL or local path to the outboard")]
    pub outboard: PathOrUrl,

    #[clap(long, default_value_t = 0, help = "Block size in log2(bytes)")]
    pub block_size_log: u8,

    #[clap(long, help = "Range of the data to read")]
    pub range: Option<String>,
}

#[derive(Debug, Parser)]
pub struct GenerateArgs {
    #[clap(long, help = "URL to the data")]
    pub data: Url,

    #[clap(long, help = "path where to create the outboard")]
    pub target: Option<PathBuf>,

    #[clap(long, default_value_t = 0, help = "Block size in log2(bytes)")]
    pub block_size_log: u8,
}

#[derive(Debug, Clone)]
pub enum PathOrUrl {
    Path(std::path::PathBuf),
    Url(Url),
}

impl Display for PathOrUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathOrUrl::Path(path) => write!(f, "{}", path.display()),
            PathOrUrl::Url(url) => write!(f, "{}", url),
        }
    }
}

impl FromStr for PathOrUrl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(url) = Url::from_str(s) {
            Ok(PathOrUrl::Url(url))
        } else {
            Ok(PathOrUrl::Path(std::path::PathBuf::from(s)))
        }
    }
}
