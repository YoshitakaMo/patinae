//! Render-input emitters.
//!
//! These helpers walk the registry in render order and emit stable
//! `ObjectId`s. `ObjectId(0)` is the picking sentinel.

use patinae_mol::{AtomIndex, CoordSet, ObjectMolecule};
use patinae_render::{
    MeasurementSegment, ObjectId, RenderMapInput, RenderMapMode, RenderMeasurementInput,
    RenderMeasurementLabel, RenderObjectInput, RepColorLutEntry, SceneLod,
};
use patinae_settings::{ResolvedSettings, Settings, ThemeMode};

use crate::object::{
    MapDisplayMode, Measurement, MeasurementType, MoleculeObject, Object, ObjectRegistry,
};

use super::{ResolvedSceneColors, ResolvedSceneMarkers};

type RenderableMoleculeData<'a> = (
    &'a MoleculeObject,
    &'a ObjectMolecule,
    &'a CoordSet,
    &'a [[f32; 4]],
    &'a [RepColorLutEntry],
);

/// Walk the registry in render order, emitting one `RenderObjectInput`
/// per enabled molecule object that has a displayed coord set and pre-
/// resolved colors. The closure also receives the object's registry name
/// — callers building an `ObjectId → name` lookup can record it without a
/// second walk.
pub fn visit_render_objects<'a>(
    registry: &'a ObjectRegistry,
    settings: &Settings,
    colors: &'a ResolvedSceneColors,
    markers: &'a ResolvedSceneMarkers,
    visit: &mut dyn FnMut(&'a str, RenderObjectInput<'a>),
) {
    let lod = scene_lod(registry, colors);

    for name in registry.names() {
        if let Some(input) = render_molecule_input(registry, settings, colors, markers, name, lod) {
            visit(name, input);
        }
    }
}

