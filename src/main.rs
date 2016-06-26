#![feature(associated_consts)]

extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
#[macro_use] extern crate lazy_static;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL};
use opengl_graphics::glyph_cache::GlyphCache;

use std::io::prelude::*;
use std::fs::File;
use std::io::BufReader;
use std::collections::LinkedList;

#[derive(Debug)]
enum Block {
    Blocked,
    Weighted { weight: u32, on_path: bool },
}

pub struct App {
    gl: GlGraphics,
    window_side: u32,
    path: LinkedList<(u32, u32)>,
    grid: Vec<Vec<Block>>,
    location: (u32, u32),
    start: Option<(u32, u32)>,
    end: Option<(u32, u32)>
}

impl App {
    const SIDE: u32 = 20;

    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        let len = self.grid.len() as usize;
        let side: f64 = (args.draw_width as f64 - len as f64 + 1f64) / App::SIDE as f64; // Square side length

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
                        Block::Weighted {weight: _, on_path } => { if on_path { BLUE } else { GREEN } }
                    };
                    let t_i = i as f64;
                    let t_j = j as f64;

                    let (x, y) = (t_i*side + t_i, t_j*side + t_j);

                    let transform = c.transform.trans(x, y);
                    rectangle(col, square, transform, gl);

                    match grid[i][j] {
                        Block::Weighted {weight: w, on_path} => {
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

    fn clear_path(&mut self) {
        {
            let mut it = self.path.iter();
            while let Some(current) = it.next() {
                match self.grid[current.0 as usize][current.1 as usize] {
                    Block::Weighted {weight: w, on_path: _ } => {
                        self.grid[current.0 as usize][current.1 as usize] = Block::Weighted { weight: w, on_path: false };
                    },
                    _ => {}
                };
            };
        }

        self.path.clear();
    }

    fn get_neighbors(&self, current: &(u32, u32)) -> LinkedList<(u32, u32)> {
        let mut neighbors = LinkedList::new();
        for x in -1i32..2i32 {
            for y in -1i32..2i32 {
                if (x, y) == (0,0) { continue; }
                let n_x = current.0 as i32 + x;
                let n_y = current.1 as i32 + y;
                if n_x < 0 || n_x >= App::SIDE as i32 { continue; }
                if n_y < 0 || n_y >= App::SIDE as i32{ continue; }
                match self.grid[n_x as usize][n_y as usize] {
                    Block::Blocked => continue,
                    _ => {}
                }
                neighbors.push_back((n_x as u32, n_y as u32));
            }
        }
        neighbors
    }

    fn calc_path(&mut self) {
        type CameFromType = (u32, u32, bool);

        let mut frontier = LinkedList::new();

        if self.start.is_none() || self.end.is_none() { return };

        frontier.push_back(self.start.clone().unwrap());

        let mut came_from = [[(0,0,false); App::SIDE as usize]; App::SIDE as usize];

        'outer: while !frontier.is_empty() {
            let current = frontier.pop_front();
            if let Some(cur) = current {
                let neighbors = self.get_neighbors(&cur);
                let mut it = neighbors.iter();
                while let Some(neighbor) = it.next() {
                    if !came_from[neighbor.0 as usize][neighbor.1 as usize].2 {
                        frontier.push_back(neighbor.clone());
                        came_from[neighbor.0 as usize][neighbor.1 as usize] = (cur.0, cur.1, true);
                        if neighbor == &self.end.unwrap() { break 'outer; }
                    }
                }
            }
        }

        self.clear_path();

        let mut current = self.end.clone().unwrap();
        let goal = self.start.clone().unwrap();
        self.path.push_back(current);
        while current != goal {
            current = (came_from[current.0 as usize][current.1 as usize].0, came_from[current.0 as usize][current.1 as usize].1);
            self.path.push_back(current);
        }
        self.enter_path();
    }

    fn enter_path(&mut self) {
        {
            let mut it = self.path.iter();
            while let Some(current) = it.next() {
                match self.grid[current.0 as usize][current.1 as usize] {
                    Block::Weighted {weight: w, on_path: _ } => {
                        self.grid[current.0 as usize][current.1 as usize] = Block::Weighted { weight: w, on_path: true };
                    },
                    _ => {}
                };
            };
        }
    }

    fn click(&mut self, args: MouseButton) {
        let offset = self.window_side / App::SIDE;
        let x = self.location.0 / offset as u32;
        let y = self.location.1 / offset as u32;
        match args {
            mouse::MouseButton::Left => {
                if self.end.is_none() ||  self.end.clone().unwrap() != (x,y) {
                    self.start = Some((x,y));
                    self.calc_path();
                }
            },
            mouse::MouseButton::Right => {
                if self.start.is_none() || self.start.clone().unwrap() != (x,y) {
                    self.end = Some((x,y));
                    self.calc_path();
                }

            },
            _ => {}
        }
    }
}

fn main() {
    let opengl = OpenGL::V3_2;

    const WINDOW_SIDE: u32 = 800;

    let mut window: Window =
        WindowSettings::new("spinning-square", [WINDOW_SIDE, WINDOW_SIDE])
    .opengl(opengl)
    .exit_on_esc(true)
    .build()
    .unwrap();



    let mut app = App {
        gl: GlGraphics::new(opengl),
        window_side: WINDOW_SIDE,
        path: LinkedList::new(),
        grid: read_map(),
        location: (0, 0),
        start: None,
        end: None,
    };

    let mut events = window.events();
    events.set_max_fps(1);
    events.set_ups(10);
    while let Some(e) = events.next(&mut window) {

        if let Some(r) = e.render_args() {
            app.render(&r);
        }

        if let Some(c) = e.press_args() {
            match c {
                Button::Mouse(button) => app.click(button),
                _ => {}
            }
        }

        if let Some(c) = e.mouse_cursor_args() {
            let temp: (u32, u32) = (c[0] as u32, c[1] as u32);
            app.location = temp;
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
            Ok(i) => grid_line.push(Block::Weighted{weight: i as u32, on_path: false}),
            Err(_) => {}
        };
    }

    return grid_line;
}
