//! How an entity's facing is kept when it is turned.
//!
//! Entity orientation is stored in class-dependent properties. `mangle` and `angle` use different
//! conventions, and `angle = -1` represents an upward direction.
//!
//! Getting this wrong is worse than doing nothing: an entity that comes out facing a wall
//! after a rotation is harder to notice than one that did not turn at all.

use geometry_math::mat::Mat4x4d;
use geometry_math::mat_ext;
use geometry_math::scalar::{self, Float as _};
use geometry_math::vec::{self, Vec3d};

use crate::entity_properties::keys;

/// The key a spotlight's facing is written in.
///
/// Not in [`keys`], since nothing outside this module has anything to do with it.
const MANGLE: &str = "mangle";

/// How the number in the facing property is meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationType {
    /// The entity's facing is not written anywhere, so there is nothing to keep.
    None,
    /// One angle about the Z axis, in degrees.
    Angle,
    /// One angle about the Z axis, where -1 is straight up and -2 straight down.
    AngleUpDown,
    /// Three angles, pitch first, with pitch counted upwards.
    Euler,
    /// Three angles, pitch first, with pitch counted downwards.
    EulerPositivePitchDown,
    /// Three angles, yaw first.
    Mangle,
}

/// Whether the facing may be changed at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationUsage {
    /// It may.
    Allowed,
    /// It may not: the entity's box is not centred on its origin, so turning it would
    /// move it as well as turn it.
    BlockRotation,
}

/// Which property holds an entity's facing, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationInfo {
    /// How the number in the property is meant.
    pub kind: RotationType,
    /// The property the facing is written in.
    pub property_key: String,
    /// Whether the facing may be changed.
    pub usage: RotationUsage,
}

/// What [`rotation_info`] needs to know about an entity beyond its properties.
///
/// Definition metadata is supplied by the caller because it is owned by the editor.
pub struct EntityFacts<'a> {
    /// The entity's class.
    pub classname: &'a str,
    /// Whether the entity is a point entity rather than one made of brushes.
    pub point_entity: bool,
    /// The centre of the box the definition gives it, relative to its origin.
    pub definition_bounds_center: Vec3d,
    /// Whether the entity points at another entity, which is what makes a spotlight aim
    /// by its target rather than by an angle.
    pub has_target: bool,
    /// The property keys the definition declares, for choosing one the entity does not
    /// carry yet.
    pub definition_keys: &'a [String],
}

/// Returns the value of the given property.
fn property<'a>(properties: &'a [(String, String)], key: &str) -> Option<&'a str> {
    properties
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.as_str())
}

/// Picks the first of the given properties the entity has, or failing that the first its
/// definition declares.
///
/// `selectEntityRotationType`.
fn select_rotation_type(
    properties: &[(String, String)],
    facts: &EntityFacts<'_>,
    mapping: &[(&str, RotationType)],
) -> Option<(String, RotationType)> {
    for (key, kind) in mapping {
        if property(properties, key).is_some() {
            return Some(((*key).to_string(), *kind));
        }
    }
    for (key, kind) in mapping {
        if facts.definition_keys.iter().any(|declared| declared == key) {
            return Some(((*key).to_string(), *kind));
        }
    }
    None
}

/// Works out which property holds this entity's facing, and how.
///
/// `entityRotationInfo`.
pub fn rotation_info(properties: &[(String, String)], facts: &EntityFacts<'_>) -> RotationInfo {
    let mut kind = RotationType::None;
    let mut property_key = String::new();
    let mut usage = RotationUsage::Allowed;

    // Uses the default Euler convention for entities without model-specific pitch metadata.
    let euler = RotationType::EulerPositivePitchDown;

    // The three-angle keys are tried before the one-angle one, since an entity carrying
    // both means the three.
    let three_then_one = [
        (keys::ANGLES, euler),
        (MANGLE, euler),
        (keys::ANGLE, RotationType::AngleUpDown),
    ];

    if !facts.classname.is_empty() {
        if facts.classname.starts_with("light") {
            if property(properties, MANGLE).is_some() {
                // A spotlight with no target aims by `mangle`.
                kind = RotationType::Mangle;
                property_key = MANGLE.to_string();
            } else if !facts.has_target {
                // Not a spotlight, but it may have a model that turns.
                if property(properties, keys::ANGLES).is_some() {
                    kind = euler;
                    property_key = keys::ANGLES.to_string();
                } else if property(properties, keys::ANGLE).is_some() {
                    kind = RotationType::Angle;
                    property_key = keys::ANGLE.to_string();
                }
            }
            // A spotlight with a target aims at the target, so nothing here to change.
        } else if !facts.point_entity {
            if let Some((key, chosen)) = select_rotation_type(properties, facts, &three_then_one) {
                property_key = key;
                kind = chosen;
            }
        } else {
            // A point entity whose box is not centred over its origin would be moved by a
            // rotation as well as turned, so it is left alone.
            let offset = facts.definition_bounds_center;
            if !scalar::is_zero(offset.x(), f64::ALMOST_ZERO)
                || !scalar::is_zero(offset.y(), f64::ALMOST_ZERO)
            {
                usage = RotationUsage::BlockRotation;
            }

            let (key, chosen) = select_rotation_type(properties, facts, &three_then_one)
                .unwrap_or_else(|| (keys::ANGLE.to_string(), RotationType::AngleUpDown));
            property_key = key;
            kind = chosen;
        }
    }

    RotationInfo {
        kind,
        property_key,
        usage,
    }
}

