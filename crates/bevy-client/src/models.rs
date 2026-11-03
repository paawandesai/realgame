//! Small generated models for the client (docs/art-bible.md: "rounded and
//! bevelled, never sharp boxes"; every asset made by our code): a mesh
//! builder with rounded boxes, tapered rounded columns and spheres, each
//! part in its own vertex colour, so one mesh and one material draw a
//! whole prop (the balloon crate, the rig, the gauntlet).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Positions, normals, vertex colours (linear RGBA) and triangles.
#[derive(Default, Clone)]
pub struct Builder {
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[f32; 3]>,
    pub col: Vec<[f32; 4]>,
    pub idx: Vec<u32>,
}

impl Builder {
    fn push(&mut self, p: Vec3, n: Vec3, c: [f32; 4]) -> u32 {
        let i = self.pos.len() as u32;
        self.pos.push(p.to_array());
        self.nrm.push(n.normalize_or_zero().to_array());
        self.col.push(c);
        i
    }

    /// A box with rounded edges and corners (radius `r`), centred at
    /// `at`, turned by `rot`, half sizes `half`. `steps` points per face
    /// edge in each rounding zone (2 is plenty at prop size).
    pub fn rounded_box(&mut self, at: Vec3, rot: Quat, half: Vec3, r: f32, c: [f32; 4]) {
        let r = r.min(half.min_element() * 0.999).max(0.0);
        let core = half - Vec3::splat(r);
        // Samples along a face axis: the rounding zone at each end, then
        // the flat middle.
        let samples = |h: f32| -> Vec<f32> {
            let mut v = vec![-h, -h + r * 0.38, -(h - r)];
            v.extend([h - r, h - r * 0.38, h]);
            v.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            v
        };
        for axis in 0..3 {
            for sign in [-1.0f32, 1.0] {
                let (u, w) = ((axis + 1) % 3, (axis + 2) % 3);
                let su = samples(half[u]);
                let sw = samples(half[w]);
                let base = self.pos.len() as u32;
                for &gw in &sw {
                    for &gu in &su {
                        let mut g = Vec3::ZERO;
                        g[axis] = sign * half[axis];
                        g[u] = gu;
                        g[w] = gw;
                        let cpt = g.clamp(-core, core);
                        let n = (g - cpt).normalize_or(Vec3::ZERO);
                        let n = if n == Vec3::ZERO {
                            let mut f = Vec3::ZERO;
                            f[axis] = sign;
                            f
                        } else {
                            n
                        };
                        let p = cpt + n * r;
                        self.push(at + rot * p, rot * n, c);
                    }
                }
                let nu = su.len() as u32;
                for j in 0..sw.len() as u32 - 1 {
                    for i in 0..nu - 1 {
                        let a = base + j * nu + i;
                        let (b, cc, d) = (a + 1, a + nu, a + nu + 1);
                        // u x w = +axis: wind so the face looks along sign.
                        if sign > 0.0 {
                            self.idx.extend_from_slice(&[a, b, d, a, d, cc]);
                        } else {
                            self.idx.extend_from_slice(&[a, d, b, a, cc, d]);
                        }
                    }
                }
            }
        }
    }

    /// A tapered column round `rot`'s y axis from `at` (radius `r0`) up
    /// `h` (radius `r1`), with `sides` facets and a rounded lip of `bevel`
    /// at both ends. Smooth shaded.
    #[allow(clippy::too_many_arguments)]
    pub fn column(
        &mut self,
        at: Vec3,
        rot: Quat,
        r0: f32,
        r1: f32,
        h: f32,
        sides: usize,
        bevel: f32,
        c: [f32; 4],
    ) {
        let b = bevel.min(h * 0.45).min(r0.min(r1) * 0.9);
        // Profile (radius, y, normal in the r-y plane), bottom centre to top
        // centre.
        let slope = (r0 - r1) / h.max(1e-6);
        let side_n = Vec2::new(1.0, slope).normalize();
        let prof: Vec<(f32, f32, Vec2)> = vec![
            (0.0, 0.0, Vec2::new(0.0, -1.0)),
            (r0 - b, 0.0, Vec2::new(0.0, -1.0)),
            (r0 - b * 0.3, b * 0.3, Vec2::new(0.7, -0.7)),
            (r0, b, side_n),
            (r1, h - b, side_n),
            (r1 - b * 0.3, h - b * 0.3, Vec2::new(0.7, 0.7)),
            (r1 - b, h, Vec2::new(0.0, 1.0)),
            (0.0, h, Vec2::new(0.0, 1.0)),
        ];
        let base = self.pos.len() as u32;
        for k in 0..=sides {
            let t = k as f32 / sides as f32 * std::f32::consts::TAU;
            let (s, co) = t.sin_cos();
            for &(rr, y, n) in &prof {
                let p = Vec3::new(rr * co, y, rr * s);
                let nn = Vec3::new(n.x * co, n.y, n.x * s);
                self.push(at + rot * p, rot * nn, c);
            }
        }
        let np = prof.len() as u32;
        for k in 0..sides as u32 {
            for j in 0..np - 1 {
                let a = base + k * np + j;
                let (b2, cc, d) = (a + 1, a + np, a + np + 1);
                self.idx.extend_from_slice(&[a, b2, d, a, d, cc]);
            }
        }
    }

