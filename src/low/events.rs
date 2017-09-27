/*!
    Low level events functions
*/
#![allow(non_upper_case_globals, unused_variables)]
use std::{fmt, any, ptr, mem};
use std::hash::{Hash, Hasher};

use winapi::{HWND, UINT, DWORD, WPARAM, LPARAM, UINT_PTR, DWORD_PTR, LRESULT, WORD, HIWORD, NMHDR,
 HMENU, c_int};

use winapi::{WM_MOVE, WM_SIZING, WM_SIZE, WM_EXITSIZEMOVE, WM_PAINT, WM_UNICHAR, WM_CHAR,
  WM_CLOSE, WM_LBUTTONUP, WM_RBUTTONUP, WM_MBUTTONUP, WM_LBUTTONDOWN, WM_RBUTTONDOWN,
  WM_MBUTTONDOWN, WM_KEYDOWN, WM_KEYUP, BN_CLICKED, BN_DBLCLK, BN_SETFOCUS, BN_KILLFOCUS,
  DTN_CLOSEUP, WM_COMMAND, WM_NOTIFY, WM_TIMER, WM_MENUCOMMAND, TVN_SELCHANGEDW, WM_MOUSEMOVE,
  NM_CLICK, NM_DBLCLK, NM_KILLFOCUS, NM_SETFOCUS, TVN_ITEMCHANGEDW, TVN_ITEMCHANGINGW, TVN_ITEMEXPANDEDW,
  TVN_ITEMEXPANDINGW, TVN_DELETEITEMW};

use ui::UiInner;
use events::EventArgs;
use controls::{AnyHandle, Timer};
use low::menu_helper::get_menu_id;
use low::defs::{NWG_DESTROY, CBN_SELCHANGE, CBN_KILLFOCUS, CBN_SETFOCUS, STN_CLICKED, STN_DBLCLK,
  LBN_SELCHANGE, LBN_DBLCLK, LBN_SETFOCUS, LBN_KILLFOCUS, EN_SETFOCUS, EN_KILLFOCUS, EN_UPDATE,
  EN_MAXTEXT};

use winapi::commctrl::{NM_CUSTOMDRAW};

/// A magic number to identify the NWG subclass that dispatches events
const EVENTS_DISPATCH_ID: UINT_PTR = 2465;

/**
    A procedure signature that takes raw message parameters and output a EventArgs structure.
    Can return None if the parameters could not be parsed
*/
pub type UnpackProc = Fn(HWND, UINT, WPARAM, LPARAM) -> Option<EventArgs>;

/**
    A procedure signature that takes raw message parameters and output a Handle
    Can return None if the handle could not be parsed
*/
pub type HandleProc = Fn(HWND, UINT, WPARAM, LPARAM) -> Option<AnyHandle>;

/**
    An enum that define events that can be used by NWG
*/
#[derive(Clone, Copy)]
pub enum Event {
    /// A special identifier that catches every system messages
    Any,

    /// Wrap a single system message identified by the first paramater
    Single(UINT, &'static UnpackProc, &'static HandleProc),

    /// Wrap a group of system messages identified by the first paramater
    Group(&'static [UINT], &'static UnpackProc, &'static HandleProc)
}

impl PartialEq for Event {
    fn eq(&self, other: &Event) -> bool {
        use std::collections::hash_map::DefaultHasher;
        let (mut s1, mut s2) = (DefaultHasher::new(), DefaultHasher::new());
        self.hash(&mut s1);
        other.hash(&mut s2);
        s1.finish() == s2.finish()
    }
}

impl Eq for Event {}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Event::Any => write!(f, "Any"),
            &Event::Single(id, _, _) => write!(f, "Single event MSG={}", id),
            &Event::Group(ids, _, _) => write!(f, "Grouped event MSG={:?}", ids),
        }
    }
}

impl Hash for Event {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            &Event::Any => 1.hash(state),
            &Event::Single(id, fnptr1, fnptr2) => { id.hash(state); hash_fn_ptr(&fnptr1, state); hash_fn_ptr(&fnptr2, state); }
            &Event::Group(ids, fnptr1, fnptr2) => { ids.hash(state); hash_fn_ptr(&fnptr1, state); hash_fn_ptr(&fnptr2, state); }
        }
    }
}

