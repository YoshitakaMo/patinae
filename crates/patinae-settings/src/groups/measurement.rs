//! Distance, angle, and dihedral measurement settings.

use crate::define_settings_group;

define_settings_group! {
    /// PyMOL-compatible measurement dash parameters.
    group MeasurementSettings / MeasurementOverrides {
        dash_length: f32 = 0.15,
            name = "dash_length",
            min = 0.0, max = 100.0,
            side_effects = [RepresentationRebuild];
        dash_gap: f32 = 0.45,
            name = "dash_gap",
            min = 0.0, max = 100.0,
            side_effects = [RepresentationRebuild];
        dash_width: f32 = 2.5,
            name = "dash_width",
            min = 0.1, max = 100.0,
            side_effects = [RepresentationRebuild];
    }
}
