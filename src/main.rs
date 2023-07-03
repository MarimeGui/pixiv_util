mod abstractions;
mod api_calls;
mod download;
mod find_not_bookmarked;
mod gen_http_client;
mod incremental;
mod user_mgmt;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use download::do_download_subcommand;
use find_not_bookmarked::do_fnb_subcommand;
use user_mgmt::do_users_subcommand;

// TODO: Print JSON option ?
// TODO: All posts from a user with specific tags ?
// TODO: Name folder after series or illust name (requires maybe having a formatting string system)
// TODO: Progress indicator showing dl speed, complete/remaining VS verbose showing all files downloaded
// TODO: Novel and ugoira dl
// TODO: Check immediately if paths are correct
// TODO: Better, friendlier errors (like cookie get when no cookie is set)
// TODO: Mode where we continue downloading even if there are errors
// TODO: Somehow immediately fail before initiating all tasks if an illust is unavailable for example
// TODO: If incremental is specified but without a path, use same path as output folder
// TODO: Some kind of double-clickable file for automatically downloading new images (part of a series for example)

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    subcommand: ModeSubcommands,
}

#[derive(Parser, Debug)]
enum ModeSubcommands {
    /// Configure users for accessing restricted content
    #[command(subcommand)]
    Users(UsersSubcommands),
    /// Download some illusts
    Download(DownloadParameters),
    /// Find all illusts on disk that haven't been bookmarked/liked
    FindNotBookmarked(FNBParameters),
}

#[derive(Subcommand, Debug)]
pub enum UsersSubcommands {
    /// Add a new user with their cookie, along with a name for identification
    AddUser { cookie: String, name: String },
    /// Remove a user
    RemoveUser { name: String },
    /// Print the cookie for a user
    PrintCookie { name: String },
    /// Sets a user as default when downloading
    SetDefault { name: String },
    /// Print the default user
    GetDefault,
    /// Set no default, i.e. specify which user to use everytime
    RemoveDefault,
    /// List all users
    ListUsers,
    /// Print the Pixiv ID for a user
    GetPixivID { name: String },
    /// Print the path to the database file for users
    PrintPath,
}

#[derive(Parser, Debug)]
pub struct DownloadParameters {
    /// Directly specify a cookie for use over everything else
    #[arg(short, long, value_name = "COOKIE")]
    cookie_override: Option<String>,
    /// Use a specific user for this download. If this isn't specified, the default user will be used.
    #[arg(short, long, value_name = "USER")]
    user_override: Option<String>,
    /// Check if a folder already has some of the illusts that are about to be downloaded and if so, don't download them again
    #[arg(short, long, value_name = "FOLDER")]
    incremental: Option<PathBuf>,
    /// Where the newly downloaded files will go
    #[arg(short, long)]
    output_folder: Option<PathBuf>,
    #[arg(short, long, value_enum, default_value_t = FolderPolicy::NeverCreate, value_name = "POLICY")]
    folder_policy: FolderPolicy,
    #[command(subcommand)]
    mode: DownloadModesSubcommands,
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
    /// Download all posts liked/bookmarked by a user. If the ID is not specified, will download the current user's
    UserBookmarks { user_id: Option<u64> },
}

#[derive(Parser, Debug)]
pub struct FNBParameters {
    /// ID of the user to check against
    user_id: u64,
    /// Folder containing the illusts
    folder: PathBuf,
    /// Use this cookie instead of the pre-configured one (if any)
    #[arg(short, long, value_name = "COOKIE")]
    cookie_override: Option<String>,
    /// If an illust is now unavailable, don't list it in the output. Disabled by default as this makes this request a lot more expensive
    #[arg(short, long, default_value_t = false)]
    ignore_missing: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.subcommand {
        ModeSubcommands::Users(s) => do_users_subcommand(s).await,
        ModeSubcommands::Download(p) => do_download_subcommand(p).await,
        ModeSubcommands::FindNotBookmarked(p) => do_fnb_subcommand(p).await,
    }
}
