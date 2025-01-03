use std::{
  borrow::Cow,
  cmp,
  fmt::Debug,
  io::Error,
  ops::Deref,
  path::{Path, PathBuf},
};

use dyn_clone::clone_box;
use egui::{
  Align2, Context, Id, Key, Layout, Pos2, RichText, ScrollArea, TextEdit, Ui, Vec2, Window,
};
use fs::FileInfo;
use fs::Fs;

mod fs;
pub mod vfs;
pub use vfs::Vfs;
use vfs::VfsFile;

/// Function that returns `true` if the path is accepted.
pub type Filter<T> = Box<dyn Fn(&<T as Deref>::Target) -> bool + Send + Sync + 'static>;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
/// Dialog state.
pub enum State {
  /// Is currently visible.
  Open,
  /// Is currently not visible.
  Closed,
  /// Was canceled.
  Cancelled,
  /// File was selected.
  Selected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Dialog type.
pub enum DialogType {
  SelectFolder,
  OpenFile,
  SaveFile,
}

/// `egui` component that represents `OpenFileDialog` or `SaveFileDialog`.
pub struct FileDialog {
  /// Current opened path.
  path: PathBuf,

  /// Editable field with path.
  path_edit: String,

  /// Selected file path (single select mode).
  selected_file: Option<Box<dyn VfsFile>>,

  /// Editable field with filename.
  filename_edit: String,

  /// Dialog title text
  title: Cow<'static, str>,

  /// Open button text
  open_button_text: Cow<'static, str>,

  /// Save button text
  save_button_text: Cow<'static, str>,

  /// Cancel button text
  cancel_button_text: Cow<'static, str>,

  /// New Folder button text
  new_folder_button_text: Cow<'static, str>,

  /// New Folder name text
  new_folder_name_text: Cow<'static, str>,

  /// Rename button text
  rename_button_text: Cow<'static, str>,

  /// Refresh button hover text
  refresh_button_hover_text: Cow<'static, str>,

  /// Parent Folder button hover text
  parent_folder_button_hover_text: Cow<'static, str>,

  /// File label text
  file_label_text: Cow<'static, str>,

  /// Show Hidden checkbox text
  show_hidden_checkbox_text: Cow<'static, str>,

  /// Files in directory.
  files: Result<Vec<Box<dyn VfsFile>>, Error>,

  /// Current dialog state.
  state: State,

  /// Dialog type.
  dialog_type: DialogType,

  id: Option<Id>,
  current_pos: Option<Pos2>,
  default_pos: Option<Pos2>,
  default_size: Vec2,
  anchor: Option<(Align2, Vec2)>,
  show_files_filter: Filter<PathBuf>,
  filename_filter: Filter<String>,
  range_start: Option<usize>,
  resizable: bool,
  rename: bool,
  new_folder: bool,
  multi_select_enabled: bool,
  keep_on_top: bool,
  show_system_files: bool,

  /// Show drive letters on Windows.
  #[cfg(windows)]
  show_drives: bool,

  /// Show hidden files on unix systems.
  #[cfg(unix)]
  show_hidden: bool,

  fs: Box<dyn Vfs + 'static>,
}

impl Debug for FileDialog {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut dbg = f.debug_struct("FileDialog");
    let dbg = dbg
      .field("path", &self.path)
      .field("path_edit", &self.path_edit)
      .field("selected_file", &self.selected_file)
      .field("filename_edit", &self.filename_edit)
      .field("files", &self.files)
      .field("state", &self.state)
      .field("dialog_type", &self.dialog_type)
      .field("current_pos", &self.current_pos)
      .field("default_pos", &self.default_pos)
      .field("default_size", &self.default_size)
      .field("anchor", &self.anchor)
      .field("resizable", &self.resizable)
      .field("rename", &self.rename)
      .field("new_folder", &self.new_folder)
      .field("multi_select", &self.multi_select_enabled)
      .field("range_start", &self.range_start)
      .field("keep_on_top", &self.keep_on_top)
      .field("show_system_files", &self.show_system_files);

    // Closures don't implement std::fmt::Debug.
    // let dbg = dbg
    //   .field("shown_files_filter", &self.shown_files_filter)
    //   .field("filename_filter", &self.filename_filter);

    #[cfg(unix)]
    let dbg = dbg.field("show_hidden", &self.show_hidden);

