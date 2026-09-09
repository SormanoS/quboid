use egui::Color32;

/// The colours used by Quboid for one of the two supported themes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Palette {
    /// Brand colour used for headings, links and status text.
    pub brand_text: Color32,
    /// Brand colour used to fill selected or pressed controls.
    pub brand_fill: Color32,
    /// Tinted brand surface for hover and selection backgrounds.
    pub brand_soft: Color32,
    /// Text painted on top of [`Palette::brand_fill`].
    pub on_brand: Color32,
    /// Primary label colour.
    pub text_primary: Color32,
    /// Secondary label colour for captions and descriptions.
    pub text_secondary: Color32,
    /// Background of the main content area.
    pub panel: Color32,
    /// Background of cards, groups and popups.
    pub surface: Color32,
    /// Background of the navigation rail and other tinted areas.
    pub surface_alt: Color32,
    /// Background of text fields and canvases.
    pub input: Color32,
    /// Background of the parts of a control that are not filled yet: the rail of
    /// a slider, the box of an unchecked checkbox. It has to stay visible on
    /// [`Palette::surface`], which is where those controls are drawn.
    pub track: Color32,
    /// Hairline colour for separators and outlines.
    pub border: Color32,
    /// Warning text colour.
    pub warning: Color32,
    /// Error text colour.
    pub error: Color32,
}

impl Palette {
    pub const DARK: Self = Self {
        brand_text: Color32::from_rgb(0x9B, 0xB0, 0xFF),
        brand_fill: Color32::from_rgb(0x3A, 0x57, 0xC9),
        brand_soft: Color32::from_rgb(0x27, 0x2C, 0x45),
        on_brand: Color32::from_rgb(0xF7, 0xF8, 0xFF),
        text_primary: Color32::from_rgb(0xE8, 0xEA, 0xF2),
        text_secondary: Color32::from_rgb(0xA7, 0xAB, 0xBD),
        panel: Color32::from_rgb(0x16, 0x17, 0x1C),
        surface: Color32::from_rgb(0x1E, 0x20, 0x27),
        surface_alt: Color32::from_rgb(0x1A, 0x1C, 0x22),
        input: Color32::from_rgb(0x13, 0x14, 0x19),
        track: Color32::from_rgb(0x44, 0x48, 0x55),
        border: Color32::from_rgb(0x33, 0x36, 0x3F),
        warning: Color32::from_rgb(0xFF, 0xC6, 0x5C),
        error: Color32::from_rgb(0xFF, 0x9E, 0x9E),
    };

    pub const LIGHT: Self = Self {
        brand_text: Color32::from_rgb(0x2F, 0x4B, 0xC4),
        brand_fill: Color32::from_rgb(0x3B, 0x5B, 0xDB),
        brand_soft: Color32::from_rgb(0xE4, 0xE9, 0xFB),
        on_brand: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        text_primary: Color32::from_rgb(0x17, 0x1A, 0x21),
        text_secondary: Color32::from_rgb(0x51, 0x56, 0x68),
        panel: Color32::from_rgb(0xF4, 0xF5, 0xF9),
        surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        surface_alt: Color32::from_rgb(0xEC, 0xEE, 0xF6),
        input: Color32::from_rgb(0xFF, 0xFF, 0xFF),
        track: Color32::from_rgb(0xC6, 0xCB, 0xDB),
        border: Color32::from_rgb(0xD3, 0xD7, 0xE3),
        warning: Color32::from_rgb(0x8A, 0x4B, 0x00),
        error: Color32::from_rgb(0xB3, 0x26, 0x1E),
    };

    /// Returns the palette matching the requested theme.
    #[must_use]
    pub const fn new(dark: bool) -> Self {
        if dark { Self::DARK } else { Self::LIGHT }
    }

    /// Returns the palette matching the theme currently applied to `ui`.
    #[must_use]
    pub fn of(ui: &egui::Ui) -> Self {
        Self::new(ui.visuals().dark_mode)
    }

    /// Returns the label colour to use on top of `background`.
    ///
    /// Both candidates come from the palette, so no pure white text can end up
    /// on a light surface.
    #[must_use]
    pub fn text_on(&self, background: Color32) -> Color32 {
        if contrast_ratio(self.text_primary, background)
            >= contrast_ratio(self.on_brand, background)
        {
            self.text_primary
        } else {
            self.on_brand
        }
    }
}

/// Blends `from` towards `to` by `amount` (`0.0..=1.0`).
#[must_use]
pub fn mix(from: Color32, to: Color32, amount: f32) -> Color32 {
    let amount = amount.clamp(0.0, 1.0);
    let channel = |from: u8, to: u8| {
        (f32::from(from) + (f32::from(to) - f32::from(from)) * amount).round() as u8
    };
    Color32::from_rgb(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
    )
}

/// WCAG relative contrast ratio between two opaque colours.
#[must_use]
pub fn contrast_ratio(first: Color32, second: Color32) -> f32 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

