// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use megalodon::entities::{Attachment, attachment::AttachmentType};
use xilem::{
    Blob, Image, ImageFormat, WidgetView,
    core::one_of::{OneOf, OneOf4},
    masonry::properties::types::AsUnit,
    view::{flex, image, prose, sized_box},
};

use crate::actions::Navigation;

/// Render a single media attachment for use in a status.
///
/// This currently doesn't perform any caching.
pub(crate) fn attachment<State: 'static>(
    attachment: &Attachment,
) -> impl WidgetView<State, Navigation> + use<State> {
    match attachment.r#type {
        AttachmentType::Audio => OneOf4::A(audio_attachment(attachment)),
        AttachmentType::Video | AttachmentType::Gifv => OneOf::B(video_attachment(attachment)),
        AttachmentType::Image => OneOf::C(image_attachment(attachment)),
        AttachmentType::Unknown => {
            OneOf::D(flex((maybe_blurhash(attachment), prose("Unknown media"))))
        }
    }
}

/// If the attachment has a [blurhash], create a view of it.
///
/// This view currently does not cache the blurhash, or take any other steps to
/// avoid recalculating the image.
/// We haven't ran into this being a performance issue.
fn maybe_blurhash<State: 'static>(
    attachment: &Attachment,
) -> Option<impl WidgetView<State, Navigation> + use<State>> {
    let blurhash = attachment.blurhash.as_deref()?;
    // TODO: Maybe use `memoize` here? We don't at the moment because we
    // wouldn't have any way to pass the "attachment" in.
    let (width, height) = 'dimensions: {
        if let Some(meta) = &attachment.meta {
            if let Some(width) = meta.width
                && let Some(height) = meta.height
            {
                break 'dimensions (width, height);
            } else if let Some(original) = &meta.original
                && let Some(width) = original.width
                && let Some(height) = original.height
            {
                break 'dimensions (width, height);
            }
        }
        tracing::error!("Couldn't find dimensions for blurhash. Using default.");
        (960, 960)
    };
    // TODO: Shrink width and height in a more sane way (e.g. aspect ratio which gets closest to 64x64?)
    let blur_width = width / 16;
    let blur_height = height / 16;
    let result_bytes = blurhash::decode(blurhash, blur_width, blur_height, 1.0).ok()?;
    let image_data = Blob::new(Arc::new(result_bytes));
    // This image format doesn't seem to be documented by the blurhash crate, but this value seems to work.
    let image2 = Image::new(image_data, ImageFormat::Rgba8, blur_width, blur_height);

    // Retain the aspect ratio, and don't go bigger than the image's actual dimensions.
    // Prefer to fill up the width rather than the height.
    Some(sized_box(sized_box(image(&image2)).expand_width()).width(width.px()))
}

/// Show some useful info for audio attachments.
fn audio_attachment<State: 'static>(
    attachment: &Attachment,
) -> impl WidgetView<State, Navigation> + use<State> {
    flex((
        maybe_blurhash(attachment),
        prose("Audio File - Unsupported"),
        attachment.meta.as_ref().map(|meta| {
            meta.length
                .as_deref()
                .map(|it| prose(format!("Length: {it}")))
        }),
        attachment.description.as_deref().map(prose),
    ))
}

/// Show some useful info for audio attachments.
fn video_attachment<State: 'static>(
    attachment: &Attachment,
) -> impl WidgetView<State, Navigation> + use<State> {
    flex((
        maybe_blurhash(attachment),
        prose("Video File - Unsupported"),
        attachment.meta.as_ref().map(|meta| {
            meta.length
                .as_deref()
                .map(|it| prose(format!("Length: {it}")))
        }),
        attachment.description.as_deref().map(prose),
    ))
}

/// Show some useful info for audio attachments.
fn image_attachment<State: 'static>(
    attachment: &Attachment,
) -> impl WidgetView<State, Navigation> + use<State> {
    flex((
        maybe_blurhash(attachment),
        prose("Image File - Unsupported"),
        attachment.meta.as_ref().map(|meta| {
            meta.size
                .as_deref()
                .or_else(|| meta.original.as_ref().and_then(|it| it.size.as_deref()))
                .map(|it| prose(format!("Dimensions: {it}")))
        }),
        attachment.description.as_deref().map(prose),
    ))
}