    /// A UV sphere (ellipsoid with `radii`), smooth shaded.
    pub fn sphere(&mut self, at: Vec3, rot: Quat, radii: Vec3, c: [f32; 4]) {
        let (nu, nv) = (16u32, 10u32);
        let base = self.pos.len() as u32;
        for j in 0..=nv {
            let phi = j as f32 / nv as f32 * std::f32::consts::PI - std::f32::consts::FRAC_PI_2;
            for i in 0..=nu {
                let th = i as f32 / nu as f32 * std::f32::consts::TAU;
                let u = Vec3::new(phi.cos() * th.cos(), phi.sin(), phi.cos() * th.sin());
                let p = u * radii;
                let n = u / radii;
                self.push(at + rot * p, rot * n, c);
            }
        }
        for j in 0..nv {
            for i in 0..nu {
                let a = base + j * (nu + 1) + i;
                let (b, cc, d) = (a + 1, a + nu + 1, a + nu + 2);
                self.idx.extend_from_slice(&[a, d, b, a, cc, d]);
            }
        }
    }

    pub fn mesh(self) -> Mesh {
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.nrm)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, self.col)
        .with_inserted_indices(Indices::U32(self.idx))
    }
}

/// An sRGB hex colour as linear RGBA (the art bible's palette is in hex).
pub fn hex(rgb: u32) -> [f32; 4] {
    let c = Color::srgb_u8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8).to_linear();
    [c.red, c.green, c.blue, 1.0]
}

/// The balloon crate: a timber crate with rounded edges, a darker frame
/// round every face and a diagonal brace, about 0.9 m on a side, centred
/// on the origin.
pub fn balloon_crate() -> Mesh {
    let mut b = Builder::default();
    let timber = hex(0x8A5A3B);
    let dark = hex(0x5E3C26);
    let iron = hex(0x55575C);
    b.rounded_box(Vec3::ZERO, Quat::IDENTITY, Vec3::splat(0.42), 0.05, timber);
    // The frame: a rounded bar along each of the 12 edges.
    let e = 0.42f32;
    for axis in 0..3 {
        for (s1, s2) in [(-1.0f32, -1.0f32), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
            let (u, w) = ((axis + 1) % 3, (axis + 2) % 3);
            let mut at = Vec3::ZERO;
            at[u] = s1 * (e - 0.02);
            at[w] = s2 * (e - 0.02);
            let mut half = Vec3::splat(0.065);
            half[axis] = e + 0.03;
            b.rounded_box(at, Quat::IDENTITY, half, 0.03, dark);
        }
    }
    // A brace across each side face.
    for k in 0..4 {
        let yaw = Quat::from_rotation_y(k as f32 * std::f32::consts::FRAC_PI_2);
        let at = yaw * Vec3::new(0.0, 0.0, e + 0.01);
        let rot = yaw * Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        b.rounded_box(at, rot, Vec3::new(0.52, 0.05, 0.03), 0.02, dark);
    }
    // The lifting ring on top.
    b.column(
        Vec3::new(0.0, e, 0.0),
        Quat::IDENTITY,
        0.08,
        0.06,
        0.08,
        8,
        0.02,
        iron,
    );
    b.mesh()
}

