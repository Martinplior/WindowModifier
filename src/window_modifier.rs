use egui::Widget;
use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND, LPARAM},
        System::{
            ProcessStatus::GetModuleFileNameExW,
            Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ},
        },
        UI::WindowsAndMessaging::{
            AdjustWindowRectEx, EnumChildWindows, GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetMenu,
            GetWindowLongPtrW, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindow,
            IsWindowVisible, SWP_ASYNCWINDOWPOS, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            SetWindowPos, WINDOW_EX_STYLE, WINDOW_STYLE,
        },
    },
    core::BOOL,
};

#[derive(Debug)]
pub struct WindowModifier {
    window_info_list: Vec<WindowInfo>,
}

impl WindowModifier {
    pub fn new() -> Self {
        let window_info_list = Vec::with_capacity(1024);
        Self { window_info_list }
    }

    pub fn update_window_info_list(&mut self) {
        enumerate_windows(&mut self.window_info_list);
    }

    pub fn window_info_list(&self) -> &[WindowInfo] {
        &self.window_info_list
    }
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub pid: u32,
    pub hwnd: HWND,
    pub title: String,
    pub program_path: String,
}

impl WindowInfo {
    pub fn is_valid(&self) -> bool {
        let is_window = unsafe { IsWindow(Some(self.hwnd)) }.as_bool();
        if !is_window {
            return false;
        }
        let pid = {
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(self.hwnd, Some(&mut pid)) };
            pid
        };
        pid == self.pid
    }

    pub fn get_window_rect(&self) -> windows::core::Result<windows::Win32::Foundation::RECT> {
        let mut rect = Default::default();
        unsafe { GetWindowRect(self.hwnd, &mut rect) }?;
        Ok(rect)
    }

    /// returns `[width, height]`
    pub fn get_size(&self) -> windows::core::Result<[i32; 2]> {
        let rect = self.get_window_rect()?;
        Ok([rect.right - rect.left, rect.bottom - rect.top])
    }

    /// returns `[width, height]`
    pub fn get_inner_size(&self) -> windows::core::Result<[i32; 2]> {
        let mut rect = Default::default();
        unsafe { GetClientRect(self.hwnd, &mut rect) }?;
        Ok([rect.right - rect.left, rect.bottom - rect.top])
    }

    /// returns `[x, y]`
    pub fn get_position(&self) -> windows::core::Result<[i32; 2]> {
        let rect = self.get_window_rect()?;
        Ok([rect.left, rect.top])
    }

    /// `size: [width, height]`
    pub fn resize(&self, size: [i32; 2]) -> windows::core::Result<()> {
        let [width, height] = size;
        const DUMMY: i32 = 0;
        unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                DUMMY,
                DUMMY,
                width,
                height,
                SWP_ASYNCWINDOWPOS | SWP_NOZORDER | SWP_NOMOVE,
            )
        }
    }

    /// `size: [width, height]`
    pub fn resize_inner(&self, size: [i32; 2]) -> windows::core::Result<()> {
        let [width, height] = size;
        const DUMMY: i32 = 0;
        let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(self.hwnd, GWL_STYLE) } as _);
        let ex_style = WINDOW_EX_STYLE(unsafe { GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) } as _);
        let has_menu = !unsafe { GetMenu(self.hwnd) }.is_invalid();
        let mut rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };
        unsafe { AdjustWindowRectEx(&mut rect, style, has_menu, ex_style) }?;
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                DUMMY,
                DUMMY,
                width,
                height,
                SWP_ASYNCWINDOWPOS | SWP_NOZORDER | SWP_NOMOVE,
            )
        }
    }

    /// `position: [x, y]`
    pub fn move_to(&self, position: [i32; 2]) -> windows::core::Result<()> {
        let [x, y] = position;
        const DUMMY: i32 = 0;
        unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                x,
                y,
                DUMMY,
                DUMMY,
                SWP_ASYNCWINDOWPOS | SWP_NOZORDER | SWP_NOSIZE,
            )
        }
    }
}

impl WindowInfo {
    pub fn show_ui(&self, ui: &mut egui::Ui) {
        const FONT_SIZE: f32 = crate::app::App::FONT_SIZE;
        egui::Frame::new()
            .inner_margin(3)
            .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY))
            .show(ui, |ui| {
                egui::Grid::new(ui.next_auto_id())
                    .num_columns(1)
                    .striped(true)
                    .show(ui, |ui| {
                        egui::Label::new(
                            egui::RichText::new(format!("PID: {}", self.pid)).size(FONT_SIZE),
                        )
                        .selectable(true)
                        .ui(ui);
                        ui.end_row();
                        egui::Label::new(
                            egui::RichText::new(format!("HWND: {}", self.hwnd.0 as usize))
                                .size(FONT_SIZE),
                        )
                        .selectable(true)
                        .ui(ui);
                        ui.end_row();
                        egui::Label::new(
                            egui::RichText::new(format!("标题: {}", self.title)).size(FONT_SIZE),
                        )
                        .selectable(true)
                        .ui(ui);
                        ui.end_row();
                        egui::Label::new(
                            egui::RichText::new(format!("路径: {}", self.program_path))
                                .size(FONT_SIZE),
                        )
                        .selectable(true)
                        .ui(ui);
                        ui.end_row();
                    })
            });
    }

    pub fn show_selectable_ui(&self, ui: &mut egui::Ui) -> bool {
        const FONT_SIZE: f32 = crate::app::App::FONT_SIZE;
        let mut selected = false;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add_space(60.0);
                selected = egui::Button::new(egui::RichText::new("选择").size(FONT_SIZE))
                    .ui(ui)
                    .clicked();
            });
            self.show_ui(ui);
        });
        selected
    }
}

fn enumerate_windows(window_info_list: &mut Vec<WindowInfo>) {
    unsafe extern "system" fn enum_child_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        const DEFAULT_RETURN_VALUE: BOOL = BOOL(1);
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return DEFAULT_RETURN_VALUE;
        }
        let pid = {
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            pid
        };
        if pid == 0 {
            return DEFAULT_RETURN_VALUE;
        }
        let title = {
            let mut title_buf = [0u16; 1024];
            let title_length = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
            if title_length == 0 {
                return DEFAULT_RETURN_VALUE;
            }
            String::from_utf16_lossy(&title_buf[..title_length as usize])
        };
        let program_path = {
            let mut program_path_buf = [0u16; 1024];
            let Ok(process_handle) = (unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
                    false,
                    pid,
                )
            }) else {
                return DEFAULT_RETURN_VALUE;
            };
            let program_path_length =
                unsafe { GetModuleFileNameExW(Some(process_handle), None, &mut program_path_buf) };
            unsafe { CloseHandle(process_handle) }.unwrap();
            String::from_utf16_lossy(&program_path_buf[..program_path_length as usize])
        };
        let window_info_list = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };
        window_info_list.push(WindowInfo {
            pid,
            hwnd,
            title,
            program_path,
        });
        DEFAULT_RETURN_VALUE
    }

    window_info_list.clear();
    let _ = unsafe {
        EnumChildWindows(
            None,
            Some(enum_child_windows_proc),
            LPARAM(window_info_list as *mut _ as _),
        )
    };
    window_info_list.sort_by_key(|x: &WindowInfo| x.pid);
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_enumerate_windows() {
        let mut window_info_list = Vec::new();
        enumerate_windows(&mut window_info_list);
        window_info_list.iter().for_each(|window_info| {
            println!("{:?}", window_info);
        });
    }
}
