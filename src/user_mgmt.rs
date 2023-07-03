use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

use anyhow::Result;
use dirs::config_dir;
use serde::{Deserialize, Serialize};

use crate::UsersSubcommands;

// TODO: Automatically update cookie with server answers ?

// ---------- File-related

const COOKIE_FILE_NAME: &str = "pixiv_util_user_db.json";

fn get_db_file_path() -> Result<PathBuf> {
    let mut config = match config_dir() {
        Some(c) => c,
        _ => return Err(anyhow::anyhow!("No suitable configuration folder !")),
    };

    config.push(COOKIE_FILE_NAME);

    Ok(config)
}

// ---------- Cookie string manipulations

fn sanitize(cookie: &str) -> &str {
    // TODO: Remove useless fields
    if let Some(s) = cookie.strip_prefix("Cookie: ") {
        s
    } else {
        cookie
    }
}

// State of the art programming
pub fn get_user_id(cookie: &str) -> Option<u64> {
    for element in cookie.split("; ").collect::<Vec<&str>>() {
        if let Some((key, value)) = element.split_once('=') {
            if key == "__utmv" {
                if let Some((_, useful)) = value.split_once('|') {
                    for inner in useful.split('^').collect::<Vec<&str>>() {
                        let sub = inner.split('=').collect::<Vec<&str>>();
                        if let Some(sk) = sub.get(1) {
                            if *sk == "user_id" {
                                if let Some(vk) = sub.get(2) {
                                    if let Ok(id) = vk.parse() {
                                        return Some(id);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

// ---------- Internal DB representation

#[derive(Serialize, Deserialize, Default)]
struct UserDatabase {
    default_user: Option<String>,
    users: HashMap<String, String>,
}

impl UserDatabase {
    // TODO: Async ?
    fn load_database(path: &Path) -> Result<UserDatabase> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let u = serde_json::from_reader(reader)?;
        Ok(u)
    }

    fn save_database(&self, path: &Path) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer(writer, &self)?;
        Ok(())
    }

    fn get_default_cookie(&self) -> Option<&String> {
        if let Some(u) = &self.default_user {
            self.users.get(u)
        } else {
            None
        }
    }
}

// ---------- High-level fns

pub async fn do_users_subcommand(s: UsersSubcommands) -> Result<()> {
    let path = get_db_file_path()?;

    match s {
        // Requires just the path
        UsersSubcommands::PrintPath => println!("{}", path.display()),
        _ => {
            let mut db = UserDatabase::load_database(&path).unwrap_or_default();
            match s {
                // Requires only reading the DB
                UsersSubcommands::ListUsers => {
                    for name in db.users.keys() {
                        println!("{}", name);
                    }
                }
                UsersSubcommands::PrintCookie { name } => match db.users.get(&name) {
                    Some(c) => println!("{}", c),
                    None => return Err(anyhow::anyhow!("No such user in database !")),
                },
                UsersSubcommands::GetDefault => {
                    if let Some(u) = db.default_user {
                        println!("{}", u)
                    } else {
                        println!("No default user.")
                    }
                }
                UsersSubcommands::GetPixivID { name } => match db.users.get(&name) {
                    Some(c) => match get_user_id(c) {
                        Some(i) => println!("{}", i),
                        None => return Err(anyhow::anyhow!("Couldn't get user id from cookie !")),
                    },
                    None => return Err(anyhow::anyhow!("No such user in database !")),
                },
                _ => {
                    match s {
                        // Requires modifying the DB
                        UsersSubcommands::AddUser { cookie, name } => {
                            db.users.insert(name, sanitize(&cookie).to_string());
                        }
                        UsersSubcommands::RemoveUser { name } => {
                            let mut delete_default = false;
                            if let Some(d) = &db.default_user {
                                delete_default = d == &name
                            }
                            if delete_default {
                                db.default_user = None;
                            }
                            match db.users.remove(&name) {
                                Some(_) => {}
                                None => return Err(anyhow::anyhow!("No such user in database !")),
                            }
                        }
                        UsersSubcommands::SetDefault { name } => {
                            if db.users.get(&name).is_some() {
                                db.default_user = Some(name)
                            } else {
                                return Err(anyhow::anyhow!("No such user in database !"));
                            }
                        }
                        UsersSubcommands::RemoveDefault => db.default_user = None,
                        _ => {}
                    }
                    db.save_database(&path)?;
                }
            }
        }
    }

    Ok(())
}

pub async fn retrieve_cookie(user_override: Option<String>) -> Result<Option<String>> {
    let path = get_db_file_path()?;
    let db = UserDatabase::load_database(&path).unwrap_or_default();

    if let Some(u) = user_override {
        if let Some(c) = db.users.get(&u) {
            Ok(Some(c.clone()))
        } else {
            Err(anyhow::anyhow!("No such user in database !"))
        }
    } else {
        Ok(db.get_default_cookie().cloned())
    }
}
