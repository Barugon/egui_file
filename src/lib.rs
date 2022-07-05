use egui::{vec2, Align2, Context, Key, ScrollArea, TextEdit, Ui, Vec2, Window};
use std::{
  env,
  ffi::OsString,
  fs,
  io::Error,
  path::{Path, PathBuf},
};

#[cfg(unix)]
use egui::Layout;

// #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, )]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]

/// Dialog state
pub enum State {
  /// Is open.
  Open,
  /// Was closed.
  Closed,
  /// Was canceled.
  Cancelled,
  /// File was selected.
  Selected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogType {
  SelectFolder,
  OpenFile,
  SaveFile,
}

/// `egui` component that represents `OpenFileDialog` or `SaveFileDialog`.
pub struct FileDialog {
  /// Current opened path
  path: PathBuf,
  /// Editable field with path
  path_edit: String,

  /// Selected file path
  selected_file: Option<PathBuf>,
  /// Editable field with filename
  filename_edit: String,

  /// Files in directory
  files: Result<Vec<PathBuf>, Error>,
  /// Current dialog state
  state: State,
  /// Dialog type
  dialog_type: DialogType,

  default_size: Vec2,
  anchor: (Align2, Vec2),
  filter: Option<OsString>,
  resizable: bool,

  // Show hidden files on unix systems.
  #[cfg(unix)]
  hidden: bool,
}

impl FileDialog {
  /// Create dialog that prompts the user to select a folder.
  pub fn select_folder(initial_path: Option<PathBuf>) -> Self {
    FileDialog::new(DialogType::SelectFolder, initial_path)
  }

  /// Create dialog that prompts the user to open a file.
  pub fn open_file(initial_path: Option<PathBuf>) -> Self {
    FileDialog::new(DialogType::OpenFile, initial_path)
  }

  /// Create dialog that prompts the user to save a file.
  pub fn save_file(initial_path: Option<PathBuf>) -> Self {
    FileDialog::new(DialogType::SaveFile, initial_path)
  }

  /// Constructs new file dialog. If no `initial_path` is passed,`env::current_dir` is used.
  fn new(dialog_type: DialogType, initial_path: Option<PathBuf>) -> Self {
    let mut path = initial_path.unwrap_or(env::current_dir().unwrap_or_default());
    let mut filename_edit = String::new();

    if path.is_file() {
      assert!(dialog_type != DialogType::SelectFolder);
      filename_edit = get_file_name(&path).to_string();
      path.pop();
    }

    let path_edit = path.to_str().unwrap_or_default().to_string();
    let files = read_folder(&path);

    Self {
      path,
      path_edit,
      selected_file: None,
      filename_edit,
      files,
      state: State::Closed,
      dialog_type,

      default_size: vec2(512.0, 512.0),
      anchor: (Align2::CENTER_CENTER, vec2(0.0, 0.0)),
      filter: None,
      resizable: true,

      #[cfg(unix)]
      hidden: false,
    }
  }

  pub fn anchor(mut self, align: Align2, offset: impl Into<Vec2>) -> Self {
    self.anchor = (align, offset.into());
    self
  }

  /// Simple single extension filter.
  pub fn filter(mut self, filter: String) -> Self {
    self.filter = Some(filter.into());
    self
  }

  pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
    self.default_size = default_size.into();
    self
  }

  pub fn resizable(mut self, resizable: bool) -> Self {
    self.resizable = resizable;
    self
  }

  pub fn dialog_type(&self) -> DialogType {
    self.dialog_type
  }

  pub fn visible(&self) -> bool {
    self.state == State::Open
  }

  /// Opens the dialog.
  pub fn open(&mut self) {
    self.state = State::Open;
  }

  /// Result.
  pub fn path(&self) -> Option<PathBuf> {
    self.selected_file.clone()
  }

  /// Dialog state.
  pub fn state(&self) -> State {
    self.state
  }

  /// Returns true, if the file was confirmed.
  pub fn selected(&self) -> bool {
    self.state == State::Selected
  }

  fn open_selected(&mut self) {
    if let Some(path) = &self.selected_file {
      if path.is_dir() {
        self.path = path.clone();
        self.refresh();
      } else if path.is_file() && self.dialog_type == DialogType::OpenFile {
        self.confirm();
      }
    }
  }

  fn confirm(&mut self) {
    self.state = State::Selected;
  }

  fn refresh(&mut self) {
    self.files = read_folder(&self.path);
    self.path_edit = String::from(self.path.to_str().unwrap_or_default());
    self.select(None);
  }

