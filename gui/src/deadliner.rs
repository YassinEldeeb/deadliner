use crate::{
    button, draw_line, get_cache_dir, get_current_file_ext, get_file_name_from_path,
    is_string_numeric, new_path, render_footer, render_header, render_input,
    render_input_with_label, render_section, save_inputs, unwrap_or_return, BACKGROUND, BLACK,
    GREY_WHITE, MARGIN, PADDING, SECONDARY, SECONDARY_BRIGHT, SECONDARY_DARK, WHITE, YELLOW,
};
use eframe::{
    self,
    egui::{
        self,
        style::{Margin, Selection, WidgetVisuals},
        CentralPanel, ComboBox, Context, FontData, FontDefinitions, Frame, RichText, TextStyle,
    },
    epaint::{Color32, FontFamily, FontId, Rounding, Stroke, TextureHandle},
    epi::App,
};
use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    fs,
    process::Command,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use wallpaper::Mode;

#[derive(Serialize, Deserialize, Debug)]
pub struct DeadlinerConf {
    pub screen_dimensions: ScreenDimensions,

    pub default_background: Background,

    pub show_months: bool,
    pub show_weeks: bool,
    pub show_days: bool,
    pub show_hours: bool,

    pub font: Font,
    pub font_size: u8,
    pub font_color: [u8; 3],
    pub custom_font_location: String,

    pub date: String,
    pub hours: String,
    pub minutes: String,
    pub period: Periods,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ScreenDimensions {
    pub width: u32,
    pub height: u32,
}

pub struct Deadliner<'a> {
    // Preloaded textures on setup to use in the lifecycle methods.
    textures: HashMap<&'a str, TextureHandle>,

    error_msg: String,
    invalid_font: bool,

    conf: DeadlinerConf,
}

#[derive(Debug, PartialEq, Copy, Clone, EnumIter, Serialize, Deserialize)]
pub enum Periods {
    AM,
    PM,
}

#[derive(Debug, PartialEq, Clone, EnumIter, Serialize, Deserialize)]
pub enum Background {
    Solid([u8; 3]),
    FromDisk {
        location: String,
        mode: WallpaperMode,
    },
    FromURL {
        url: String,
        mode: WallpaperMode,
    },
}

impl Background {
    pub fn mode(&self) -> WallpaperMode {
        match self {
            Background::FromURL { mode, .. } | Background::FromDisk { mode, .. } => *mode,
            Background::Solid(_) => WallpaperMode::Center,
        }
    }
}

impl std::fmt::Display for Background {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Solid(_) => "Solid",
                Self::FromDisk { .. } => "From Disk",
                Self::FromURL { .. } => "From URL",
            }
        )
    }
}

#[derive(PartialEq, Debug, Clone, Copy, EnumIter, Serialize, Deserialize)]
pub enum Font {
    PoppinsBlack,
    PoppinsMedium,
    PoppinsRegular,
    PoppinsLight,
    ChooseFromDisk,
}

#[derive(Debug, PartialEq, Clone, Copy, EnumIter, Serialize, Deserialize)]
pub enum WallpaperMode {
    Center,
    Crop,
    Fit,
    Span,
}

impl Default for WallpaperMode {
    fn default() -> Self {
        WallpaperMode::Center
    }
}

impl From<WallpaperMode> for Mode {
    fn from(mode: WallpaperMode) -> Self {
        match mode {
            WallpaperMode::Center => Mode::Center,
            WallpaperMode::Crop => Mode::Crop,
            WallpaperMode::Fit => Mode::Fit,
            WallpaperMode::Span => Mode::Span,
        }
    }
}