/// UnpackProc for events that have no arguments
pub fn event_unpack_no_args(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> { Some(EventArgs::None) }

/// HandleProc for events that simply wrap the hwnd parameter
pub fn hwnd_handle(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<AnyHandle> { Some(AnyHandle::HWND(hwnd)) }

/// HandleProc for events using a WM_COMMAND message. Will return None if cmd do not match the WM_COMMAND code
/// Should be used in a closure like this `&|h,m,w,l|{ command_handle(h,m,w,l,BN_CLICKED) }`
pub fn command_handle(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM, cmd: WORD) -> Option<AnyHandle> { 
    let ncode = HIWORD(w as DWORD);
    if ncode == cmd {
        let nhandle = unsafe{ mem::transmute(l) };
        Some(AnyHandle::HWND(nhandle)) 
    } else {
        None
    }
}

/// Same as `HandleProc`, but accepts a list of command ids instead of one
pub fn command_2_handle(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM, cmd1: WORD, cmd2: WORD) -> Option<AnyHandle> {
    let ncode = HIWORD(w as DWORD);
    if cmd1 == ncode || cmd2 == ncode {
        let nhandle = unsafe{ mem::transmute(l) };
        Some(AnyHandle::HWND(nhandle)) 
    } else {
        None
    }
}

/// HandleProc for events using a WM_NOTIFY message. Will return None if cmd do not match the WM_COMMAND code
pub fn notify_handle(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM, cmd: DWORD) -> Option<AnyHandle> {
    let nmhdr: &NMHDR = unsafe{ mem::transmute(l) };
    if nmhdr.code == cmd {
        Some(AnyHandle::HWND(nmhdr.hwndFrom)) 
    } else {
        None
    }
}

/// HandleProc for events using a WM_NOTIFY message. Will return None if cmd do not match the WM_COMMAND code
pub fn notify_2_handle(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM, cmd1: DWORD, cmd2: DWORD) -> Option<AnyHandle> {
    let nmhdr: &NMHDR = unsafe{ mem::transmute(l) };
    if cmd1 == nmhdr.code || cmd2 == nmhdr.code {
        Some(AnyHandle::HWND(nmhdr.hwndFrom)) 
    } else {
        None
    }
}

fn menuitem_handle(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<AnyHandle> {
    unsafe{
        let parent_menu: HMENU = mem::transmute(l);
        Some(AnyHandle::HMENU_ITEM(parent_menu, get_menu_id(parent_menu, w as c_int)))
    }
}

// Definition of common system events
pub const Destroyed: Event = Event::Single(NWG_DESTROY, &event_unpack_no_args, &hwnd_handle);
pub const Paint: Event = Event::Single(WM_PAINT, &event_unpack_no_args, &hwnd_handle);
pub const Closed: Event = Event::Single(WM_CLOSE, &event_unpack_no_args, &hwnd_handle);
pub const Moved: Event = Event::Single(WM_MOVE, &unpack_move, &hwnd_handle);
pub const KeyDown: Event = Event::Single(WM_KEYDOWN, &unpack_key, &hwnd_handle);
pub const KeyUp: Event = Event::Single(WM_KEYUP, &unpack_key, &hwnd_handle);
pub const Resized: Event = Event::Group(&[WM_SIZING, WM_SIZE, WM_EXITSIZEMOVE], &unpack_size, &hwnd_handle);
pub const Char: Event = Event::Group(&[WM_UNICHAR, WM_CHAR], &unpack_char, &hwnd_handle);
pub const MouseUp: Event = Event::Group(&[WM_LBUTTONUP, WM_RBUTTONUP, WM_MBUTTONUP], &unpack_mouseclick, &hwnd_handle);
pub const MouseDown: Event = Event::Group(&[WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_MBUTTONDOWN], &unpack_mouseclick, &hwnd_handle);
pub const MouseMove: Event = Event::Single(WM_MOUSEMOVE, &unpack_mousemove, &hwnd_handle);

// Button events
const btnclick_h:&'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,BN_CLICKED)};
pub const BtnClick: Event = Event::Single(WM_COMMAND, &event_unpack_no_args,btnclick_h);

const btndoubleclick_h:&'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,BN_DBLCLK)};
pub const BtnDoubleClick: Event = Event::Single(WM_COMMAND, &event_unpack_no_args, btndoubleclick_h);


const btnfocus_h:&'static HandleProc = &|h,m,w,l|{ command_2_handle(h,m,w,l,BN_SETFOCUS,BN_KILLFOCUS) };
pub const BtnFocus: Event = Event::Single(WM_COMMAND, &unpack_btn_focus, &btnclick_h);

// Combobox events
const cbnfocus_h:&'static HandleProc = &|h,m,w,l|{ command_2_handle(h,m,w,l,CBN_SETFOCUS,CBN_KILLFOCUS) };
pub const CbnFocus: Event = Event::Single(WM_COMMAND, &unpack_cbn_focus,cbnfocus_h);

const cbnselectionchanged_h:&'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,CBN_SELCHANGE) };
pub const CbnSelectionChanged: Event = Event::Single(WM_COMMAND, &event_unpack_no_args, cbnselectionchanged_h);

// Static events
const stnclick_h: &'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,STN_CLICKED) };
pub const StnClick: Event = Event::Single(WM_COMMAND, &event_unpack_no_args,stnclick_h);

const stndoubleclick_h: &'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,STN_DBLCLK) };
pub const StnDoubleClick: Event = Event::Single(WM_COMMAND, &event_unpack_no_args, stndoubleclick_h);

// Datepicker events
const datechanged_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, DTN_CLOSEUP) };
pub const DateChanged: Event = Event::Single(WM_NOTIFY, &event_unpack_no_args,datechanged_h);

// Listbox events
const lbnselectionchange_h: &'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,LBN_SELCHANGE) };
pub const LbnSelectionChanged: Event = Event::Single(WM_COMMAND, &event_unpack_no_args, lbnselectionchange_h);

const lbndoubleclick_h: &'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,LBN_DBLCLK) };
pub const LbnDoubleClick: Event = Event::Single(WM_COMMAND, &event_unpack_no_args, lbndoubleclick_h);

const lbnfocus_h: &'static HandleProc = &|h,m,w,l|{ command_2_handle(h,m,w,l,LBN_SETFOCUS,LBN_KILLFOCUS) };
pub const LbnFocus: Event = Event::Single(WM_COMMAND, &unpack_lbn_focus, lbnfocus_h);

// Textedit events
const envaluechanged_h: &'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,EN_UPDATE) };
pub const EnValueChanged: Event = Event::Single(WM_COMMAND, &event_unpack_no_args, envaluechanged_h);
const enlimit_h: &'static HandleProc = &|h,m,w,l|{ command_handle(h,m,w,l,EN_MAXTEXT) };
pub const EnLimit: Event = Event::Single(WM_COMMAND, &event_unpack_no_args, enlimit_h);
const enfocus_h: &'static HandleProc = &|h,m,w,l|{ command_2_handle(h,m,w,l,EN_SETFOCUS,EN_KILLFOCUS) };
pub const EnFocus: Event = Event::Single(WM_COMMAND, &unpack_en_focus, enfocus_h);

// Timer events
const timertick_h: &'static HandleProc = &|h,m,w,l|{ Some( AnyHandle::Custom(any::TypeId::of::<Timer>(), w as usize) ) };
pub const TimerTick: Event = Event::Single(WM_TIMER, &event_unpack_no_args, timertick_h);

// Menu item events
pub const MenuTrigger: Event = Event::Single(WM_MENUCOMMAND, &event_unpack_no_args, &menuitem_handle);

// TreeView events
const treeviewselectionchanged_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, TVN_SELCHANGEDW) };
pub const TreeViewSelectionChanged: Event = Event::Single(WM_NOTIFY, &event_unpack_no_args, treeviewselectionchanged_h);