/// Reads three angles out of a property, or zeroes if it holds nothing usable.
fn angles(properties: &[(String, String)], key: &str) -> Vec3d {
    let Some(value) = property(properties, key) else {
        return Vec3d::zero();
    };
    let mut parsed = [0.0f64; 3];
    let mut count = 0;
    for (index, part) in value.split_whitespace().take(3).enumerate() {
        match part.parse() {
            Ok(number) => parsed[index] = number,
            Err(_) => return Vec3d::zero(),
        }
        count = index + 1;
    }
    if count < 3 {
        return Vec3d::zero();
    }
    Vec3d::new(parsed[0], parsed[1], parsed[2])
}

/// Reads one angle out of a property, or zero if it holds nothing usable.
fn angle(properties: &[(String, String)], key: &str) -> Option<f64> {
    let value = property(properties, key)?;
    if value.is_empty() {
        return None;
    }
    Some(value.trim().parse().unwrap_or(0.0))
}

/// The rotation the entity's own property describes.
///
/// `entityRotation`.
pub fn rotation(properties: &[(String, String)], info: &RotationInfo) -> Mat4x4d {
    match info.kind {
        RotationType::None => Mat4x4d::identity(),
        RotationType::Angle => match angle(properties, &info.property_key) {
            Some(degrees) => mat_ext::rotation_matrix_about_axis(
                Vec3d::new(0.0, 0.0, 1.0),
                scalar::to_radians(degrees),
            ),
            None => Mat4x4d::identity(),
        },
        RotationType::AngleUpDown => match angle(properties, &info.property_key) {
            // Straight up and straight down are written as numbers no angle could be.
            Some(-1.0) => Mat4x4d::rot_90_y_cw(),
            Some(-2.0) => Mat4x4d::rot_90_y_ccw(),
            Some(degrees) => mat_ext::rotation_matrix_about_axis(
                Vec3d::new(0.0, 0.0, 1.0),
                scalar::to_radians(degrees),
            ),
            None => Mat4x4d::identity(),
        },
        RotationType::Euler => {
            // x is minus the pitch, y the yaw, z the roll. The inverted pitch is what
            // QuakeSpasm's `R_RotateForEntity` does.
            let a = angles(properties, &info.property_key);
            mat_ext::rotation_matrix_from_euler(
                scalar::to_radians(a.z()),
                -scalar::to_radians(a.x()),
                scalar::to_radians(a.y()),
            )
        }
        RotationType::EulerPositivePitchDown => {
            // x is the pitch, y the yaw, z the roll.
            let a = angles(properties, &info.property_key);
            mat_ext::rotation_matrix_from_euler(
                scalar::to_radians(a.z()),
                scalar::to_radians(a.x()),
                scalar::to_radians(a.y()),
            )
        }
        RotationType::Mangle => {
            // x is the yaw, y minus the pitch, z the roll.
            let a = angles(properties, &info.property_key);
            mat_ext::rotation_matrix_from_euler(
                scalar::to_radians(a.z()),
                -scalar::to_radians(a.y()),
                scalar::to_radians(a.x()),
            )
        }
    }
}

/// The yaw, pitch and roll the entity ends up with, in degrees.
///
/// `entityYawPitchRoll`. Rebuilding the matrix from three orthogonal vectors rather than
/// reading the angles straight out is what keeps a transformation that also scales from
/// producing angles that mean something else.
fn yaw_pitch_roll(transformation: &Mat4x4d, rotation: &Mat4x4d) -> Vec3d {
    let m = mat_ext::strip_translation(transformation) * mat_ext::strip_translation(rotation);

    let new_x = vec::normalize(m * Vec3d::new(1.0, 0.0, 0.0));
    let new_y = vec::normalize(vec::cross(m * Vec3d::new(0.0, 0.0, 1.0), new_x));
    let new_z = vec::normalize(vec::cross(new_x, new_y));

    // Each transformed axis is a column, as `rotMat[0] = vec4d(newPosX, 0)` makes it:
    // matrices here are stored column by column, so `rebuilt[column][row]`.
    let mut rebuilt = Mat4x4d::identity();
    for (column, axis) in [new_x, new_y, new_z].into_iter().enumerate() {
        for row in 0..3 {
            rebuilt[column][row] = axis[row];
        }
    }

    let roll_pitch_yaw = mat_ext::rotation_matrix_to_euler_angles(&rebuilt);
    Vec3d::new(
        scalar::to_degrees(roll_pitch_yaw[2]),
        scalar::to_degrees(roll_pitch_yaw[1]),
        scalar::to_degrees(roll_pitch_yaw[0]),
    )
}