/// The electrolysis rig's stand under its two tubes (centred on the rig's
/// cell top): a clay basin with a brass rim, two iron electrodes with
/// wires, and a brass cap on each tube's rack.
pub fn rig_stand() -> Mesh {
    let mut b = Builder::default();
    let clay = hex(0xD99A6C);
    let brass = hex(0xB08A3A);
    let iron = hex(0x4A4C52);
    let wire = hex(0x7A2E1E);
    // The basin round the tubes' feet.
    b.column(Vec3::ZERO, Quat::IDENTITY, 0.46, 0.5, 0.22, 16, 0.05, clay);
    b.column(
        Vec3::new(0.0, 0.2, 0.0),
        Quat::IDENTITY,
        0.52,
        0.52,
        0.05,
        16,
        0.02,
        brass,
    );
    // The rack the tubes hang from.
    for dx in [-0.42f32, 0.42] {
        b.rounded_box(
            Vec3::new(dx, 0.85, 0.0),
            Quat::IDENTITY,
            Vec3::new(0.035, 0.65, 0.035),
            0.015,
            brass,
        );
    }
    b.rounded_box(
        Vec3::new(0.0, 1.5, 0.0),
        Quat::IDENTITY,
        Vec3::new(0.46, 0.035, 0.05),
        0.015,
        brass,
    );
    // Electrodes and their wires running down to the iron blocks beside.
    for dx in [-0.25f32, 0.25] {
        b.column(
            Vec3::new(dx, 0.1, 0.0),
            Quat::IDENTITY,
            0.03,
            0.03,
            0.3,
            6,
            0.01,
            iron,
        );
        let side = dx.signum();
        let rot = Quat::from_rotation_z(-side * 1.1);
        b.rounded_box(
            Vec3::new(dx + side * 0.42, 0.25, 0.05),
            rot,
            Vec3::new(0.018, 0.45, 0.018),
            0.008,
            wire,
        );
    }
    b.mesh()
}

/// The first-person gauntlet (art bible: rounded, bevelled, a teal rune
/// inlay that glows in the selected spell's colour), in the viewmodel's
/// own space: forearm along +z behind the hand, fingers towards -z.
/// Returns the bronze-and-gold body; `gauntlet_runes` is the inlay, drawn
/// with the glowing rune material.
pub fn gauntlet() -> Mesh {
    let mut b = Builder::default();
    let bronze = hex(0xB0763F);
    let dark = hex(0x7A4E2A);
    let gold = hex(0xE0B050);
    let leather = hex(0x5A3A26);
    let along = |d: Vec3| Quat::from_rotation_arc(Vec3::Y, d.normalize());
    let back = Vec3::NEG_Z;
    // Forearm: a leather sleeve, then a bronze bracer over it.
    b.column(
        Vec3::new(0.0, 0.0, 0.34),
        along(back),
        0.05,
        0.05,
        0.06,
        10,
        0.012,
        leather,
    );
    b.column(
        Vec3::new(0.0, 0.0, 0.3),
        along(back),
        0.064,
        0.054,
        0.28,
        12,
        0.016,
        bronze,
    );
    // A raised spine along the bracer's top, and a ring at each end.
    b.rounded_box(
        Vec3::new(0.0, 0.052, 0.16),
        Quat::from_rotation_x(0.035),
        Vec3::new(0.026, 0.012, 0.12),
        0.01,
        dark,
    );
    b.column(
        Vec3::new(0.0, 0.0, 0.305),
        along(back),
        0.068,
        0.068,
        0.025,
        12,
        0.008,
        gold,
    );
    // The cuff at the wrist.
    b.column(
        Vec3::new(0.0, 0.0, 0.03),
        along(back),
        0.068,
        0.062,
        0.06,
        12,
        0.014,
        gold,
    );
    // The hand: a rounded back plate, a palm under it, a knuckle guard.
    b.rounded_box(
        Vec3::new(0.0, 0.004, -0.11),
        Quat::IDENTITY,
        Vec3::new(0.058, 0.03, 0.058),
        0.024,
        bronze,
    );
    b.rounded_box(
        Vec3::new(0.0, 0.03, -0.152),
        Quat::from_rotation_x(0.2),
        Vec3::new(0.054, 0.012, 0.022),
        0.01,
        gold,
    );
    // Fingers: two rounded segments each, curling down a little more at
    // the tips.
    for k in 0..4 {
        let x = -0.039 + k as f32 * 0.026;
        let len = if k == 1 || k == 2 { 0.052 } else { 0.044 };
        let d1 = Vec3::new(0.0, -0.3, -1.0);
        let p1 = Vec3::new(x, 0.006, -0.162);
        b.column(p1, along(d1), 0.0125, 0.0115, len, 8, 0.005, gold);
        let p2 = p1 + d1.normalize() * (len - 0.004);
        let d2 = Vec3::new(0.0, -0.75, -1.0);
        b.column(p2, along(d2), 0.0115, 0.0095, len * 0.8, 8, 0.005, bronze);
    }
    // The thumb, along the inner side.
    let dt = Vec3::new(-0.45, -0.2, -0.9);
    let pt = Vec3::new(-0.055, -0.006, -0.09);
    b.column(pt, along(dt), 0.015, 0.013, 0.055, 8, 0.006, gold);
    b.column(
        pt + dt.normalize() * 0.052,
        along(Vec3::new(-0.2, -0.35, -1.0)),
        0.013,
        0.011,
        0.04,
        8,
        0.005,
        bronze,
    );
    // The gem's setting on the back of the hand.
    b.column(
        Vec3::new(0.0, 0.03, -0.11),
        Quat::IDENTITY,
        0.032,
        0.03,
        0.014,
        12,
        0.005,
        gold,
    );
    b.mesh()
}