impl<'a> App for Deadliner<'a> {
    fn setup(
        &mut self,
        ctx: &Context,
        _frame: &eframe::epi::Frame,
        _storage: Option<&dyn eframe::epi::Storage>,
    ) {
        self.load_logo_texture(ctx);
        self.load_footer_github_texture(ctx);
        self.set_custom_fonts(ctx);

        // ctx.set_debug_on_hover(true);
        let mut style = (*ctx.style()).clone();

        let base = WidgetVisuals {
            bg_fill: SECONDARY,
            bg_stroke: Stroke {
                color: GREY_WHITE,
                width: 0.,
            },
            rounding: Rounding {
                sw: 5.,
                ne: 5.,
                nw: 5.,
                se: 5.,
            },
            expansion: 1.,
            fg_stroke: Stroke {
                color: GREY_WHITE,
                width: 1.,
            },
        };

        // Make small text slightly bigger
        style
            .text_styles
            .get_mut(&egui::TextStyle::Small)
            .unwrap()
            .size = 14.0;

        style.visuals.widgets.inactive = base;
        style.visuals.widgets.active = base;

        style.visuals.widgets.open = WidgetVisuals {
            bg_stroke: Stroke {
                color: GREY_WHITE,
                width: 1.,
            },
            ..base
        };
        style.visuals.widgets.noninteractive = WidgetVisuals {
            bg_fill: SECONDARY_BRIGHT,
            ..base
        };

        style.visuals.widgets.hovered = WidgetVisuals {
            bg_fill: SECONDARY_DARK,
            ..base
        };

        style.visuals.selection = Selection {
            bg_fill: SECONDARY_DARK,
            stroke: Stroke {
                color: GREY_WHITE,
                width: 1.,
            },
        };

        style.visuals.extreme_bg_color = SECONDARY;
        style.visuals.override_text_color = Some(GREY_WHITE);
        ctx.set_style(style);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &eframe::epi::Frame) {
        let logo = self
            .textures
            .get("logo")
            .expect("Logo texture wasn't preloaded");

        let central_panel = CentralPanel::frame(
            CentralPanel::default(),
            Frame {
                fill: BACKGROUND,
                margin: Margin {
                    left: MARGIN,
                    right: MARGIN,
                    top: MARGIN,
                    bottom: MARGIN,
                },
                ..Default::default()
            },
        );

        central_panel.show(ctx, |ui| {
            render_header(ui, logo);
            draw_line(ui, 2.);

            render_section(ui, "Styling", |ui| {
                background_edit(ui, &mut self.conf.default_background);

                ui.add_space(PADDING);

                ui.add_space(PADDING);

                ui.horizontal(|ui| {
                    ui.label("Time in:");
                    ui.checkbox(&mut self.conf.show_hours, "Hours");
                    ui.checkbox(&mut self.conf.show_days, "Days");
                    ui.checkbox(&mut self.conf.show_weeks, "Weeks");
                    ui.checkbox(&mut self.conf.show_months, "Months");
                });

                ui.add_space(PADDING);

                ui.horizontal(|ui| {
                    ui.label("Font:");

                    ComboBox::from_id_source("font_family")
                        .width(125.)
                        .selected_text(format!("{:?}", self.conf.font))
                        .show_ui(ui, |ui| {
                            for option in Font::iter().collect::<Vec<_>>() {
                                ui.selectable_value(
                                    &mut self.conf.font,
                                    option,
                                    format!("{:?}", option),
                                );
                            }
                        });
                });

                ui.add_space(PADDING);

                if self.conf.font == Font::ChooseFromDisk {
                    ui.horizontal(|ui| {
                        if ui.button("Open font…").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                let location = path.display().to_string();
                                let file_name = get_file_name_from_path(&location);
                                let supported_file_ext = ["ttf", "otf"];
                                let file_ext =
                                    file_name.split(".").collect::<Vec<&str>>().pop().unwrap();

                                if supported_file_ext.contains(&file_ext) {
                                    self.invalid_font = false;
                                    self.conf.custom_font_location = location;
                                } else {
                                    self.invalid_font = true;
                                }
                            }
                        }

                        if self.invalid_font {
                            ui.colored_label(Color32::from_rgb(255, 48, 48), "Not a font");
                        } else if !self.conf.custom_font_location.is_empty() {
                            ui.colored_label(
                                Color32::from_rgba_unmultiplied(254, 216, 67, 200),
                                get_file_name_from_path(&self.conf.custom_font_location),
                            );
                        }
                    });

                    ui.add_space(PADDING);
                }

                ui.horizontal(|ui| {
                    ui.label("Font Size:");
                    ui.add(egui::Slider::new(&mut self.conf.font_size, 5..=255));
                });

                ui.add_space(PADDING);

                ui.horizontal(|ui| {
                    ui.label("Font Color:");
                    ui.color_edit_button_srgb(&mut self.conf.font_color);
                });
            });

            render_section(ui, "Pick your Deadline", |ui| {
                let date_error_popup_id = ui.make_persistent_id("invalid-date-error");

                render_input_with_label(ui, "Date:", &mut self.conf.date, "2022-08-26");

                ui.add_space(PADDING);

                ui.horizontal(|ui| {
                    ui.label("Time:");

                    render_input(ui, &mut self.conf.hours, "7", 18.);
                    ui.label(":");
                    render_input(ui, &mut self.conf.minutes, "28", 18.);

                    // Check if inputs are numeric
                    if !is_string_numeric(&self.conf.hours) {
                        self.conf.hours = String::new();
                    }
                    if !is_string_numeric(&self.conf.minutes) {
                        self.conf.minutes = String::new();
                    }

                    ComboBox::from_id_source("time_period")
                        .width(70.)
                        .selected_text(format!("{:?}", self.conf.period))
                        .show_ui(ui, |ui| {
                            for option in Periods::iter().collect::<Vec<_>>() {
                                ui.selectable_value(
                                    &mut self.conf.period,
                                    option,
                                    format!("{:?}", option),
                                );
                            }
                        });
                });

                ui.add_space(20.);

                ui.horizontal(|ui| {
                    let start_button = button("Save!", BLACK, YELLOW, 600, 32.);

                    let start_button = ui.add(start_button);

                    // Setup error popups
                    egui::popup::popup_below_widget(ui, date_error_popup_id, &start_button, |ui| {
                        ui.set_min_width(200.0); // if you want to control the size
                        ui.label(&self.error_msg);
                    });

                    let start_clicked = start_button.clicked();

                    if start_clicked {
                        // Pass true to exit only if the user hit save
                        match save_inputs(&self.conf) {
                            Err(msg) => {
                                self.error_msg = msg;
                                ui.memory().toggle_popup(date_error_popup_id);
                            }
                            _ => (),
                        }
                    };
                });
            });

            let github = self
                .textures
                .get("github")
                .expect("Github texture wasn't preloaded");

            render_footer(&ctx, ui, github);
        });
    }

    fn name(&self) -> &str {
        "Deadliner"
    }
}