/// The angle about the Z axis a direction points along, in whole degrees.
///
/// `getEntityRotationAngle`.
fn direction_angle(direction: Vec3d) -> f64 {
    let flat = vec::normalize(Vec3d::new(direction.x(), direction.y(), 0.0));
    let mut angle = scalar::round(scalar::to_degrees(flat.x().clamp(-1.0, 1.0).acos()));
    if flat.y() < 0.0 {
        angle = 360.0 - angle;
    }
    scalar::normalize_degrees(angle)
}

/// Formats whole numbers without a trailing decimal point.
fn write_number(value: f64) -> String {
    format!("{}", scalar::round(value))
}

/// Writes three whole numbers, space separated.
fn write_angles(angles: Vec3d) -> String {
    format!(
        "{} {} {}",
        write_number(angles.x()),
        write_number(angles.y()),
        write_number(angles.z())
    )
}

/// Returns the property to write so the entity still faces where it did, after the given
/// transformation.
///
/// `applyEntityRotation`. Returns nothing when there is no facing to keep, or when
/// keeping it would move the entity.
pub fn apply_rotation(
    properties: &[(String, String)],
    info: &RotationInfo,
    transformation: &Mat4x4d,
) -> Option<(String, String)> {
    if info.usage == RotationUsage::BlockRotation {
        return None;
    }

    let rotation = rotation(properties, info);
    match info.kind {
        RotationType::None => None,
        RotationType::Angle => {
            let direction = vec::normalize(*transformation * rotation * Vec3d::new(1.0, 0.0, 0.0));
            Some((
                info.property_key.clone(),
                write_number(direction_angle(direction)),
            ))
        }
        RotationType::AngleUpDown => {
            let direction = vec::normalize(*transformation * rotation * Vec3d::new(1.0, 0.0, 0.0));
            if direction.z() > 0.9 {
                return Some((info.property_key.clone(), "-1".to_string()));
            }
            if direction.z() < -0.9 {
                return Some((info.property_key.clone(), "-2".to_string()));
            }
            Some((
                info.property_key.clone(),
                write_number(direction_angle(direction)),
            ))
        }
        RotationType::Euler => {
            let ypr = yaw_pitch_roll(transformation, &rotation);
            Some((
                info.property_key.clone(),
                write_angles(Vec3d::new(-ypr.y(), ypr.x(), ypr.z())),
            ))
        }
        RotationType::EulerPositivePitchDown => {
            let ypr = yaw_pitch_roll(transformation, &rotation);
            Some((
                info.property_key.clone(),
                write_angles(Vec3d::new(ypr.y(), ypr.x(), ypr.z())),
            ))
        }
        RotationType::Mangle => {
            let ypr = yaw_pitch_roll(transformation, &rotation);
            Some((
                info.property_key.clone(),
                write_angles(Vec3d::new(ypr.x(), -ypr.y(), ypr.z())),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts<'a>(classname: &'a str, point: bool, keys: &'a [String]) -> EntityFacts<'a> {
        EntityFacts {
            classname,
            point_entity: point,
            definition_bounds_center: Vec3d::zero(),
            has_target: false,
            definition_keys: keys,
        }
    }

    fn properties(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn a_point_entity_with_no_facing_property_still_gets_one() {
        // `entityRotationInfo` falls back to `angle` for a point entity, so turning one
        // that has never been turned writes the property rather than doing nothing.
        let info = rotation_info(&[], &facts("monster_army", true, &[]));
        assert_eq!(info.kind, RotationType::AngleUpDown);
        assert_eq!(info.property_key, "angle");
        assert_eq!(info.usage, RotationUsage::Allowed);
    }

    #[test]
    fn a_brush_entity_with_no_facing_property_is_left_alone() {
        let info = rotation_info(&[], &facts("func_door", false, &[]));
        assert_eq!(info.kind, RotationType::None);
    }

    #[test]
    fn a_light_with_mangle_aims_by_it() {
        let info = rotation_info(
            &properties(&[("mangle", "0 0 0")]),
            &facts("light", true, &[]),
        );
        assert_eq!(info.kind, RotationType::Mangle);
        assert_eq!(info.property_key, "mangle");
    }

    #[test]
    fn a_light_that_targets_something_aims_at_it_rather_than_by_an_angle() {
        let mut facts = facts("light", true, &[]);
        facts.has_target = true;
        let info = rotation_info(&properties(&[("angle", "90")]), &facts);
        assert_eq!(info.kind, RotationType::None);
    }

    #[test]
    fn an_entity_off_centre_in_its_own_box_does_not_turn() {
        let mut facts = facts("monster_army", true, &[]);
        facts.definition_bounds_center = Vec3d::new(8.0, 0.0, 0.0);
        let info = rotation_info(&properties(&[("angle", "0")]), &facts);
        assert_eq!(info.usage, RotationUsage::BlockRotation);
        assert_eq!(
            apply_rotation(
                &properties(&[("angle", "0")]),
                &info,
                &mat_ext::rotation_matrix_about_axis(
                    Vec3d::new(0.0, 0.0, 1.0),
                    std::f64::consts::FRAC_PI_2,
                ),
            ),
            None
        );
    }

    #[test]
    fn turning_a_quarter_turn_adds_ninety_degrees() {
        let props = properties(&[("angle", "0")]);
        let info = rotation_info(&props, &facts("monster_army", true, &[]));
        let turn = mat_ext::rotation_matrix_about_axis(
            Vec3d::new(0.0, 0.0, 1.0),
            std::f64::consts::FRAC_PI_2,
        );
        assert_eq!(
            apply_rotation(&props, &info, &turn),
            Some(("angle".to_string(), "90".to_string()))
        );
    }

    #[test]
    fn an_angle_wraps_rather_than_growing_past_a_full_turn() {
        let props = properties(&[("angle", "315")]);
        let info = rotation_info(&props, &facts("monster_army", true, &[]));
        let turn = mat_ext::rotation_matrix_about_axis(
            Vec3d::new(0.0, 0.0, 1.0),
            std::f64::consts::FRAC_PI_2,
        );
        assert_eq!(
            apply_rotation(&props, &info, &turn),
            Some(("angle".to_string(), "45".to_string()))
        );
    }

    #[test]
    fn straight_up_and_straight_down_are_written_as_themselves() {
        let props = properties(&[("angle", "0")]);
        let info = rotation_info(&props, &facts("monster_army", true, &[]));

        // Pitching a quarter turn up about the Y axis points the entity at the sky, which
        // no angle in degrees can say, so `angle` takes -1.
        let up = mat_ext::rotation_matrix_about_axis(
            Vec3d::new(0.0, 1.0, 0.0),
            -std::f64::consts::FRAC_PI_2,
        );
        assert_eq!(
            apply_rotation(&props, &info, &up),
            Some(("angle".to_string(), "-1".to_string()))
        );

        let down = mat_ext::rotation_matrix_about_axis(
            Vec3d::new(0.0, 1.0, 0.0),
            std::f64::consts::FRAC_PI_2,
        );
        assert_eq!(
            apply_rotation(&props, &info, &down),
            Some(("angle".to_string(), "-2".to_string()))
        );
    }

    #[test]
    fn an_entity_pointing_up_reads_back_as_pointing_up() {
        // The -1 written above has to mean the same thing when it is read again, or two
        // rotations in a row would not compose.
        let props = properties(&[("angle", "-1")]);
        let info = rotation_info(&props, &facts("monster_army", true, &[]));
        let facing = rotation(&props, &info) * Vec3d::new(1.0, 0.0, 0.0);
        assert!(facing.z() > 0.9, "{facing:?}");
    }

    #[test]
    fn three_angles_are_kept_as_three() {
        let props = properties(&[("angles", "0 90 0")]);
        let info = rotation_info(&props, &facts("misc_model", true, &[]));
        assert_eq!(info.kind, RotationType::EulerPositivePitchDown);

        let (key, value) =
            apply_rotation(&props, &info, &Mat4x4d::identity()).expect("the facing is kept");
        assert_eq!(key, "angles");
        // Unchanged by an identity transformation, up to the rounding the writer does.
        let parsed = angles(&properties(&[("angles", &value)]), "angles");
        assert!((parsed.y() - 90.0).abs() < 1.0, "{value}");
    }

    #[test]
    fn a_definition_that_declares_angles_is_enough_to_choose_them() {
        // `selectEntityRotationType` falls back to what the class takes, so an entity that
        // has never been turned still gets the right property.
        let declared = vec!["angles".to_string()];
        let info = rotation_info(&[], &facts("misc_model", true, &declared));
        assert_eq!(info.property_key, "angles");
    }
}
