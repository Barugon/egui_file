extern crate egui_file;
extern crate eframe;

use eframe::{
  egui::{CentralPanel, Context},
  App, Frame,
};
use egui_file::FileDialog;
use std::{
  ffi::OsStr,
  path::{Path, PathBuf},
};

#[derive(Default)]
pub struct Demo {
  opened_file: Option<PathBuf>,
  open_file_dialog: Option<FileDialog>,
}

impl App for Demo {
  fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
    CentralPanel::default().show(ctx, |ui| {
      if (ui.button("Open")).clicked() {
        // Show only files with the extension "txt".
        let filter = Box::new({
          let ext = Some(OsStr::new("txt"));
          move |path: &Path| -> bool { path.extension() == ext }
        });
        let mut dialog = FileDialog::open_file(self.opened_file.clone()).show_files_filter(filter);
        dialog.open();
        self.open_file_dialog = Some(dialog);
      }

      if let Some(dialog) = &mut self.open_file_dialog {
        if dialog.show(ctx).selected() {
          if let Some(file) = dialog.path() {
            self.opened_file = Some(file.to_path_buf());
          }
        }
      }
    });
  }
}

fn main() {
  let _ = eframe::run_native(
    "File Dialog Demo",
    eframe::NativeOptions::default(),
    Box::new(|_cc| Ok(Box::new(Demo::default()))),
  );
}