fn background_edit(ui: &mut egui::Ui, bg: &mut Background) {
    ui.horizontal(|ui| {
        ui.label("Background:");

        ComboBox::from_id_source("background_options")
            .selected_text(bg.to_string())
            .show_ui(ui, |ui| {
                for option in Background::iter().collect::<Vec<_>>() {
                    let label = option.to_string();
                    ui.selectable_value(bg, option, label);
                }
            });
    });

    ui.add_space(PADDING);

    match bg {
        Background::Solid(color) => {
            ui.horizontal(|ui| {
                ui.label("Pick a Color:");
                ui.color_edit_button_srgb(color);
            });
        }
        Background::FromURL { url, .. } => {
            ui.horizontal(|ui| {
                ui.label("Image URL:");
                ui.add(
                    egui::TextEdit::singleline(url)
                        .desired_width(180.)
                        .hint_text(
                            RichText::new("https://source.unsplash.com/random")
                                .color(Color32::from_white_alpha(45)),
                        ),
                );
            });
        }
        Background::FromDisk { location, .. } => {
            ui.horizontal(|ui| {
                #[derive(Clone)]
                struct IsValid(bool);

                if ui.button("Open file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        let new_location = path.display().to_string();

                        let file_name = get_file_name_from_path(&new_location);
                        let file_ext = file_name.split(".").collect::<Vec<&str>>().pop().unwrap();
                        let supported_file_ext = ["png", "gif", "jpg", "jpeg"];

                        let mut data = ui.data();
                        let is_valid = data.get_temp_mut_or(ui.id(), IsValid(true));
                        
                        if supported_file_ext.contains(&file_ext) {
                            *location = new_location;

                            *is_valid = IsValid(true);
                        } else {
                            *is_valid = IsValid(false);
                        }
                    }
                }

                let is_valid = ui.data().get_temp_mut_or(ui.id(), IsValid(true)).0;
                if !is_valid {
                    ui.colored_label(Color32::from_rgb(255, 48, 48), "Not an Image");
                } else if !location.is_empty() {
                    ui.colored_label(
                        Color32::from_rgba_unmultiplied(254, 216, 67, 200),
                        get_file_name_from_path(location),
                    );
                }
            });
        }
    }

    if let Background::FromDisk { mode, .. } | Background::FromURL { mode, .. } = bg {
        ui.add_space(PADDING);

        ui.horizontal(|ui| {
            ui.label("Wallpaper Mode:");

            ComboBox::from_id_source("background_mode")
                .selected_text(format!("{:?}", mode))
                .show_ui(ui, |ui| {
                    for option in WallpaperMode::iter().collect::<Vec<_>>() {
                        ui.selectable_value(mode, option, format!("{:?}", option));
                    }
                });
        });
    }
}

