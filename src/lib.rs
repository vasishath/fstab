#[macro_use]
extern crate log;

use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

const UNSUPPORTED_FSTAB: &str = "Unsupported file passed";
const UNEXPECTED_ENTRY: &str = "This entry is unexpected";
const FLAGS_MISSING: &str = "The attribute flags= was defined but no flags are present";

#[test]
fn test_parser() {
    use std::io::Cursor;
    let expected_results = vec![
        LinuxFsEntry {
            fs_spec: "/dev/mapper/xubuntu--vg--ssd-root".to_string(),
            mountpoint: PathBuf::from("/"),
            vfs_type: "ext4".to_string(),
            mount_options: vec!["noatime".to_string(), "errors=remount-ro".to_string()],
            dump: false,
            fsck_order: 1,
        },
        LinuxFsEntry {
            fs_spec: "UUID=378f3c86-b21a-4172-832d-e2b3d4bc7511".to_string(),
            mountpoint: PathBuf::from("/boot"),
            vfs_type: "ext2".to_string(),
            mount_options: vec!["defaults".to_string()],
            dump: false,
            fsck_order: 2,
        },
        LinuxFsEntry {
            fs_spec: "/dev/mapper/xubuntu--vg--ssd-swap_1".to_string(),
            mountpoint: PathBuf::from("none"),
            vfs_type: "swap".to_string(),
            mount_options: vec!["sw".to_string()],
            dump: false,
            fsck_order: 0,
        },
        LinuxFsEntry {
            fs_spec: "UUID=be8a49b9-91a3-48df-b91b-20a0b409ba0f".to_string(),
            mountpoint: PathBuf::from("/mnt/raid"),
            vfs_type: "ext4".to_string(),
            mount_options: vec!["errors=remount-ro".to_string(), "user".to_string()],
            dump: false,
            fsck_order: 1,
        },
    ];
    let input = r#"
# /etc/fstab: static file system information.
#
# Use 'blkid' to print the universally unique identifier for a
# device; this may be used with UUID= as a more robust way to name devices
# that works even if disks are added and removed. See fstab(5).
#
# <file system> <mount point>   <type>  <options>       <dump>  <pass>
/dev/mapper/xubuntu--vg--ssd-root /               ext4    noatime,errors=remount-ro 0       1
# /boot was on /dev/sda1 during installation
UUID=378f3c86-b21a-4172-832d-e2b3d4bc7511 /boot           ext2    defaults        0       2
/dev/mapper/xubuntu--vg--ssd-swap_1 none            swap    sw              0       0
UUID=be8a49b9-91a3-48df-b91b-20a0b409ba0f /mnt/raid ext4 errors=remount-ro,user 0 1
# tmpfs /tmp tmpfs rw,nosuid,nodev
"#;
    let bytes = input.as_bytes();
    let mut buff = Cursor::new(bytes);
    let fstab = FsTab::new(&Path::new("/fake"));
    //let results = fstab.parse_entries(&mut buff).unwrap();
    //println!("Result: {:?}", results);
    //assert_eq!(results, expected_results);

    //Modify an entry and then update it and see what the results are

    //let bytes_written = super::add_entry(expected_results[1].clone(), Path::new("/tmp/fstab"))
    //    .unwrap();
    //println!("Wrote: {}", bytes_written);
}

/// For help with what these fields mean consult: `man fstab` on linux.


#[derive(Debug)]
pub struct FsTab {
    location: PathBuf,
}


impl FsTab {
    pub fn new(fstab: &Path) -> Result<Self, Error> {
        let file = File::open(fstab.to_path_buf());
        match file {
            Ok(fd) => Ok(FsTab { location: fstab.to_path_buf() }),
            Err(e) => Err(e),
        }
    }

    pub fn save_entry(&self, fstab_type:FstabType) -> Result<usize, Error> {
        match fstab_type {
            FstabType::Linux(a) => LinuxFsEntry::save_fstab(self, a),
            FstabType::AndroidV2(a) => AndroidV2FsEntry::save_fstab(self, a),
            FstabType::AndroidV1(a) => AndroidV1FsEntry::save_fstab(self, a),
        }
    }

