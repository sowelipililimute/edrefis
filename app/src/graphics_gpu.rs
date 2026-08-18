// SPDX-FileCopyrightText: 2024 Janet Blackquill <uhhadd@gmail.com>
//
// SPDX-License-Identifier: MPL-2.0

use crate::gpu::{parallelogram, rectangle, Camera2D, Camera3D, State};
use glam::{Vec2, Vec3};
use logic::{
    field::level_to_gravity,
    piece::Piece,
    well::{Block, BlockDirections, Well, WELL_COLS, WELL_ROWS},
};
use std::rc::Rc;

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

// fn color(block: Block) -> wgpu::Color {
//     match block {
//     Block::Red => wgpu::Color { r: 1.0, g: 0.0, b: 0.18823529411764706, a: 1.0 },
//     Block::Orange => wgpu::Color { r: 1.0, g: 0.4392156862745098, b: 0.0, a: 1.0 },
//     Block::Yellow => wgpu::Color { r: 1.0, g: 0.7647058823529411, b: 0.0, a: 1.0 },
//     Block::Green => wgpu::Color { r: 0.4588235294117647, g: 0.9333333333333333, b: 0.2235294117647059, a: 1.0 },
//     Block::Cyan => wgpu::Color { r: 0.0, g: 0.9411764705882353, b: 0.8274509803921568, a: 1.0 },
//     Block::Blue => wgpu::Color { r: 0.25098039215686274, g: 0.6235294117647059, b: 0.9725490196078431, a: 1.0 },
//     Block::Purple => wgpu::Color { r: 0.7137254901960784, g: 0.47058823529411764, b: 0.9607843137254902, a: 1.0 },
//     }
// }

fn tilemap_position(block: Block, directions: BlockDirections) -> Vec2 {
    Vec2::new(
        directions.bits() as f32 * TILEMAP_WIDTH,
        texture_index(block) as f32 * 1. / 8.,
    )
}

const TILEMAP_WIDTH: f32 = 1. / 16.;
const TILEMAP_HEIGHT: f32 = 1. / 8.;

pub struct Graphics {
    tilemap: Rc<wgpu::BindGroup>,
    level000: Rc<wgpu::BindGroup>,
    level100: Rc<wgpu::BindGroup>,
    level200: Rc<wgpu::BindGroup>,
    level300: Rc<wgpu::BindGroup>,
    level400: Rc<wgpu::BindGroup>,
    level500: Rc<wgpu::BindGroup>,
    level600: Rc<wgpu::BindGroup>,
    level700: Rc<wgpu::BindGroup>,
    level800: Rc<wgpu::BindGroup>,
    level900: Rc<wgpu::BindGroup>,
    level1000: Rc<wgpu::BindGroup>,
    well: (Rc<wgpu::BindGroup>, Rc<wgpu::TextureView>),
    next: (Rc<wgpu::BindGroup>, Rc<wgpu::TextureView>),
    score_buffer: glyphon::Buffer,
}

