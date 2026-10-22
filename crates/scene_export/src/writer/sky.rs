//! The editor's procedural sky, baked for engines that take a sky as a texture.
//!
//! The 3D view draws its sky from three colours -- zenith, horizon, ground -- warmed
//! towards the sun near sunset, with a glow round the sun. This evaluates the same
//! function as the view's `sky_radiance` (render_wgpu `common.wgsl`), so an exported
//! sky matches the one the map was built under, and writes it as an HDR cube map.

/// The sky and sun of a map, as the editor lights it. Colours are linear, as the
/// renderer takes them; angles are in degrees, azimuth measured from map +X towards +Y.
#[derive(Debug, Clone, PartialEq)]
pub struct Sky {
    pub sky: [f32; 3],
    pub horizon: [f32; 3],
    pub ground: [f32; 3],
    pub sun_color: [f32; 3],
    pub sun_intensity: f32,
    pub ambient_intensity: f32,
    pub sun_elevation: f32,
    pub sun_azimuth: f32,
    /// The view's exposure, applied before its AgX tone mapping.
    pub exposure: f32,
    /// Exponential fog the map asks for: its colour and density per metre.
    pub fog: Option<([f32; 3], f32)>,
}

impl Sky {
    /// Takes the sky and fog a map sets on its worldspawn, through the default game
    /// configuration's `sky`, `sky_top_color`, `sky_horizon_color`, `fog_enabled`,
    /// `fog_light_color` and `fog_density`, over the editor's own.
    pub fn with_worldspawn(mut self, level: &crate::level::Level) -> Self {
        use crate::level::Value;
        let Some(world) = level.entities.iter().find(|e| e.classname == "worldspawn") else {
            return self;
        };
        let switch = |key: &str| match world.properties.get(key) {
            Some(Value::Int(n)) => *n == 1,
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s.trim() == "1",
            _ => false,
        };
        let color = |key: &str| -> Option<[f32; 3]> {
            match world.properties.get(key)? {
                Value::Color(c) => Some([c[0] as f32, c[1] as f32, c[2] as f32]),
                Value::String(s) => {
                    let v: Vec<f32> = s
                        .split_whitespace()
                        .filter_map(|x| x.parse().ok())
                        .collect();
                    (v.len() >= 3).then(|| {
                        let scale = if v[..3].iter().any(|x| *x > 1.) {
                            255.
                        } else {
                            1.
                        };
                        [v[0] / scale, v[1] / scale, v[2] / scale]
                    })
                }
                _ => None,
            }
        };
        if switch("sky") {
            if let Some(top) = color("sky_top_color") {
                self.sky = top;
            }
            if let Some(horizon) = color("sky_horizon_color") {
                self.horizon = horizon;
            }
        }
        if switch("fog_enabled") {
            let density = match world.properties.get("fog_density") {
                Some(Value::Float(f)) => *f as f32,
                Some(Value::Int(i)) => *i as f32,
                Some(Value::String(s)) => s.trim().parse().unwrap_or(0.01),
                _ => 0.01,
            };
            self.fog = Some((
                color("fog_light_color").unwrap_or(self.horizon),
                density.max(0.),
            ));
        }
        self
    }
}

