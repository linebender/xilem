// Copyright 2026 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use masonry::core::{NewWidget, PropertySet, StyleProperty, Widget};
use masonry::layout::AsUnit as _;
use masonry::peniko::{ImageAlphaType, ImageData, ImageFormat};
use masonry::properties::ObjectFit;
use masonry::properties::types::CrossAxisAlignment;
use masonry::widgets::{Flex, Image, Label, SizedBox};

use crate::demo::{CONTENT_GAP, DemoPage, ShellTags, wrap_in_shell};

pub(crate) fn make_image_data() -> ImageData {
    let image_bytes = include_bytes!("../assets/PicWithAlpha.png");
    let image_data = image::load_from_memory(image_bytes).unwrap().to_rgba8();
    let (width, height) = image_data.dimensions();
    ImageData {
        data: image_data.to_vec().into(),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    }
}

pub(crate) struct ImageDemo {
    shell: ShellTags,
}

impl ImageDemo {
    pub(crate) fn new(shell: ShellTags) -> Self {
        Self { shell }
    }
}

impl DemoPage for ImageDemo {
    fn name(&self) -> &'static str {
        "Image"
    }

    fn shell_tags(&self) -> ShellTags {
        self.shell
    }

    fn build(&self) -> NewWidget<dyn Widget> {
        let image = NewWidget::new_with_props(
            Image::new(make_image_data()),
            PropertySet::one(ObjectFit::Contain),
        );

        let body = Flex::column()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_fixed(
                Label::new("An `Image` widget (ObjectFit::Contain).")
                    .with_style(StyleProperty::FontSize(14.0))
                    .with_auto_id(),
            )
            .with_fixed_spacer(CONTENT_GAP)
            .with_fixed(
                SizedBox::new(image)
                    .size(420.0.px(), 280.0.px())
                    .with_auto_id(),
            );

        wrap_in_shell(self.shell, NewWidget::new(body).erased())
    }
}