impl Graphics {
    pub fn new(state: &mut State) -> Result<Graphics, String> {
        let tilemap =
            state.upload_texture(include_bytes!("gfx/tiles.png"), wgpu::FilterMode::Linear)?;

        let well = state.create_texture(WELL_COLS as u32 * 8, WELL_ROWS as u32 * 8);
        let next = state.create_texture(4 * 8, 4 * 8);
        let mut buffer = state.create_buffer();
        Graphics::score_text(&mut buffer, state, 0, 0);

        Ok(Graphics {
            tilemap,
            well,
            next,
            score_buffer: buffer,
            level000: state.upload_texture(
                include_bytes!("gfx/level000.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level100: state.upload_texture(
                include_bytes!("gfx/level100.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level200: state.upload_texture(
                include_bytes!("gfx/level200.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level300: state.upload_texture(
                include_bytes!("gfx/level300.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level400: state.upload_texture(
                include_bytes!("gfx/level400.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level500: state.upload_texture(
                include_bytes!("gfx/level500.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level600: state.upload_texture(
                include_bytes!("gfx/level600.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level700: state.upload_texture(
                include_bytes!("gfx/level700.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level800: state.upload_texture(
                include_bytes!("gfx/level800.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level900: state.upload_texture(
                include_bytes!("gfx/level900.png"),
                wgpu::FilterMode::Nearest,
            )?,
            level1000: state.upload_texture(
                include_bytes!("gfx/level1000.png"),
                wgpu::FilterMode::Nearest,
            )?,
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

                    let check = |dx: i32, dy: i32| {
                        let row_idx = i as i32 + dy;
                        let col_idx = j as i32 + dx;
                        if row_idx < 0 || col_idx < 0 {
                            false
                        } else if row_idx as usize >= rotation.len()
                            || col_idx as usize >= row.len()
                        {
                            false
                        } else {
                            rotation[row_idx as usize][col_idx as usize] != false
                        }
                    };

                    let up = check(0, -1);
                    let down = check(0, 1);
                    let left = check(-1, 0);
                    let right = check(1, 0);

                    state.queue_draw(rectangle(
                        Vec3::new(bx, by, 0.),
                        1.,
                        1.,
                        tilemap_position(piece.color, BlockDirections::new(up, down, left, right)),
                        TILEMAP_WIDTH,
                        TILEMAP_HEIGHT,
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
            Some(self.well.1.clone()),
        ));
        state.start_render_pass(Some(wgpu::Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a: 0.,
        }));

        state.set_texture(Some(self.tilemap.clone()));

        for (i, row) in well.blocks.iter().enumerate() {
            for (j, col) in row.iter().enumerate() {
                if let Some(block) = col {
                    let bx = j as f32;
                    let by = i as f32;

                    let fetch = |dx: i32, dy: i32| {
                        let row_idx = i as i32 + dy;
                        let col_idx = j as i32 + dx;
                        if row_idx < 0 || col_idx < 0 {
                            None
                        } else if row_idx as usize >= WELL_ROWS || col_idx as usize >= WELL_COLS {
                            None
                        } else {
                            well.blocks[row_idx as usize][col_idx as usize]
                                .filter(|it| it.color == block.color)
                                .map(|it| it.directions)
                        }
                    };

                    let up = fetch(0, -1);
                    let down = fetch(0, 1);
                    let left = fetch(-1, 0);
                    let right = fetch(1, 0);

                    state.queue_draw(rectangle(
                        Vec3::new(bx, by, 0.),
                        1.,
                        1.,
                        tilemap_position(
                            block.color,
                            block.directions.match_with(up, down, left, right),
                        ),
                        TILEMAP_WIDTH,
                        TILEMAP_HEIGHT,
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
                        Vec2::new(0., 0.),
                        1.,
                        1.,
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
                    let bx = j as f32 * DST_BLOCK_SIZE;
                    let by = i as f32 * DST_BLOCK_SIZE;

                    let check = |dx: i32, dy: i32| {
                        let row_idx = i as i32 + dy;
                        let col_idx = j as i32 + dx;
                        if row_idx < 0 || col_idx < 0 {
                            false
                        } else if row_idx as usize >= WELL_ROWS || col_idx as usize >= WELL_COLS {
                            false
                        } else {
                            well.blocks[row_idx as usize][col_idx as usize].is_none()
                        }
                    };

                    let mut top = false;
                    let mut left = false;
                    let mut right = false;
                    let mut bottom = false;

                    if check(0, -1) {
                        state.queue_draw(rectangle(
                            Vec3::new(bx, by, 0.),
                            DST_BLOCK_SIZE,
                            DST_PIXEL_SIZE,
                            Vec2::ZERO,
                            1.,
                            1.,
                            pixel_color,
                        ));
                        top = true;
                    }
                    if check(0, 1) {
                        state.queue_draw(rectangle(
                            Vec3::new(bx, by + DST_BLOCK_SIZE - DST_PIXEL_SIZE, 0.),
                            DST_BLOCK_SIZE,
                            DST_PIXEL_SIZE,
                            Vec2::ZERO,
                            1.,
                            1.,
                            pixel_color,
                        ));
                        bottom = true;
                    }
                    if check(-1, 0) {
                        state.queue_draw(rectangle(
                            Vec3::new(bx, by, 0.),
                            DST_PIXEL_SIZE,
                            DST_BLOCK_SIZE,
                            Vec2::ZERO,
                            1.,
                            1.,
                            pixel_color,
                        ));
                        left = true;
                    }
                    if check(1, 0) {
                        state.queue_draw(rectangle(
                            Vec3::new(bx + DST_BLOCK_SIZE - DST_PIXEL_SIZE, by, 0.),
                            DST_PIXEL_SIZE,
                            DST_BLOCK_SIZE,
                            Vec2::ZERO,
                            1.,
                            1.,
                            pixel_color,
                        ));
                        right = true;
                    }

                    if !left && !top && check(-1, -1) {
                        state.queue_draw(rectangle(
                            Vec3::new(bx, by, 0.),
                            DST_PIXEL_SIZE,
                            DST_PIXEL_SIZE,
                            Vec2::ZERO,
                            1.,
                            1.,
                            pixel_color,
                        ));
                    }
                    if !right && !top && check(1, -1) {
                        state.queue_draw(rectangle(
                            Vec3::new(bx + DST_BLOCK_SIZE - DST_PIXEL_SIZE, by, 0.),
                            DST_PIXEL_SIZE,
                            DST_PIXEL_SIZE,
                            Vec2::ZERO,
                            1.,
                            1.,
                            pixel_color,
                        ));
                    }
                    if !left && !bottom && check(-1, 1) {
                        state.queue_draw(rectangle(
                            Vec3::new(bx, by + DST_BLOCK_SIZE - DST_PIXEL_SIZE, 0.),
                            DST_PIXEL_SIZE,
                            DST_PIXEL_SIZE,
                            Vec2::ZERO,
                            1.,
                            1.,
                            pixel_color,
                        ));
                    }
                    if !right && !bottom && check(1, 1) {
                        state.queue_draw(rectangle(
                            Vec3::new(
                                bx + DST_BLOCK_SIZE - DST_PIXEL_SIZE,
                                by + DST_BLOCK_SIZE - DST_PIXEL_SIZE,
                                0.,
                            ),
                            DST_PIXEL_SIZE,
                            DST_PIXEL_SIZE,
                            Vec2::ZERO,
                            1.,
                            1.,
                            pixel_color,
                        ));
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
                            Vec2::new(0., 0.),
                            1.,
                            1.,
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
            Some(self.next.1.clone()),
        ));

        state.start_render_pass(Some(wgpu::Color::TRANSPARENT));
        state.set_texture(Some(self.tilemap.clone()));
        self.queue_piece(next, false, state);
        state.do_draw()?;
        state.complete_render_pass()?;

        Ok(())
    }
    pub fn render_background(&self, level: u32, state: &mut State) -> Result<(), String> {
        let bg = if level >= 1000 {
            &self.level1000
        } else if level >= 900 {
            &self.level900
        } else if level >= 800 {
            &self.level800
        } else if level >= 700 {
            &self.level700
        } else if level >= 600 {
            &self.level600
        } else if level >= 500 {
            &self.level500
        } else if level >= 400 {
            &self.level400
        } else if level >= 300 {
            &self.level300
        } else if level >= 200 {
            &self.level200
        } else if level >= 100 {
            &self.level100
        } else {
            &self.level000
        };

        state.set_texture(Some(bg.clone()));

        state.queue_draw(rectangle(
            Vec3::ZERO,
            1.,
            1.,
            Vec2::ZERO,
            1.,
            1.,
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
        }));
        self.render_background(level, state)?;

        state.set_camera(&Camera3D::default());

        state.set_texture(None);

        Graphics::queue_well_bg(state);
        state.do_draw()?;

        state.set_texture(Some(self.well.0.clone()));

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

        state.set_texture(Some(self.next.0.clone()));
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