  fn select(&mut self, file: Option<PathBuf>) {
    self.filename_edit = match &file {
      Some(path) => get_file_name(path).to_string(),
      None => String::new(),
    };
    self.selected_file = file;
  }

  fn can_save(&self) -> bool {
    self.selected_file.is_some() || !self.filename_edit.is_empty()
  }

  fn can_open(&self) -> bool {
    self.selected_file.is_some()
  }

  fn can_rename(&self) -> bool {
    if !self.filename_edit.is_empty() {
      if let Some(file) = &self.selected_file {
        return get_file_name(file) != self.filename_edit;
      }
    }
    false
  }

  fn title(&self) -> &str {
    match self.dialog_type {
      DialogType::SelectFolder => "📁 Select Folder",
      DialogType::OpenFile => "📂 Open File",
      DialogType::SaveFile => "💾 Save File",
    }
  }

  /// Shows the dialog if it is open. It is also responsible for state management.
  /// Should be called every ui update.
  pub fn show(&mut self, ctx: &Context) -> &Self {
    if ctx.input().key_pressed(Key::Escape) {
      self.state = State::Cancelled;
    }

    self.state = match self.state {
      State::Open => {
        let mut is_open = true;
        self.ui(ctx, &mut is_open); // may change self.state
        if is_open {
          self.state
        } else {
          State::Closed
        }
      }
      _ => State::Closed,
    };

    self
  }

  fn ui(&mut self, ctx: &Context, is_open: &mut bool) {
    let (align, offset) = self.anchor;
    let window = Window::new(self.title())
      .open(is_open)
      .default_size(self.default_size)
      .anchor(align, offset)
      .resizable(self.resizable)
      .collapsible(false);

    window.show(ctx, |ui| self.ui_in_window(ui));
  }

