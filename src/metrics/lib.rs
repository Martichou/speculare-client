pub mod cpu;
pub mod disks;
pub mod miscs;
pub mod models;
pub mod network;
pub mod sensors;
pub mod users;

use std::{fs, io::Error};

/// Read from path to content, trim it and return the String
pub fn read_and_trim(path: &str) -> Result<String, Error> {
    let content = fs::read_to_string(path)?;
    Ok(content.trim().to_owned())
}

pub fn is_physical_filesys(filesysteme: &str) -> bool {
    match filesysteme {
        "ext2" => true,
        "ext3" => true,
        "ext4" => true,
        "vfat" => true,
        "ntfs" => true,
        "zfs" => true,
        "hfs" => true,
        "reiserfs" => true,
        "reiser4" => true,
        "exfat" => true,
        "f2fs" => true,
        "hfsplus" => true,
        "jfs" => true,
        "btrfs" => true,
        "minix" => true,
        "nilfs" => true,
        "xfs" => true,
        "apfs" => true,
        "fuseblk" => true,
        _ => false,
    }
}
