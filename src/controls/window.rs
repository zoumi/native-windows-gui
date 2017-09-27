/*!
    Window control definition
*/

use std::any::TypeId;
use std::hash::Hash;
use std::ptr;

use winapi::{HWND, UINT, WPARAM, LPARAM, LRESULT, WM_SETICON, WM_GETICON, HICON};
use user32::SendMessageW;

use ui::Ui;
use controls::{Control, ControlT, ControlType, AnyHandle};
use error::Error;

/// System class identifier
const WINDOW_CLASS_NAME: &'static str = "NWG_BUILTIN_WINDOW";

/**
    A template that will create a window.

    Members:  
      • `title` : The title of the window (in the title bar)  
      • `position` : Starting posiion of the window after it is created  
      • `size` : Starting size of the window after it is created  
      • `resizable` : If the user can resize the window or not  
      • `visible` : If the user can see the window or not  
      • `disabled` : If the window is enabled or not. A disabled window do not process events  
      • `exit_on_close` : If NWG should break the event processing loop when this window is closed  
*/
#[derive(Clone)]
pub struct WindowT<ID: Hash+Clone, S: Clone+Into<String>> {
    pub title: S,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub resizable: bool,
    pub visible: bool,
    pub disabled: bool,
    pub exit_on_close: bool,
    pub icon: Option<ID>
}

impl<S: Clone+Into<String>, ID: Hash+Clone> ControlT<ID> for WindowT<ID, S> {
    fn type_id(&self) -> TypeId { TypeId::of::<Window>() }

    fn build(&self, ui: &Ui<ID>) -> Result<Box<Control>, Error> {
        unsafe{
            // Extract the icon handle
            let icon = if let &Some(ref i) = &self.icon {
                match ui.handle_of(i) {
                    Ok(AnyHandle::HICON(h)) => h,
                    Ok(h) =>  { return Err(Error::BadResource(format!("An icon resource is required, got {:?}", h))) },
                    Err(e) => { return Err(e); }
                }
            } else {
                ptr::null_mut()
            };
            
            // Build the window handle
            if let Err(e) = build_sysclass() { return Err(e); }
            match build_window(&self) {
                Ok(h) => { 
                    SendMessageW(h, WM_SETICON, 0, icon as LPARAM);
                    Ok( Box::new(Window{handle: h}) as Box<Control> ) 
                },
                Err(e) => Err(e)
            }
        } // unsafe
    }
}

/**
    A template that wraps a window created outside of nwg

    Members:  
          • `handle` : The external window handle
*/
#[derive(Clone)]
pub struct ExternWindowT {
    pub handle: HWND
}

impl<ID: Hash+Clone> ControlT<ID> for ExternWindowT {
    fn type_id(&self) -> TypeId { TypeId::of::<Window>() }

    #[allow(unused_variables)]
    fn build(&self, ui: &Ui<ID>) -> Result<Box<Control>, Error> {
        Ok( Box::new(Window{handle: self.handle}) as Box<Control> ) 
    }
}

/**
    A window control.
*/
#[allow(dead_code)]
pub struct Window {
    handle: HWND,
}

impl Window {
    /**
        Close the window as if the user clicked on the X button. This do **NOT** remove the window from the ui,
        it only set it hidden. In order to also destroy the window, add an unpack statement on the **Closed** event.
    */
    pub fn close(&self) {
        use user32::PostMessageW;
        use winapi::WM_CLOSE;

        unsafe{ PostMessageW(self.handle, WM_CLOSE, 0, 0) };
    }

    /// Activate the window and set it above the other windows
    pub fn activate(&self) { unsafe{ 
        use user32::SetForegroundWindow;
        SetForegroundWindow(self.handle); 
    } }

    /// Set the window icon. Pass `None` to remove the icon
    pub fn set_icon<ID: Hash+Clone>(&self, ui: &Ui<ID>, icon: Option<&ID>) -> Result<(), Error> {
        if !ui.has_handle(&self.handle()) {
            return Err(Error::BadUi("Icon resource and control must be in the same Ui.".to_string()));
        }

        let icon = if let Some(id) = icon {
            match ui.handle_of(id) {
                Ok(AnyHandle::HICON(h)) => h,
                Ok(h) => { return Err(Error::BadResource(format!("An icon resource is required, got {:?}", h))) },
                Err(e) => { return Err(e); }
            }
        } else {
            ptr::null_mut()
        };

        unsafe{ SendMessageW(self.handle, WM_SETICON, 0, icon as LPARAM); }

        Ok(())
    }

    /// Return the window icon identifier in the UI or None if there are no icon set
    pub fn get_icon<ID: Hash+Clone>(&self, ui: &Ui<ID>) -> Option<ID> {
        let icon = unsafe{ SendMessageW(self.handle, WM_GETICON, 0, 0) as HICON };
        if icon.is_null() {
            return None;
        }

        match ui.id_from_handle(&AnyHandle::HICON(icon)) {
            Ok(id) => Some(id),
            Err(_) => None
        }
    }

