//! A 3D version of the falling-sand step, sized like the slice design's
//! "reaction bubble": awake 16x16x16 chunks with species + temperature.
//! Rules (deliberately close to `crates/sim/src/grid.rs` in cost): solids
//! stay, powder and liquid fall (straight, then 4 diagonals), liquid spreads
//! sideways, gas rises, and every cell conducts heat to its 6 neighbours.
//! No dependencies, so it builds for wasm32 as a bare module.

pub const N: usize = 16;
const AIR: u8 = 0;
const STONE: u8 = 1;
const SAND: u8 = 2;
const WATER: u8 = 3;
const GAS: u8 = 4;

pub struct World {
    /// Chunks laid out in a row along x: (chunks*16) x 16 x 16 cells.
    pub w: usize,
    pub cells: Vec<u8>,
    pub temp: Vec<f32>,
    next_temp: Vec<f32>,
    moved: Vec<u32>,
    frame: u32,
    rng: u32,
}

impl World {
    pub fn new(chunks: usize) -> World {
        let w = chunks * N;
        let n = w * N * N;
        let mut world = World {
            w,
            cells: vec![AIR; n],
            temp: vec![20.0; n],
            next_temp: vec![20.0; n],
            moved: vec![0; n],
            frame: 0,
            rng: 0x9e37_79b9,
        };
        // A busy mix: stone floor, a sand pile, a water body, rising gas, a hot spot.
        for z in 0..N {
            for y in 0..N {
                for x in 0..w {
                    let i = world.idx(x, y, z);
                    let r = world.rand();
                    world.cells[i] = if y == 0 {
                        STONE
                    } else if y > 10 && r % 3 == 0 {
                        SAND
                    } else if (4..8).contains(&y) && r % 2 == 0 {
                        WATER
                    } else if y < 4 && r % 5 == 0 {
                        GAS
                    } else {
                        AIR
                    };
                    if x % N < 3 && y < 4 {
                        world.temp[i] = 900.0;
                    }
                }
            }
        }
        world
    }

    #[inline]
    fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        (z * N + y) * self.w + x
    }

    #[inline]
    fn rand(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng
    }

    fn try_move(&mut self, from: usize, x: i32, y: i32, z: i32, into: &[u8]) -> bool {
        if x < 0 || y < 0 || z < 0 || x as usize >= self.w || y as usize >= N || z as usize >= N {
            return false;
        }
        let to = self.idx(x as usize, y as usize, z as usize);
        if into.contains(&self.cells[to]) && self.moved[to] != self.frame {
            self.cells.swap(from, to);
            self.temp.swap(from, to);
            self.moved[to] = self.frame;
            return true;
        }
        false
    }

    /// One step over every cell. Returns cells visited.
    pub fn step(&mut self) -> usize {
        self.frame = self.frame.wrapping_add(1);
        const DIAG: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        // Bottom-up for falling; y = 0 is the floor.
        for y in 0..N {
            for z in 0..N {
                for x in 0..self.w {
                    let i = self.idx(x, y, z);
                    if self.moved[i] == self.frame {
                        continue;
                    }
                    let (xi, yi, zi) = (x as i32, y as i32, z as i32);
                    let s = self.cells[i];
                    let r = self.rand() as usize;
                    match s {
                        SAND | WATER => {
                            let into: &[u8] = if s == SAND {
                                &[AIR, GAS, WATER]
                            } else {
                                &[AIR, GAS]
                            };
                            if self.try_move(i, xi, yi - 1, zi, into) {
                                continue;
                            }
                            let (dx, dz) = DIAG[r & 3];
                            if self.try_move(i, xi + dx, yi - 1, zi + dz, into) {
                                continue;
                            }
                            if s == WATER {
                                let (dx, dz) = DIAG[(r >> 2) & 3];
                                self.try_move(i, xi + dx, yi, zi + dz, &[AIR, GAS]);
                            }
                        }
                        GAS => {
                            if !self.try_move(i, xi, yi + 1, zi, &[AIR]) {
                                let (dx, dz) = DIAG[r & 3];
                                self.try_move(i, xi + dx, yi, zi + dz, &[AIR]);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // Heat: explicit 6-neighbour diffusion (double-buffered).
        let w = self.w;
        for z in 0..N {
            for y in 0..N {
                for x in 0..w {
                    let i = self.idx(x, y, z);
                    let t = self.temp[i];
                    let mut sum = 0.0;
                    let mut k = 0.0;
                    if x > 0 {
                        sum += self.temp[i - 1];
                        k += 1.0;
                    }
                    if x + 1 < w {
                        sum += self.temp[i + 1];
                        k += 1.0;
                    }
                    if y > 0 {
                        sum += self.temp[i - w];
                        k += 1.0;
                    }
                    if y + 1 < N {
                        sum += self.temp[i + w];
                        k += 1.0;
                    }
                    if z > 0 {
                        sum += self.temp[i - w * N];
                        k += 1.0;
                    }
                    if z + 1 < N {
                        sum += self.temp[i + w * N];
                        k += 1.0;
                    }
                    let c = if self.cells[i] == STONE { 0.05 } else { 0.15 };
                    self.next_temp[i] = t + c * (sum - k * t) / 6.0;
                }
            }
        }
        std::mem::swap(&mut self.temp, &mut self.next_temp);
        w * N * N
    }
}

/// WASM entry: steps `chunks` awake chunks `steps` times; returns a checksum
/// so the work cannot be optimised away. Time it from JavaScript.
#[no_mangle]
pub extern "C" fn bench(chunks: u32, steps: u32) -> f64 {
    let mut world = World::new(chunks as usize);
    for _ in 0..steps {
        world.step();
    }
    world.temp.iter().map(|&t| t as f64).sum::<f64>()
        + world.cells.iter().map(|&c| c as f64).sum::<f64>()
}