    #[cfg(windows)]
    let dbg = dbg.field("show_drives", &self.show_drives);

    dbg.finish()
  }
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
    let mut path = initial_path.unwrap_or_else(|| Path::new("/").to_owned());
    let mut filename_edit = String::new();
    let info = FileInfo::new(path.clone());

    if info.is_file() {
      assert!(dialog_type != DialogType::SelectFolder);
      filename_edit = info.get_file_name().to_string();
      path.pop();
    }

    let path_edit = path.to_str().unwrap_or_default().to_string();
    Self {
      path,
      path_edit,
      selected_file: None,
      filename_edit,
      title: match dialog_type {
        DialogType::SelectFolder => "📁  Select Folder",
        DialogType::OpenFile => "📂  Open File",
        DialogType::SaveFile => "💾  Save File",
      }
      .into(),
      open_button_text: "Open".into(),
      save_button_text: "Save".into(),
      cancel_button_text: "Cancel".into(),
      new_folder_button_text: "New Folder".into(),
      new_folder_name_text: "New folder".into(),
      rename_button_text: "Rename".into(),
      refresh_button_hover_text: "Refresh".into(),
      parent_folder_button_hover_text: "Parent Folder".into(),
      file_label_text: "File:".into(),
      show_hidden_checkbox_text: "Show Hidden".into(),
      files: Ok(Vec::new()),
      state: State::Closed,
      dialog_type,

      id: None,
      current_pos: None,
      default_pos: None,
      default_size: egui::vec2(512.0, 512.0),
      anchor: None,
      show_files_filter: Box::new(|_| true),
      filename_filter: Box::new(|_| true),
      resizable: true,
      rename: true,
      new_folder: true,

      #[cfg(windows)]
      show_drives: true,

      #[cfg(unix)]
      show_hidden: false,
      multi_select_enabled: false,
      range_start: None,
      keep_on_top: false,
      show_system_files: false,
      fs: Box::new(Fs {}),
    }
  }

  /// Set the default file name.
  pub fn default_filename(mut self, filename: impl Into<String>) -> Self {
    self.filename_edit = filename.into();
    self
  }

  /// Set the window title text.
  pub fn title(mut self, title: &str) -> Self {
    self.title = (match self.dialog_type {
      DialogType::SelectFolder => "📁  ",
      DialogType::OpenFile => "📂  ",
      DialogType::SaveFile => "💾  ",
    }
    .to_string()
      + title)
      .into();
    self
  }

  /// Set the open button text.
  pub fn open_button_text(mut self, text: Cow<'static, str>) -> Self {
    self.open_button_text = text;
    self
  }

  /// Set the save button text.
  pub fn save_button_text(mut self, text: Cow<'static, str>) -> Self {
    self.save_button_text = text;
    self
  }

  /// Set the cancel button text.
  pub fn cancel_button_text(mut self, text: Cow<'static, str>) -> Self {
    self.cancel_button_text = text;
    self
  }

  /// Set the new folder button text.
  pub fn new_folder_button_text(mut self, text: Cow<'static, str>) -> Self {
    self.new_folder_button_text = text;
    self
  }

  /// Set the new folder name text.
  pub fn new_folder_name_text(mut self, text: Cow<'static, str>) -> Self {
    self.new_folder_name_text = text;
    self
  }

  /// Set the refresh button hover text.
  pub fn refresh_button_hover_text(mut self, text: Cow<'static, str>) -> Self {
    self.refresh_button_hover_text = text;
    self
  }

  /// Set the parent folder button hover text.
  pub fn parent_folder_button_hover_text(mut self, text: Cow<'static, str>) -> Self {
    self.parent_folder_button_hover_text = text;
    self
  }

  /// Set the rename button text.
  pub fn rename_button_text(mut self, text: Cow<'static, str>) -> Self {
    self.rename_button_text = text;
    self
  }

  /// Set the file label text.
  pub fn file_label_text(mut self, text: Cow<'static, str>) -> Self {
    self.file_label_text = text;
    self
  }

  /// Set the show hidden checkbox text.
  pub fn show_hidden_checkbox_text(mut self, text: Cow<'static, str>) -> Self {
    self.show_hidden_checkbox_text = text;
    self
  }

  /// Set the window ID.
  pub fn id(mut self, id: impl Into<Id>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// Set the window anchor.
  pub fn anchor(mut self, align: Align2, offset: impl Into<Vec2>) -> Self {
    self.anchor = Some((align, offset.into()));
    self
  }

  /// Set the window's current position.
  pub fn current_pos(mut self, current_pos: impl Into<Pos2>) -> Self {
    self.current_pos = Some(current_pos.into());
    self
  }

  /// Set the window's default position.
  pub fn default_pos(mut self, default_pos: impl Into<Pos2>) -> Self {
    self.default_pos = Some(default_pos.into());
    self
  }

  /// Set the window's default size.
  pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
    self.default_size = default_size.into();
    self
  }

  /// Enable/disable resizing the window. Default is `true`.
  pub fn resizable(mut self, resizable: bool) -> Self {
    self.resizable = resizable;
    self
  }

  /// Show the Rename button. Default is `true`.
  pub fn show_rename(mut self, rename: bool) -> Self {
    self.rename = rename;
    self
  }

  /// Show the New Folder button. Default is `true`.
  pub fn show_new_folder(mut self, new_folder: bool) -> Self {
    self.new_folder = new_folder;
    self
  }

  pub fn multi_select(mut self, multi_select: bool) -> Self {
    self.multi_select_enabled = multi_select;
    self
  }

  pub fn has_multi_select(&self) -> bool {
    self.multi_select_enabled
  }

  /// Show the mapped drives on Windows. Default is `true`.
  #[cfg(windows)]
  pub fn show_drives(mut self, drives: bool) -> Self {
    self.show_drives = drives;
    self
  }

  /// Set a function to filter listed files.
  pub fn show_files_filter(mut self, filter: Filter<PathBuf>) -> Self {
    self.show_files_filter = filter;
    self
  }

  /// Set a function to filter the selected filename.
  pub fn filename_filter(mut self, filter: Filter<String>) -> Self {
    self.filename_filter = filter;
    self
  }

  /// Set to true in order to keep this window on top of other windows. Default is `false`.
  pub fn keep_on_top(mut self, keep_on_top: bool) -> Self {
    self.keep_on_top = keep_on_top;
    self
  }

  pub fn with_fs(mut self, fs: Box<dyn Vfs>) -> Self {
    self.fs = fs;
    self
  }

  /// Set to true in order to show system files. Default is `false`.
  pub fn show_system_files(mut self, show_system_files: bool) -> Self {
    self.show_system_files = show_system_files;
    self
  }

  /// Get the dialog type.
  pub fn dialog_type(&self) -> DialogType {
    self.dialog_type
  }

  /// Get the window's visibility.
  pub fn visible(&self) -> bool {
    self.state == State::Open
  }

  /// Opens the dialog.
  pub fn open(&mut self) {
    self.state = State::Open;
    self.refresh();
  }

  /// Resulting file path.
  pub fn path(&self) -> Option<&Path> {
    self.selected_file.as_ref().map(|info| info.path())
  }

  /// Retrieves multi selection as a vector.
  pub fn selection(&self) -> Vec<&Path> {
    match self.files {
      Ok(ref files) => files
        .iter()
        .filter_map(|info| {
          if info.selected() {
            Some(info.path())
          } else {
            None
          }
        })
        .collect(),
      Err(_) => Vec::new(),
    }
  }

  /// Currently mounted directory that is being shown in the dialog box
  pub fn directory(&self) -> &Path {
    self.path.as_path()
  }

  /// Set the dialog's current opened path
  pub fn set_path(&mut self, path: impl Into<PathBuf>) {
    self.path = path.into();
    self.refresh();
  }

  /// Dialog state.
  pub fn state(&self) -> State {
    self.state
  }

  /// Returns true, if the file selection was confirmed.
  pub fn selected(&self) -> bool {
    self.state == State::Selected
  }

  fn open_selected(&mut self) {
    if let Some(info) = &self.selected_file {
      if info.is_dir() {
        self.set_path(info.path().to_owned());
      } else if self.dialog_type == DialogType::OpenFile {
        self.confirm();
      }
    } else if self.multi_select_enabled && self.dialog_type == DialogType::OpenFile {
      self.confirm();
    }
  }

  fn confirm(&mut self) {
    self.state = State::Selected;
  }

  fn refresh(&mut self) {
    self.files = self.fs.read_folder(
      &self.path,
      self.show_system_files,
      &self.show_files_filter,
      #[cfg(unix)]
      self.show_hidden,
      #[cfg(windows)]
      self.show_drives,
    );
    self.path_edit = String::from(self.path.to_str().unwrap_or_default());
    self.select(None);
    self.selected_file = None;
  }

  fn select(&mut self, file: Option<Box<dyn VfsFile>>) {
    if let Some(info) = &file {
      if !info.is_dir() {
        info.get_file_name().clone_into(&mut self.filename_edit);
      }
    }
    self.selected_file = file;
  }

  fn select_reset_multi(&mut self, idx: usize) {
    if let Ok(files) = &mut self.files {
      let selected_val = files[idx].selected();
      for file in files.iter_mut() {
        file.set_selected(false);
      }
      files[idx].set_selected(!selected_val);
      self.range_start = Some(idx);
    }
  }

  fn select_switch_multi(&mut self, idx: usize) {
    if let Ok(files) = &mut self.files {
      let old = !files[idx].selected();
      files[idx].set_selected(old);
      if files[idx].selected() {
        self.range_start = Some(idx);
      } else {
        self.range_start = None;
      }
    } else {
      self.range_start = None;
    }
  }

  fn select_range(&mut self, idx: usize) {
    if let Ok(files) = &mut self.files {
      if let Some(range_start) = self.range_start {
        let range = cmp::min(idx, range_start)..=cmp::max(idx, range_start);
        for i in range {
          files[i].set_selected(true);
        }
      }
    }
  }

  fn can_save(&self) -> bool {
    !self.filename_edit.is_empty() && (self.filename_filter)(self.filename_edit.as_str())
  }

  fn can_open(&self) -> bool {
    if self.multi_select_enabled {
      if let Ok(files) = &self.files {
        for file in files {
          if file.selected() && (self.filename_filter)(file.get_file_name()) {
            return true;
          }
        }
      }
      false
    } else {
      !self.filename_edit.is_empty() && (self.filename_filter)(self.filename_edit.as_str())
    }
  }

  fn can_rename(&self) -> bool {
    if !self.filename_edit.is_empty() {
      if let Some(file) = &self.selected_file {
        return file.get_file_name() != self.filename_edit;
      }
    }
    false
  }

  /// Shows the dialog if it is open. It is also responsible for state management.
  /// Should be called every ui update.
  pub fn show(&mut self, ctx: &Context) -> &Self {
    self.state = match self.state {
      State::Open => {
        if ctx.input(|state| state.key_pressed(Key::Escape)) {
          self.state = State::Cancelled;
        }

        let mut is_open = true;
        self.ui(ctx, &mut is_open);
        match is_open {
          true => self.state,
          false => State::Cancelled,
        }
      }
      _ => State::Closed,
    };

    self
  }

  fn ui(&mut self, ctx: &Context, is_open: &mut bool) {
    let mut window = Window::new(RichText::new(self.title.as_ref()).strong())
      .open(is_open)
      .default_size(self.default_size)
      .resizable(self.resizable)
      .collapsible(false);

    if let Some(id) = self.id {
      window = window.id(id);
    }

    if let Some((align, offset)) = self.anchor {
      window = window.anchor(align, offset);
    }

    if let Some(current_pos) = self.current_pos {
      window = window.current_pos(current_pos);
    }

    if let Some(default_pos) = self.default_pos {
      window = window.default_pos(default_pos);
    }

    window.show(ctx, |ui| {
      if self.keep_on_top {
        ui.ctx().move_to_top(ui.layer_id());
      }
      self.ui_in_window(ui)
    });
  }

  fn ui_in_window(&mut self, ui: &mut Ui) {
    enum Command {
      Cancel,
      CreateDirectory,
      Folder,
      Open(Box<dyn VfsFile>),
      OpenSelected,
      BrowseDirectory(Box<dyn VfsFile>),
      Refresh,
      Rename(PathBuf, PathBuf),
      Save(Box<dyn VfsFile>),
      Select(Box<dyn VfsFile>),
      MultiSelectRange(usize),
      MultiSelect(usize),
      MultiSelectSwitch(usize),
      UpDirectory,
    }
    let mut command: Option<Command> = None;

    // Top directory field with buttons.
    egui::TopBottomPanel::top("egui_file_top").show_inside(ui, |ui| {
      ui.horizontal(|ui| {
        ui.add_enabled_ui(self.path.parent().is_some(), |ui| {
          let response = ui
            .button("⬆")
            .on_hover_text(self.parent_folder_button_hover_text.as_ref());
          if response.clicked() {
            command = Some(Command::UpDirectory);
          }
        });
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
          let response = ui
            .button("⟲")
            .on_hover_text(self.refresh_button_hover_text.as_ref());
          if response.clicked() {
            command = Some(Command::Refresh);
          }

          let response = ui.add_sized(
            ui.available_size(),
            TextEdit::singleline(&mut self.path_edit),
          );

          if response.lost_focus() {
            let path = PathBuf::from(&self.path_edit);
            command = Some(Command::Open(Box::new(FileInfo::new(path))));
          }
        });
      });
      ui.add_space(ui.spacing().item_spacing.y);
    });

    // Bottom file field.
    egui::TopBottomPanel::bottom("egui_file_bottom").show_inside(ui, |ui| {
      ui.add_space(ui.spacing().item_spacing.y * 2.0);
      ui.horizontal(|ui| {
        ui.label(self.file_label_text.as_ref());
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
          if self.new_folder && ui.button(self.new_folder_button_text.as_ref()).clicked() {
            command = Some(Command::CreateDirectory);
          }

          if self.rename {
            ui.add_enabled_ui(self.can_rename(), |ui| {
              if ui.button(self.rename_button_text.as_ref()).clicked() {
                if let Some(from) = self.selected_file.clone() {
                  let to = from.path().with_file_name(&self.filename_edit);
                  command = Some(Command::Rename(from.path().to_owned(), to));
                }
              }
            });
          }

          let response = ui.add_sized(
            ui.available_size(),
            TextEdit::singleline(&mut self.filename_edit),
          );

          if response.lost_focus() {
            let ctx = response.ctx;
            let enter_pressed = ctx.input(|state| state.key_pressed(Key::Enter));

            if enter_pressed && (self.filename_filter)(self.filename_edit.as_str()) {
              let path = self.path.join(&self.filename_edit);
              match self.dialog_type {
                DialogType::SelectFolder => command = Some(Command::Folder),
                DialogType::OpenFile => {
                  if path.exists() {
                    command = Some(Command::Open(Box::new(FileInfo::new(path))));
                  }
                }
                DialogType::SaveFile => {
                  let file_info = Box::new(FileInfo::new(path));
                  command = Some(match file_info.is_dir() {
                    true => Command::Open(file_info),
                    false => Command::Save(file_info),
                  });
                }
              }
            }
          }
        });
      });

      ui.add_space(ui.spacing().item_spacing.y);

      // Confirm, Cancel buttons.
      ui.horizontal(|ui| {
        match self.dialog_type {
          DialogType::SelectFolder => {
            ui.horizontal(|ui| {
              if ui.button(self.open_button_text.as_ref()).clicked() {
                command = Some(Command::Folder);
              };
            });
          }
          DialogType::OpenFile => {
            ui.horizontal(|ui| {
              if !self.can_open() {
                ui.disable();
              }

              if ui.button(self.open_button_text.as_ref()).clicked() {
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
              if ui.button(self.open_button_text.as_ref()).clicked() {
                command = Some(Command::OpenSelected);
              };
            } else {
              ui.horizontal(|ui| {
                if !self.can_save() {
                  ui.disable();
                }

                if ui.button(self.save_button_text.as_ref()).clicked() {
                  let filename = &self.filename_edit;
                  let path = self.path.join(filename);
                  command = Some(Command::Save(Box::new(FileInfo::new(path))));
                };
              });
            }
          }
        }

        if ui.button(self.cancel_button_text.as_ref()).clicked() {
          command = Some(Command::Cancel);
        }

        #[cfg(unix)]
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
          if ui
            .checkbox(
              &mut self.show_hidden,
              self.show_hidden_checkbox_text.as_ref(),
            )
            .changed()
          {
            self.refresh();
          }
        });
      });
    });

    // File list.
    egui::CentralPanel::default().show_inside(ui, |ui| {
      ScrollArea::vertical().show_rows(
        ui,
        ui.text_style_height(&egui::TextStyle::Body),
        self.files.as_ref().map_or(0, |files| files.len()),
        |ui, range| match self.files.as_ref() {
          Ok(files) => {
            ui.with_layout(ui.layout().with_cross_justify(true), |ui| {
              let selected = self.selected_file.as_ref().map(|info| info.path());
              let range_start = range.start;

              for (n, info) in files[range].iter().enumerate() {
                let idx = n + range_start;
                let label = match info.is_dir() {
                  true => "🗀 ",
                  false => "🗋 ",
                }
                .to_string()
                  + info.get_file_name();

                let is_selected = if self.multi_select_enabled {
                  files[idx].selected()
                } else {
                  Some(info.path()) == selected
                };
                let response = ui.selectable_label(is_selected, label);
                if response.clicked() {
                  if self.multi_select_enabled {
                    if ui.input(|i| i.modifiers.shift) {
                      command = Some(Command::MultiSelectRange(idx))
                    } else if ui.input(|i| i.modifiers.ctrl) {
                      command = Some(Command::MultiSelectSwitch(idx))
                    } else {
                      command = Some(Command::MultiSelect(idx))
                    }
                  } else {
                    command = Some(Command::Select(dyn_clone::clone_box(info.as_ref())));
                  }
                }

                if response.double_clicked() {
                  match self.dialog_type {
                    DialogType::SelectFolder => {
                      // Always open folder on double click, otherwise SelectFolder cant enter sub-folders.
                      command = Some(Command::OpenSelected);
                    }
                    // Open or save file only if name matches filter.
                    DialogType::OpenFile => {
                      if info.is_dir() {
                        command = Some(Command::BrowseDirectory(clone_box(info.as_ref())));
                      } else if (self.filename_filter)(self.filename_edit.as_str()) {
                        command = Some(Command::Open(clone_box(info.as_ref())));
                      }
                    }
                    DialogType::SaveFile => {
                      if info.is_dir() {
                        command = Some(Command::OpenSelected);
                      } else if (self.filename_filter)(self.filename_edit.as_str()) {
                        command = Some(Command::Save(info.clone()));
                      }
                    }
                  }
                }
              }
            })
            .response
          }
          Err(e) => ui.label(e.to_string()),
        },
      );
    });

    if let Some(command) = command {
      match command {
        Command::Select(info) => self.select(Some(info)),
        Command::MultiSelect(idx) => self.select_reset_multi(idx),
        Command::MultiSelectRange(idx) => self.select_range(idx),
        Command::MultiSelectSwitch(idx) => self.select_switch_multi(idx),
        Command::Folder => {
          let path = self.get_folder().to_owned();
          self.selected_file = Some(Box::new(FileInfo::new(path)));
          self.confirm();
        }
        Command::Open(path) => {
          self.select(Some(path));
          self.open_selected();
        }
        Command::OpenSelected => self.open_selected(),
        Command::BrowseDirectory(dir) => {
          self.selected_file = Some(dir);
          self.open_selected();
        }
        Command::Save(file) => {
          self.selected_file = Some(file);
          self.confirm();
        }
        Command::Cancel => self.state = State::Cancelled,
        Command::Refresh => self.refresh(),
        Command::UpDirectory => {
          if self.path.pop() {
            self.refresh();
          }
        }
        Command::CreateDirectory => {
          let mut path = self.path.clone();
          let name = match self.filename_edit.is_empty() {
            true => self.new_folder_name_text.as_ref(),
            false => self.filename_edit.as_ref(),
          };
          path.push(name);
          match self.fs.create_dir(&path) {
            Ok(_) => {
              self.refresh();
              self.select(Some(Box::new(FileInfo::new(path))));
              // TODO: scroll to selected?
            }
            Err(err) => println!("Error while creating directory: {err}"),
          }
        }
        Command::Rename(from, to) => match self.fs.rename(from.as_path(), to.as_path()) {
          Ok(_) => {
            self.refresh();
            self.select(Some(Box::new(FileInfo::new(to))));
          }
          Err(err) => println!("Error while renaming: {err}"),
        },
      };
    }
  }

  fn get_folder(&self) -> &Path {
    if let Some(info) = &self.selected_file {
      if info.is_dir() {
        return info.path();
      }
    }

    // No selected file or it's not a folder, so use the current path.
    &self.path
  }
}
