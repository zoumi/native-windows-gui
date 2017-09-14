/*!
    A very high level native gui library for Windows.
*/
/*
    Copyright (C) 2016  Gabriel Dubé

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
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
mod events;
mod controls;
mod resources;
mod ui;

pub mod templates;

pub mod custom {
    /*!
        Custom control creation resources
    */
    pub use controls::{ControlT, Control, AnyHandle};
    pub use resources::{ResourceT, Resource};
    pub use low::window_helper::{build_window, build_sysclass, SysclassParams, WindowParams, set_window_long, get_window_long,
    get_window_text, set_window_text, get_window_visibility, set_window_visibility, get_window_position, set_window_position,
    get_window_size, set_window_size, get_window_enabled, set_window_enabled};

}

pub mod constants {
    /*!
        Controls constants
    */
    pub use defs::*;
    pub use controls::ControlType;
}

pub use error::{Error, SystemError};
pub use events::{EventCallback, Event, EventArgs};
pub use low::other_helper::{message, simple_message, fatal_message, error_message};
pub use controls::{WindowT, Window, MenuT, Menu, MenuItemT, MenuItem, ButtonT, Button, ListBoxT, ListBox, CheckBoxT, CheckBox,
 RadioButtonT, RadioButton, TimerT, Timer, LabelT, Label, ComboBoxT, ComboBox, SeparatorT, Separator, TextInputT, TextInput,
 FileDialogT, FileDialog, CanvasT, Canvas, CanvasRenderer, TextBoxT, TextBox, GroupBoxT, GroupBox, ProgressBarT, ProgressBar,
 DatePickerT, DatePicker, ListViewT,ListView,ViewMode};
pub use resources::{FontT, Font};
pub use ui::{Ui, dispatch_events, exit};