impl<'a> Deadliner<'a> {
    pub fn new(screen_width: u32, screen_height: u32) -> Deadliner<'a> {
        let default = Deadliner {
            textures: HashMap::new(),
            error_msg: String::new(),
            invalid_font: false,
            conf: DeadlinerConf {
                screen_dimensions: ScreenDimensions {
                    width: screen_width,
                    height: screen_height,
                },
                default_background: Background::Solid([0; 3]),
                custom_font_location: String::new(),
                font: Font::PoppinsBlack,
                date: String::new(),
                hours: String::new(),
                minutes: String::new(),
                period: Periods::AM,
                font_size: 100,
                font_color: [255, 255, 255],
                show_hours: true,
                show_days: true,
                show_weeks: false,
                show_months: false,
            },
        };
        let cached = get_cache_dir().join("raw_config.json");

        if cached.exists() {
            let conf_str = fs::read_to_string(&cached).unwrap();

            Deadliner {
                conf: serde_json::from_str(&conf_str).unwrap_or_else(|_| {
                    fs::remove_file(&cached).unwrap();

                    default.conf
                }),
                ..default
            }
        } else {
            default
        }
    }

    fn set_custom_fonts(&mut self, ctx: &Context) {
        let mut fonts = FontDefinitions::default();

        let fonts_data: Vec<(&str, u16, Vec<u8>)> = vec![
            (
                "Poppins-Regular",
                400,
                fs::read(new_path("assets/fonts/PoppinsLight.ttf")).unwrap(),
            ),
            (
                "Poppins-Medium",
                500,
                fs::read(new_path("assets/fonts/PoppinsRegular.ttf")).unwrap(),
            ),
            (
                "Poppins-SemiBold",
                600,
                fs::read(new_path("assets/fonts/PoppinsMedium.ttf")).unwrap(),
            ),
        ];

        // Emoji Fonts
        fonts.font_data.insert(
            "emoji-icon-font".to_owned(),
            FontData::from_owned(fs::read(new_path("assets/fonts/EmojiFont.ttf")).unwrap()),
        );
        fonts.font_data.insert(
            "noto-emoji-font".to_owned(),
            FontData::from_owned(fs::read(new_path("assets/fonts/NotoEmojiRegular.ttf")).unwrap()),
        );

        // Insert all of the fonts data
        for (name, font_weight, buffer) in fonts_data {
            fonts
                .font_data
                .insert(name.to_owned(), FontData::from_owned(buffer));

            fonts.families.insert(
                FontFamily::Name(format!("Poppins-{}", font_weight).into()),
                vec![
                    name.into(),
                    // Add emoji fonts as a fallback
                    "noto-emoji-font".into(),
                    "emoji-icon-font".into(),
                ],
            );
        }

        ctx.set_fonts(fonts);

        // Set text styles
        let mut text_styles = BTreeMap::new();

        text_styles.insert(
            TextStyle::Heading,
            FontId {
                family: FontFamily::Name("Poppins-600".into()),
                size: 35.,
            },
        );

        text_styles.insert(
            TextStyle::Body,
            FontId {
                family: FontFamily::Name("Poppins-400".into()),
                size: 20.,
            },
        );

        text_styles.insert(
            TextStyle::Button,
            FontId {
                family: FontFamily::Name("Poppins-400".into()),
                size: 18.,
            },
        );

        text_styles.insert(
            TextStyle::Monospace,
            FontId {
                family: FontFamily::Name("Poppins-400".into()),
                size: 18.,
            },
        );

        text_styles.insert(
            TextStyle::Small,
            FontId {
                family: FontFamily::Name("Poppins-400".into()),
                size: 12.,
            },
        );

        ctx.set_style(egui::Style {
            text_styles,
            ..Default::default()
        });
    }

    fn load_logo_texture(&mut self, ctx: &Context) {
        let image = image::load_from_memory(include_bytes!("../assets/icon.png")).unwrap();
        let size = [image.width() as _, image.height() as _];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();

        let texture = ctx.load_texture(
            "logo",
            egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
        );

        self.textures.insert("logo", texture);
    }

    fn load_footer_github_texture(&mut self, ctx: &Context) {
        let image = image::load_from_memory(include_bytes!("../assets/github.png")).unwrap();
        let size = [image.width() as _, image.height() as _];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();

        let texture = ctx.load_texture(
            "github",
            egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()),
        );

        self.textures.insert("github", texture);
    }
}
