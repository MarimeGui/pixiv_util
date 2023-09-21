use std::{fs::File as StdFile, path::Path};

use anyhow::{anyhow, Result};

use crate::{CreateUpdateFileParameters, DownloadIllustModes};

pub static UPDATE_FILE: &str = ".pixiv_update";

pub fn do_create_update_file_subcommand(params: CreateUpdateFileParameters) -> Result<()> {
    if let DownloadIllustModes::Individual { illust_ids: _ } = &params.mode {
        return Err(anyhow!(
            "Cannot create an update file for individual illusts"
        ));
    }
    create_update_file(&params.output_directory, &params.mode)
}

pub fn create_update_file(output_dir: &Path, mode: &DownloadIllustModes) -> Result<()> {
    let mut update_file_path = output_dir.to_path_buf();
    update_file_path.push(UPDATE_FILE);

    let mut file = StdFile::create(update_file_path)?;

    serde_json::to_writer_pretty(&mut file, mode)?;

    Ok(())
}
