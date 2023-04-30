mod abstractions;
mod api_calls;
mod cookie_file;
mod download;
mod gen_http_client;
mod incremental;

use std::path::PathBuf;

use abstractions::{get_all_series_works, get_all_user_bookmarks};
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use cookie_file::{get_cookie_file_path, get_cookie_from_file, set_cookie_to_file};
use download::dl_illust;
use gen_http_client::{make_client, make_headers};
use incremental::is_illust_in_files;

use crate::incremental::list_all_files;

// Print JSON option ?
// All posts from a user with specific tags ?
// Name folder after series or illust name (requires maybe having a formatting string system)
// Move incremental and cookie to subcommands
// Progress indicator showing dl speed, complete/remaining VS verbose showing all files downloaded
// Novel and ugoira dl
// Automatically update cookie with server answers ?
// Check immediately if paths are correct
// Better, friendlier errors (like cookie get when no cookie is set)
// Ignore errors while downloading mode
// Try to immediately fail before initiating all tasks if an illust is unavailable for example
// Stream DLs to disk ?
// Make incremental matching a bit smarter

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
    /// Prints the path to the cookie file
    PrintPath,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.subcommand {
        ModeSubcommands::Cookie(s) => match s {
            CookieSubcommands::Get => println!("{}", get_cookie_from_file().await?),
            CookieSubcommands::Set { cookie } => set_cookie_to_file(&cookie).await?,
            CookieSubcommands::PrintPath => println!("{}", get_cookie_file_path()?.display()),
        },
        ModeSubcommands::Download {
            cookie_override,
            incremental,
            output_folder,
            folder_policy,
            mode,
        } => {
            // Get a cookie, if any
            let cookie = match cookie_override {
                Some(c) => Some(c),
                // TODO: This should be a bit smarter, like if the file is empty
                None => match get_cookie_from_file().await {
                    Ok(c) => Some(c),
                    Err(_) => None,
                },
            };

            // Make the HTTP client with correct headers
            let client = make_client(make_headers(cookie.as_deref())?)?;

            // If incremental is active, list all files
            let file_list = if let Some(p) = incremental {
                Some(list_all_files(p)?)
            } else {
                None
            };

            // Closure for initiating downloads
            let mut tasks = Vec::new();
            let mut f = |illust_id: u64| {
                // If this ID is already found among files, don't download it
                if let Some(l) = &file_list {
                    if is_illust_in_files(&illust_id.to_string(), l) {
                        return;
                    }
                }
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
