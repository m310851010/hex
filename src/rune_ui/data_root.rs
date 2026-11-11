//! General › Advanced › Files: shows the one folder Hex keeps its data in
//! and lets a person move it. The choice is a small pointer file beside the
//! executable, read before settings load; moving data is done here, off a
//! frame, and the window asks for a restart because log handles and cached
//! paths still point at the old folder.

use std::path::{Path, PathBuf};

use rune::{El, row, text};

use super::Ui;
use super::controls::{ButtonKind, DESCRIPTION, Paint, button, note, section, setting_row};
use crate::desktop::file_dialog::{Dialog, Pending, Target};
use crate::desktop::i18n::tr;

/// The folder each kind of data lives in, relative to the data root.
const FOLDERS: &[(&str, &str, &str)] = &[
    ("settings", "Settings", "settings.json"),
    ("history", "History", "history.json"),
    ("logs", "Logs", "logs"),
    ("models", "Models", "models"),
    ("captures", "Captures", "captures"),
    ("meetings", "Meetings", "meetings"),
    ("updates", "Updates", "updates"),
];

#[derive(Default)]
pub(crate) struct DataRoot {
    choice: Option<Pending>,
    error: Option<String>,
    /// Set once the move succeeded; the current process still uses the old
    /// folder, so the pane says to restart Hex.
    moved: bool,
    waker: Option<rune::Waker>,
}

impl DataRoot {
    pub(super) fn set_waker(&mut self, waker: rune::Waker) {
        self.waker = Some(waker);
    }

    /// Applies a chosen folder when the dialog answers. Returns whether the
    /// pane should redraw.
    pub(super) fn poll(&mut self) -> bool {
        let Some(choice) = &self.choice else {
            return false;
        };
        let Some(answer) = choice.poll() else {
            return false;
        };
        self.choice = None;
        match answer {
            Ok(Some(folder)) => match crate::app_paths::set_data_root(&folder) {
                Ok(()) => {
                    self.error = None;
                    self.moved = true;
                }
                Err(error) => self.error = Some(format!("{error:#}")),
            },
            Ok(None) => {}
            Err(error) => self.error = Some(error),
        }
        true
    }

    fn choose(&mut self) {
        if self.choice.is_some() {
            return;
        }
        let waker = self.waker.clone();
        self.error = None;
        self.choice = Some(crate::desktop::file_dialog::open(
            Dialog {
                title: tr("Choose data folder").into(),
                target: Target::Folder,
                directory: crate::app_paths::support_dir().ok(),
            },
            move || {
                if let Some(waker) = &waker {
                    waker();
                }
            },
        ));
    }
}

/// The Files section in General's Advanced fold. Windows only: on the other
/// platforms the data root is the OS's own application-support folder.
#[cfg(target_os = "windows")]
pub(super) fn rows(ui: &Ui, p: Paint) -> Vec<El> {
    let root = crate::app_paths::support_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| tr("Unavailable").to_string());
    let mut rows = vec![setting_row(
        p,
        "data-root",
        tr("Data folder"),
        None,
        row()
            .key("data-root-controls")
            .gap(8.0)
            .center_y()
            .child(text(root).key("value").font(DESCRIPTION).color(p.text()))
            .child(button(
                p,
                "choose-data-root",
                tr("Change…"),
                ButtonKind::Secondary,
                |ui: &mut Ui, _| ui.data_root.choose(),
            )),
    )];
    if let Some(error) = &ui.data_root.error {
        rows.push(note("data-root-error", error, p.danger()));
    }
    if ui.data_root.moved {
        rows.push(note(
            "data-root-restart",
            tr("Restart Hex to use the new data folder"),
            p.muted(),
        ));
    }
    for &(key, name, child) in FOLDERS {
        let path = crate::app_paths::support_dir()
            .map(|root| root.join(child))
            .unwrap_or_else(|_| PathBuf::from(child));
        rows.push(setting_row(
            p,
            key,
            tr(name),
            None,
            text(shorten(&path))
                .key("value")
                .font(DESCRIPTION)
                .color(p.muted()),
        ));
    }
    rows
}

#[cfg(target_os = "windows")]
fn shorten(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(not(target_os = "windows"))]
pub(super) fn rows(_ui: &Ui, _p: Paint) -> Vec<El> {
    Vec::new()
}

/// The section wrapper, so General's Advanced fold can add it as one card.
#[cfg(target_os = "windows")]
pub(super) fn files_section(ui: &Ui, p: Paint) -> El {
    section(p, "files", tr("Files"), rows(ui, p))
}

#[cfg(not(target_os = "windows"))]
pub(super) fn files_section(_ui: &Ui, _p: Paint) -> El {
    row().key("files-empty")
}
