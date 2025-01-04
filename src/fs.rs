use std::{
  cmp::Ordering,
  fs::{self, FileType},
  io::{self, Error},
  path::{Path, PathBuf},
};

use crate::{vfs::VfsFile, Filter, Vfs};

#[derive(Default)]
pub struct Fs {}

impl Vfs for Fs {
  fn create_dir(&self, path: &Path) -> io::Result<()> {
    std::fs::create_dir(path)
  }

  fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
    std::fs::rename(from, to)
  }

  fn read_folder(
    &self,
    path: &Path,
    show_system_files: bool,
    show_files_filter: &Filter<PathBuf>,
    #[cfg(unix)] show_hidden: bool,
    #[cfg(windows)] show_drives: bool,
  ) -> Result<Vec<Box<dyn VfsFile>>, Error> {
    std::fs::read_dir(path).map(|entries| {
      let mut file_infos: Vec<Box<dyn VfsFile>> = entries
        .filter_map(|result| result.ok())
        .filter_map(|entry| {
          let info: Box<FileInfo> = Box::new(FileInfo::new(entry.path()));
          if !info.is_dir() {
            if !show_system_files && !info.path.is_file() {
              // Do not show system files.
              return None;
            }

            // Filter.
            if !(show_files_filter)(&info.path) {
              return None;
            }
          }

          #[cfg(unix)]
          if !show_hidden && info.get_file_name().starts_with('.') {
            return None;
          }

          let info: Box<dyn VfsFile> = info;
          Some(info)
        })
        .collect();

      // Sort with folders before files.
      file_infos.sort_by(|a, b| match b.is_dir().cmp(&a.is_dir()) {
        Ordering::Less => Ordering::Less,
        Ordering::Equal => a.path().file_name().cmp(&b.path().file_name()),
        Ordering::Greater => Ordering::Greater,
      });

      #[cfg(windows)]
      let file_infos = match show_drives {
        true => {
          let drives = get_drives();
          let mut infos = Vec::with_capacity(drives.len() + file_infos.len());
          for drive in drives {
            infos.push(Box::new(FileInfo::new(drive)) as Box<dyn VfsFile>);
          }
          infos.append(&mut file_infos);
          infos
        }
        false => file_infos,
      };

      file_infos
    })
  }
}

#[derive(Clone, Debug, Default)]
pub struct FileInfo {
  pub(crate) path: PathBuf,
  file_type: Option<FileType>,
  pub(crate) selected: bool,
}

impl FileInfo {
  pub fn new(path: PathBuf) -> Self {
    let file_type = fs::metadata(&path).ok().map(|meta| meta.file_type());
    Self {
      path,
      file_type,
      selected: false,
    }
  }
}

impl VfsFile for FileInfo {
  fn is_file(&self) -> bool {
    self.file_type.is_some_and(|file_type| file_type.is_file())
  }

  fn is_dir(&self) -> bool {
    self.file_type.is_some_and(|file_type| file_type.is_dir())
  }

  fn path(&self) -> &Path {
    &self.path
  }

  fn selected(&self) -> bool {
    self.selected
  }

  fn set_selected(&mut self, selected: bool) {
    self.selected = selected;
  }

  fn get_file_name(&self) -> &str {
    #[cfg(windows)]
    if self.is_dir() && is_drive_root(&self.path) {
      return self.path.to_str().unwrap_or_default();
    }
    self
      .path()
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or_default()
  }
}

#[cfg(windows)]
pub fn get_drives() -> Vec<PathBuf> {
  let mut drive_names = Vec::new();
  let mut drives = unsafe { GetLogicalDrives() };
  let mut letter = b'A';
  while drives > 0 {
    if drives & 1 != 0 {
      drive_names.push(format!("{}:\\", letter as char).into());
    }
    drives >>= 1;
    letter += 1;
  }
  drive_names
}

#[cfg(windows)]
pub fn is_drive_root(path: &Path) -> bool {
  path
    .to_str()
    .filter(|path| &path[1..] == ":\\")
    .and_then(|path| path.chars().next())
    .map_or(false, |ch| ch.is_ascii_uppercase())
}

#[cfg(windows)]
extern "C" {
  pub fn GetLogicalDrives() -> u32;
}