/// The gauntlet's rune inlay: a line down the bracer's spine with three
/// chevrons, a band of studs round the cuff, and a ring round the gem.
pub fn gauntlet_runes() -> Mesh {
    let mut b = Builder::default();
    let c = [1.0, 1.0, 1.0, 1.0];
    b.rounded_box(
        Vec3::new(0.0, 0.0655, 0.16),
        Quat::from_rotation_x(0.035),
        Vec3::new(0.005, 0.0025, 0.105),
        0.002,
        c,
    );
    for (k, z) in [0.07f32, 0.15, 0.23].into_iter().enumerate() {
        let w = 0.02 + 0.004 * k as f32;
        for s in [-1.0f32, 1.0] {
            b.rounded_box(
                Vec3::new(s * w * 0.55, 0.058 - 0.006 * k as f32 * 0.0, z),
                Quat::from_rotation_y(s * 0.7) * Quat::from_rotation_z(-s * 0.55),
                Vec3::new(w * 0.6, 0.003, 0.004),
                0.002,
                c,
            );
        }
    }
    for k in 0..10 {
        let a = k as f32 / 10.0 * std::f32::consts::TAU;
        let (s, co) = a.sin_cos();
        b.rounded_box(
            Vec3::new(co * 0.066, s * 0.066, 0.0),
            Quat::from_rotation_z(a),
            Vec3::new(0.004, 0.009, 0.02),
            0.003,
            c,
        );
    }
    b.column(
        Vec3::new(0.0, 0.0435, -0.11),
        Quat::IDENTITY,
        0.034,
        0.034,
        0.003,
        16,
        0.001,
        c,
    );
    b.mesh()
}

/// The gem (radius 1): a faceted, slightly tall stone.
pub fn gem() -> Mesh {
    let mut b = Builder::default();
    b.sphere(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::new(1.0, 0.8, 1.0),
        [1.0; 4],
    );
    b.mesh()
}

/// The Kiln doorway (art bible: "an arched, bevelled timber door in a clay
/// frame"): the opening is cells x 127..130, y 41..44 in the dome's south
/// side (`gen::stamps`); its arch springs at y 43 with a 1.5 m radius.
pub const KILN_ARCH: (f32, f32, f32) = (128.5, 43.0, 1.5);
/// The door's front plane (z) and its hinge on the west side.
pub const KILN_DOOR_Z: f32 = 173.75;
pub const KILN_HINGE: [f32; 3] = [127.0, 41.0, 173.75];

/// The clay frame round the Kiln doorway, in world space: two jambs on
/// plinths, nine voussoirs round the arch and a keystone.
pub fn kiln_frame() -> Mesh {
    let mut b = Builder::default();
    let clay = hex(0xB9784F);
    let light = hex(0xD08A5E);
    let stone = hex(0x8C8079);
    let (cx, sy, r) = KILN_ARCH;
    let z = KILN_DOOR_Z + 0.1;
    for side in [-1.0f32, 1.0] {
        let x = cx + side * (r + 0.3);
        b.rounded_box(
            Vec3::new(x, 42.0, z),
            Quat::IDENTITY,
            Vec3::new(0.32, 1.02, 0.62),
            0.1,
            clay,
        );
        b.rounded_box(
            Vec3::new(x, 41.12, z + 0.04),
            Quat::IDENTITY,
            Vec3::new(0.4, 0.14, 0.68),
            0.06,
            stone,
        );
    }
    let n = 9;
    let rm = r + 0.3;
    for k in 0..n {
        let a = (k as f32 + 0.5) / n as f32 * std::f32::consts::PI;
        let at = Vec3::new(cx + rm * a.cos(), sy + rm * a.sin(), z);
        let rot = Quat::from_rotation_z(a);
        let key = k == n / 2;
        let half = Vec3::new(
            if key { 0.42 } else { 0.32 },
            rm * std::f32::consts::PI / n as f32 * 0.5 * if key { 1.15 } else { 0.96 },
            if key { 0.68 } else { 0.62 },
        );
        b.rounded_box(at, rot, half, 0.08, if k % 2 == 0 { clay } else { light });
    }
    b.mesh()
}

