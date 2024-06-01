mod abstractions;
mod api_calls;
mod download;
mod find_not_bookmarked;
mod gen_http_client;
mod incremental;
mod parsers;
mod update_file;
mod user_mgmt;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use download::do_download_subcommand;
use find_not_bookmarked::do_fnb_subcommand;
use serde::{Deserialize, Serialize};
use update_file::do_create_update_file_subcommand;
use user_mgmt::do_users_subcommand;

use parsers::*;

// -----

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
enum Args {
    /// Configure users for accessing restricted content
    #[command(subcommand)]
    Users(UsersSubcommands),
    /// Download something !
    Download(DownloadParameters),
    /// Find all illusts on disk that haven't been bookmarked/liked
    FindNotBookmarked(FNBParameters),
    /// Creates an update file if necessary. One should be created automatically when downloading normally
    CreateUpdateFile(CreateUpdateFileParameters),
}

// -----

#[derive(Subcommand, Debug)]
pub enum UsersSubcommands {
    /// Add a new (or update) user with their cookie, along with a name for identification
    AddUser {
        /// Cookie for this user. Can be pulled from a web browser-based session with dev tools
        #[arg(value_parser = sanitize_cookie)]
        cookie: String,
        /// Name for identifying this user locally. It does not have to be the same as on the website, this is just for this program
        username: String,
    },
    /// Remove a user
    RemoveUser { username: String },
    /// Print the cookie for a user
    PrintCookie { username: String },
    /// Sets a user as default when downloading
    SetDefault { username: String },
    /// Print the default user
    GetDefault,
    /// Set no default, i.e. specify user everytime
    RemoveDefault,
    /// List all users
    ListUsers,
    /// Print the Pixiv ID of a user
    GetPixivID { username: String },
    /// Print the path of the database file
    PrintPath,
}

// -----

#[derive(Parser, Debug)]
pub struct DownloadParameters {
    /// Directly specify a cookie for use over everything else
    #[arg(short, long, value_name = "COOKIE", value_parser = sanitize_cookie)]
    cookie_override: Option<String>,
    /// Use a specific user for this download. If this isn't specified, the default user will be used.
    #[arg(short, long, value_name = "USER")]
    user_override: Option<String>,
    /// What kind of media we are downloading
    #[command(subcommand)]
    media_params: DownloadMediaParameters,
}

#[derive(Subcommand, Debug)]
pub enum DownloadMediaParameters {
    /// Download Illusts or Manga
    Illust(DownloadIllustParameters),
    /// Download Novels
    Novel(DownloadNovelParameters),
    /// Check for new media and download automatically new posts
    Update(DownloadUpdateParameters),
}

#[derive(Parser, Debug)]
pub struct DownloadIllustParameters {
    /// Check if a directory already has some of the illusts that are about to be downloaded and if so, don't download them again. If option is specified but no path is given, will use same path as output
    #[arg(short, long, value_name = "DIR", require_equals = true)]
    incremental: Option<Option<PathBuf>>,
    /// When available, stop checking with server early as soon as an illust was found on drive. Use this option wisely
    #[arg(long)]
    fast_incremental: bool,
    /// Do not create an update file for use with update functionality
    #[arg(long)]
    no_update_file: bool,
    /// Where the newly downloaded files will go. If not specified, will use working directory
    #[arg(short, long)]
    output_directory: Option<PathBuf>,
    /// Changes the directory creation behavior
    #[arg(short, long, value_enum, default_value_t = DirectoryPolicy::NeverCreate, value_name = "POLICY")]
    directory_policy: DirectoryPolicy,
    /// What to download exactly
    #[command(subcommand)]
    mode: DownloadIllustModes,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
pub enum DirectoryPolicy {
    /// In provided output directory, always create a subdir per illust (named with work ID) and put all images from this illust in it.
    AlwaysCreate,
    /// Always save all images directly to output directory
    NeverCreate,
    /// If illust only contains one page, save directly to output directory. Otherwise, create a subdir. (Not recommended when downloading multiple illusts)
    CreateIfMultiple,
}

#[derive(Subcommand, Serialize, Deserialize, Debug)]
pub enum DownloadIllustModes {
    /// Download a single illust
    Individual {
        #[arg(value_parser = parse_illust_id)]
        illust_ids: Vec<u64>,
    },
    /// Download a series
    Series {
        #[arg(value_parser = parse_series_id)]
        series_id: u64,
    },
    /// Download all posts from a user
    UserPosts {
        /// Only download posts with a specific tag applied
        #[arg(short, long, value_name = "TAG")]
        tag: Option<String>,
        /// ID of user to download from
        #[arg(value_parser = parse_user_id)]
        user_id: u64,
    },
    /// Download all posts liked/bookmarked by a user. If the ID is not specified, will download the current user's
    UserBookmarks { user_id: Option<u64> },
}

#[derive(Parser, Debug)]
pub struct DownloadNovelParameters {
    /// Directly specify a cookie for use over everything else
    #[arg(short, long, value_name = "COOKIE", value_parser = sanitize_cookie)]
    cookie_override: Option<String>,
    /// Use a specific user for this download. If this isn't specified, the default user will be used.
    #[arg(short, long, value_name = "USER")]
    user_override: Option<String>,
    /// ID of the novel to download
    novel_id: u64,
    /// Where the text file will be
    destination_file: PathBuf,
}

#[derive(Parser, Debug)]
pub struct DownloadUpdateParameters {
    /// When specified, go down all sub-directories and update everything
    #[arg(short, long)]
    recursive: bool,
    /// Where we are updating. If omitted, uses current directory
    directory: Option<PathBuf>,
}

// -----

#[derive(Parser, Debug)]
pub struct FNBParameters {
    /// ID of the user to check against
    user_id: u64,
    /// Directory containing the illusts
    dir: PathBuf,
    /// Use this cookie instead of the pre-configured one (if any)
    #[arg(short, long, value_name = "COOKIE", value_parser = sanitize_cookie)]
    cookie_override: Option<String>,
    /// If an illust is now unavailable, don't list it in the output. Disabled by default as this makes this request a lot more expensive
    #[arg(short, long, default_value_t = false)]
    ignore_missing: bool,
}

// -----

#[derive(Parser, Debug)]
pub struct CreateUpdateFileParameters {
    /// What directory will contain the update file
    output_directory: PathBuf,
    #[command(subcommand)]
    mode: DownloadIllustModes,
}

// -----

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args {
        Args::Users(s) => do_users_subcommand(s).await,
        Args::Download(p) => do_download_subcommand(p).await,
        Args::FindNotBookmarked(p) => do_fnb_subcommand(p).await,
        Args::CreateUpdateFile(c) => do_create_update_file_subcommand(c),
    }
}
