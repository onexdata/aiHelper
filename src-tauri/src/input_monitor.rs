use crate::db::Database;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

enum KeyEvent {
    Char(char),
    Special(&'static str),
}

// --- Windows implementation ---

#[cfg(target_os = "windows")]
mod platform {
    use super::KeyEvent;
    use std::sync::{mpsc::Sender, OnceLock};
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyboardState, ToUnicode,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowTextW, SetWindowsHookExW,
        KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    // VK constants
    const VK_BACK: u32 = 0x08;
    const VK_TAB: u32 = 0x09;
    const VK_RETURN: u32 = 0x0D;
    const VK_SHIFT: u32 = 0x10;
    const VK_CONTROL: u32 = 0x11;
    const VK_MENU: u32 = 0x12;
    const VK_CAPITAL: u32 = 0x14;
    const VK_ESCAPE: u32 = 0x1B;
    const VK_LEFT: u32 = 0x25;
    const VK_UP: u32 = 0x26;
    const VK_RIGHT: u32 = 0x27;
    const VK_DOWN: u32 = 0x28;
    const VK_DELETE: u32 = 0x2E;
    const VK_LSHIFT: u32 = 0xA0;
    const VK_RSHIFT: u32 = 0xA1;
    const VK_LCONTROL: u32 = 0xA2;
    const VK_RCONTROL: u32 = 0xA3;
    const VK_LMENU: u32 = 0xA4;
    const VK_RMENU: u32 = 0xA5;
    const VK_LWIN: u32 = 0x5B;
    const VK_RWIN: u32 = 0x5C;

    static KEY_SENDER: OnceLock<Sender<KeyEvent>> = OnceLock::new();

    fn send_key(evt: KeyEvent) {
        if let Some(sender) = KEY_SENDER.get() {
            let _ = sender.send(evt);
        }
    }

    unsafe extern "system" fn keyboard_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            let msg = w_param.0 as u32;
            if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                let kb = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
                let vk = kb.vkCode;