    pub fn get_title(&self) -> String { unsafe{ ::low::window_helper::get_window_text(self.handle) } }
    pub fn set_title<'a>(&self, text: &'a str) { unsafe{ ::low::window_helper::set_window_text(self.handle, text); } }
    pub fn get_visibility(&self) -> bool { unsafe{ ::low::window_helper::get_window_visibility(self.handle) } }
    pub fn set_visibility(&self, visible: bool) { unsafe{ ::low::window_helper::set_window_visibility(self.handle, visible); }}
    pub fn get_position(&self) -> (i32, i32) { unsafe{ ::low::window_helper::get_window_position(self.handle) } }
    pub fn set_position(&self, x: i32, y: i32) { unsafe{ ::low::window_helper::set_window_position(self.handle, x, y); }}
    pub fn get_size(&self) -> (u32, u32) { unsafe{ ::low::window_helper::get_window_size(self.handle) } }
    pub fn set_size(&self, w: u32, h: u32) { unsafe{ ::low::window_helper::set_window_size(self.handle, w, h, true); } }
    pub fn get_enabled(&self) -> bool { unsafe{ ::low::window_helper::get_window_enabled(self.handle) } }
    pub fn set_enabled(&self, e:bool) { unsafe{ ::low::window_helper::set_window_enabled(self.handle, e); } }
    pub fn update(&self) { unsafe{ ::low::window_helper::update(self.handle); } }
    pub fn focus(&self) { unsafe{ ::user32::SetFocus(self.handle); } }
}

impl Control for Window {

    fn handle(&self) -> AnyHandle {
        AnyHandle::HWND(self.handle)
    }

    fn control_type(&self) -> ControlType {
        ControlType::Window 
    }

    fn children(&self) -> Vec<AnyHandle> {
        use low::window_helper::list_window_children;
        unsafe{ list_window_children(self.handle) }
    }

    fn free(&mut self) {
        use user32::DestroyWindow;
        unsafe{ DestroyWindow(self.handle) };
    }

}


/*
    Private unsafe control methods
*/

#[allow(unused_variables)]
unsafe extern "system" fn window_sysproc(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> LRESULT {
    use winapi::{WM_CREATE, WM_CLOSE, GWL_USERDATA, WM_PAINT,WM_NOTIFY};
    use user32::{DefWindowProcW, PostQuitMessage, ShowWindow};
    use low::window_helper::get_window_long;

    let handled = match msg {
        WM_CREATE => true,
        WM_PAINT => {
            false
        },
        WM_CLOSE => {
            ShowWindow(hwnd, 0);

            let exit_on_close = get_window_long(hwnd, GWL_USERDATA) & 0x01 == 1;
            if exit_on_close {
                PostQuitMessage(0);
            }
            true
        },
        WM_NOTIFY => true,
        _ => false
    };

    if handled {
        0
    } else {
        match msg{
        WM_NOTIFY => println!{"WM_NOTIFY still here"},
        _ => {}
        };
        DefWindowProcW(hwnd, msg, w, l)
    }
}

#[inline(always)]
unsafe fn build_sysclass() -> Result<(), Error> {
    use low::window_helper::{SysclassParams, build_sysclass};
    let params = SysclassParams { 
        class_name: WINDOW_CLASS_NAME,
        sysproc: Some(window_sysproc),
        background: None, style: None
    };
    
    if let Err(e) = build_sysclass(params) {
        Err(Error::System(e))
    } else {
        Ok(())
    }
}

#[inline(always)]
unsafe fn build_window<ID: Hash+Clone, S: Clone+Into<String>>(t: &WindowT<ID, S>) -> Result<HWND, Error> {
    use low::window_helper::{WindowParams, build_window, set_window_long};
    use winapi::{DWORD, WS_VISIBLE, WS_DISABLED, WS_OVERLAPPEDWINDOW, WS_CAPTION, WS_OVERLAPPED, WS_MINIMIZEBOX,
      WS_MAXIMIZEBOX, WS_SYSMENU, GWL_USERDATA, WS_CLIPCHILDREN};

    let fixed_window: DWORD = WS_CLIPCHILDREN | WS_SYSMENU | WS_CAPTION | WS_OVERLAPPED | WS_MINIMIZEBOX | WS_MAXIMIZEBOX;
    let flags: DWORD = 
    if t.visible    { WS_VISIBLE }   else { 0 } |
    if t.disabled   { WS_DISABLED }  else { 0 } |
    if !t.resizable { fixed_window } else { WS_OVERLAPPEDWINDOW } ;

    let params = WindowParams {
        title: t.title.clone().into(),
        class_name: WINDOW_CLASS_NAME,
        position: t.position.clone(),
        size: t.size.clone(),
        flags: flags,
        ex_flags: None,
        parent: ::std::ptr::null_mut()
    };

    match build_window(params) {
        Ok(h) => {
            set_window_long(h, GWL_USERDATA, t.exit_on_close as usize);
            Ok(h)
        },
        Err(e) => Err(Error::System(e))
    }
}