const treeviewclick_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, NM_CLICK) };
pub const TreeViewClick: Event = Event::Single(WM_NOTIFY, &event_unpack_no_args, treeviewselectionchanged_h);

const treeviewdoubleclick_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, NM_DBLCLK) };
pub const TreeViewDoubleClick: Event = Event::Single(WM_NOTIFY, &event_unpack_no_args,treeviewselectionchanged_h );

const treeviewfocus_h: &'static HandleProc = &|h,m,w,l|{ notify_2_handle(h,m,w,l, NM_KILLFOCUS, NM_SETFOCUS) };
pub const TreeViewFocus: Event = Event::Single(WM_NOTIFY, &unpack_tree_focus, treeviewselectionchanged_h);

const treeviewdeleteitem_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, TVN_DELETEITEMW) };
pub const TreeViewDeleteItem: Event = Event::Single(WM_NOTIFY, &unpack_tree_focus, treeviewdeleteitem_h);

const treeviewitemchanged: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, TVN_ITEMCHANGEDW) };
pub const TreeViewItemChanged: Event = Event::Single(WM_NOTIFY, &unpack_tree_focus, treeviewitemchanged);

const treeviewitemchanging_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, TVN_ITEMCHANGINGW) };
pub const TreeViewItemChanging: Event = Event::Single(WM_NOTIFY, &unpack_tree_focus, treeviewitemchanging_h);

