// VoiceType - Keyboard hook using crossbeam channel
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static PRESSING: AtomicBool = AtomicBool::new(false);
static SENDER: OnceLock<crossbeam_channel::Sender<bool>> = OnceLock::new();

pub fn start_hook() -> crossbeam_channel::Receiver<bool> {
    let (tx, rx) = crossbeam_channel::unbounded();
    let _ = SENDER.set(tx);

    std::thread::spawn(|| {
        unsafe {
            use winapi::um::winuser::*;
            unsafe extern "system" fn hook_proc(code: i32, wparam: usize, lparam: isize) -> isize {
                if code >= 0 {
                    let kbd = lparam as *const KBDLLHOOKSTRUCT;
                    let vk = (*kbd).vkCode;
                    let is_down = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
                    let is_up = wparam == WM_KEYUP as usize || wparam == WM_SYSKEYUP as usize;

                    if vk == 0x78 { // F9
                        if is_down && !PRESSING.load(Ordering::SeqCst) {
                            PRESSING.store(true, Ordering::SeqCst);
                            eprintln!("[Hook] F9 DOWN");
                            if let Some(tx) = SENDER.get() { let _ = tx.send(true); }
                        } else if is_up && PRESSING.load(Ordering::SeqCst) {
                            PRESSING.store(false, Ordering::SeqCst);
                            eprintln!("[Hook] F9 UP");
                            if let Some(tx) = SENDER.get() { let _ = tx.send(false); }
                        }
                    }
                }
                CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
            }
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), std::ptr::null_mut(), 0);
            if hook.is_null() { eprintln!("[Hook] Failed"); return; }
            eprintln!("[Hook] F9 hook installed");
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });

    rx
}

pub fn simulate_paste() -> Result<(), String> {
    use winapi::um::winuser::{keybd_event, KEYEVENTF_KEYUP, VK_CONTROL, GetForegroundWindow, SetForegroundWindow};
    const VK_V: u8 = 0x56;

    std::thread::sleep(std::time::Duration::from_millis(200));

    unsafe {
        keybd_event(VK_CONTROL as u8, 0, 0, 0);
        std::thread::sleep(std::time::Duration::from_millis(50));
        keybd_event(VK_V, 0, 0, 0);
        std::thread::sleep(std::time::Duration::from_millis(50));
        keybd_event(VK_V, 0, KEYEVENTF_KEYUP, 0);
        std::thread::sleep(std::time::Duration::from_millis(50));
        keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
    }
    Ok(())
}
