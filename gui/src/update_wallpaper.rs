use std::fs;

use chrono::{Local, NaiveDateTime};
use image::{DynamicImage, Rgb, RgbImage};
use imageproc::{
    drawing::{draw_filled_rect_mut, Canvas},
    rect::Rect,
};
use text_to_png::TextRenderer;

use crate::{
    download_image, get_cache_dir, new_path, unwrap_or_return, BackgroundOptions, SanitizedConf,
    ScreenDimensions,
};

pub fn update_wallpaper(conf: &SanitizedConf) -> Result<(), String> {
    let today = Local::now().naive_local();
    let deadline = NaiveDateTime::parse_from_str(&conf.deadline_str, "%Y-%m-%d %I:%M %p").unwrap();
    let diff = deadline.signed_duration_since(today);

    let minutes = diff.num_minutes();

    let remaining_days = diff.num_days();
    let months = remaining_days / 30;
    let mut weeks = remaining_days / 7;
    let mut days = remaining_days;
    let mut hours = diff.num_hours();

    if conf.show_months {
        // Month = 30 days - Month = 4 Weeks = 28 days
        // Reminder is 2 days from the 4 weeks of each month
        let days_in_months = months * 30;
        let weeks_in_months = days_in_months / 7;
        let days_reminder = days_in_months - weeks_in_months * 7;

        weeks = weeks - weeks_in_months;
        days = days - months * (30 - days_reminder);
        hours = hours - months * (30 - days_reminder) * 24;
    }

    if conf.show_weeks {
        days = days - weeks * 7;
        hours = hours - weeks * 7 * 24;
    }

    if conf.show_days {
        hours = hours - days * 24
    }

    // TODO: approximate values
    // Ex: 1 hour and 31 minutes
    // Should be "2 hours remaining"
    // And not "1 hours remaining"

    let mut deadline_str = String::new();

    if conf.show_months {
        deadline_str.push_str(&format!("{} Months", months));
    }

    if conf.show_weeks {
        if conf.show_months {
            deadline_str.push_str(", ");
        }
        deadline_str.push_str(&format!("{} Weeks", weeks));
    }

    if conf.show_days {
        if conf.show_months || conf.show_weeks {
            deadline_str.push_str(", ");
        }
        deadline_str.push_str(&format!("{} Days", days));
    }

    if conf.show_hours {
        if conf.show_months || conf.show_weeks || conf.show_days {
            deadline_str.push_str(", ");
        }
        deadline_str.push_str(&format!("{} Hours", hours));
    }

    deadline_str.push_str(" Left.");

    if minutes <= 0 {
        return Err(String::from("Deadline must be a future date!"));
    }

    // TODO: Prevent blocking the main thread cause it freezes the UI.
    let file_path = generate_wallpaper(&deadline_str, &conf);

    match file_path {
        Ok(file_path) => {
            // Sets the wallpaper for the current desktop from a URL.
            wallpaper::set_mode(conf.bg_mode.into()).unwrap();
            wallpaper::set_from_path(&file_path).unwrap();

            Ok(())
        }
        Err(msg) => Err(msg),
    }
}

fn generate_wallpaper(deadline_str: &str, conf: &SanitizedConf) -> Result<String, String> {
    let font_date_bytes = fs::read(new_path(&format!("assets/fonts/{:?}.ttf", conf.font))).unwrap();

    let renderer = TextRenderer::try_new_with_ttf_font_data(font_date_bytes).unwrap();

    let text_png = renderer
        .render_text_to_png_data(deadline_str, conf.font_size, conf.font_color.as_str())
        .unwrap();

    let text_image = image::load_from_memory(&text_png.data).unwrap();

    let mut background;

    if conf.bg_type == BackgroundOptions::FromDisk {
        background = image::open(conf.bg_location.as_ref().unwrap()).unwrap();
    } else if conf.bg_type == BackgroundOptions::Solid {
        let ScreenDimensions { width, height } = conf.screen_dimensions;

        let mut image = RgbImage::new(width, height);

        draw_filled_rect_mut(
            &mut image,
            Rect::at(0, 0).of_size(width, height),
            Rgb(conf.bg_color_arr),
        );

        background = DynamicImage::ImageRgb8(image);
    } else {
        let downloaded_image = match download_image(conf.bg_url.as_ref().unwrap()) {
            Ok(img) => img,
            Err(_) => {
                return Err(String::from(
                    "Couldn't download the Image from the supplied URL!",
                ))
            }
        };

        background = image::io::Reader::open(downloaded_image)
            .unwrap()
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();
    }

    if background.width() <= text_png.size.width || background.height() <= text_png.size.height {
        return Err(String::from(
            "Font size is bigger than wallpaper's dimensions!",
        ));
    }

    // 50% Background Image width or height - 50% Text Image width or height
    // To Center the text both horizontally and vertically
    let x = background.width() / 2 - text_png.size.width / 2;
    let y = background.height() / 2 - text_png.size.height / 2;

    image::imageops::overlay(&mut background, &text_image, x, y);

    let file_path = get_cache_dir().join("result.png");
    let file_path = file_path.to_str().unwrap().to_owned();

    unwrap_or_return!(background.save(&file_path), "Couldn't save result.png");

    Ok(file_path)
}