const treeviewitemexpanded_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, TVN_ITEMEXPANDEDW) };
pub const TreeViewItemExpanded: Event = Event::Single(WM_NOTIFY, &unpack_tree_focus, treeviewitemexpanded_h);

const treeviewitemexpanding_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, TVN_ITEMEXPANDINGW) };
pub const TreeViewItemExpanding: Event = Event::Single(WM_NOTIFY, &unpack_tree_focus, treeviewitemexpanding_h);

// ListView events
// ListView will send this event to it's parent
const listviewcustomdraw_h: &'static HandleProc = &|h,m,w,l|{ notify_handle(h,m,w,l, NM_CUSTOMDRAW) };
pub const ListViewCustomDraw: Event = Event::Single(WM_NOTIFY, &unpack_raw, listviewcustomdraw_h);

// No unpack
fn unpack_raw(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    Some(EventArgs::Raw(msg,w,l))
}

// Event unpackers for the events defined above
fn unpack_move(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    use winapi::{LOWORD, HIWORD};
    
    let (x, y) = (LOWORD(l as u32), HIWORD(l as u32));
    Some(EventArgs::Position(x as i32, y as i32))
}

fn unpack_size(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    use winapi::RECT;
    use user32::GetClientRect;

    let mut r: RECT = unsafe{ mem::uninitialized() };

    unsafe{ GetClientRect(hwnd, &mut r); }
    let w: u32 = (r.right-r.left) as u32;
    let h: u32 = (r.bottom-r.top) as u32;

    Some(EventArgs::Size(w, h))
}

fn unpack_char(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    use winapi::UNICODE_NOCHAR;

    if w == UNICODE_NOCHAR { 
      return None; 
    } 

    if let Some(c) = ::std::char::from_u32(w as u32) {
      Some( EventArgs::Char( c ) )
    } else {
      None
    }
}

fn unpack_mousemove(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    use winapi::{GET_X_LPARAM, GET_Y_LPARAM};
    let x = GET_X_LPARAM(l) as i32; 
    let y = GET_Y_LPARAM(l) as i32;
    Some(EventArgs::Position(x, y))
}

fn unpack_mouseclick(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
  use defs::MouseButton;
  use winapi::{GET_X_LPARAM, GET_Y_LPARAM};

  let btn = match msg {
    WM_LBUTTONUP | WM_LBUTTONDOWN => MouseButton::Left,
    WM_RBUTTONUP | WM_RBUTTONDOWN => MouseButton::Right,
    WM_MBUTTONUP | WM_MBUTTONDOWN => MouseButton::Middle,
    _ => MouseButton::Left
  };

  let x = GET_X_LPARAM(l) as i32; 
  let y = GET_Y_LPARAM(l) as i32;

  Some(EventArgs::MouseClick{btn: btn, pos: (x, y)})
}

fn unpack_key(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
   Some(EventArgs::Key(w as u32))
}

