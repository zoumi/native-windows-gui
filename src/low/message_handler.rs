/*!
    A message-only window that dispatch events not targeted to any control
*/

use std::mem;
use std::ptr;
use std::hash::Hash;
use std::marker::PhantomData;
use std::any::Any;

use winapi::{HWND, UINT, WPARAM, LPARAM, LRESULT};

use ui::UiInner;
use low::other_helper::to_utf16;
use error::{Error, SystemError};

/// Unique class name that identify the nwg message-only windows.
const MESSAGE_HANDLE_CLASS_NAME: &'static str = "NWG_MESSAGE";

/**
    Object that dispatch events not targeted at any control.

    No automatic resources freeing, `MessageHandle.free` must be called before the struct goes out of scope.
*/
pub struct MessageHandler<ID: Hash+Clone+'static> {
    pub hwnd: HWND,
    pub last_error: Option<Error>,
    pub p: PhantomData<ID>
}

impl<ID: Hash+Clone+'static> MessageHandler<ID> {

    /**
        Create a new message handle. 

        * If the window creation was successful, returns the new message handler
        * If the system was not capable to create the window, returns a `Error::System`
    */
    pub fn new() -> Result<MessageHandler<ID>, Error> {
        let hwnd_result = unsafe{ create_message_only_window::<ID>() };
        match hwnd_result {
            Ok(h) => 
            Ok( 
                MessageHandler::<ID>{ 
                    hwnd: h, 
                    last_error: None,
                    p: PhantomData,
                } 
            ),
            Err(e) => Err(Error::System(e))
        }
    }

    /**
        Execute the waiting custom commands in the message queue.

        * Returns `Ok(())` if everything was executed without errors
        * Returns `Err(Error)` if an error was encountered while executing the waiting events.
          The following events will not be touched.
    */
    pub fn commit(&mut self) -> Result<(), Error> {
        use winapi::{MSG, PM_REMOVE};
        use user32::{PeekMessageW, DispatchMessageW};
        use low::defs::{NWG_CUSTOM_MAX, NWG_CUSTOM_MIN, COMMIT_FAILED};

        let ok = unsafe{
            let mut ok = true;
            let mut msg: MSG = mem::uninitialized();
            while PeekMessageW(&mut msg, self.hwnd, NWG_CUSTOM_MIN, NWG_CUSTOM_MAX, PM_REMOVE) != 0 {
                if DispatchMessageW(&msg) == COMMIT_FAILED {
                    ok = false;
                    break;
                }
            }

            ok
        };

        if ok {
            Ok(())
        } else {
            // if the commit failed, it is certain that last_error is not null
            Err(self.last_error.take().unwrap())
        }
    }

    /**
        Post a message to the message only queue.
    */
    pub fn post(&self, ui: *mut UiInner<ID>, msg: UINT, data: Box<Any>) {
        use user32::PostMessageW;

        unsafe {
            let ui_wparam: WPARAM = mem::transmute(ui);
            let data_ptr: *mut Any = Box::into_raw(data);
            let data_ptr: *mut *mut Any = Box::into_raw(Box::new(data_ptr));
            let data_lparam: LPARAM = mem::transmute(data_ptr);
            PostMessageW(self.hwnd, msg, ui_wparam, data_lparam);
        }
    }

    /**
        Destroy the underlying window and try to free the class. Errors are ignored.

        If multiple UI were created, the class destruction will silently fail (and it's ok).
        The class will be freed when the last Ui is freed.
    */
    pub fn free(&self) {
        use kernel32::GetModuleHandleW;
        use user32::{DestroyWindow, UnregisterClassW};

        let class_name = to_utf16(MESSAGE_HANDLE_CLASS_NAME);

        unsafe{ DestroyWindow(self.hwnd); }
        unsafe{ UnregisterClassW(class_name.as_ptr(), GetModuleHandleW(ptr::null_mut())); }
    }
}

