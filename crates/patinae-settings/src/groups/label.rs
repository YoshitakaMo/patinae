//! Label typography, placement, background, and connector settings.

use crate::{define_settings_group, Color};

define_settings_group! {
    /// PyMOL-compatible label settings.
    group LabelSettings / LabelOverrides {
        anchor: String = String::new(), name = "label_anchor",
            side_effects = [RepresentationRebuild];
        distance_digits: i32 = -1, name = "label_distance_digits",
            min = -1, max = 10, side_effects = [RepresentationRebuild];
        angle_digits: i32 = -1, name = "label_angle_digits",
            min = -1, max = 10, side_effects = [RepresentationRebuild];
        font_id: i32 = 5, name = "label_font_id",
            side_effects = [RepresentationRebuild];
        bg_color: Color = Color(-1), name = "label_bg_color",
            side_effects = [ColorRebuild];
        multiline_justification: f32 = 1.0, name = "label_multiline_justification",
            min = -1.0, max = 1.0, side_effects = [RepresentationRebuild];
        bg_outline: bool = false, name = "label_bg_outline",
            side_effects = [RepresentationRebuild];
        multiline_spacing: f32 = 1.2, name = "label_multiline_spacing",
            min = 0.0, max = 10.0, side_effects = [RepresentationRebuild];
        bg_transparency: f32 = 0.6, name = "label_bg_transparency",
            min = 0.0, max = 1.0, side_effects = [ColorRebuild];
        outline_color: Color = Color(-1), name = "label_outline_color",
            side_effects = [ColorRebuild];
        color: Color = Color(-6), name = "label_color",
            side_effects = [ColorRebuild];
        padding: [f32; 3] = [0.2, 0.2, 0.0], name = "label_padding",
            side_effects = [RepresentationRebuild];
        connector: bool = false, name = "label_connector",
            side_effects = [RepresentationRebuild];
        placement_offset: [f32; 3] = [0.0, 0.0, 0.0], name = "label_placement_offset",
            side_effects = [RepresentationRebuild];
        position: [f32; 3] = [0.0, 0.0, 1.75], name = "label_position",
            side_effects = [RepresentationRebuild];
        connector_color: Color = Color(-6), name = "label_connector_color",
            side_effects = [ColorRebuild];
        relative_mode: i32 = 0, name = "label_relative_mode",
            min = 0, max = 2, side_effects = [RepresentationRebuild];
        connector_ext_length: f32 = 2.5, name = "label_connector_ext_length",
            min = 0.0, max = 100.0, side_effects = [RepresentationRebuild];
        screen_point: [f32; 3] = [0.0, 0.0, 0.0], name = "label_screen_point",
            side_effects = [RepresentationRebuild];
        connector_mode: i32 = 0, name = "label_connector_mode",
            min = 0, max = 4, side_effects = [RepresentationRebuild];
        shadow_mode: i32 = 0, name = "label_shadow_mode",
            min = 0, max = 3, side_effects = [RepresentationRebuild];
        connector_width: f32 = 2.0, name = "label_connector_width",
            min = 0.1, max = 100.0, side_effects = [RepresentationRebuild];
        digits: i32 = 3, name = "label_digits",
            min = 0, max = 10, side_effects = [RepresentationRebuild];
        size: f32 = 24.0, name = "label_size",
            min = 1.0, max = 256.0, side_effects = [RepresentationRebuild];
        dihedral_digits: i32 = -1, name = "label_dihedral_digits",
            min = -1, max = 10, side_effects = [RepresentationRebuild];
        z_target: i32 = 0, name = "label_z_target",
            side_effects = [RepresentationRebuild];
    }
}
