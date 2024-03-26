# File dialog window (a.k.a. file picker) for [egui](https://github.com/emilk/egui)

[![Crates.io](https://img.shields.io/crates/v/egui_file)](https://crates.io/crates/egui_file)
[![docs.rs](https://img.shields.io/badge/docs-website-blue)](https://docs.rs/egui_file)

Taken from the [Dotrix](https://github.com/lowenware/dotrix) project, made into a stand-alone library and modified for more use cases.

![Screenshot from 2022-08-18 07-41-11](https://user-images.githubusercontent.com/16503728/185423412-32cd1b6d-0c2e-48e9-bc08-77c7278d2f1e.png)

## Example

````toml
[dependencies]
egui_file = "0.17"
eframe = "0.27"
````

````rust
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
    Box::new(|_cc| Box::new(Demo::default())),
  );
}
````