fn unpack_tree_focus(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    let nmhdr: &NMHDR = unsafe{ mem::transmute(l) };
    Some(EventArgs::Focus(nmhdr.code==NM_SETFOCUS))
}

fn unpack_btn_focus(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    Some(EventArgs::Focus(HIWORD(w as DWORD)==BN_SETFOCUS))
}

fn unpack_cbn_focus(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    Some(EventArgs::Focus(HIWORD(w as DWORD)==CBN_SETFOCUS))
}

fn unpack_lbn_focus(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    Some(EventArgs::Focus(HIWORD(w as DWORD)==LBN_SETFOCUS))
}

fn unpack_en_focus(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> Option<EventArgs> {
    Some(EventArgs::Focus(HIWORD(w as DWORD)==EN_SETFOCUS))
}

/**
  Proc that dispatches the NWG events
*/
#[allow(unused_variables)]
unsafe extern "system" fn process_events<ID: Hash+Clone+'static>(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM, id: UINT_PTR, data: DWORD_PTR) -> LRESULT {
    use comctl32::DefSubclassProc;
    use low::defs::{NWG_CUSTOM_MIN, NWG_CUSTOM_MAX};

    let inner: &mut UiInner<ID> = mem::transmute(data);
    let inner_id: u64;

    let trigger_event = |inner: &mut UiInner<ID>, evt: &Event, get_handle: &HandleProc, get_params: &UnpackProc| {
        if let Some(handle) = (get_handle)(hwnd, msg, w, l) {
            if let Some(inner_id) = inner.inner_id_from_handle( &handle ) {
                if let Some(args) = get_params(hwnd, msg, w, l) {
                    inner.trigger(inner_id, *evt, args);
                }
            }
        }
    };

    if let Some(events) = inner.event_handlers(msg as u32) {
        for event in events.iter() {
            match event {
                &Event::Single(_, p, h) | &Event::Group(_, p, h) => trigger_event(inner, event, h, p),
                &Event::Any => unreachable!() // Any event is not stored by bind
            }
        }
    }

    // Trigger the `Any` event 
    if msg < NWG_CUSTOM_MIN || msg > NWG_CUSTOM_MAX {
      if let Some(inner_id) = inner.inner_id_from_handle( &AnyHandle::HWND(hwnd) ) {
        inner.trigger(inner_id, Event::Any, EventArgs::Raw(msg, w, l));
      }
    }

    DefSubclassProc(hwnd, msg, w, l)
}

/**
    Add a subclass that dispatches the system event to the application callbacks to a window control.
*/
pub fn hook_window_events<ID: Hash+Clone+'static>(uiinner: &mut UiInner<ID>, handle: HWND) { unsafe {
  // While definitely questionable in term of safety, the reference to the UiInner is actually (always)
  // a raw pointer belonging to a Ui. Also, when the Ui goes out of scope, every window control
  // gets destroyed BEFORE the UiInner, this guarantees that uinner lives long enough.
  let ui_inner_raw: *mut UiInner<ID> = uiinner as *mut UiInner<ID>;
  set_window_subclass(handle, Some(process_events::<ID>), EVENTS_DISPATCH_ID, mem::transmute(ui_inner_raw));
}}

/**
  Remove a subclass and free the associated data
*/
pub fn unhook_window_events<ID: Hash+Clone+'static>(handle: HWND) { unsafe {
  use comctl32::RemoveWindowSubclass;
  use winapi::{TRUE, DWORD_PTR};

  let mut data: DWORD_PTR = 0;
  if get_window_subclass(handle, Some(process_events::<ID>), EVENTS_DISPATCH_ID, &mut data) == TRUE {
    RemoveWindowSubclass(handle, Some(process_events::<ID>), EVENTS_DISPATCH_ID);
  }
}}

/**
    Dispatch the messages waiting the the system message queue to the associated Uis. This includes NWG custom messages.

    Return once a quit event was received.
*/
#[inline(always)]
pub unsafe fn dispatch_events() {
  use winapi::MSG;
  use user32::{GetMessageW, TranslateMessage, DispatchMessageW};

  let mut msg: MSG = mem::uninitialized();
  while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
      TranslateMessage(&msg); 
      DispatchMessageW(&msg); 
      // TODO dispatch events sent from other thread / other processes ( after first stable release )
  }
}

