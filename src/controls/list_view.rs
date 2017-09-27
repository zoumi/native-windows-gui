//! A list view control
//!
//! Copyright (C) 2016  Gabriel Dubé
//!
//! This program is free software: you can redistribute it and/or modify
//! it under the terms of the GNU General Public License as published by
//! the Free Software Foundation, either version 3 of the License, or
//! (at your option) any later version.
//!
//! This program is distributed in the hope that it will be useful,
//! but WITHOUT ANY WARRANTY; without even the implied warranty of
//! MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//! GNU General Public License for more details.
//!
//! You should have received a copy of the GNU General Public License
//! along with this program.  If not, see <http://www.gnu.org/licenses/>.
//!

use std::hash::Hash;
use std::any::TypeId;
use std::mem::{transmute, zeroed, size_of};

use winapi::{LPARAM, HWND, HFONT, BOOL};
use winapi::commctrl::*;
use low::other_helper::{from_utf16, to_utf16};

use ui::Ui;
use controls::{Control, ControlT, ControlType, AnyHandle};
use error::Error;
use events::Event;
use defs::HTextAlign;
use user32::SendMessageW;
use low::window_helper::{WindowParams, build_window, set_window_font_raw, handle_of_window,
                         handle_of_font};
use winapi::{DWORD, WS_VISIBLE, WS_DISABLED, WS_CHILD, BS_NOTIFY, BS_GROUPBOX, BS_TOP, BS_CENTER,
             BS_LEFT, BS_RIGHT, WS_TABSTOP, WS_EX_CLIENTEDGE, WS_CHILDWINDOW, WS_VSCROLL,
             WS_EX_LTRREADING, WS_EX_RIGHTSCROLLBAR};
use winapi::winuser::{WS_EX_LEFT, WS_EX_RIGHT};

use std::thread::sleep;
use std::time::Duration;

/**
    A template that creates a standard list view

    Available events:  
    Event::Destroyed, Event::Moved, Event::Resized, Event::Raw  

    Members:  
    • `text`: The text of the groupbox  
    • `position`: The start position of groupbox  
    • `size`: The start size of the groupbox  
    • `visible`: If the groupbox should be visible to the user   
    • `disabled`: If the user can or can't click on the groupbox  
    • `parent`: The groupbox parent  
    • `font`: The groupbox font. If None, use the system default  
*/
#[derive(Clone, Debug)]
pub enum ViewMode {
    Report,
    Tile,
    IconSmall,
    List,
    Icon,
}

#[derive(Clone, Debug)]
pub struct ListViewT<ID: Hash + Clone>{
    pub column: Vec<String>,
    pub view_mode: ViewMode,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub visible: bool,
    pub disabled: bool,
    pub align: HTextAlign,
    pub parent: ID,
    pub font: Option<ID>,
}

// MSDN:Dynamically switching to and from the LVS_OWNERDATA style is not supported
// So we need a new struct
#[derive(Clone, Debug)]
pub struct VitualListViewT<ID: Hash + Clone>{
    pub column: Vec<String>,
    pub view_mode: ViewMode,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub visible: bool,
    pub disabled: bool,
    pub align: HTextAlign,
    pub parent: ID,
    pub font: Option<ID>,
}

impl<ID: Hash + Clone> ControlT<ID> for ListViewT<ID>{
    fn type_id(&self) -> TypeId {
        TypeId::of::<ListView>()
    }

