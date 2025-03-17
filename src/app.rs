use std::sync::Arc;

use eframe::Renderer;
use egui::{ThemePreference, ViewportBuilder, Widget};

use crate::{
    message_dialog, utils,
    window_modifier::{WindowInfo, WindowModifier},
};

pub struct MainApp {
    //
}

impl MainApp {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(self) {
        let native_options = eframe::NativeOptions {
            viewport: ViewportBuilder::default()
                .with_min_inner_size([600.0, 600.0])
                .with_inner_size([1280.0, 720.0]),
            renderer: Renderer::Wgpu,
            ..Default::default()
        };
        eframe::run_native(
            "Window Modifier",
            native_options,
            Box::new(|cc| Ok(Box::new(App::new(cc)))),
        )
        .unwrap();
    }
}

#[derive(Debug, Default)]
struct WindowModificationCache {
    width: i32,
    height: i32,
    inner_width: i32,
    inner_height: i32,
    x: i32,
    y: i32,
}

pub(crate) struct App {
    window_modifier: WindowModifier,
    current_window_info_index: Option<usize>,
    window_modification_cache: WindowModificationCache,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx.set_theme(ThemePreference::Dark);
        let mut font_definitions = egui::FontDefinitions::default();
        let proportional = font_definitions
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap();
        let font_data = include_bytes!("simhei.ttf");
        font_definitions.font_data.insert(
            "simhei".to_string(),
            Arc::new(egui::FontData::from_static(font_data)),
        );
        proportional.insert(0, "simhei".to_string());
        cc.egui_ctx.set_fonts(font_definitions);
        Self {
            window_modifier: WindowModifier::new(),
            current_window_info_index: None,
            window_modification_cache: Default::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel")
            .min_width(300.0)
            .default_width(480.0)
            .resizable(true)
            .show(ctx, |ui| self.window_select_panel(ui));

        egui::CentralPanel::default().show(ctx, |ui| self.modification_panel(ui));
    }
}

impl App {
    fn current_window_info(&self) -> Option<&WindowInfo> {
        self.current_window_info_index
            .as_ref()
            .map(|&index| self.window_modifier.window_info_list().get(index).unwrap())
    }

    fn current_window_info_mut(&mut self) -> Option<&mut WindowInfo> {
        self.current_window_info_index.as_ref().map(|&index| {
            self.window_modifier
                .window_info_list_mut()
                .get_mut(index)
                .unwrap()
        })
    }

    fn update_current_window_info(&mut self) {
        self.current_window_info()
            .map(|window_info| !window_info.is_valid())
            .map(|is_invalid| {
                is_invalid.then(|| {
                    self.current_window_info_index = None;
                    message_dialog::info("当前窗口句柄已失效。").show();
                })
            });
    }

