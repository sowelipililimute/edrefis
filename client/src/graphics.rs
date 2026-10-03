// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use crate::gpu::{Camera2D, Camera3D, State, Texture, parallelogram, rectangle, solid_rectangle};
use glam::{Vec2, Vec3};
use logic::{
    field::level_to_gravity,
    piece::Piece,
    well::{Block, BlockDirections, WELL_COLS, WELL_ROWS, Well},
};

fn lerp(a: f32, b: f32, f: f32) -> f32 {
    a * (1.0 - f) + (b * f)
}

fn texture_index(block: Block) -> i32 {
    match block {
        Block::Red => 0,
        Block::Orange => 1,
        Block::Yellow => 2,
        Block::Green => 3,
        Block::Cyan => 4,
        Block::Blue => 5,
        Block::Purple => 6,
    }
}

struct GridAtlas {
    cols: u32,
    rows: u32,
}

impl GridAtlas {
    fn uv(&self, col: u32, row: u32) -> (Vec2, Vec2) {
        let size = Vec2::new(1. / self.cols as f32, 1. / self.rows as f32);
        (Vec2::new(col as f32, row as f32) * size, size)
    }
}

const TILES_ATLAS: GridAtlas = GridAtlas { cols: 16, rows: 8 };

pub struct Graphics {
    tilemap: Texture,
    backgrounds: Vec<Texture>,
    well: Texture,
    next: Texture,
    score_buffer: glyphon::Buffer,
}

const BACKGROUNDS: &[&[u8]] = &[
    include_bytes!("gfx/level000.png"),
    include_bytes!("gfx/level100.png"),
    include_bytes!("gfx/level200.png"),
    include_bytes!("gfx/level300.png"),
    include_bytes!("gfx/level400.png"),
    include_bytes!("gfx/level500.png"),
    include_bytes!("gfx/level600.png"),
    include_bytes!("gfx/level700.png"),
    include_bytes!("gfx/level800.png"),
    include_bytes!("gfx/level900.png"),
    include_bytes!("gfx/level1000.png"),
];

fn at<T, R: AsRef<[T]>>(rows: &[R], x: i32, y: i32) -> Option<&T> {
    let row = rows.get(usize::try_from(y).ok()?)?;
    row.as_ref().get(usize::try_from(x).ok()?)
}

const P: f32 = 1. / 8.;

fn edge_rect(dx: i32, dy: i32) -> (Vec2, Vec2) {
    let pos = |d: i32| if d > 0 { 1. - P } else { 0. };
    let len = |d: i32| if d != 0 { P } else { 1. };
    (Vec2::new(pos(dx), pos(dy)), Vec2::new(len(dx), len(dy)))
}