/** 
    Proc for the nwg Ui message-only window. Basically, it dispatches async events to the inner ui.

    * `msg` holds the nwg command identifier
    * `w`   holds a pointer to the Ui
    * `l`   holds the parameters for the messages
*/
#[allow(unused_variables)]
unsafe extern "system" fn message_window_proc<ID: Hash+Clone+'static>(hwnd: HWND, msg: UINT, w: WPARAM, l: LPARAM) -> LRESULT {
    use user32::{DefWindowProcW};
    use low::defs::{NWG_PACK_USER_VALUE, NWG_PACK_CONTROL, NWG_UNPACK, NWG_BIND, NWG_UNBIND, NWG_TRIGGER, NWG_PACK_RESOURCE, COMMIT_SUCCESS, COMMIT_FAILED};
    use low::defs::{PackUserValueArgs, PackControlArgs, UnpackArgs, BindArgs, UnbindArgs, PackResourceArgs, TriggerArgs};

    let ui: &mut UiInner<ID> = mem::transmute(w);
    let args: *mut *mut Any = mem::transmute::<LPARAM, *mut *mut Any>(l);
        match msg {
        WM_NOTIFY => {println!("WM_NOTIFY get in message_handler");return COMMIT_SUCCESS;},
        _ => {}
        }

    // Eval NWG messages
    let (processed, error): (bool, Option<Error>) = match msg {
        NWG_PACK_USER_VALUE => {
            let args: Box<Any> = Box::from_raw(*Box::from_raw(args));
            if let Ok(params) = args.downcast::<PackUserValueArgs<ID>>() {
                (true, ui.pack_user_value(*params))
            } else {
                panic!("Could not downcast command PACK_USER_VALUE args into a PackUserValueArgs struct.");
            }
        },
        NWG_PACK_CONTROL => {
            let args: Box<Any> = Box::from_raw(*Box::from_raw(args));
            if let Ok(params) = args.downcast::<PackControlArgs<ID>>() {
                (true, ui.pack_control(*params))
            } else {
                panic!("Could not downcast command PACK_CONTROL args into a PackControlArgs struct.");
            }
        },
        NWG_PACK_RESOURCE => {
            let args: Box<Any> = Box::from_raw(*Box::from_raw(args));
            if let Ok(params) = args.downcast::<PackResourceArgs<ID>>() {
                (true, ui.pack_resource(*params))
            } else {
                panic!("Could not downcast command NWG_UNBIND args into a UnbindArgs struct.");
            }
        },
        NWG_UNPACK => {
            let args: Box<Any> = Box::from_raw(*Box::from_raw(args));
            if let Ok(params) = args.downcast::<UnpackArgs>() {
                (true, ui.unpack(*params))
            } else {
                panic!("Could not downcast command NWG_UNPACK_CONTROL args into a inner id.");
            }
        },
        NWG_BIND => {
            let args: Box<Any> = Box::from_raw(*Box::from_raw(args));
            if let Ok(params) = args.downcast::<BindArgs<ID>>() {
                (true, ui.bind(*params))
            } else {
                panic!("Could not downcast command NWG_BIND args into a BindArgs struct.");
            }
        },
        NWG_UNBIND => {
            let args: Box<Any> = Box::from_raw(*Box::from_raw(args));
            if let Ok(params) = args.downcast::<UnbindArgs>() {
                (true, ui.unbind(*params))
            } else {
                panic!("Could not downcast command NWG_UNBIND args into a UnbindArgs struct.");
            }
        },
        NWG_TRIGGER => {
            let args: Box<Any> = Box::from_raw(*Box::from_raw(args));
            if let Ok(params) = args.downcast::<TriggerArgs>() {
                let TriggerArgs{ id, event, args } = *params;
                (true, ui.trigger(id, event, args))
            } else {
                panic!("Could not downcast command NWG_UNBIND args into a UnbindArgs struct.");
            }
        },
        _ => (false, None)
    };

    if processed {
        if error.is_some() {
            ui.messages.last_error = error;
            COMMIT_FAILED
        } else {
            COMMIT_SUCCESS
        }
    } else {
        DefWindowProcW(hwnd, msg, w, l)
    }
}

/// Create a unique class name for a inner window based on its type.
/// If a unique class is not used per Ui with different type, a NWG message could get posted
/// to the wrong message proc and then the application would crash
fn class_name<ID: Hash+Clone+'static>() -> String {
    format!("{}-{:?}", MESSAGE_HANDLE_CLASS_NAME, ::std::any::TypeId::of::<ID>())
}


/**
    Create the message handler window class.

    * If the class creation is successful or the class already exists, returns `Ok`
    * If there was an error while creating the class, returns a `Err(SystemError::UiCreation)`
*/
unsafe fn setup_class<ID: Hash+Clone+'static>() -> Result<(), SystemError> {
    use low::window_helper::{SysclassParams, build_sysclass};
    let params = SysclassParams{ 
        class_name: class_name::<ID>(), 
        sysproc: Some(message_window_proc::<ID>),
        background: Some(ptr::null_mut()), style: None
    };
    
    if let Err(_) = build_sysclass(params) {
        Err(SystemError::UiCreation)
    } else {
        Ok(())
    }
}

/**
    Create a NWG message-only window.

    * If the window creation is successful, returns `Ok(window_handle)`  
    * If the window creation fails, returns `Err(SystemError::UiCreation)`
*/
unsafe fn create_window<ID: Hash+Clone+'static>() -> Result<HWND, SystemError> {
    use kernel32::GetModuleHandleW;
    use user32::CreateWindowExW;
    use winapi::HWND_MESSAGE;

    let hmod = GetModuleHandleW(ptr::null_mut());
    if hmod.is_null() { return Err(SystemError::UiCreation); }

    let class_name = to_utf16(&class_name::<ID>());
    let window_name = to_utf16("");

    let handle = CreateWindowExW (
        0, 
        class_name.as_ptr(), window_name.as_ptr(), 
        0, 0, 0, 0, 0,
        HWND_MESSAGE,
        ptr::null_mut(),
        hmod,
        ptr::null_mut()
    );

    if handle.is_null() {
        Err(SystemError::UiCreation)
    } else {
        Ok(handle)
    }
}

/**
    Create a message only window for an UI. See `setup_class` && `create_window` docs for more info.
*/
unsafe fn create_message_only_window<ID: Hash+Clone+'static>() -> Result<HWND, SystemError> {
    match setup_class::<ID>() {
        Ok(_) => create_window::<ID>(),
        Err(e) => Err(e)
    }
}
