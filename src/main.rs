mod abstractions;
mod api_calls;
mod download;
mod gen_http_client;
mod incremental;

use std::path::PathBuf;

use abstractions::{get_all_series_works, get_all_user_bookmarks};
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use download::dl_illust;
use gen_http_client::{make_client, make_headers};

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
        #[arg(short, long, value_name = "COOKIE")]
        cookie_override: Option<String>,
        /// Check if a folder already has some of the illusts that are about to be downloaded and if so, don't download them again
        #[arg(short, long, value_name = "FOLDER")]
        incremental: Option<PathBuf>,
        /// Where the newly downloaded files will go
        #[arg(short, long)]
        output_folder: Option<PathBuf>,
        #[arg(short, long, value_enum, default_value_t = FolderPolicy::AlwaysCreate, value_name = "POLICY")]
        folder_policy: FolderPolicy,
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

#[derive(ValueEnum, Debug, Copy, Clone)]
pub enum FolderPolicy {
    /// In provided output_folder, always create a subfolder per illust (named with work ID) and put all images from this illust in it.
    AlwaysCreate,
    /// Always save all images directly to output_folder
    NeverCreate,
    /// If illust only contains one page, save directly to output_folder. Otherwise, create a subfolder. (Not recommended when downloading multiple illusts)
    Auto,
}

#[derive(Subcommand, Debug)]
enum DownloadModesSubcommands {
    /// Download a single illust
    Individual { illust_ids: Vec<u64> },
    /// Download a series
    Series { series_id: u64 },
    /// Download all posts from a user
    UserPosts { user_id: u64 },
    /// Download all posts liked/bookmarked by a user
    UserBookmarks { user_id: u64 },
}

#[derive(Subcommand, Debug)]
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
            output_folder,
            folder_policy,
            mode,
        } => {
            // Make the HTTP client with correct headers
            let client = make_client(make_headers(cookie_override.as_deref())?)?;

            // If incremental is active, list all files
            // ...

            // Closure for initiating downloads
            let mut tasks = Vec::new();
            let mut f = |illust_id: u64| {
                // TODO: Check if we already have this illust downloaded
                let client = client.clone();
                let output_folder = output_folder.clone();
                tasks.push(tokio::spawn(async move {
                    dl_illust(&client, illust_id, output_folder, folder_policy).await
                }));
            };

            // Run all tasks
            match mode {
                DownloadModesSubcommands::Individual { illust_ids } => {
                    for illust_id in illust_ids {
                        f(illust_id)
                    }
                }
                DownloadModesSubcommands::Series { series_id } => {
                    get_all_series_works(&client, series_id, f).await?
                }
                DownloadModesSubcommands::UserPosts { user_id } => {
                    unimplemented!()
                }
                DownloadModesSubcommands::UserBookmarks { user_id } => {
                    get_all_user_bookmarks(&client, user_id, f).await?;
                }
            }

            // Check if every download went okay
            for task in tasks {
                task.await??
            }
        }
        _ => unimplemented!("Can only download for now"),
    }

    Ok(())
}