type V = [f32; 3];
fn mix(a: V, b: V, t: f32) -> V {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn dot(a: V, b: V) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn normalize(v: V) -> V {
    let l = dot(v, v).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

impl Sky {
    /// The direction towards the sun in map space (Z up).
    pub fn to_sun_map(&self) -> V {
        let (e, a) = (
            self.sun_elevation.to_radians(),
            self.sun_azimuth.to_radians(),
        );
        [e.cos() * a.cos(), e.cos() * a.sin(), e.sin()]
    }
    /// The direction towards the sun in the exported frame: right-handed and Y up, the
    /// frame every engine target is written in.
    pub fn to_sun(&self) -> V {
        to_export(self.to_sun_map())
    }

    /// The sky's radiance looking along `direction`, a unit vector in map space (Z up).
    pub fn radiance_map(&self, direction: V) -> V {
        let up = direction[2].clamp(-1., 1.);
        let sun = normalize(self.to_sun_map());
        let sun_height = sun[2];
        let daylight = smoothstep(-0.12, 0.08, sun_height);
        let sunset = (1. - smoothstep(0.04, 0.4, sun_height.abs())) * daylight;
        let flat = (direction[0] * sun[0] + direction[1] * sun[1]).max(0.);
        let facing = flat * flat * flat;
        let horizon = mix(self.horizon, [1.0, 0.40, 0.12], sunset * facing * 0.55);
        let zenith = mix(
            self.sky,
            [self.sky[0] * 0.65, self.sky[1] * 0.55, self.sky[2] * 0.85],
            sunset * 0.3,
        );
        let above = mix(horizon, zenith, up.max(0.).powf(0.38));
        let below = mix(horizon, self.ground, (-up).max(0.).powf(0.35));
        let base = if up >= 0. { above } else { below };
        let night = 0.055 + (1. - 0.055) * daylight;
        let angle = dot(direction, sun).clamp(-1., 1.).acos();
        let halo = 0.65 * (-angle * 18.).exp() + 0.12 * (-angle * 3.).exp();
        let warmth = mix([1., 1., 1.], [1.0, 0.48, 0.16], sunset);
        let glow = halo
            * smoothstep(-0.12, 0.06, up)
            * smoothstep(-0.06, 0., sun_height + 0.75f32.to_radians());
        [0, 1, 2].map(|i| base[i] * night + self.sun_color[i] * warmth[i] * glow)
    }

    /// The radiance along `direction` in the exported frame.
    pub fn radiance(&self, direction: V) -> V {
        self.radiance_map(from_export(direction))
    }

    /// A cube map of the sky as a DDS file: RGBA half floats, `size` square faces in
    /// Direct3D order (+X, -X, +Y, -Y, +Z, -Z) and orientation, in the exported frame.
    /// Direct3D samples cubes left-handed, so a right-handed engine that hands a
    /// direction straight to the sampler sees its own +Z on the -Z face; `flip_z`
    /// writes the faces for that.
    pub fn cube_dds(&self, size: u32, flip_z: bool) -> Vec<u8> {
        let mut out = dds_header(size);
        for face in 0..6 {
            for y in 0..size {
                for x in 0..size {
                    let u = (x as f32 + 0.5) / size as f32 * 2. - 1.;
                    let v = (y as f32 + 0.5) / size as f32 * 2. - 1.;
                    let mut d = cube_direction(face, u, v);
                    if flip_z {
                        d[2] = -d[2];
                    }
                    let c = self.radiance(d);
                    for value in [c[0], c[1], c[2], 1.] {
                        out.extend(half(value).to_le_bytes());
                    }
                }
            }
        }
        out
    }
}

impl Sky {
    /// A cube map of the sky as a KTX2 file: RGBA16F, `size` square faces in the order
    /// +X, -X, +Y, -Y, +Z, -Z, one level, no supercompression.
    pub fn cube_ktx2(&self, size: u32, direction: impl Fn(usize, f32, f32) -> V) -> Vec<u8> {
        // Header, index and level index; the data format descriptor follows at 104.
        const DFD_OFFSET: u32 = 104;
        const DFD_LENGTH: u32 = 92;
        let data_offset = (DFD_OFFSET + DFD_LENGTH).next_multiple_of(8);
        let face_bytes = (size * size * 8) as u64;
        let mut out = vec![
            0xAB, b'K', b'T', b'X', b' ', b'2', b'0', 0xBB, b'\r', b'\n', 0x1A, b'\n',
        ];
        // VK_FORMAT_R16G16B16A16_SFLOAT, type size, width, height, depth, layers, faces, levels, supercompression.
        for value in [
            97u32, 2, size, size, 0, 0, 6, 1, 0, DFD_OFFSET, DFD_LENGTH, 0, 0,
        ] {
            out.extend(value.to_le_bytes());
        }
        for value in [0u64, 0, data_offset as u64, face_bytes * 6, face_bytes * 6] {
            out.extend(value.to_le_bytes());
        }
        // A basic descriptor block: RGBSDA, BT.709, linear, four signed float samples.
        out.extend(DFD_LENGTH.to_le_bytes());
        for value in [0u32, 2 | (88 << 16), 1 | (1 << 8) | (1 << 16), 0, 8, 0] {
            out.extend(value.to_le_bytes());
        }
        for (channel, offset) in [(0u32, 0u32), (1, 16), (2, 32), (15, 48)] {
            for value in [
                offset | (15 << 16) | ((channel | 0xC0) << 24),
                0,
                0xBF80_0000,
                0x3F80_0000,
            ] {
                out.extend(value.to_le_bytes());
            }
        }
        out.resize(data_offset as usize, 0);
        for face in 0..6 {
            for y in 0..size {
                for x in 0..size {
                    let u = (x as f32 + 0.5) / size as f32 * 2. - 1.;
                    let v = (y as f32 + 0.5) / size as f32 * 2. - 1.;
                    let c = self.radiance(direction(face, u, v));
                    for value in [c[0], c[1], c[2], 1.] {
                        out.extend(half(value).to_le_bytes());
                    }
                }
            }
        }
        out
    }
}

/// The direction a cube face's pixel looks along, in Direct3D's layout: faces +X, -X,
/// +Y, -Y, +Z, -Z, with `u` right and `v` down across the face, both in -1..1.
pub fn cube_direction(face: usize, u: f32, v: f32) -> V {
    normalize(match face {
        0 => [1., -v, -u],
        1 => [-1., -v, u],
        2 => [u, 1., v],
        3 => [u, -1., -v],
        4 => [u, -v, 1.],
        _ => [-u, -v, -1.],
    })
}

fn srgb(linear: f32) -> u8 {
    let c = linear.clamp(0., 1.);
    let encoded = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1. / 2.4) - 0.055
    };
    (encoded * 255. + 0.5) as u8
}

