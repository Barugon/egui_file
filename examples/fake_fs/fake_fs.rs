use std::{
  io::{self, Error},
  path::{Path, PathBuf},
  sync::Mutex,
};

use egui_file::{
  vfs::{Vfs, VfsFile},
  Filter,
};

pub struct FakeFs {
  nodes: Mutex<Vec<Node>>,
}

impl FakeFs {
  pub fn new() -> Self {
    let mut nodes = vec![];
    for (f, n) in [
      ("/", false),
      ("/abc", false),
      ("/abc/def", false),
      ("/x", true),
      ("/abc/y", true),
    ] {
      nodes.push(Node {
        path: Path::new(f).to_owned(),
        is_file: n,
        selected: false,
      });
    }
    Self {
      nodes: Mutex::new(nodes),
    }
  }
}

impl Vfs for FakeFs {
  fn create_dir(&self, path: &Path) -> io::Result<()> {
    self.nodes.lock().unwrap().push(Node::new(path.as_ref()));
    Ok(())
  }

  fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
    Ok(())
  }

  fn read_folder(
    &self,
    path: &Path,
    _show_system_files: bool,
    _show_files_filter: &Filter<PathBuf>,
    _show_hidden: bool,
  ) -> Result<Vec<Box<dyn VfsFile>>, Error> {
    let mut ret: Vec<Box<dyn VfsFile>> = vec![];
    for f in self.nodes.lock().unwrap().iter() {
      if let Some(parent) = f.path.parent() {
        if parent == path {
          ret.push(Box::new(f.clone()))
        }
      }
    }
    Ok(ret)
  }
}

#[derive(Debug, Clone)]
struct Node {
  path: PathBuf,
  selected: bool,
  is_file: bool,
}

impl Node {
  pub fn new(path: &Path) -> Self {
    Node {
      path: path.into(),
      selected: false,
      is_file: true,
    }
  }
}

impl VfsFile for Node {
  fn is_file(&self) -> bool {
    self.is_file
  }

  fn is_dir(&self) -> bool {
    !self.is_file()
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
    &self.path.file_name().unwrap().to_str().unwrap()
  }
}