    pub fn parse_entries(&self) -> Result<FstabType, Error> {
        let mut contents = String::new();
        let mut file = File::open(&self.location)?;
        file.read_to_string(&mut contents)?;

        let mut fstab_type:FstabType = FstabType::Linux(Vec::new());
        let mut type_detected = false;
        for line in contents.lines() {
            if line.starts_with("#") {
                trace!("Skipping commented line: {}", line);
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 6 {
                fstab_type = FstabType::Linux(Vec::new());
                type_detected = true;
                break;
            }  else if parts.len() <= 5 && parts.len() >= 3 && !parts[0].starts_with("/dev/") {
                fstab_type = FstabType::AndroidV1(Vec::new());
                type_detected = true;
                break;
            } else if parts.len() == 5 {
                fstab_type = FstabType::AndroidV2(Vec::new());
                type_detected = true;
                break;
            }
        }
        if !type_detected {
            return Err(Error::new(ErrorKind::InvalidInput, UNSUPPORTED_FSTAB))
        }
        let entry = match fstab_type {
            FstabType::Linux(_) => LinuxFsEntry::parse_entries(&contents),
            FstabType::AndroidV2(_) => AndroidV2FsEntry::parse_entries(&contents),
            FstabType::AndroidV1(_) => AndroidV1FsEntry::parse_entries(&contents),
        };
        entry
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct LinuxFsEntry {
    /// The device identifier
    pub fs_spec: String,
    /// The mount point
    pub mountpoint: PathBuf,
    /// Which filesystem type it is
    pub vfs_type: String,
    /// Mount options to use
    pub mount_options: Vec<String>,
    /// This field is used by dump(8) to determine which filesystems need to be dumped
    pub dump: bool,
    /// This field is used by fsck(8) to determine the order in which filesystem checks
    /// are done at boot time.
    pub fsck_order: u16,
}

//Android fstab formats. Ref:- https://source.android.com/devices/storage/config
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct AndroidV2FsEntry {
    /// The device identifier
    pub fs_spec: String,
    /// The mount point
    pub mountpoint: PathBuf,
    /// Which filesystem type it is
    pub vfs_type: String,
    /// Mount options to use
    pub mount_options: Vec<String>,
    /// This field is used by android fsmgr to determine mount flags
    pub fsmgr_flags: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct AndroidV1FsEntry {
    /// The device identifier
    pub fs_spec: String,
    /// The mount point
    pub mountpoint: PathBuf,
    /// Which filesystem type it is
    pub vfs_type: String,

    pub fs_spec2: Option<String>,
    /// This field is used by android fsmgr to determine mount flags
    pub fsmgr_flags: Option<Vec<String>>,
}

trait FsEntry {
    fn parse_entry(contents: &str) -> Result<Self, Error> where Self: Sized;
    fn get_struct(vector: Vec<Self>) -> FstabType where Self: Sized;
    fn parse_entries(contents: &str) -> Result<FstabType, Error> where Self: Sized {
        let mut fstab_vec:Vec<Self> = Vec::new();
        for line in contents.lines() {
            if line.starts_with("#") {
                trace!("Skipping commented line: {}", line);
                continue;
            }
            match Self::parse_entry(line) {
                Ok(entry) => fstab_vec.push(entry),
                Err(_) => (),
            }
        }
        Ok(Self::get_struct(fstab_vec))
    }
    fn save_fstab(fstab: &FsTab, vec:Vec<Self>) -> Result<usize, Error> where Self:Sized;
}



impl FsEntry for AndroidV2FsEntry {
    fn parse_entry(contents: &str) -> Result<AndroidV2FsEntry, Error> {
        let parts: Vec<&str> = contents.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(Error::new(ErrorKind::InvalidInput, UNEXPECTED_ENTRY))
        }
        let entry = AndroidV2FsEntry {
            fs_spec: parts[0].to_string(),
            mountpoint: PathBuf::from(parts[1]),
            vfs_type: parts[2].to_string(),
            mount_options: parts[3].split(",").map(|s| s.to_string()).collect(),
            fsmgr_flags: parts[4].split(",").map(|s| s.to_string()).collect(),
        };
        Ok(entry)
    }
    fn get_struct(vector: Vec<Self>) -> FstabType {
        FstabType::AndroidV2(vector)
    }

    fn save_fstab(fstab: &FsTab, entries:Vec<Self>) -> Result<usize, Error> {
        let mut file = File::create(&fstab.location)?;
        let mut bytes_written: usize = 0;
        for entry in entries {
            bytes_written += file.write(&format!(
                "{spec} {mount} {vfs} {options} {flags}\n",
                spec = entry.fs_spec,
                mount = entry.mountpoint.display(),
                vfs = entry.vfs_type,
                options = entry.mount_options.join(","),
                flags = entry.fsmgr_flags.join(","),
            ).as_bytes())?;
        }
        file.flush()?;
        debug!("Wrote {} bytes to fstab", bytes_written);
        Ok(bytes_written)
    }
}

impl FsEntry for AndroidV1FsEntry {
    fn parse_entry(contents: &str) -> Result<AndroidV1FsEntry, Error> {
        let parts: Vec<&str> = contents.split_whitespace().collect();
        if parts.len() < 3 || parts.len() > 5 {
            return Err(Error::new(ErrorKind::InvalidInput, UNEXPECTED_ENTRY))
        }
        if parts.len() == 3 {
            let entry = AndroidV1FsEntry {
                fs_spec: parts[2].to_string(),
                mountpoint: PathBuf::from(parts[0]),
                vfs_type: parts[1].to_string(),
                fs_spec2: None,
                fsmgr_flags: None
            };
            Ok(entry)
        } else if parts.len() == 4 {
            if parts[3].starts_with("flags=") {
                let flags_str:Vec<&str> = parts[3].split("flags=").collect();
                if flags_str.len() < 2 {
                    return Err(Error::new(ErrorKind::InvalidInput, FLAGS_MISSING));
                }
                let flags = flags_str[1];
                let entry = AndroidV1FsEntry {
                    fs_spec: parts[2].to_string(),
                    mountpoint: PathBuf::from(parts[0]),
                    vfs_type: parts[1].to_string(),
                    fs_spec2: None,
                    fsmgr_flags: Some(flags.split(";").map(|s| s.to_string()).collect()),
                };
                Ok(entry)
            } else {
                let entry = AndroidV1FsEntry {
                    fs_spec: parts[2].to_string(),
                    mountpoint: PathBuf::from(parts[0]),
                    vfs_type: parts[1].to_string(),
                    fs_spec2: Some(parts[3].to_string()),
                    fsmgr_flags: None,
                };
                Ok(entry)
            }
        } else if parts.len() == 5 {
            let flags_str:Vec<&str> = parts[4].split("flags=").collect();
            if flags_str.len() < 2 {
                return Err(Error::new(ErrorKind::InvalidInput, FLAGS_MISSING));
            }
            let flags = flags_str[1];
            let entry = AndroidV1FsEntry {
                fs_spec: parts[2].to_string(),
                mountpoint: PathBuf::from(parts[0]),
                vfs_type: parts[1].to_string(),
                fs_spec2: Some(parts[3].to_string()),
                fsmgr_flags: Some(flags.split(";").map(|s| s.to_string()).collect()),
            };
            Ok(entry)
        } else {
            Err(Error::new(ErrorKind::InvalidInput, UNEXPECTED_ENTRY))
        }
    }
    fn get_struct(vector: Vec<Self>) -> FstabType {
        FstabType::AndroidV1(vector)
    }

    fn save_fstab(fstab: &FsTab, entries:Vec<Self>) -> Result<usize, Error> {
        let mut file = File::create(&fstab.location)?;
        let mut bytes_written: usize = 0;
        for entry in entries {
            bytes_written += file.write(&format!(
                "{mount} {vfs} {spec} {spec2} {flags}\n",
                spec = entry.fs_spec,
                mount = entry.mountpoint.display(),
                vfs = entry.vfs_type,
                spec2 = {
                    match &entry.fs_spec2 {
                        Some(s) => s,
                        None => "",
                    }
                },
                flags = {
                    match &entry.fsmgr_flags {
                        Some(s) => format!("flags={}", (s.join(";"))),
                        None => format!(""),
                    }
                },
            ).as_bytes())?;
        }
        file.flush()?;
        debug!("Wrote {} bytes to fstab", bytes_written);
        Ok(bytes_written)
    }

}

impl FsEntry for LinuxFsEntry {
    fn parse_entry(contents: &str) -> Result<LinuxFsEntry, Error> {
        let parts: Vec<&str> = contents.split_whitespace().collect();
        if parts.len() != 6 {
            return Err(Error::new(ErrorKind::InvalidInput, UNEXPECTED_ENTRY));
        }
        let fsck_order = u16::from_str(parts[5]).map_err(|e| {
            Error::new(ErrorKind::InvalidInput, e)
        });
        let entry = LinuxFsEntry {
            fs_spec: parts[0].to_string(),
            mountpoint: PathBuf::from(parts[1]),
            vfs_type: parts[2].to_string(),
            mount_options: parts[3].split(",").map(|s| s.to_string()).collect(),
            dump: if parts[4] == "0" { false } else { true },
            fsck_order: fsck_order.unwrap(),
        };
        Ok(entry)
    }
    fn get_struct(vector: Vec<Self>) -> FstabType {
        FstabType::Linux(vector)
    }

    fn save_fstab(fstab: &FsTab, entries:Vec<Self>) -> Result<usize, Error> {
        let mut file = File::create(&fstab.location)?;
        let mut bytes_written: usize = 0;
        for entry in entries {
            bytes_written += file.write(&format!(
                "{spec} {mount} {vfs} {options} {dump} {fsck}\n",
                spec = entry.fs_spec,
                mount = entry.mountpoint.display(),
                vfs = entry.vfs_type,
                options = entry.mount_options.join(","),
                dump = if entry.dump { "1" } else { "0" },
                fsck = entry.fsck_order
            ).as_bytes())?;
        }
        file.flush()?;
        debug!("Wrote {} bytes to fstab", bytes_written);
        Ok(bytes_written)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum FstabType {
    Linux (Vec<LinuxFsEntry>),
    AndroidV1 (Vec<AndroidV1FsEntry>),
    AndroidV2 (Vec<AndroidV2FsEntry>),
}