/// The Kiln's firebox seen through the open door (Paawan's walkthrough:
/// the inside looked awful): a back wall of sooty brick courses in
/// running bond over dark mortar, an iron grate across the floor and three
/// charred logs on it. In world space; the alcove is x 127-130, y 41-44,
/// its back wall's face at z 172.
pub fn kiln_firebox() -> Mesh {
    let mut b = Builder::default();
    let mortar = hex(0x2A2220);
    let iron = hex(0x3A3A40);
    let char_log = hex(0x2B1D16);
    let bark = hex(0x4A3020);
    let (x0, x1, y0, y1, z) = (127.0f32, 130.0f32, 41.0f32, 44.0f32, 172.0f32);
    b.rounded_box(
        Vec3::new((x0 + x1) / 2.0, (y0 + y1) / 2.0, z - 0.03),
        Quat::IDENTITY,
        Vec3::new((x1 - x0) / 2.0, (y1 - y0) / 2.0, 0.03),
        0.0,
        mortar,
    );
    let (bw, bh) = (0.5f32, 0.25f32);
    let courses = ((y1 - y0) / bh).round() as i32;
    for row in 0..courses {
        let y = y0 + bh * (row as f32 + 0.5);
        let shift = if row % 2 == 0 { 0.0 } else { bw / 2.0 };
        let mut x = x0 - shift;
        let mut k = 0;
        while x < x1 - 0.01 {
            let (a, e) = (x.max(x0), (x + bw).min(x1));
            if e - a > 0.08 {
                // Soot: darker low in the middle, where the fire burns.
                let cxn = ((a + e) / 2.0 - (x0 + x1) / 2.0).abs() / 1.5;
                let soot = (1.0 - (y - y0) / 2.2).clamp(0.0, 1.0) * (1.0 - cxn).clamp(0.0, 1.0);
                let n = ((row * 7 + k * 13) % 5) as f32 / 5.0;
                let base = [0.46 + 0.08 * n, 0.2 + 0.04 * n, 0.14 + 0.03 * n];
                let f = 1.0 - 0.7 * soot;
                b.rounded_box(
                    Vec3::new((a + e) / 2.0, y, z + 0.05),
                    Quat::IDENTITY,
                    Vec3::new((e - a) / 2.0 - 0.015, bh / 2.0 - 0.015, 0.05),
                    0.02,
                    [base[0] * f, base[1] * f, base[2] * f, 1.0],
                );
            }
            x += bw;
            k += 1;
        }
    }
    // The grate: six bars front to back on two cross rails.
    for i in 0..6 {
        let x = x0 + 0.45 + i as f32 * 0.42;
        b.rounded_box(
            Vec3::new(x, y0 + 0.3, z + 0.85),
            Quat::IDENTITY,
            Vec3::new(0.035, 0.035, 0.7),
            0.015,
            iron,
        );
    }
    for dz in [0.35f32, 1.35] {
        b.rounded_box(
            Vec3::new((x0 + x1) / 2.0, y0 + 0.24, z + dz),
            Quat::IDENTITY,
            Vec3::new(1.2, 0.04, 0.04),
            0.015,
            iron,
        );
        for sx in [x0 + 0.35, x1 - 0.35] {
            b.rounded_box(
                Vec3::new(sx, y0 + 0.12, z + dz),
                Quat::IDENTITY,
                Vec3::new(0.04, 0.12, 0.04),
                0.015,
                iron,
            );
        }
    }
    // Three logs on the grate, two side by side and one across them.
    for (at, rot) in [
        (
            Vec3::new(128.1, y0 + 0.48, z + 0.85),
            Quat::from_rotation_y(0.15),
        ),
        (
            Vec3::new(128.9, y0 + 0.48, z + 0.85),
            Quat::from_rotation_y(-0.12),
        ),
        (
            Vec3::new(128.5, y0 + 0.78, z + 0.8),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2 + 0.2),
        ),
    ] {
        b.rounded_box(at, rot, Vec3::new(0.16, 0.15, 0.6), 0.12, char_log);
        b.rounded_box(
            at + rot * Vec3::new(0.0, 0.0, 0.6),
            rot,
            Vec3::new(0.13, 0.12, 0.02),
            0.05,
            bark,
        );
    }
    b.mesh()
}

