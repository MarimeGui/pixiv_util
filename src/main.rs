mod api_calls;
mod gen_http_client;
mod tasks;

use anyhow::Result;
use clap::Parser;
use gen_http_client::{make_client, make_headers};

use crate::tasks::{dl_illust, dl_series};

// Print JSON option ?
// All posts from a user with specific tags ?
// When downloading multiple illusts with multiple pages, choose between all files in the same folder or subfolders with names or ID
// Move incremental and cookie to subcommands
// Progress indicator showing dl speed, complete/remaining VS verbose showing all files downloaded
// Novel and ugoira dl

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    subcommand: ModeSubcommands,
}

#[derive(Parser, Debug)]
enum ModeSubcommands {
    /// Configure a user cookie to use for accessing limited content
    #[command(subcommand)]
    Cookie(CookieSubcommands),
    /// Download some illusts
    Download {
        /// Use this cookie instead of the pre-configured one (if any)
        #[arg(short, long)]
        cookie_override: Option<String>,
        /// Check if the destination folder already has some of the illusts that are about to be downloaded and if so, don't download them again
        #[arg(short, long, default_value_t = false)]
        incremental: bool,
        #[command(subcommand)]
        mode: DownloadModesSubcommands,
    },
    /// Find all illusts on disk that haven't been liked
    FindNotLiked {
        /// If an illust is now publicly unavailable, don't list it in the output. Disabled by default as this makes this request a lot more expensive
        #[arg(short, long, default_value_t = false)]
        ignore_missing: bool,
    },
}

#[derive(Parser, Debug)]
enum DownloadModesSubcommands {
    /// Download a single illust
    Illust {
        /// ID of the illust to download
        illust_id: u64,
    },
    /// Download a series
    Series { series_id: u64 },
    /// Download all posts from a user
    UserPosts { user_id: u64 },
    /// Download all posts liked by a user
    UserLikes { user_id: u64 },
}

#[derive(Parser, Debug)]
enum CookieSubcommands {
    /// Set a cookie
    Set { cookie: String },
    /// Get the cookie
    Get,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.subcommand {
        ModeSubcommands::Download {
            cookie_override,
            incremental,
            mode,
        } => {
            let client = make_client(make_headers(cookie_override.as_deref())?)?;
            match mode {
                DownloadModesSubcommands::Illust { illust_id } => {
                    println!("Downloading illust ID {}", illust_id);
                    dl_illust(&client, illust_id).await?;
                }
                DownloadModesSubcommands::Series { series_id } => {
                    println!("Downloading series ID {}", series_id);
                    dl_series(&client, series_id).await?;
                }
                _ => unimplemented!("Can only download single illusts for now"),
            }
        }
        _ => unimplemented!("Can only download for now"),
    }

    Ok(())
}