                match vk {
                    // Modifier-only keys — skip (they affect the next char via GetAsyncKeyState)
                    VK_SHIFT | VK_CONTROL | VK_MENU | VK_CAPITAL
                    | VK_LSHIFT | VK_RSHIFT | VK_LCONTROL | VK_RCONTROL
                    | VK_LMENU | VK_RMENU | VK_LWIN | VK_RWIN => {}

                    // Special keys — send as tags
                    VK_BACK => send_key(KeyEvent::Special("[BS]")),
                    VK_TAB => send_key(KeyEvent::Special("[TAB]")),
                    VK_RETURN => send_key(KeyEvent::Special("[ENTER]")),
                    VK_ESCAPE => send_key(KeyEvent::Special("[ESC]")),
                    VK_LEFT => send_key(KeyEvent::Special("[LEFT]")),
                    VK_UP => send_key(KeyEvent::Special("[UP]")),
                    VK_RIGHT => send_key(KeyEvent::Special("[RIGHT]")),
                    VK_DOWN => send_key(KeyEvent::Special("[DOWN]")),
                    VK_DELETE => send_key(KeyEvent::Special("[DEL]")),

                    // Printable keys — use ToUnicode with real-time modifier state
                    _ => {
                        let mut keyboard_state = [0u8; 256];
                        let _ = GetKeyboardState(&mut keyboard_state);

                        // Fix modifier state with GetAsyncKeyState (real-time hardware state)
                        // GetKeyboardState in hook thread can lag behind actual modifier state
                        if GetAsyncKeyState(VK_SHIFT as i32) < 0 {
                            keyboard_state[VK_SHIFT as usize] = 0x80;
                            if GetAsyncKeyState(VK_LSHIFT as i32) < 0 {
                                keyboard_state[VK_LSHIFT as usize] = 0x80;
                            }
                            if GetAsyncKeyState(VK_RSHIFT as i32) < 0 {
                                keyboard_state[VK_RSHIFT as usize] = 0x80;
                            }
                        } else {
                            keyboard_state[VK_SHIFT as usize] = 0;
                            keyboard_state[VK_LSHIFT as usize] = 0;
                            keyboard_state[VK_RSHIFT as usize] = 0;
                        }

                        if GetAsyncKeyState(VK_CONTROL as i32) < 0 {
                            keyboard_state[VK_CONTROL as usize] = 0x80;
                        } else {
                            keyboard_state[VK_CONTROL as usize] = 0;
                        }

                        if GetAsyncKeyState(VK_MENU as i32) < 0 {
                            keyboard_state[VK_MENU as usize] = 0x80;
                        } else {
                            keyboard_state[VK_MENU as usize] = 0;
                        }

                        // Caps Lock: toggle bit (bit 0) from GetAsyncKeyState
                        if GetAsyncKeyState(VK_CAPITAL as i32) & 1 != 0 {
                            keyboard_state[VK_CAPITAL as usize] = 0x01;
                        } else {
                            keyboard_state[VK_CAPITAL as usize] = 0x00;
                        }

                        let scan = kb.scanCode;
                        let mut buf = [0u16; 4];
                        let result =
                            ToUnicode(vk, scan, Some(&keyboard_state), &mut buf, 0);

                        if result > 0 {
                            if let Some(ch) =
                                char::decode_utf16(buf[..result as usize].iter().copied())
                                    .next()
                                    .and_then(|r| r.ok())
                            {
                                // Skip control chars from Ctrl+key combos (except tab/enter already handled)
                                if !ch.is_control() {
                                    send_key(KeyEvent::Char(ch));
                                }
                            }
                        }
                    }
                }
            }
        }
        unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
    }

    pub fn start_hook_thread(sender: Sender<KeyEvent>) {
        KEY_SENDER.set(sender).ok();

        std::thread::spawn(|| unsafe {
            let _hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
        });
    }

    pub fn get_cursor_pos() -> (i32, i32) {
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        use windows::Win32::Foundation::POINT;

        unsafe {
            let mut pt = POINT::default();
            let _ = GetCursorPos(&mut pt);
            (pt.x, pt.y)
        }
    }

    pub fn get_foreground_title() -> String {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_invalid() {
                return String::new();
            }
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buf);
            if len > 0 {
                String::from_utf16_lossy(&buf[..len as usize])
            } else {
                String::new()
            }
        }
    }

    pub fn get_foreground_app_name() -> String {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };
        use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
        use windows::core::PWSTR;

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_invalid() {
                return String::new();
            }

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return String::new();
            }

            if let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                let mut buf = [0u16; 1024];
                let mut size = buf.len() as u32;
                if QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_WIN32,
                    PWSTR(buf.as_mut_ptr()),
                    &mut size,
                )
                .is_ok()
                {
                    let _ = CloseHandle(handle);
                    let path = String::from_utf16_lossy(&buf[..size as usize]);
                    // "C:\...\Code.exe" -> "Code"
                    let filename = path.rsplit('\\').next().unwrap_or(&path);
                    return filename
                        .strip_suffix(".exe")
                        .or_else(|| filename.strip_suffix(".EXE"))
                        .unwrap_or(filename)
                        .to_string();
                }
                let _ = CloseHandle(handle);
            }

            String::new()
        }
    }

    pub fn get_screen_resolution_inner() -> (i32, i32) {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        unsafe {
            let w = GetSystemMetrics(SM_CXSCREEN);
            let h = GetSystemMetrics(SM_CYSCREEN);
            (w, h)
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::KeyEvent;
    use std::sync::mpsc::Sender;

    pub fn start_hook_thread(_sender: Sender<KeyEvent>) {
        // No-op on non-Windows
    }

    pub fn get_cursor_pos() -> (i32, i32) {
        (0, 0)
    }

    pub fn get_foreground_title() -> String {
        "Unknown".to_string()
    }

    pub fn get_foreground_app_name() -> String {
        "Unknown".to_string()
    }

    pub fn get_screen_resolution_inner() -> (i32, i32) {
        (1920, 1080)
    }
}

pub fn get_screen_resolution() -> (i32, i32) {
    platform::get_screen_resolution_inner()
}

pub fn start_monitoring(db_path: String) {
    let (tx, rx) = mpsc::channel::<KeyEvent>();

    // Thread 1: keyboard hook (Windows only)
    platform::start_hook_thread(tx);

    // Thread 2: monitor thread — polls mouse, receives keys, batches DB writes
    std::thread::spawn(move || {
        let db = match Database::initialize(&db_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Input monitor: failed to open DB: {e}");
                return;
            }
        };

        let mut last_cursor = platform::get_cursor_pos();
        let mut mouse_accum: f64 = 0.0;
        let mut last_mouse_flush = Instant::now();

        let mut last_keystroke_flush = Instant::now();
        // Buffer: app_name -> (latest_window_title, chars)
        let mut keystroke_buf: HashMap<String, (String, String)> = HashMap::new();

        let mut last_app = platform::get_foreground_app_name();
        let mut last_window_title = platform::get_foreground_title();
        let mut window_start = Instant::now();

        loop {
            std::thread::sleep(Duration::from_millis(100));

            // Drain keyboard events — group by app_name
            while let Ok(evt) = rx.try_recv() {
                let app = platform::get_foreground_app_name();
                let title = platform::get_foreground_title();
                let entry = keystroke_buf.entry(app).or_insert_with(|| (title.clone(), String::new()));
                entry.0 = title; // keep latest window title
                match evt {
                    KeyEvent::Char(ch) => entry.1.push(ch),
                    KeyEvent::Special(tag) => entry.1.push_str(tag),
                }
            }

            // Mouse distance
            let cur = platform::get_cursor_pos();
            let dx = (cur.0 - last_cursor.0) as f64;
            let dy = (cur.1 - last_cursor.1) as f64;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 0.5 {
                mouse_accum += dist;
            }
            last_cursor = cur;

            // Window focus tracking — only triggers on app change, not title change
            let current_app = platform::get_foreground_app_name();
            let current_title = platform::get_foreground_title();
            if current_app != last_app {
                let elapsed = window_start.elapsed().as_secs() as i64;
                if elapsed > 0 && !last_app.is_empty() {
                    let _ = db.insert_window_activity(&last_app, &last_window_title, elapsed);
                }
                last_app = current_app;
                last_window_title = current_title;
                window_start = Instant::now();
            } else {
                last_window_title = current_title;
            }

            // Flush keystrokes every 1s
            if last_keystroke_flush.elapsed() >= Duration::from_secs(1) && !keystroke_buf.is_empty() {
                for (app_name, (window_title, chars)) in keystroke_buf.drain() {
                    let _ = db.insert_keystrokes(&chars, &app_name, &window_title);
                }
                last_keystroke_flush = Instant::now();
            }

            // Flush mouse distance every 60s
            if last_mouse_flush.elapsed() >= Duration::from_secs(60) {
                if mouse_accum > 0.0 {
                    let _ = db.insert_mouse_distance(mouse_accum);
                    mouse_accum = 0.0;
                }
                last_mouse_flush = Instant::now();
            }
        }
    });
}