impl Sky {
    /// Six sRGB PNG faces for an engine that takes a sky as images, clamped to the
    /// displayable range. `direction` gives the exported-frame direction of each face's
    /// pixel, so the caller fits the faces to its engine's cube convention.
    pub fn png_faces(&self, size: u32, direction: impl Fn(usize, f32, f32) -> V) -> Vec<Vec<u8>> {
        self.png_faces_mapped(size, direction, |c| c)
    }

    /// Six faces as they look on screen in the 3D view, after its exposure and AgX
    /// tone mapping: for an engine that draws a texture's colour as it is.
    pub fn display_faces(
        &self,
        size: u32,
        direction: impl Fn(usize, f32, f32) -> V,
    ) -> Vec<Vec<u8>> {
        let exposure = self.exposure * AGX_EXPOSURE;
        self.png_faces_mapped(size, direction, |c| agx(c.map(|v| v * exposure)))
    }

    fn png_faces_mapped(
        &self,
        size: u32,
        direction: impl Fn(usize, f32, f32) -> V,
        map: impl Fn(V) -> V,
    ) -> Vec<Vec<u8>> {
        (0..6)
            .map(|face| png(size, size, &self.face_rgba(face, size, &direction, &map)))
            .collect()
    }

    /// The six display faces stacked top to bottom in one PNG, `size` wide and six
    /// times as tall: raylib's `CUBEMAP_LAYOUT_LINE_VERTICAL`.
    pub fn display_strip(&self, size: u32, direction: impl Fn(usize, f32, f32) -> V) -> Vec<u8> {
        let exposure = self.exposure * AGX_EXPOSURE;
        let map = |c: V| agx(c.map(|v| v * exposure));
        let rgba: Vec<u8> = (0..6)
            .flat_map(|face| self.face_rgba(face, size, &direction, &map))
            .collect();
        png(size, size * 6, &rgba)
    }

    fn face_rgba(
        &self,
        face: usize,
        size: u32,
        direction: &impl Fn(usize, f32, f32) -> V,
        map: &impl Fn(V) -> V,
    ) -> Vec<u8> {
        let mut rgba = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let u = (x as f32 + 0.5) / size as f32 * 2. - 1.;
                let v = (y as f32 + 0.5) / size as f32 * 2. - 1.;
                let c = map(self.radiance(direction(face, u, v)));
                rgba.extend([srgb(c[0]), srgb(c[1]), srgb(c[2]), 255]);
            }
        }
        rgba
    }
}