    fn build(&self, ui: &Ui<ID>) -> Result<Box<Control>, Error> {
        let style: DWORD = WS_CHILDWINDOW | WS_VSCROLL | WS_TABSTOP | LVS_SHOWSELALWAYS |
                           LVS_REPORT |
                           if self.visible {
            WS_VISIBLE
        } else {
            0
        } |
                           if self.disabled {
            WS_DISABLED
        } else {
            0
        };

        let ex_style: DWORD = WS_EX_LTRREADING | WS_EX_RIGHTSCROLLBAR | WS_EX_LEFT |
                              WS_EX_CLIENTEDGE;

        let list_view_ex_style = LVS_EX_GRIDLINES | LVS_EX_HEADERDRAGDROP | LVS_EX_FULLROWSELECT |
                                 LVS_EX_DOUBLEBUFFER;

        // match self.align   { HTextAlign::Center=>WS_EX_CENTER, HTextAlign::Left=>WS_EX_LEFT, HTextAlign::Right=>WS_EX_RIGHT};
        // Get the parent handle
        let parent = match handle_of_window(ui,
                                            &self.parent,
                                            "The parent of a ListBox must be a window-like \
                                             control.") {
            Ok(h) => h,
            Err(e) => {
                return Err(e);
            }
        };

        // Get the font handle (if any)
        let font_handle: Option<HFONT> = match self.font.as_ref() {
            Some(font_id) => {
                match handle_of_font(ui,
                                     &font_id,
                                     "The font of a ListBox must be a font resource.") {
                    Ok(h) => Some(h),
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            None => None,
        };

        let params = WindowParams {
            title: "", 
            class_name: "SysListView32",
            position: self.position.clone(),
            size: self.size.clone(),
            flags: style,
            ex_flags: Some(ex_style),
            parent: parent,
        };

        match unsafe { build_window(params) } {
            Ok(h) => {
                unsafe {

                    set_window_font_raw(h, font_handle, true); 
                    set_lv_ex_style(h, list_view_ex_style);
                }
                Ok(Box::new(ListView { handle: h }))
            }
            Err(e) => Err(Error::System(e)),
        }
    }
}

// get current ListView ex_style
fn get_lv_ex_style(h: HWND) -> DWORD {
    unsafe { SendMessageW(h, LVM_GETEXTENDEDLISTVIEWSTYLE, 0, 0) as DWORD }
}

// remove all exiest style and set given style
fn set_lv_ex_style(h: HWND, exsty: DWORD) -> DWORD {
    let old_sty = get_lv_ex_style(h);
    let new_sty = old_sty | exsty;
    unsafe { SendMessageW(h, LVM_SETEXTENDEDLISTVIEWSTYLE, new_sty, exsty as i32) as DWORD }
}

/**
    A ListView
*/
#[derive(Clone, Debug)]
pub struct ListView {
    handle: HWND,
}


#[derive(Clone, Debug)]
pub struct ImageList {}

/// https://msdn.microsoft.com/en-us/library/windows/desktop/ff485961(v=vs.85).aspx
impl ListView {
    // method for rows
    pub fn add(&self, data: Vec<String>) -> i32{
        self.insert(10000, data)
    }

    pub fn insert(&self, aindex: isize,mut data: Vec<String>) -> i32{
        // LVM_INSERTITEM
        unsafe {
            let mut item: LVITEMW = zeroed();
            item.iItem = aindex as i32;
            item.mask = LVIF_TEXT;
            let mut re = -1;
            match data.pop(){
                Some(x) => {
                    let mut item_text = to_utf16(&x);
                    // the first item in current row
                    item.pszText = item_text.as_mut_ptr();
                    re = SendMessageW(self.handle, LVM_INSERTITEMW, 0, transmute(&mut item));
                    for i in 0..data.len() {
                        // add subitem
                        let mut sub_item: LVITEMW = zeroed();
                        sub_item.iSubItem = (i + 1) as i32;
                        let mut item_text = to_utf16(&(data[i].clone()));
                        sub_item.pszText = item_text.as_mut_ptr();
                        re = SendMessageW(self.handle,
                                                LVM_SETITEMTEXTW,
                                                aindex as DWORD,
                                                transmute(&sub_item))
                    }
                },
                None => {}
            }
            re
        }
    }

    pub fn modify(&self) {
        // LVM_SETITEM
    }
    pub fn delete(&self) {
        // LVM_DELETEITEM
    }

    // method for col
    pub fn delete_col(&self) {}

    pub fn insert_col(&self, col_index: isize, col_name: &str, col_width: usize) -> isize {
        // LVM_INSERTCOLUMN
        unsafe {
            let mut col_item: LVCOLUMNW = zeroed();
            col_item.mask = LVCF_TEXT | LVCF_FMT | LVCF_WIDTH;
            col_item.fmt = LVCFMT_LEFT;
            col_item.cx = col_width as i32;
            let mut col_name = to_utf16(col_name);
            col_item.pszText = col_name.as_mut_ptr();
            let lre = SendMessageW(self.handle,
                                   LVM_INSERTCOLUMNW,
                                   col_index as DWORD,
                                   transmute(&col_item));
            println!("insert_col: return {:?}", lre);
            // sleep(Duration::from_secs(50));
            lre as isize
        }
    }

    pub fn custom_sort(&self) {
        // LVM_SORTITEMS
    }
    pub fn sort(&self) {
        // LVS_SORTASCENDING or LVS_SORTDESCENDING
    }

    pub fn modify_col(&self) {}

    // other func
    pub fn get_count(&self) {}
    pub fn get_next(&self) {}
    pub fn get_text(&self) {
        // LVM_GETITEMTEXT
    }
    pub fn set_img_list(&self) {
        // LVM_SETIMAGELIST

    }

    pub fn set_text<'a>(&self, text: &'a str) {
        unsafe {
            ::low::window_helper::set_window_text(self.handle, text);
        }
    }
    pub fn get_visibility(&self) -> bool {
        unsafe { ::low::window_helper::get_window_visibility(self.handle) }
    }
    pub fn set_visibility(&self, visible: bool) {
        unsafe {
            ::low::window_helper::set_window_visibility(self.handle, visible);
        }
    }
    pub fn get_position(&self) -> (i32, i32) {
        unsafe { ::low::window_helper::get_window_position(self.handle) }
    }
    pub fn set_position(&self, x: i32, y: i32) {
        unsafe {
            ::low::window_helper::set_window_position(self.handle, x, y);
        }
    }
    pub fn get_size(&self) -> (u32, u32) {
        unsafe { ::low::window_helper::get_window_size(self.handle) }
    }
    pub fn set_size(&self, w: u32, h: u32) {
        unsafe {
            ::low::window_helper::set_window_size(self.handle, w, h, false);
        }
    }
    pub fn get_enabled(&self) -> bool {
        unsafe { ::low::window_helper::get_window_enabled(self.handle) }
    }
    pub fn set_enabled(&self, e: bool) {
        unsafe {
            ::low::window_helper::set_window_enabled(self.handle, e);
        }

    }
    pub fn set_costum_draw() {}
}

impl Control for ListView {
    fn handle(&self) -> AnyHandle {
        AnyHandle::HWND(self.handle)
    }

    fn control_type(&self) -> ControlType {
        ControlType::GroupBox
    }

    fn free(&mut self) {
        use user32::DestroyWindow;
        unsafe { DestroyWindow(self.handle) };
    }
}
