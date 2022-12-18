// This software is licensed under Apache License 2.0 and distributed on an
// "as-is" basis without warranties of any kind. See the LICENSE file for
// details.

//! This showcase demonstrates how to use the image widget and is
//! propperties. You can change the parameters in the GUI to see how
//! everything behaves.

// On Windows platform, don't show a console when opening the app.
#![windows_subsystem = "windows"]

use masonry::widget::{FillStrat, Image};
use masonry::{AppLauncher, ImageBuf, WindowDescription};

pub fn main() {
    let png_data = ImageBuf::from_data(include_bytes!("./assets/PicWithAlpha.png")).unwrap();
    let image = Image::new(png_data).fill_mode(FillStrat::Contain);

    let main_window = WindowDescription::new(image)
        .window_size((650., 450.))
        .title("Flex Container Options");

    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch()
        .expect("Failed to launch application");
}