impl Sky {
    /// An equirectangular panorama as a Radiance `.hdr` (RGBE, flat scanlines), for an
    /// engine that takes the sky as one HDR image. `direction` gives the exported-frame
    /// direction of each pixel from its `u` (across) and `v` (down), both in 0..1.
    pub fn panorama_hdr(
        &self,
        width: u32,
        height: u32,
        direction: impl Fn(f32, f32) -> V,
    ) -> Vec<u8> {
        let mut out =
            format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
        for y in 0..height {
            for x in 0..width {
                let c = self.radiance(direction(
                    (x as f32 + 0.5) / width as f32,
                    (y as f32 + 0.5) / height as f32,
                ));
                out.extend(rgbe(c));
            }
        }
        out
    }
}

/// The direction a panorama pixel looks along as Godot's `PanoramaSkyMaterial` samples
/// it: `u` turns from -Z through +X, `v` runs from straight up to straight down.
pub fn godot_panorama_direction(u: f32, v: f32) -> V {
    let (phi, theta) = (u * std::f32::consts::TAU, v * std::f32::consts::PI);
    [
        theta.sin() * phi.sin(),
        theta.cos(),
        -theta.sin() * phi.cos(),
    ]
}

/// The direction a panorama pixel looks along as three.js samples an equirectangular
/// texture loaded top row first: `u` turns from -X through +Z, `v` runs from straight
/// up to straight down.
pub fn three_panorama_direction(u: f32, v: f32) -> V {
    let (phi, latitude) = (
        (u - 0.5) * std::f32::consts::TAU,
        (0.5 - v) * std::f32::consts::PI,
    );
    [
        phi.cos() * latitude.cos(),
        latitude.sin(),
        phi.sin() * latitude.cos(),
    ]
}

/// The view's exposure multiplier ahead of AgX (render_wgpu `post.wgsl`).
const AGX_EXPOSURE: f32 = 1.8;

/// The view's AgX tone map (render_wgpu `post.wgsl`): linear light in, linear display
/// colour out.
fn agx(color: V) -> V {
    // Columns, as WGSL's mat3x3 takes them.
    const INSET: [V; 3] = [
        [0.842479062253094, 0.0423282422610123, 0.0423756549057051],
        [0.0784335999999992, 0.878468636469772, 0.0784336],
        [0.0792237451477643, 0.0791661274605434, 0.879142973793104],
    ];
    const OUTSET: [V; 3] = [
        [1.19687900512017, -0.0528968517574562, -0.0529716355144438],
        [-0.0980208811401368, 1.15190312990417, -0.0980434501171241],
        [-0.0990297440797205, -0.0989611768448433, 1.15107367264116],
    ];
    let apply =
        |m: &[V; 3], x: V| [0, 1, 2].map(|i| m[0][i] * x[0] + m[1][i] * x[1] + m[2][i] * x[2]);
    let (min_ev, max_ev) = (-12.47393f32, 4.026069f32);
    let x = apply(&INSET, color.map(|v| v.max(1e-10))).map(|v| {
        let x = ((v.log2() - min_ev) / (max_ev - min_ev)).clamp(0., 1.);
        let (x2, x4) = (x * x, x * x * x * x);
        let x = 15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x
            + 0.4298 * x2
            + 0.1191 * x
            - 0.00232;
        x.max(0.).powf(1.2)
    });
    let luma = 0.2126 * x[0] + 0.7152 * x[1] + 0.0722 * x[2];
    let x = x.map(|v| luma + 1.1 * (v - luma));
    apply(&OUTSET, x).map(|v| v.max(0.).powf(2.2))
}

/// A linear colour as Radiance's shared-exponent RGBE.
fn rgbe(c: V) -> [u8; 4] {
    let m = c[0].max(c[1]).max(c[2]);
    if m < 1e-32 {
        return [0; 4];
    }
    let exponent = m.log2().floor() as i32 + 1;
    let scale = 256. / 2f32.powi(exponent);
    [
        (c[0] * scale) as u8,
        (c[1] * scale) as u8,
        (c[2] * scale) as u8,
        (exponent + 128) as u8,
    ]
}

fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(rgba))
        .expect("PNG into memory");
    out
}

/// Map space (Z up) to the exported frame, as the level's own positions convert.
pub fn to_export(v: V) -> V {
    [v[1], v[2], v[0]]
}
/// The exported frame back to map space.
pub fn from_export(v: V) -> V {
    [v[2], v[0], v[1]]
}

