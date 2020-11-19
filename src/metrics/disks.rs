use super::is_physical_filesys;

use crate::models;

#[cfg(target_os = "macos")]
use futures::executor;
#[cfg(target_os = "macos")]
use futures_util::stream::StreamExt;
use models::{Disks, IoStats};
#[cfg(target_os = "macos")]
use nix::libc::statfs;
use nix::sys;
#[cfg(target_os = "macos")]
use std::ffi::CStr;
#[cfg(target_family = "unix")]
use std::io::{Error, ErrorKind};
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::{
    fs::File,
    io::{BufRead, BufReader},
};
#[cfg(target_os = "linux")]
use unescape::unescape;

#[cfg(target_os = "macos")]
extern "C" {
    fn getfsstat64(buf: *mut statfs, bufsize: libc::c_int, flags: libc::c_int) -> libc::c_int;
}

/// Retrieve the partitions and return them as a Vec<Disks>.
/// Contains name, mount_point and total/free space.
/// LINUX => read info from /proc/mounts.
#[cfg(target_os = "linux")]
pub fn get_partitions_info() -> Result<Vec<Disks>, Error> {
    let mut vdisks: Vec<Disks> = Vec::new();
    let file = File::open("/proc/mounts")?;
    let file = BufReader::with_capacity(6144, file);

    for line in file.lines() {
        let line = line.unwrap();
        let fields = line.split_whitespace().collect::<Vec<&str>>();
        if !is_physical_filesys(fields[2]) {
            continue;
        }
        let m_p = PathBuf::from(unescape(fields[1]).unwrap());
        let usage: (u64, u64) = match disk_usage(&m_p) {
            Ok(val) => val,
            Err(_) => (0, 0),
        };
        vdisks.push(Disks {
            name: fields[0].to_owned(),
            mount_point: m_p.into_os_string().into_string().unwrap(),
            total_space: (usage.0 / 100000) as i64,
            avail_space: (usage.1 / 100000) as i64,
        });
    }

    Ok(vdisks)
}

/// Retrieve the partitions and return them as a Vec<Disks>.
/// Contains name, mount_point and total/free space.
/// macOS => use C's function getfsstat64.
#[cfg(target_os = "macos")]
pub fn get_partitions_info() -> Result<Vec<Disks>, Error> {
    let expected_len = unsafe { getfsstat64(std::ptr::null_mut(), 0, 2) };
    let mut mounts: Vec<statfs> = Vec::with_capacity(expected_len as usize);

    let result = unsafe {
        getfsstat64(
            mounts.as_mut_ptr(),
            std::mem::size_of::<statfs>() as libc::c_int * expected_len,
            2,
        )
    };
    if result < 0 {
        return Err(Error::last_os_error());
    }
    unsafe {
        mounts.set_len(result as usize);
    }

    let mut vdisks: Vec<Disks> = Vec::with_capacity(expected_len as usize);
    for stat in mounts {
        if !is_physical_filesys(unsafe {
            &CStr::from_ptr(stat.f_fstypename.as_ptr()).to_string_lossy()
        }) {
            continue;
        }
        let m_p = PathBuf::from(unsafe {
            CStr::from_ptr(stat.f_mntonname.as_ptr())
                .to_string_lossy()
                .to_string()
        });
        let usage: (u64, u64) = match disk_usage(&m_p) {
            Ok(val) => val,
            Err(_) => (0, 0),
        };
        vdisks.push(Disks {
            name: unsafe {
                CStr::from_ptr(stat.f_mntfromname.as_ptr())
                    .to_string_lossy()
                    .to_string()
            },
            mount_point: m_p.into_os_string().into_string().unwrap(),
            total_space: (usage.0 / 100000) as i64,
            avail_space: (usage.1 / 100000) as i64,
        });
    }

    Ok(vdisks)
}

/// Return the total/free space of a Disk from it's path (mount_point).
/// For both Linux and macOS.
pub fn disk_usage<P>(path: P) -> Result<(u64, u64), Error>
where
    P: AsRef<Path>,
{
    let statvfs = match sys::statvfs::statvfs(path.as_ref()) {
        Ok(val) => val,
        Err(x) => return Err(Error::new(ErrorKind::Other, x)),
    };
    let total = statvfs.blocks() as u64 * statvfs.fragment_size() as u64;
    let free = statvfs.blocks_available() as u64 * statvfs.fragment_size() as u64;

    Ok((total, free))
}

/// Return the disk io usage, number of sectors read, wrtn.
/// From that you can compute the mb/s.
/// LINUX -> Read data from /proc/diskstats.
#[cfg(target_os = "linux")]
pub fn get_iostats() -> Result<Vec<IoStats>, Error> {
    let mut viostats: Vec<IoStats> = Vec::new();
    let file = File::open("/proc/diskstats")?;
    let file = BufReader::with_capacity(2048, file);

    for line in file.lines() {
        let line = line.unwrap();
        let fields = line.split_whitespace().collect::<Vec<&str>>();
        if fields.len() < 14 {
            continue;
        }
        viostats.push(IoStats {
            device_name: fields[2].to_owned(),
            sectors_read: fields[5].parse::<i64>().unwrap(),
            sectors_wrtn: fields[9].parse::<i64>().unwrap(),
        });
    }

    Ok(viostats)
}

/// Return the disk io usage, number of sectors read, wrtn.
/// From that you can compute the mb/s.
/// macOS -> Read data using heim_disks.
#[cfg(target_os = "macos")]
pub fn get_iostats() -> Result<Vec<IoStats>, Error> {
    let mut viostats: Vec<IoStats> = Vec::new();
    let mut counters = heim_disk::io_counters();

    while let Some(count) = executor::block_on(counters.next()) {
        let count = count.unwrap();
        viostats.push(IoStats {
            device_name: count.device_name().to_str().unwrap_or("?").to_owned(),
            sectors_read: count.read_bytes().value as i64,
            sectors_wrtn: count.write_bytes().value as i64,
        });
    }

    Ok(viostats)
}