  fn ui_in_window(&mut self, ui: &mut Ui) {
    enum Command {
      Cancel,
      CreateDirectory,
      Folder,
      Open(PathBuf),
      OpenSelected,
      Refresh,
      Rename(PathBuf, PathBuf),
      Save(PathBuf),
      Select(PathBuf),
      UpDirectory,
    }
    let mut command: Option<Command> = None;

    // Top directory field with buttons
    ui.horizontal(|ui| {
      if ui
        .button("⬆ ")
        .on_hover_ui_at_pointer(|ui| {
          ui.label("Parent Folder");
        })
        .clicked()
      {
        command = Some(Command::UpDirectory);
      }
      ui.with_layout(Layout::right_to_left(), |ui| {
        if ui
          .button("⟲ ")
          .on_hover_ui_at_pointer(|ui| {
            ui.label("Refresh");
          })
          .clicked()
        {
          command = Some(Command::Refresh);
        }
        if ui
          .add_sized(
            ui.available_size(),
            TextEdit::singleline(&mut self.path_edit),
          )
          .lost_focus()
        {
          let path = PathBuf::from(&self.path_edit);
          command = Some(Command::Open(path));
        };
      });
    });

    // Rows with files
    ui.separator();
    ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
      match &self.files {
        Ok(files) => {
          for path in files {
            let is_dir = path.is_dir();

            if !is_dir {
              // Do not show system files
              if !path.is_file() {
                continue;
              }

              // Filter.
              if let Some(filter) = &self.filter {
                if path.extension() != Some(filter.as_os_str()) {
                  continue;
                }
              } else if self.dialog_type == DialogType::SelectFolder {
                continue;
              }
            }

            let filename = get_file_name(path);

            #[cfg(unix)]
            if !self.hidden && filename.starts_with('.') {
              continue;
            }

            ui.with_layout(ui.layout().with_cross_justify(true), |ui| {
              let mut label = String::new();
              if is_dir {
                label += "🗀 ";
              } else {
                label += "🗋 ";
              }
              label += filename;

              let is_selected = Some(path) == self.selected_file.as_ref();
              let selectable_label = ui.selectable_label(is_selected, label);
              if selectable_label.clicked() {
                command = Some(Command::Select(path.clone()));
              };
              if selectable_label.double_clicked() {
                command = Some(Command::Open(path.clone()));
              }
            });
          }
        }
        Err(e) => {
          ui.label(e.to_string());
        }
      }
    });

    // Bottom file field
    ui.separator();
    ui.horizontal(|ui| {
      ui.label("File:");
      ui.with_layout(Layout::right_to_left(), |ui| {
        if ui.button("New Folder").clicked() {
          command = Some(Command::CreateDirectory);
        }

        ui.add_enabled_ui(self.can_rename(), |ui| {
          if ui.button("Rename").clicked() {
            if let Some(from) = self.selected_file.clone() {
              let to = from.with_file_name(&self.filename_edit);
              command = Some(Command::Rename(from, to));
            }
          }
        });

        let result = ui.add_sized(
          ui.available_size(),
          TextEdit::singleline(&mut self.filename_edit),
        );

        if result.lost_focus()
          && result.ctx.input().key_pressed(egui::Key::Enter)
          && !self.filename_edit.is_empty()
        {
          let path = self.path.join(&self.filename_edit);
          match self.dialog_type {
            DialogType::SelectFolder => {
              command = Some(Command::Folder);
            }
            DialogType::OpenFile => {
              if path.exists() {
                command = Some(Command::Open(path));
              }
            }
            DialogType::SaveFile => {
              if path.exists() {
                if path.is_dir() {
                  command = Some(Command::Open(path));
                } else if path.is_file() {
                  command = Some(Command::Save(path));
                }
              } else {
                let filename = &self.filename_edit;
                let path = path.join(filename);
                command = Some(Command::Save(path));
              }
            }
          }
        }
      });
    });

    // Confirm, Cancel buttons
    ui.horizontal(|ui| {
      match self.dialog_type {
        DialogType::SelectFolder => {
          let should_open = match &self.selected_file {
            Some(file) => file.is_dir(),
            None => true,
          };

          ui.horizontal(|ui| {
            ui.set_enabled(should_open);
            if ui.button("Open").clicked() {
              command = Some(Command::Folder);
            };
          });
        }
        DialogType::OpenFile => {
          ui.horizontal(|ui| {
            ui.set_enabled(self.can_open());
            if ui.button("Open").clicked() {
              command = Some(Command::OpenSelected);
            };
          });
        }
        DialogType::SaveFile => {
          let should_open_directory = match &self.selected_file {
            Some(file) => file.is_dir(),
            None => false,
          };

          if should_open_directory {
            if ui.button("Open").clicked() {
              command = Some(Command::OpenSelected);
            };
          } else {
            ui.horizontal(|ui| {
              ui.set_enabled(self.can_save());
              if ui.button("Save").clicked() {
                let filename = &self.filename_edit;
                let path = self.path.join(filename);
                command = Some(Command::Save(path));
              };
            });
          }
        }
      }

      if ui.button("Cancel").clicked() {
        command = Some(Command::Cancel);
      }

      #[cfg(unix)]
      ui.with_layout(Layout::right_to_left(), |ui| {
        ui.checkbox(&mut self.hidden, "Show Hidden");
      });
    });

    if let Some(command) = command {
      match command {
        Command::Select(file) => {
          self.select(Some(file));
        }
        Command::Folder => {
          let path = self.path.join(&self.filename_edit);
          if path.is_dir() {
            self.selected_file = Some(path);
          } else {
            self.selected_file = Some(self.path.clone());
          }
          self.confirm();
        }
        Command::Open(path) => {
          self.select(Some(path));
          self.open_selected();
        }
        Command::OpenSelected => {
          self.open_selected();
        }
        Command::Save(file) => {
          self.selected_file = Some(file);
          self.confirm();
        }
        Command::Cancel => {
          self.state = State::Cancelled;
        }
        Command::Refresh => {
          self.refresh();
        }
        Command::UpDirectory => {
          if self.path.pop() {
            self.refresh();
          }
        }
        Command::CreateDirectory => {
          let mut path = self.path.clone();
          let name = if self.filename_edit.is_empty() {
            "New folder"
          } else {
            &self.filename_edit
          };
          path.push(name);
          match fs::create_dir(&path) {
            Ok(_) => {
              self.refresh();
              self.select(Some(path));
              // TODO: scroll to selected?
            }
            Err(e) => println!("Error while creating directory: {}", e),
          }
        }
        Command::Rename(from, to) => match fs::rename(from, &to) {
          Ok(_) => {
            self.refresh();
            self.select(Some(to));
          }
          Err(e) => println!("Error while renaming: {}", e),
        },
      };
    }
  }
}

fn get_file_name(path: &Path) -> &str {
  if path.is_file() || path.is_dir() {
    return path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or_default();
  }
  Default::default()
}

fn read_folder(path: &Path) -> Result<Vec<PathBuf>, Error> {
  match fs::read_dir(path) {
    Ok(paths) => {
      let mut result: Vec<PathBuf> = paths
        .filter_map(|result| result.ok())
        .map(|entry| entry.path())
        .collect();
      result.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
      Ok(result)
    }
    Err(e) => Err(e),
  }
}