/// Walk the registry in render order, emitting molecules and renderable maps.
pub fn visit_render_scene<'a>(
    registry: &'a ObjectRegistry,
    settings: &Settings,
    colors: &'a ResolvedSceneColors,
    markers: &'a ResolvedSceneMarkers,
    visit_object: &mut dyn FnMut(&'a str, RenderObjectInput<'a>),
    visit_map: &mut dyn FnMut(&'a str, RenderMapInput<'a>),
    visit_measurement: &mut dyn FnMut(&'a str, RenderMeasurementInput),
) {
    let lod = scene_lod(registry, colors);

    for name in registry.names() {
        if let Some(input) = render_molecule_input(registry, settings, colors, markers, name, lod) {
            visit_object(name, input);
            continue;
        }

        if let Some(obj) = registry.get_measurement(name) {
            if !obj.state().enabled {
                continue;
            }
            let Some(id) = render_object_id(registry, name) else {
                continue;
            };
            let resolved_measurements: Vec<_> = obj
                .measurements()
                .iter()
                .map(|measurement| resolve_measurement_for_current_state(registry, measurement))
                .collect();
            let object_color = resolved_measurements
                .first()
                .map(|measurement| measurement.color)
                .unwrap_or([1.0, 1.0, 0.0, 1.0]);
            let color = if settings.ui.theme == ThemeMode::Light {
                [0.0, 0.0, 0.0, 1.0]
            } else {
                object_color
            };
            let label_color = colors.resolve_setting_color(settings.label.color, color);
            let label_outline_color = if settings.label.outline_color.0 < 0 {
                [0.0, 0.0, 0.0, 0.0]
            } else {
                colors.resolve_setting_color(settings.label.outline_color, [0.0, 0.0, 0.0, 1.0])
            };
            let mut label_bg_color = if settings.label.bg_color.0 < 0 {
                [0.0, 0.0, 0.0, 0.0]
            } else {
                colors.resolve_setting_color(settings.label.bg_color, [0.0, 0.0, 0.0, 1.0])
            };
            label_bg_color[3] *= 1.0 - settings.label.bg_transparency;
            let label_connector_color =
                colors.resolve_setting_color(settings.label.connector_color, label_color);
            let segments = resolved_measurements
                .iter()
                .flat_map(|measurement| {
                    measurement.segments(
                        settings.measurement.dash_length,
                        settings.measurement.dash_gap,
                    )
                })
                .map(|segment| MeasurementSegment {
                    p0: [segment[0].x, segment[0].y, segment[0].z],
                    p1: [segment[1].x, segment[1].y, segment[1].z],
                })
                .collect();
            let labels = resolved_measurements
                .iter()
                .map(|measurement| RenderMeasurementLabel {
                    anchor: [
                        measurement.label_position.x + settings.label.placement_offset[0],
                        measurement.label_position.y + settings.label.placement_offset[1],
                        measurement.label_position.z + settings.label.placement_offset[2],
                    ],
                    text: format_measurement_label(
                        measurement.value,
                        measurement.kind,
                        settings.label.digits,
                        settings.label.distance_digits,
                        settings.label.angle_digits,
                        settings.label.dihedral_digits,
                    ),
                    offset_px: match settings.label.relative_mode {
                        1 | 2 => [
                            settings.label.screen_point[0],
                            settings.label.screen_point[1],
                        ],
                        _ => [settings.label.position[0], settings.label.position[1]],
                    },
                })
                .collect();
            visit_measurement(
                name,
                RenderMeasurementInput {
                    object_id: id,
                    segments,
                    labels,
                    color,
                    label_color,
                    label_outline_color,
                    label_bg_color,
                    label_connector_color,
                    label_size: settings.label.size,
                    label_padding: settings.label.padding,
                    label_bg_outline: settings.label.bg_outline,
                    label_connector: settings.label.connector,
                    label_connector_width: settings.label.connector_width,
                    label_connector_ext_length: settings.label.connector_ext_length,
                    label_shadow_mode: settings.label.shadow_mode,
                    label_z_target: settings.label.z_target,
                    dash_width: settings.measurement.dash_width,
                },
            );
            continue;
        }

        let Some(map_obj) = registry.get_map(name) else {
            continue;
        };
        if !map_obj.state().enabled || !map_obj.is_renderable() {
            continue;
        }
        let Some(grid) = map_obj.grid() else {
            continue;
        };
        let Some(mode) = render_map_mode(map_obj.display_mode()) else {
            continue;
        };
        let Some(id) = render_object_id(registry, name) else {
            continue;
        };
        visit_map(
            name,
            RenderMapInput {
                object_id: id,
                grid,
                mode,
                level: map_obj.level(),
                color: map_obj.mesh_color(),
                transform: mat4_to_cols(&map_obj.state().transform),
                geometry_revision: map_obj.geometry_revision(),
                material_revision: map_obj.material_revision(),
                dirty: map_obj.is_dirty(),
            },
        );
    }
}

fn resolve_measurement_for_current_state(
    registry: &ObjectRegistry,
    measurement: &Measurement,
) -> Measurement {
    let expected = match measurement.kind {
        MeasurementType::Distance => 2,
        MeasurementType::Angle => 3,
        MeasurementType::Dihedral => 4,
    };
    if measurement.atom_refs.len() != expected {
        return measurement.clone();
    }

    let points: Option<Vec<_>> = measurement
        .atom_refs
        .iter()
        .map(|atom_ref| {
            registry
                .get_molecule(&atom_ref.object_name)
                .and_then(|object| object.display_coord(AtomIndex(atom_ref.atom_index)))
        })
        .collect();
    let Some(points) = points else {
        return measurement.clone();
    };

    let resolved = match measurement.kind {
        MeasurementType::Distance => Measurement::distance(points[0], points[1], measurement.color),
        MeasurementType::Angle => {
            Measurement::angle(points[0], points[1], points[2], measurement.color)
        }
        MeasurementType::Dihedral => Measurement::dihedral(
            points[0],
            points[1],
            points[2],
            points[3],
            measurement.color,
        ),
    };
    resolved.with_atom_refs(measurement.atom_refs.clone())
}

