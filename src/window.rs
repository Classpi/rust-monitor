use crate::{
    AppError,
    eventloop::{Event, EventLoop},
};
use log::debug;
use std::{
    sync::mpsc::Sender,
    thread::{self, JoinHandle},
};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{BeginPaint, EndPaint, HBRUSH, PAINTSTRUCT},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::GetDpiForSystem,
            WindowsAndMessaging::{
                CS_PARENTDC, CS_SAVEBITS, CreateWindowExW, DispatchMessageW, FindWindowExW, GWLP_USERDATA, GetMessageW, GetWindowRect, HWND_TOPMOST, IDC_ARROW, LWA_COLORKEY, LoadCursorW, MSG,
                RegisterClassW, SWP_NOACTIVATE, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, TranslateMessage, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
                WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
            },
        },
    },
    core::w,
};

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy)]
pub struct WindowHandle(pub HWND);
unsafe impl Send for WindowHandle {}
impl From<HWND> for WindowHandle {
    fn from(value: HWND) -> Self {
        Self(value)
    }
}
impl Into<HWND> for WindowHandle {
    fn into(self) -> HWND {
        self.0
    }
}
#[derive(Debug, Default, Copy, Clone)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

#[derive(Debug)]
pub struct Window {
    pub hwnd: WindowHandle,
    pub thread_handle: Option<JoinHandle<()>>,
    pub parent_hwnd: Option<WindowHandle>,
    pub rect: Rect,
    pub event_sender: Sender<Event>,
}
impl Window {
    pub fn init(event_loop: &EventLoop) -> Result<Self, AppError> {
        Self::register_class()?;
        let event_sender = event_loop.event_sender.clone();
        let (result_tx, result_rx) = std::sync::mpsc::channel();
        // window thread
        let thread_handle = thread::Builder::new()
            .name("window_thread".into())
            .spawn(move || {
                let sys_tray = unsafe { FindWindowExW(None, None, w!("Shell_TrayWnd"), None) }.expect("Failed to find SysTray");

                let sys_tray_notify = unsafe { FindWindowExW(Some(sys_tray), None, w!("TrayNotifyWnd"), None) }.expect("Failed to find SysTrayNotify");

                let mut notify_rect: RECT = RECT::default();
                unsafe {
                    GetWindowRect(sys_tray_notify, &mut notify_rect).expect("Failed to get notify rect");
                }
                let dpi = unsafe { GetDpiForSystem() as f32 };
                let scale = dpi / 96.0;

                let height = ((notify_rect.bottom - notify_rect.top - 20) as f32 * scale) as i32;
                let width = (height as f32 * 3.0) as i32;
                let x = notify_rect.left - width - 10;
                let y = notify_rect.top + 10;

                let hwnd = unsafe {
                    CreateWindowExW(
                        WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                        w!("RUST_CAT"),
                        w!("rust_cat"),
                        WS_VISIBLE | WS_POPUP,
                        x,
                        y,
                        width,
                        height,
                        None,
                        None,
                        None,
                        None,
                    )
                    .expect("Failed to create window")
                };
                unsafe {
                    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_COLORKEY);
                    let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, width, height, SWP_NOACTIVATE);
                }

                debug!("实际窗口位置: {:?}", {
                    let mut actual_rect = RECT::default();
                    let _ = unsafe { GetWindowRect(hwnd, &mut actual_rect) };
                    actual_rect
                });

                result_tx
                    .send((
                        hwnd.into(),
                        Some(sys_tray.into()),
                        Rect {
                            x: x as u16,
                            y: y as u16,
                            w: width as u16,
                            h: height as u16,
                        },
                    ))
                    .expect("Failed to send window info");
                drop(result_tx);
                // Start window msg loop

                unsafe {
                    let sender_ptr = Box::into_raw(Box::new(event_sender)) as isize;
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, sender_ptr);
                }
                let mut msg = MSG::default();
                while unsafe { GetMessageW(&mut msg, Some(hwnd), 0, 0).into() } {
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            })
            .unwrap();

        // wait window created
        let (hwnd, parent_hwnd, rect) = result_rx.recv().unwrap();

        Ok(Window {
            thread_handle: Some(thread_handle),
            hwnd,
            parent_hwnd,
            rect,
            event_sender: event_loop.event_sender.clone(),
        })
    }

    pub fn register_class() -> Result<(), AppError> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let wc = WNDCLASSW {
                style: CS_PARENTDC | CS_SAVEBITS,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hInstance: instance.into(),
                lpszClassName: w!("RUST_CAT"),
                lpfnWndProc: Some(wnd_proc),
                hbrBackground: HBRUSH::default(),
                ..Default::default()
            };
            if RegisterClassW(&wc) == 0 {
                return Err(AppError("Error register class :(".into()));
            }
            Ok(())
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;

    let sender_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Sender<Event> };

    if !sender_ptr.is_null() {
        let sender = unsafe { &*sender_ptr };

        match msg {
            WM_PAINT => {
                unsafe {
                    let mut ps = PAINTSTRUCT::default();
                    let _hdc = BeginPaint(hwnd, &mut ps);
                    let _ = sender.send(Event::Paint);
                    let _ = EndPaint(hwnd, &ps);
                }
                return LRESULT(0);
            }
            WM_SIZE => {
                let width = (lparam.0 & 0xFFFF) as u32;
                let height = ((lparam.0 >> 16) & 0xFFFF) as u32;
                let _ = sender.send(Event::Resize(width, height));
            }
            WM_CLOSE => {
                let _ = sender.send(Event::Close);
            }
            WM_KEYDOWN => {
                let _ = sender.send(Event::KeyDown(wparam.0 as u32));
            }
            WM_MOUSEMOVE => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                let _ = sender.send(Event::MouseMove(x, y));
            }
            WM_DESTROY => {
                unsafe {
                    let _ = Box::from_raw(sender_ptr);
                    PostQuitMessage(0);
                };
                return LRESULT(0);
            }
            _ => unsafe {
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE);
                DefWindowProcW(hwnd, msg, wparam, lparam);
            },
        }
        return LRESULT(0);
    }

    LRESULT(10086)
}

pub fn get_tray_notify() -> HWND {
    let sys_tray = unsafe { FindWindowExW(None, None, w!("Shell_TrayWnd"), None) }.expect("Failed to find SysTray");
    let sys_tray_notify = unsafe { FindWindowExW(Some(sys_tray), None, w!("TrayNotifyWnd"), None) }.expect("Failed to find SysTrayNotify");
    sys_tray_notify
}