impl Graphics {
    pub fn new(state: &mut State) -> Result<Graphics, String> {
        let tilemap =
            state.texture_from_png(include_bytes!("gfx/tiles.png"), wgpu::FilterMode::Linear)?;

        let well = state.render_target(WELL_COLS as u32 * 8, WELL_ROWS as u32 * 8);
        let next = state.render_target(4 * 8, 4 * 8);
        let mut buffer = state.create_buffer();
        Graphics::score_text(&mut buffer, state, 0, 0);

        let backgrounds = BACKGROUNDS
            .iter()
            .map(|png| state.texture_from_png(png, wgpu::FilterMode::Nearest))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Graphics {
            tilemap,
            well,
            next,
            score_buffer: buffer,
            backgrounds,
        })
    }
    pub fn score_text(buffer: &mut glyphon::Buffer, state: &mut State, gravity: i32, level: u32) {
        let attrs = glyphon::Attrs::new()
            .family(glyphon::Family::Name("Hanken Grotesk"))
            .weight(glyphon::Weight::MEDIUM)
            .color(glyphon::Color::rgba(255, 255, 255, 180));

        let is_20g = gravity >= 256;
        let gravity_amount = if !is_20g { gravity / 2 } else { gravity / 256 };

        state.set_buffer_text(
            buffer,
            [
                (
                    "Gravity\n",
                    attrs.metrics(glyphon::Metrics::relative(24., 1.2)),
                ),
                (
                    &format!("{}", gravity_amount),
                    attrs
                        .metrics(glyphon::Metrics::relative(32., 1.2))
                        .weight(glyphon::Weight::BOLD)
                        .color(glyphon::Color::rgba(255, 255, 255, 255)),
                ),
                if is_20g {
                    ("G", attrs.metrics(glyphon::Metrics::relative(32., 1.2)))
                } else {
                    (" /128", attrs.metrics(glyphon::Metrics::relative(24., 1.2)))
                },
                ("\n", attrs),
                (
                    "Level\n",
                    attrs.metrics(glyphon::Metrics::relative(24., 1.2)),
                ),
                (
                    &format!("{}", level),
                    attrs
                        .metrics(glyphon::Metrics::relative(32., 1.2))
                        .weight(glyphon::Weight::BOLD)
                        .color(glyphon::Color::rgba(255, 255, 255, 255)),
                ),
                (" /", attrs.metrics(glyphon::Metrics::relative(24., 1.2))),
                (
                    &format!("{}\n", ((level / 100) + 1) * 100),
                    attrs.metrics(glyphon::Metrics::relative(24., 1.2)),
                ),
            ],
            attrs,
        );
    }
    pub fn queue_well_bg(state: &mut State) {
        let well_width = WELL_COLS as f32;
        let well_height = WELL_ROWS as f32;
        let wall = wgpu::Color {
            r: 0.77625,
            g: 0.96804,
            b: 1.00513,
            a: 0.1,
        };

        // well bg
        state.queue_draw(parallelogram(
            Vec3::new(well_width / -2., well_height / -2., -1.),
            well_width * Vec3::X,
            well_height * Vec3::Y,
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            wgpu::Color {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 0.4,
            },
        ));

        // bottom
        state.queue_draw(parallelogram(
            Vec3::new(well_width / -2., well_height / -2., -1.),
            well_width * Vec3::X,
            2. * Vec3::Z,
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            wall,
        ));

        // left
        state.queue_draw(parallelogram(
            Vec3::new(well_width / -2., well_height / -2., -1.),
            well_height * Vec3::Y,
            2. * Vec3::Z,
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            wall,
        ));

        // right
        state.queue_draw(parallelogram(
            Vec3::new(well_width / 2., well_height / -2., -1.),
            well_height * Vec3::Y,
            2. * Vec3::Z,
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            wall,
        ));
    }
    pub fn queue_piece(&self, piece: &Piece, respect_position: bool, state: &mut State) {
        let rotation = piece.rotations.piece_map()[piece.rotation];
        for (i, row) in rotation.iter().enumerate() {
            for (j, col) in row.iter().enumerate() {
                if *col {
                    let bx = if respect_position { piece.x as f32 } else { 0. } + j as f32;
                    let by = if respect_position { piece.y as f32 } else { 0. } + i as f32;

                    let check = |dx, dy| {
                        at(rotation, j as i32 + dx, i as i32 + dy)
                            .copied()
                            .unwrap_or(false)
                    };

                    let up = check(0, -1);
                    let down = check(0, 1);
                    let left = check(-1, 0);
                    let right = check(1, 0);

                    let (uv_pos, uv_size) = TILES_ATLAS.uv(
                        BlockDirections::new(up, down, left, right).bits() as u32,
                        texture_index(piece.color) as u32,
                    );

                    state.queue_draw(rectangle(
                        Vec3::new(bx, by, 0.),
                        1.,
                        1.,
                        uv_pos,
                        uv_size,
                        wgpu::Color::WHITE,
                    ));
                }
            }
        }
    }
    pub fn render_well(
        &self,
        well: &Well,
        piece: Option<&Piece>,
        state: &mut State,
    ) -> Result<(), String> {
        state.set_camera(&Camera2D::from_rect(
            Vec2::new(0., 0.),
            Vec2::new(WELL_COLS as f32, WELL_ROWS as f32),
            Some(self.well.view.clone()),
        ));
        state.start_render_pass(Some(wgpu::Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 0.,
        }))?;

        state.set_texture(Some(&self.tilemap));

        for (i, row) in well.blocks.iter().enumerate() {
            for (j, col) in row.iter().enumerate() {
                if let Some(block) = col {
                    let bx = j as f32;
                    let by = i as f32;

                    let fetch = |dx: i32, dy: i32| {
                        at(&well.blocks, j as i32 + dx, i as i32 + dy)
                            .and_then(|b| b.as_ref())
                            .filter(|b| b.color == block.color)
                            .map(|b| b.directions)
                    };

                    let up = fetch(0, -1);
                    let down = fetch(0, 1);
                    let left = fetch(-1, 0);
                    let right = fetch(1, 0);

                    let (uv_pos, uv_size) = TILES_ATLAS.uv(
                        block.directions.match_with(up, down, left, right).bits() as u32,
                        texture_index(block.color) as u32,
                    );

                    state.queue_draw(rectangle(
                        Vec3::new(bx, by, 0.),
                        1.,
                        1.,
                        uv_pos,
                        uv_size,
                        wgpu::Color::WHITE,
                    ));
                }
            }
        }

        if let Some(piece) = piece {
            self.queue_piece(piece, true, state);
        }

        state.do_draw()?;

        state.set_texture(None);

        for (i, row) in well.blocks.iter().enumerate() {
            for (j, col) in row.iter().enumerate() {
                if col.is_some() {
                    let bx = j as f32;
                    let by = i as f32;

                    state.queue_draw(rectangle(
                        Vec3::new(bx, by, 0.),
                        1.,
                        1.,
                        Vec2::ZERO,
                        Vec2::ONE,
                        wgpu::Color {
                            r: 0.,
                            g: 0.,
                            b: 0.,
                            a: 0.5,
                        },
                    ));
                }
            }
        }

        let pixel_color = wgpu::Color {
            r: 0.9,
            g: 0.9,
            b: 0.9,
            a: 0.4,
        };
        const DST_BLOCK_SIZE: f32 = 1.;
        const DST_PIXEL_SIZE: f32 = 1. / 8.;

        for (i, row) in well.blocks.iter().enumerate() {
            for (j, col) in row.iter().enumerate() {
                if col.is_some() {
                    let cell = Vec2::new(j as f32, i as f32);

                    let check = |dx, dy| {
                        matches!(at(&well.blocks, j as i32 + dx, i as i32 + dy), Some(None))
                    };

                    for (dx, dy) in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                        if check(dx, dy) {
                            let (off, size) = edge_rect(dx, dy);
                            state.queue_draw(solid_rectangle(cell + off, size, pixel_color));
                        }
                    }
                    for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                        if !check(dx, 0) && !check(0, dy) && check(dx, dy) {
                            let (off, size) = edge_rect(dx, dy);
                            state.queue_draw(solid_rectangle(cell + off, size, pixel_color));
                        }
                    }
                }
            }
        }

        if let Some(piece) = piece {
            for (i, row) in piece.rotations.piece_map()[piece.rotation]
                .iter()
                .enumerate()
            {
                for (j, col) in row.iter().enumerate() {
                    if *col {
                        let bx = piece.x as f32 + j as f32;
                        let by = piece.y as f32 + i as f32;

                        state.queue_draw(rectangle(
                            Vec3::new(bx, by, 0.),
                            1.,
                            1.,
                            Vec2::ZERO,
                            Vec2::ONE,
                            wgpu::Color {
                                r: 0.,
                                g: 0.,
                                b: 0.,
                                a: lerp(0.8, 0., piece.ticks_to_lock as f32 / 30.) as f64,
                            },
                        ));
                    }
                }
            }
        }
        state.do_draw()?;
        state.complete_render_pass()?;

        Ok(())
    }
    pub fn render_next(&mut self, next: &Piece, state: &mut State) -> Result<(), String> {
        state.set_camera(&Camera2D::from_rect(
            Vec2::new(0., 0.),
            Vec2::new(4., 4.),
            Some(self.next.view.clone()),
        ));

        state.start_render_pass(Some(wgpu::Color::TRANSPARENT))?;
        state.set_texture(Some(&self.tilemap));
        self.queue_piece(next, false, state);
        state.do_draw()?;
        state.complete_render_pass()?;

        Ok(())
    }
    pub fn render_background(&self, level: u32, state: &mut State) -> Result<(), String> {
        let bg = &self.backgrounds[(level / 100).min(self.backgrounds.len() as u32 - 1) as usize];

        state.set_texture(Some(bg));

        state.queue_draw(rectangle(
            Vec3::ZERO,
            1.,
            1.,
            Vec2::ZERO,
            Vec2::ONE,
            wgpu::Color::WHITE,
        ));
        state.do_draw()?;

        Ok(())
    }
    pub fn render(
        &mut self,
        level: u32,
        well: &Well,
        piece: Option<&Piece>,
        next: &Piece,
        state: &mut State,
    ) -> Result<(), String> {
        self.render_well(well, piece, state)?;
        self.render_next(next, state)?;

        state.set_camera(&Camera2D::from_rect(Vec2::ZERO, Vec2::new(1., 1.), None));
        state.start_render_pass(Some(wgpu::Color {
            r: 0.05,
            g: 0.05,
            b: 0.1,
            a: 1.0,
        }))?;
        self.render_background(level, state)?;

        state.set_camera(&Camera3D::default());

        state.set_texture(None);

        Graphics::queue_well_bg(state);
        state.do_draw()?;

        state.set_texture(Some(&self.well));

        let well_width = WELL_COLS as f32;
        let well_height = WELL_ROWS as f32;
        state.queue_draw(parallelogram(
            Vec3::new(well_width / -2., well_height / -2., 0.),
            well_width * Vec3::X,
            well_height * Vec3::Y,
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            wgpu::Color::WHITE,
        ));

        state.do_draw()?;

        state.set_texture(Some(&self.next));
        state.queue_draw(parallelogram(
            Vec3::new(4. / -2., 4. / -2. + well_height / 2. + 1.5, 0.),
            4. * Vec3::X,
            4. * Vec3::Y,
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            wgpu::Color::WHITE,
        ));
        state.do_draw()?;

        let point = state.world_to_view(Vec3::new(well_width / 2. + 1., well_height / 2., 0.));
        Graphics::score_text(
            &mut self.score_buffer,
            state,
            level_to_gravity(level),
            level,
        );
        state.draw_text(&mut self.score_buffer, point)?;

        state.complete_render_pass()?;

        state.present()?;

        Ok(())
    }
}
