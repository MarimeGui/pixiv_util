mod abstractions;
mod api_calls;
mod download;
mod download_novel;
mod find_not_bookmarked;
mod gen_http_client;
mod incremental;
mod parsers;
mod user_mgmt;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use download::do_download_subcommand;
use find_not_bookmarked::do_fnb_subcommand;
use user_mgmt::do_users_subcommand;
use download_novel::do_download_novel_subcommand;

use parsers::*;

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
    /// Download a novel
    DownloadNovel(DownloadNovelParameters),
}

#[derive(Subcommand, Debug)]
pub enum UsersSubcommands {
    /// Add a new user with their cookie, along with a name for identification
    AddUser {
        #[arg(value_parser = sanitize_cookie)]
        cookie: String,
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
    /// Set no default, i.e. specify which user to use everytime
    RemoveDefault,
    /// List all users
    ListUsers,
    /// Print the Pixiv ID for a user
    GetPixivID { username: String },
    /// Print the path to the database file for users
    PrintPath,
}

#[derive(Parser, Debug)]
pub struct DownloadParameters {
    /// Directly specify a cookie for use over everything else
    #[arg(short, long, value_name = "COOKIE", value_parser = sanitize_cookie)]
    cookie_override: Option<String>,
    /// Use a specific user for this download. If this isn't specified, the default user will be used.
    #[arg(short, long, value_name = "USER")]
    user_override: Option<String>,
    /// Check if a directory already has some of the illusts that are about to be downloaded and if so, don't download them again. If option is specified but no path is given, will use same path as output
    #[arg(short, long, value_name = "DIR")]
    incremental: Option<Option<PathBuf>>,
    /// By default and when available, stop checking with server early as soon as an illust was found on drive. Use this option if downloads seem incomplete
    #[arg(long)]
    disable_fast_incremental: bool,
    /// Where the newly downloaded files will go
    #[arg(short, long)]
    output_directory: Option<PathBuf>,
    #[arg(short, long, value_enum, default_value_t = DirectoryPolicy::NeverCreate, value_name = "POLICY")]
    directory_policy: DirectoryPolicy,
    #[command(subcommand)]
    mode: DownloadModesSubcommands,
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

#[derive(Subcommand, Debug)]
enum DownloadModesSubcommands {
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
        #[arg(value_parser = parse_user_id)]
        user_id: u64,
    },
    /// Download all posts liked/bookmarked by a user. If the ID is not specified, will download the current user's
    UserBookmarks { user_id: Option<u64> },
}

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.subcommand {
        ModeSubcommands::Users(s) => do_users_subcommand(s).await,
        ModeSubcommands::Download(p) => do_download_subcommand(p).await,
        ModeSubcommands::FindNotBookmarked(p) => do_fnb_subcommand(p).await,
        ModeSubcommands::DownloadNovel(n) => do_download_novel_subcommand(n).await,
    }
}