fn format_measurement_label(
    value: f64,
    kind: MeasurementType,
    default_digits: i32,
    distance_digits: i32,
    angle_digits: i32,
    dihedral_digits: i32,
) -> String {
    let specific = match kind {
        MeasurementType::Distance => distance_digits,
        MeasurementType::Angle => angle_digits,
        MeasurementType::Dihedral => dihedral_digits,
    };
    let digits = if specific < 0 {
        default_digits
    } else {
        specific
    }
    .clamp(0, 10) as usize;
    format!("{value:.digits$}")
}

fn render_object_id(registry: &ObjectRegistry, name: &str) -> Option<ObjectId> {
    registry.render_id(name).map(|id| ObjectId(id.get()))
}

fn renderable_molecule_data<'a>(
    registry: &'a ObjectRegistry,
    colors: &'a ResolvedSceneColors,
    name: &str,
) -> Option<RenderableMoleculeData<'a>> {
    let mol_obj = registry.get_molecule(name)?;
    if !mol_obj.state().enabled {
        return None;
    }
    let mol = mol_obj.molecule();
    let coord = mol_obj.display_coord_set()?;
    let atom_colors = colors.get(name)?;
    let atom_rep_colors = colors.get_rep(name)?;
    if atom_colors.len() != mol.atom_count() || atom_rep_colors.len() != mol.atom_count() {
        return None;
    }
    Some((mol_obj, mol, coord, atom_colors, atom_rep_colors))
}

fn render_molecule_input<'a>(
    registry: &'a ObjectRegistry,
    settings: &Settings,
    colors: &'a ResolvedSceneColors,
    markers: &'a ResolvedSceneMarkers,
    name: &'a str,
    lod: SceneLod,
) -> Option<RenderObjectInput<'a>> {
    let (mol_obj, mol, coord, atom_colors, atom_rep_colors) =
        renderable_molecule_data(registry, colors, name)?;
    let id = render_object_id(registry, name)?;
    let mut dirty = mol_obj.dirty_flags();
    if markers.is_dirty(name) {
        dirty |= markers.dirty_flags(name);
    }
    Some(RenderObjectInput {
        object_id: id,
        molecule: mol,
        coord_set: coord,
        visible_reps: mol_obj.visible_reps(),
        draw_reps: mol_obj.draw_reps(),
        object_settings: mol_obj
            .overrides()
            .map(|overrides| ResolvedSettings::resolve(settings, Some(overrides))),
        atom_colors,
        atom_rep_colors,
        atom_markers: markers.get(name).unwrap_or(&[]),
        marker_updates: markers.updates(name).unwrap_or(&[]),
        has_markers: markers.has_markers(name),
        lod,
        dirty,
    })
}

fn scene_lod(registry: &ObjectRegistry, colors: &ResolvedSceneColors) -> SceneLod {
    let mut total_atoms: usize = 0;
    for name in registry.names() {
        if let Some((_mol_obj, mol, _coord, _atom_colors, _atom_rep_colors)) =
            renderable_molecule_data(registry, colors, name)
        {
            total_atoms += mol.atom_count();
        }
    }
    SceneLod::from_atom_count(total_atoms)
}

fn render_map_mode(mode: MapDisplayMode) -> Option<RenderMapMode> {
    match mode {
        MapDisplayMode::Isomesh => Some(RenderMapMode::Isomesh),
        MapDisplayMode::Isosurface => Some(RenderMapMode::Isosurface),
        MapDisplayMode::None | MapDisplayMode::Isodot | MapDisplayMode::Volume => None,
    }
}

fn mat4_to_cols(m: &lin_alg::f32::Mat4) -> [[f32; 4]; 4] {
    let d = m.data;
    [
        [d[0], d[1], d[2], d[3]],
        [d[4], d[5], d[6], d[7]],
        [d[8], d[9], d[10], d[11]],
        [d[12], d[13], d[14], d[15]],
    ]
}