/// The thin teal rune line round the inside of the frame (drawn with the
/// glowing rune material): one hair-thin bar per voussoir and up each jamb.
pub fn kiln_frame_runes() -> Mesh {
    let mut b = Builder::default();
    let teal = hex(0x40FFD9);
    let (cx, sy, r) = KILN_ARCH;
    let z = KILN_DOOR_Z + 0.73;
    let rl = r + 0.12;
    let n = 18;
    for k in 0..n {
        let a = (k as f32 + 0.5) / n as f32 * std::f32::consts::PI;
        let at = Vec3::new(cx + rl * a.cos(), sy + rl * a.sin(), z);
        let half = Vec3::new(0.022, rl * std::f32::consts::PI / n as f32 * 0.5, 0.01);
        b.rounded_box(at, Quat::from_rotation_z(a), half, 0.005, teal);
    }
    for side in [-1.0f32, 1.0] {
        b.rounded_box(
            Vec3::new(cx + side * rl, 42.05, z),
            Quat::IDENTITY,
            Vec3::new(0.022, 0.95, 0.01),
            0.005,
            teal,
        );
    }
    b.mesh()
}

/// The plank door leaf, in the hinge's space (+x across the doorway, +z
/// out of the Kiln): five bevelled planks cut to the arch, two iron straps
/// with studs and a ring pull.
pub fn kiln_door_leaf() -> Mesh {
    let mut b = Builder::default();
    let timber = hex(0x8A5A3B);
    let timber2 = hex(0x7A4E32);
    let iron = hex(0x34353A);
    let (_, sy, r) = KILN_ARCH;
    let w = 2.0 * r;
    let n = 5;
    let pw = w / n as f32;
    for k in 0..n {
        let xc = (k as f32 + 0.5) * pw;
        let dx = (xc - r).abs() + pw * 0.25;
        let top = sy - 41.0 + (r * r - dx.min(r) * dx.min(r)).sqrt() - 0.04;
        let c = if k % 2 == 0 { timber } else { timber2 };
        b.rounded_box(
            Vec3::new(xc, top / 2.0, 0.0),
            Quat::IDENTITY,
            Vec3::new(pw * 0.5 - 0.012, top / 2.0, 0.07),
            0.035,
            c,
        );
    }
    for y in [0.55f32, 1.75] {
        b.rounded_box(
            Vec3::new(r, y, 0.08),
            Quat::IDENTITY,
            Vec3::new(r - 0.12, 0.07, 0.025),
            0.02,
            iron,
        );
        for k in 0..6 {
            let x = 0.3 + k as f32 * (w - 0.6) / 5.0;
            b.sphere(
                Vec3::new(x, y, 0.11),
                Quat::IDENTITY,
                Vec3::splat(0.035),
                iron,
            );
        }
    }
    b.column(
        Vec3::new(w - 0.45, 1.2, 0.08),
        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        0.11,
        0.11,
        0.04,
        10,
        0.015,
        iron,
    );
    b.mesh()
}

/// The Kiln Spark's socket on the door: a thin teal ring and dot (rune
/// material), in the hinge's space.
pub fn kiln_door_runes() -> Mesh {
    let mut b = Builder::default();
    let teal = hex(0x40FFD9);
    let (_, _, r) = KILN_ARCH;
    let c = Vec3::new(r, 1.2, 0.08);
    let n = 12;
    let rr = 0.2;
    for k in 0..n {
        let a = k as f32 / n as f32 * std::f32::consts::TAU;
        let at = c + Vec3::new(rr * a.cos(), rr * a.sin(), 0.0);
        let half = Vec3::new(0.018, rr * std::f32::consts::PI / n as f32 * 0.55, 0.012);
        b.rounded_box(at, Quat::from_rotation_z(a), half, 0.005, teal);
    }
    b.sphere(
        c + Vec3::Z * 0.01,
        Quat::IDENTITY,
        Vec3::new(0.05, 0.05, 0.02),
        teal,
    );
    b.mesh()
}