/// The style every Quboid window is drawn with.
///
/// It lives next to the widgets it dresses, so the settings window is tested
/// with the same colours and metrics the application runs with.
#[must_use]
pub fn style(dark: bool) -> egui::Style {
    let mut style = egui::Style::default();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.window_margin = egui::Margin::same(20);
    style.spacing.menu_margin = egui::Margin::same(10);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.interact_size = egui::vec2(40.0, 34.0);
    style.spacing.icon_width = 18.0;
    style.spacing.slider_width = SLIDER_WIDTH;
    style.spacing.slider_rail_height = 6.0;
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(26.0));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));

    let palette = Palette::new(dark);
    let accent = palette.brand_fill;
    let accent_soft = palette.brand_soft;
    let control = if dark {
        mix(palette.surface, palette.text_primary, 0.10)
    } else {
        palette.surface
    };
    let radius = egui::CornerRadius::same(8);

    let visuals = &mut style.visuals;
    *visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = palette.panel;
    visuals.window_fill = palette.surface;
    visuals.window_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.window_corner_radius = egui::CornerRadius::same(12);
    visuals.menu_corner_radius = egui::CornerRadius::same(8);
    visuals.extreme_bg_color = palette.input;
    visuals.text_edit_bg_color = Some(palette.input);
    visuals.faint_bg_color = palette.surface_alt;
    visuals.code_bg_color = palette.surface_alt;
    visuals.weak_text_color = Some(palette.text_secondary);
    visuals.hyperlink_color = palette.brand_text;
    visuals.warn_fg_color = palette.warning;
    visuals.error_fg_color = palette.error;
    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = egui::Stroke::new(1.0, palette.on_brand);
    // The part of the rail the handle has already passed is filled with the
    // accent, so the value of a slider can be read without looking at its box.
    visuals.slider_trailing_fill = true;
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 96 } else { 40 }),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 112 } else { 48 }),
    };

    visuals.widgets.noninteractive.bg_fill = palette.surface;
    visuals.widgets.noninteractive.weak_bg_fill = palette.surface;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.inactive.bg_fill = palette.track;
    visuals.widgets.inactive.weak_bg_fill = control;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.hovered.bg_fill = accent_soft;
    visuals.widgets.hovered.weak_bg_fill = accent_soft;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    visuals.widgets.active.bg_fill = accent;
    visuals.widgets.active.weak_bg_fill = accent;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, palette.on_brand);

    visuals.widgets.open.bg_fill = accent_soft;
    visuals.widgets.open.weak_bg_fill = accent_soft;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = radius;
        widget.expansion = 0.0;
    }

    style
}

/// Length of the rail of a slider, wide enough to aim at a value.
pub const SLIDER_WIDTH: f32 = 220.0;

fn relative_luminance(color: Color32) -> f32 {
    let linear = |channel: u8| {
        let channel = f32::from(channel) / 255.0;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

#[cfg(test)]
mod tests {
    use super::{Palette, contrast_ratio};
    use egui::Color32;

    fn assert_readable(label: &str, text: Color32, background: Color32) {
        let ratio = contrast_ratio(text, background);
        assert!(
            ratio >= 4.5,
            "{label} has a contrast ratio of {ratio:.2}, below the 4.5:1 minimum"
        );
    }

    #[test]
    fn every_text_colour_is_readable_on_every_surface() {
        for (theme, palette) in [("dark", Palette::DARK), ("light", Palette::LIGHT)] {
            for (surface_name, surface) in [
                ("panel", palette.panel),
                ("surface", palette.surface),
                ("surface_alt", palette.surface_alt),
                ("input", palette.input),
            ] {
                assert_readable(
                    &format!("{theme} primary text on {surface_name}"),
                    palette.text_primary,
                    surface,
                );
                assert_readable(
                    &format!("{theme} secondary text on {surface_name}"),
                    palette.text_secondary,
                    surface,
                );
                assert_readable(
                    &format!("{theme} brand text on {surface_name}"),
                    palette.brand_text,
                    surface,
                );
            }
            assert_readable(
                &format!("{theme} text on brand fill"),
                palette.on_brand,
                palette.brand_fill,
            );
            assert_readable(
                &format!("{theme} primary text on brand soft"),
                palette.text_primary,
                palette.brand_soft,
            );
        }
    }

    #[test]
    fn text_on_picks_the_readable_candidate() {
        assert_eq!(
            Palette::LIGHT.text_on(Palette::LIGHT.surface),
            Palette::LIGHT.text_primary
        );
        assert_eq!(
            Palette::LIGHT.text_on(Palette::LIGHT.brand_fill),
            Palette::LIGHT.on_brand
        );
        assert_eq!(
            Palette::DARK.text_on(Palette::DARK.brand_fill),
            Palette::DARK.on_brand
        );
    }

    /// The slider rail and the unchecked checkbox use `widgets.inactive.bg_fill`:
    /// the card colour there would paint them white on white.
    #[test]
    fn controls_stay_visible_on_the_card_they_are_drawn_on() {
        for (name, dark) in [("dark", true), ("light", false)] {
            let palette = Palette::new(dark);
            let track = super::style(dark).visuals.widgets.inactive.bg_fill;

            assert_eq!(track, palette.track);
            for (surface_name, surface) in [
                ("surface", palette.surface),
                ("panel", palette.panel),
                ("surface_alt", palette.surface_alt),
            ] {
                let ratio = contrast_ratio(track, surface);
                assert!(
                    ratio >= 1.3,
                    "{name} slider rail has a contrast ratio of {ratio:.2} against {surface_name}"
                );
            }
        }
    }

    #[test]
    fn a_slider_is_wide_enough_to_aim_with_and_shows_the_value_it_holds() {
        for dark in [true, false] {
            let style = super::style(dark);

            assert!(style.spacing.slider_width >= 200.0);
            assert!(style.visuals.slider_trailing_fill);
        }
    }
}