/// An IEEE half float, rounded to nearest; enough for colours.
fn half(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = bits & 0x7f_ffff;
    if !value.is_finite() {
        return sign | 0x7c00;
    }
    if exponent >= 31 {
        return sign | 0x7bff;
    }
    if exponent <= 0 {
        if exponent < -10 {
            return sign;
        }
        let m = (mantissa | 0x80_0000) >> (1 - exponent);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    let rounded = ((exponent as u32) << 10 | mantissa >> 13) + ((mantissa >> 12) & 1);
    sign | rounded.min(0x7bff) as u16
}

/// A DDS header with the DX10 extension for an RGBA16F cube map without mipmaps.
fn dds_header(size: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(148);
    h.extend(b"DDS ");
    let flags = 0x1 | 0x2 | 0x4 | 0x1000 | 0x8; // caps, height, width, pixelformat, pitch
    for value in [124, flags, size, size, size * 8, 0, 1] {
        h.extend(u32::to_le_bytes(value));
    }
    h.extend([0u8; 44]);
    // Pixel format: FourCC "DX10".
    h.extend(32u32.to_le_bytes());
    h.extend(0x4u32.to_le_bytes());
    h.extend(b"DX10");
    h.extend([0u8; 20]);
    let caps = 0x8 | 0x1000; // complex, texture
    let caps2 = 0x200 | 0xfc00; // cube map, all six faces
    for value in [caps, caps2, 0, 0, 0] {
        h.extend(u32::to_le_bytes(value));
    }
    // DX10: DXGI_FORMAT_R16G16B16A16_FLOAT, TEXTURE2D, misc TEXTURECUBE, one array element.
    for value in [10u32, 3, 0x4, 1, 0] {
        h.extend(value.to_le_bytes());
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sky() -> Sky {
        Sky {
            sky: [0.1, 0.4, 0.85],
            horizon: [0.7, 0.8, 0.9],
            ground: [0.2, 0.18, 0.15],
            sun_color: [1., 0.95, 0.9],
            sun_intensity: 3.,
            ambient_intensity: 1.,
            sun_elevation: 45.,
            sun_azimuth: 135.,
            exposure: 1.,
            fog: None,
        }
    }

    #[test]
    fn the_zenith_is_the_sky_colour_and_below_the_horizon_is_the_ground() {
        let s = sky();
        let zenith = s.radiance([0., 1., 0.]);
        for i in 0..3 {
            assert!((zenith[i] - s.sky[i]).abs() < 0.05, "{zenith:?}");
        }
        let nadir = s.radiance([0., -1., 0.]);
        for i in 0..3 {
            assert!((nadir[i] - s.ground[i]).abs() < 1e-3, "{nadir:?}");
        }
    }

    #[test]
    fn the_sun_glows_where_the_sun_is() {
        let s = sky();
        let sun = s.to_sun();
        let away = [-sun[0], sun[1], -sun[2]];
        assert!(s.radiance(sun)[0] > s.radiance(normalize(away))[0] + 0.5);
        // Map +X is the export frame's +Z, map Z up its Y up.
        assert_eq!(to_export([1., 0., 0.]), [0., 0., 1.]);
        assert_eq!(to_export([0., 0., 1.]), [0., 1., 0.]);
        assert_eq!(from_export(to_export([0.3, -0.2, 0.9])), [0.3, -0.2, 0.9]);
    }

    #[test]
    fn the_cube_is_a_dds_of_six_half_float_faces() {
        let bytes = sky().cube_dds(8, false);
        assert_eq!(&bytes[0..4], b"DDS ");
        assert_eq!(bytes.len(), 148 + 6 * 8 * 8 * 8);
        assert_eq!(
            u32::from_le_bytes(bytes[128..132].try_into().unwrap()),
            10,
            "RGBA16F"
        );
        // The +Y face, whose centre looks straight up, holds the zenith.
        let face = 148 + 2 * 8 * 8 * 8;
        let centre = face + (4 * 8 + 4) * 8;
        let blue = u16::from_le_bytes(bytes[centre + 4..centre + 6].try_into().unwrap());
        assert!((half_to_f32(blue) - 0.85).abs() < 0.05);
    }

    fn half_to_f32(h: u16) -> f32 {
        let e = ((h >> 10) & 0x1f) as i32;
        let m = (h & 0x3ff) as f32;
        let v = if e == 0 {
            m / 1024. * 2f32.powi(-14)
        } else {
            (1. + m / 1024.) * 2f32.powi(e - 15)
        };
        if h & 0x8000 != 0 {
            -v
        } else {
            v
        }
    }

    #[test]
    fn a_worldspawn_that_sets_its_sky_and_fog_overrides_the_editors() {
        use crate::level::{Entity, Level, Value};
        let mut world = Entity {
            name: "worldspawn".into(),
            classname: "worldspawn".into(),
            ..Default::default()
        };
        world.properties.insert("sky".into(), Value::Int(0));
        world
            .properties
            .insert("sky_top_color".into(), Value::Color([0.25, 0.5, 1., 1.]));
        let mut level = Level {
            format: 2,
            generator: "test".into(),
            name: "a".into(),
            scale: 32.,
            textures: vec![],
            entities: vec![world],
        };
        assert_eq!(
            sky().with_worldspawn(&level),
            sky(),
            "0 leaves the editor's sky"
        );
        let world = &mut level.entities[0];
        world.properties.insert("sky".into(), Value::Int(1));
        world.properties.insert(
            "sky_horizon_color".into(),
            Value::String("255 128 0".into()),
        );
        world
            .properties
            .insert("fog_enabled".into(), Value::String("1".into()));
        world
            .properties
            .insert("fog_density".into(), Value::Float(0.05));
        let s = sky().with_worldspawn(&level);
        assert_eq!((s.sky, s.horizon), ([0.25, 0.5, 1.], [1., 128. / 255., 0.]));
        assert_eq!(s.fog, Some(([1., 128. / 255., 0.], 0.05)));
    }

    #[test]
    fn the_panorama_is_radiance_hdr_with_the_zenith_on_top() {
        let s = sky();
        let bytes = s.panorama_hdr(16, 8, godot_panorama_direction);
        let header = b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 8 +X 16\n";
        assert!(bytes.starts_with(header));
        assert_eq!(bytes.len(), header.len() + 16 * 8 * 4);
        let top = &bytes[header.len()..header.len() + 4];
        let blue = top[2] as f32 * 2f32.powi(top[3] as i32 - 136);
        assert!((blue - 0.85).abs() < 0.08, "{top:?}");
        // u = 0.25 looks along +X, u = 0.5 along +Z.
        let d = godot_panorama_direction(0.25, 0.5);
        assert!((d[0] - 1.).abs() < 1e-5 && d[2].abs() < 1e-5);
        assert!((godot_panorama_direction(0.5, 0.5)[2] - 1.).abs() < 1e-5);
    }

    #[test]
    fn agx_keeps_black_black_and_mid_grey_near_the_views() {
        assert!(agx([0.; 3])[0] < 1e-3);
        let grey = agx([0.18 * AGX_EXPOSURE; 3]);
        assert!(
            grey[0] > 0.1 && grey[0] < 0.4 && (grey[0] - grey[2]).abs() < 1e-3,
            "{grey:?}"
        );
        assert!(agx([100.; 3])[0] <= 1.05);
    }

    #[test]
    fn the_ktx2_cube_has_six_faces_after_its_descriptor() {
        let bytes = sky().cube_ktx2(4, cube_direction);
        assert_eq!(&bytes[..12], b"\xABKTX 20\xBB\r\n\x1A\n");
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!(
            (word(12), word(20), word(36)),
            (97, 4, 6),
            "format, width, faces"
        );
        let offset = u64::from_le_bytes(bytes[80..88].try_into().unwrap()) as usize;
        assert_eq!(offset % 8, 0);
        assert_eq!(word(104), 92, "descriptor size");
        assert_eq!(bytes.len(), offset + 6 * 4 * 4 * 8);
    }

    #[test]
    fn halves_round_trip_colour_values() {
        for v in [0., 0.001, 0.055, 0.5, 1., 3.25, 100.] {
            assert!((half_to_f32(half(v)) - v).abs() <= v * 1e-3 + 1e-6, "{v}");
        }
    }
}
