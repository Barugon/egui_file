use std::io::{self, Error};
use std::path::{Path, PathBuf};

use dyn_clone::DynClone;

use crate::Filter;

pub trait Vfs {
  fn create_dir(&self, path: &Path) -> io::Result<()>;

  fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

  fn read_folder(
    &self,
    path: &Path,
    show_system_files: bool,
    show_files_filter: &Filter<PathBuf>,
    #[cfg(unix)] show_hidden: bool,
    #[cfg(windows)] show_drives: bool,
  ) -> Result<Vec<Box<dyn VfsFile>>, Error>;
}

pub trait VfsFile: std::fmt::Debug + DynClone {
  fn is_file(&self) -> bool;
  fn is_dir(&self) -> bool;
  fn path(&self) -> &Path;
  fn selected(&self) -> bool;
  fn set_selected(&mut self, selected: bool);
  fn get_file_name(&self) -> &str;
}

dyn_clone::clone_trait_object!(VfsFile);
