/*!
    A very high level native gui library for Windows.
*/

#![cfg(windows)]
extern crate winapi;

extern crate user32;
extern crate kernel32;
extern crate comctl32;
extern crate gdi32;
extern crate ole32;

mod low;
mod defs;
mod error;
mod cursor;
mod controls;
mod resources;
mod ui;

pub mod events;
pub mod templates;

pub mod custom {
    /*!
        Custom control creation resources
    */
    pub use controls::{ControlT, Control, AnyHandle, HandleSpec};
    pub use resources::{ResourceT, Resource};
    pub use low::events::{Event, UnpackProc, HandleProc, event_unpack_no_args, hwnd_handle, command_handle, notify_handle};
    pub use low::window_helper::{build_window, build_sysclass, SysclassParams, WindowParams, set_window_long, get_window_long,
    get_window_text, set_window_text, get_window_visibility, set_window_visibility, get_window_position, set_window_position,
    get_window_size, set_window_size, get_window_enabled, set_window_enabled, set_window_font, get_window_font, list_window_children};
    pub use low::menu_helper::list_menu_children;

}

pub mod constants {
    /*!
        Controls constants
    */
    pub use defs::*;
    pub use controls::ControlType;
}

pub use events::EventArgs;
pub use error::{Error, SystemError};
pub use low::other_helper::{message, simple_message, fatal_message, error_message};
pub use controls::{WindowT, Window, MenuT, Menu, MenuItemT, MenuItem, ButtonT, Button, ListBoxT, ListBox, CheckBoxT, CheckBox,
 RadioButtonT, RadioButton, TimerT, Timer, LabelT, Label, ComboBoxT, ComboBox, SeparatorT, Separator, TextInputT, TextInput,
 FileDialogT, FileDialog, TextBoxT, TextBox, GroupBoxT, GroupBox, ProgressBarT, ProgressBar, DatePickerT, DatePicker, ImageFrameT,
 ImageFrame, TreeViewT, TreeView, TreeViewItemT, TreeViewItem, TreeItemIterator, FrameT, Frame, ContextMenuT, ContextMenu,
 TabViewT, TabView, TabT, Tab, ListViewT, ListView, ViewMode};

pub use resources::{FontT, Font, ImageT, OemImageT, MemoryImageT, Image};
pub use ui::{Ui, dispatch_events, exit, toggle_console};
pub use cursor::Cursor;

#[cfg(feature = "canvas")]
pub use controls::{CanvasT, Canvas, CanvasRenderer};

#[cfg(feature = "canvas")]
pub use resources::{BrushT, Brush, PenT, Pen};