/**
    Send a WM_QUIT to the system queue. Breaks the dispatch_events loop.
*/
#[inline(always)]
pub unsafe fn exit() {
  use user32::PostMessageW;
  use winapi::WM_QUIT;

  PostMessageW(ptr::null_mut(), WM_QUIT, 0, 0);
}

/**
    Hash the function pointer of an events. Assumes the pointer as a size of [usize; 2].
    There's a test that check this.
*/
#[inline(always)]
pub fn hash_fn_ptr<T: Sized, H: Hasher>(fnptr: &T, state: &mut H) {
    unsafe{
        let ptr_v: [usize; 2] = mem::transmute_copy(fnptr);
        ptr_v.hash(state);
    }
}

// GetWindowSubclass workaround

#[cfg(target_env="gnu")]
mod hackyty_hack {
    use std::sync::Mutex;
    use std::collections::HashSet;
    use winapi::{HWND, SUBCLASSPROC, UINT_PTR, DWORD_PTR, BOOL};

    struct ClassSetMutexPtr(*mut Mutex<HashSet<HWND>>);
    unsafe impl Sync for ClassSetMutexPtr {}

    static mut subclass_map: Option<ClassSetMutexPtr> = None;

    pub unsafe fn get_window_subclass(handle: HWND, proc_: SUBCLASSPROC, id: UINT_PTR, data: *mut DWORD_PTR) -> BOOL {
        // GNU cannot link the GetWindowSubclass function, so we have to use a hacky workaround
        let (ok, need_free) = if let Some(ref ptr) = subclass_map {
            let ClassSetMutexPtr(ptr) = *ptr;
            let classes_mut: &mut Mutex<HashSet<HWND>> = &mut *ptr;
            let mut classes = classes_mut.lock().unwrap();
            (classes.remove(&handle) as BOOL, classes.is_empty())
        } else {
            (0, false)
        };

        // Free the class set it is empty
        if need_free {
            if let Some(ptr) = subclass_map.take() {
                let ClassSetMutexPtr(ptr) = ptr;
                let classes_mut = Box::from_raw(ptr);
                // classes_mut freed here
            }
        }

        ok
    }

    pub unsafe fn set_window_subclass(handle: HWND, proc_: SUBCLASSPROC, id: UINT_PTR, data: DWORD_PTR) -> BOOL {
        use comctl32::SetWindowSubclass;

        // I hope the creation of the set will not cause race condition...
        if subclass_map.is_none() {
            let set_mut = Mutex::new(HashSet::with_capacity(64));
            let set_mut_ptr = Box::into_raw(Box::new(set_mut));
            subclass_map = Some( ClassSetMutexPtr(set_mut_ptr) );
        }

        // Insert the handle in the class set
        if let Some(ref ptr) = subclass_map {
            let ClassSetMutexPtr(ptr) = *ptr;
            let classes_mut: &mut Mutex<HashSet<HWND>> = &mut *ptr;
            classes_mut.lock().unwrap().insert(handle);
        }

        SetWindowSubclass(handle, proc_, id, data)
    }
}

#[cfg(target_env="gnu")] use self::hackyty_hack::{get_window_subclass, set_window_subclass};


#[cfg(target_env="msvc")] use winapi::{SUBCLASSPROC, BOOL};

#[cfg(target_env="msvc")]
#[inline(always)]
unsafe fn get_window_subclass(handle: HWND, proc_: SUBCLASSPROC, id: UINT_PTR, data: *mut DWORD_PTR) -> BOOL {
    use comctl32::GetWindowSubclass;

    // No workaround for msvc
    GetWindowSubclass(handle, proc_, id, data)
}

#[cfg(target_env="msvc")]
#[inline(always)]
unsafe fn set_window_subclass(handle: HWND, proc_: SUBCLASSPROC, id: UINT_PTR, data: DWORD_PTR) -> BOOL {
    use comctl32::SetWindowSubclass;

    // No workaround for msvc
    SetWindowSubclass(handle, proc_, id, data)
}