    fn window_select_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::Label::new(Self::egui_text("当前窗口: "))
                        .selectable(true)
                        .ui(ui);
                    self.current_window_info()
                        .map(|window_info| window_info.show_ui(ui))
                        .unwrap_or_else(|| {
                            egui::Label::new(Self::egui_text("无"))
                                .selectable(true)
                                .ui(ui);
                        });
                });
                ui.add_space(10.0);
            });
        });
        ui.separator();
        egui::Button::new(Self::egui_text("刷新窗口列表"))
            .ui(ui)
            .clicked()
            .then(|| {
                let pid_and_hwnd = self
                    .current_window_info()
                    .map(|window_info| (window_info.pid, window_info.hwnd));
                self.window_modifier.update_window_info_list();
                pid_and_hwnd.map(|(pid, hwnd)| {
                    self.current_window_info_index = self
                        .window_modifier
                        .window_info_list()
                        .iter()
                        .position(|window_info| window_info.pid == pid && window_info.hwnd == hwnd);
                });
            });
        ui.separator();

        egui::ScrollArea::both().show_rows(
            ui,
            ui.text_style_height(&egui::TextStyle::Body),
            self.window_modifier.window_info_list().len(),
            |ui, range| {
                self.window_modifier
                    .window_info_list()
                    .iter()
                    .enumerate()
                    .skip(range.start)
                    .take(range.end - range.start)
                    .for_each(|(index, window_info)| {
                        window_info
                            .show_selectable_ui(ui)
                            .then(|| self.current_window_info_index = Some(index));
                    });
                ui.add_space(10.0);
            },
        );
    }

    fn modification_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new(ui.next_auto_id())
                .spacing([10.0; 2])
                .num_columns(1)
                .striped(true)
                .show(ui, |ui| {
                    self.modify_window_size(ui);
                    ui.end_row();
                    self.modify_window_inner_size(ui);
                    ui.end_row();
                    self.modify_window_position(ui);
                    ui.end_row();
                    self.modify_window_top_most(ui);
                    ui.end_row();
                    self.modify_window_resizable(ui);
                    ui.end_row();
                    self.modify_window_maximizable_and_minimizable(ui);
                    ui.end_row();
                    self.modify_window_fullscreen_status(ui);
                    ui.end_row();
                });
            ui.add_space(10.0);
        });
    }

    fn modify_window_size(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            egui::Label::new(Self::egui_text("窗口大小: "))
                .selectable(false)
                .ui(ui);
            egui::Button::new(Self::egui_text("读取"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_index.as_ref().map(|&index| {
                        self.window_modifier
                            .window_info_list()
                            .get(index)
                            .unwrap()
                            .get_size()
                            .map(|[width, height]| {
                                self.window_modification_cache.width = width;
                                self.window_modification_cache.height = height;
                            })
                            .map_err(|err| message_dialog::warning(&err.to_string()).show())
                    });
                });
            egui::Button::new(Self::egui_text("应用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info().map(|window_info| {
                        window_info
                            .resize([
                                self.window_modification_cache.width,
                                self.window_modification_cache.height,
                            ])
                            .map_err(|err| message_dialog::warning(&err.to_string()).show())
                    });
                });
            egui::Label::new(Self::egui_text("宽度: ")).ui(ui);
            egui::Slider::new(&mut self.window_modification_cache.width, 0..=8192)
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
            egui::Label::new(Self::egui_text("高度: ")).ui(ui);
            egui::Slider::new(&mut self.window_modification_cache.height, 0..=8192)
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
            ui.separator();
            egui::Label::new(Self::egui_text("当前比例: ")).ui(ui);
            egui::Label::new(Self::egui_text(Self::size_to_ratio_string([
                self.window_modification_cache.width,
                self.window_modification_cache.height,
            ])))
            .ui(ui);
        });
    }

    fn modify_window_inner_size(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            egui::Label::new(Self::egui_text("窗口内部大小: ")).ui(ui);
            egui::Button::new(Self::egui_text("读取"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info()
                        .map(|window_info| window_info.get_inner_size())
                        .map(|inner_size| {
                            inner_size
                                .map(|[inner_width, inner_height]| {
                                    self.window_modification_cache.inner_width = inner_width;
                                    self.window_modification_cache.inner_height = inner_height;
                                })
                                .map_err(|err| message_dialog::warning(&err.to_string()).show())
                        });
                });
            egui::Button::new(Self::egui_text("应用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info().map(|window_info| {
                        window_info
                            .resize_inner([
                                self.window_modification_cache.inner_width,
                                self.window_modification_cache.inner_height,
                            ])
                            .map_err(|err| message_dialog::warning(&err.to_string()).show())
                    });
                });
            egui::Label::new(Self::egui_text("宽度: ")).ui(ui);
            egui::Slider::new(&mut self.window_modification_cache.inner_width, 0..=8192)
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
            egui::Label::new(Self::egui_text("高度: ")).ui(ui);
            egui::Slider::new(&mut self.window_modification_cache.inner_height, 0..=8192)
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
            ui.separator();
            egui::Label::new(Self::egui_text("当前比例: ")).ui(ui);
            egui::Label::new(Self::egui_text(Self::size_to_ratio_string([
                self.window_modification_cache.inner_width,
                self.window_modification_cache.inner_height,
            ])))
            .ui(ui);
        });
    }

    fn modify_window_position(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            egui::Label::new(Self::egui_text("窗口位置: ")).ui(ui);
            egui::Button::new(Self::egui_text("读取"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info()
                        .map(|window_info| window_info.get_position())
                        .map(|position| {
                            position
                                .map(|[x, y]| {
                                    self.window_modification_cache.x = x;
                                    self.window_modification_cache.y = y;
                                })
                                .map_err(|err| message_dialog::warning(&err.to_string()).show())
                        });
                });
            egui::Button::new(Self::egui_text("应用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info().map(|window_info| {
                        window_info
                            .move_to([
                                self.window_modification_cache.x,
                                self.window_modification_cache.y,
                            ])
                            .map_err(|err| message_dialog::warning(&err.to_string()).show())
                    });
                });
            egui::Label::new(Self::egui_text("x: ")).ui(ui);
            egui::Slider::new(&mut self.window_modification_cache.x, -8192..=8192)
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
            egui::Label::new(Self::egui_text("y: ")).ui(ui);
            egui::Slider::new(&mut self.window_modification_cache.y, -8192..=8192)
                .logarithmic(true)
                .drag_value_speed(1.0)
                .ui(ui);
        });
    }

    fn modify_window_top_most(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            egui::Label::new(Self::egui_text("窗口置顶设置: ")).ui(ui);
            egui::Button::new(Self::egui_text("启用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut().map(|window_info| {
                        window_info
                            .set_top_most(true)
                            .map_err(|err| message_dialog::warning(&err.to_string()).show())
                    });
                });
            egui::Button::new(Self::egui_text("禁用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut().map(|window_info| {
                        window_info
                            .set_top_most(false)
                            .map_err(|err| message_dialog::warning(&err.to_string()).show())
                    });
                });
        });
    }

    fn modify_window_maximizable_and_minimizable(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            egui::Label::new(Self::egui_text("窗口最大化/最小化设置: ")).ui(ui);
            egui::Label::new(Self::egui_text("最大化: ")).ui(ui);
            egui::Button::new(Self::egui_text("启用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut()
                        .map(|window_info| window_info.set_maximizable(true));
                });
            egui::Button::new(Self::egui_text("禁用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut()
                        .map(|window_info| window_info.set_maximizable(false));
                });
            ui.separator();
            egui::Label::new(Self::egui_text("最小化: ")).ui(ui);
            egui::Button::new(Self::egui_text("启用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut()
                        .map(|window_info| window_info.set_minimizable(true));
                });
            egui::Button::new(Self::egui_text("禁用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut()
                        .map(|window_info| window_info.set_minimizable(false));
                });
        });
    }

    fn modify_window_resizable(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            egui::Label::new(Self::egui_text("窗口边框拖拽设置: ")).ui(ui);
            egui::Button::new(Self::egui_text("启用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut()
                        .map(|window_info| window_info.set_resizable(true));
                });
            egui::Button::new(Self::egui_text("禁用"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut()
                        .map(|window_info| window_info.set_resizable(false));
                });
        });
    }

    fn modify_window_fullscreen_status(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            egui::Label::new(Self::egui_text("窗口全屏设置: ")).ui(ui);
            egui::Button::new(Self::egui_text("强制无边框全屏"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut().map(|window_info| {
                        window_info
                            .set_borderless_fullscreen()
                            .map_err(|err| message_dialog::warning(&err.to_string()).show())
                    });
                });
            egui::Button::new(Self::egui_text("还原窗口化"))
                .ui(ui)
                .clicked()
                .then(|| {
                    self.update_current_window_info();
                    self.current_window_info_mut().map(|window_info| {
                        window_info
                            .restore_from_borderless_fullscreen()
                            .map_err(|err| message_dialog::warning(&err.to_string()).show())
                    });
                });
        });
    }
}

impl App {
    pub const FONT_SIZE: f32 = 16.0;

    fn egui_text(text: impl Into<String>) -> egui::RichText {
        egui::RichText::new(text).size(Self::FONT_SIZE)
    }

    fn size_to_ratio_string(size: [i32; 2]) -> String {
        let [width, height] = size;
        if width == 0 || height == 0 {
            format!("{}:{}", width, height)
        } else {
            let gcd = utils::gcd(width, height);
            format!("{}:{}", width / gcd, height / gcd)
        }
    }
}
