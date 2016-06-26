extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
#[macro_use] extern crate lazy_static;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL, glyph_cache};
use opengl_graphics::glyph_cache::GlyphCache;

use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug)]
enum Block {
    Blocked,
    Weighted(u32, bool),
}

pub struct App {
    gl: GlGraphics,
    height: u32,
    width: u32,
    path: Vec<(u32, u32)>,
    grid: Vec<Vec<Block>>,
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;


        let side: f64 = args.draw_width as f64 / self.width as f64; // Square side length

        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
        const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

        let len = self.grid.len() as usize;
        let grid = &self.grid;

        let mut font = GlyphCache::new("assets/Ubuntu-R.ttf").unwrap();

        self.gl.draw(args.viewport(), |c, gl| {
            clear(BLACK, gl);

            let square = rectangle::square(0.0, 0.0, side);

            // Draw grid rectangles
            for i in 0..len {
                for j in 0..len {
                    let col = match grid[i][j] {
                        Block::Blocked => RED,
                        Block::Weighted(_, on_path) => { if on_path { BLUE } else { GREEN } }
                    };
                    let t_i = i as f64;
                    let t_j = j as f64;

                    let (x, y) = (t_i*side + t_i, t_j*side + t_j);

                    let transform = c.transform.trans(x, y);
                    rectangle(col, square, transform, gl);

                    match grid[i][j] {
                        Block::Weighted(w, on_path) => {
                            let col = if on_path { WHITE } else { BLACK };
                            let (f_x, f_y) = (x+5f64, y+15f64);
                            let transform = c.transform.trans(f_x, f_y);
                            text(col, 14, &w.to_string(), &mut font, transform, gl);
                        },
                        _ => {}
                    };
                }
            }
        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        // Do nothing
    }

    fn click(&mut self, args: &Button) {
        println!("HERE");
    }
}

fn main() {
    let opengl = OpenGL::V3_2;

    const WINDOW_HEIGHT: u32 = 800;
    const WINDOW_WIDTH: u32 = 800;

    let mut window: Window =
        WindowSettings::new("spinning-square", [WINDOW_WIDTH, WINDOW_HEIGHT])
    .opengl(opengl)
    .exit_on_esc(true)
    .build()
    .unwrap();



    let mut app = App {
        gl: GlGraphics::new(opengl),
        path: Vec::new(),
        height: 10,
        width: 10,
        grid: read_map(),
    };

    let mut events = window.events();
    events.set_max_fps(1);
    events.set_ups(1);
    while let Some(e) = events.next(&mut window) {
        println!("{:?}", e);
        if let Some(r) = e.render_args() {
            app.render(&r);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }

        if let Some(c) = e.press_args() {
            app.click(&c);
        }
    }
}

fn read_map() -> Vec<Vec<Block>> {
    let mut grid: Vec<Vec<Block>> = Vec::new();
    let file = File::open("assets/map.txt").unwrap();
    let reader = BufReader::new(file);

    let lines = reader.lines();
    for line in lines {
        grid.push(chop_line(&line.unwrap()));
    }

    return grid;
}

fn chop_line(line: &String) -> Vec<Block> {
    let mut grid_line: Vec<Block> = Vec::new();
    let columns: Vec<&str> = line.split(' ').collect::<Vec<_>>();

    for col in columns {
        match col.parse::<i64>() {
            Ok(i) if i < 0 => {
                grid_line.push(Block::Blocked)
            },
            Ok(i) => grid_line.push(Block::Weighted(i as u32, true)),
            Err(_) => {}
        };
    }

    return grid_line;
}
