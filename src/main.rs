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
use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Debug)]
enum Block {
    Blocked,
    Weighted { weight: usize, on_path: bool },
}

pub struct App {
    gl: GlGraphics,
    window_side: usize,
    path: LinkedList<Point>,
    grid: Vec<Vec<Block>>,
    location: Point,
    start: Option<Point>,
    end: Option<Point>
}

#[derive(PartialEq,Eq,Clone,Copy)]
struct Point {
    pub x: usize,
    pub y: usize
}

impl From<(usize, usize)> for Point {
    fn from(f: (usize, usize)) -> Point {
        Point {
            x: f.0,
            y: f.1
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct WeightedPoint {
    pub point: Point,
    pub weight: usize
}

impl Ord for WeightedPoint {
    fn cmp(&self, other: &WeightedPoint) -> Ordering {
        other.weight.cmp(&self.weight)
    }
}
impl PartialOrd for WeightedPoint {
    fn partial_cmp(&self, other: &WeightedPoint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<(Point, usize)> for WeightedPoint {
    fn from(f: (Point, usize)) -> WeightedPoint {
        WeightedPoint {
            point: f.0,
            weight: f.1
        }
    }
}

impl App {
    const SIDE: usize = 40;

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
                            text(col, 16, &w.to_string(), &mut font, transform, gl);
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
                match self.grid[current.x][current.y] {
                    Block::Weighted {weight: w, on_path: _ } => {
                        self.grid[current.x][current.y] = Block::Weighted { weight: w, on_path: false };
                    },
                    _ => {}
                };
            };
        }

        self.path.clear();
    }

    fn is_pathable_point(&self, p: &Point) -> bool {
        if p.x > App::SIDE || p.y > App::SIDE {
            return false;
        }
        match self.grid[p.x][p.y] {
            Block::Blocked => false,
            _ => true
        }
    }

    fn get_diagonal_neighbors(&self, current: &Point) -> LinkedList<Point> {
        let mut neighbors = LinkedList::new();
        for x in -1i32..2i32 {
            for y in -1i32..2i32 {
                if (x, y) == (0,0) { continue; }
                let n_x = current.x as i32 + x;
                let n_y = current.y as i32 + y;
                if n_x < 0 || n_x >= App::SIDE as i32 { continue; }
                if n_y < 0 || n_y >= App::SIDE as i32{ continue; }
                match self.grid[n_x as usize][n_y as usize] {
                    Block::Blocked => continue,
                    _ => {}
                }
                neighbors.push_back(Point::from((n_x as usize, n_y as usize)));
            }
        }
        neighbors
    }

    fn get_neighbors(&self, current: &Point) -> LinkedList<Point> {
        let neighbors = LinkedList::<Point>::new();
        let mut vals: Vec<(i32,i32)> = vec![(0,1), (1,0)];
        if current.x > 0 { vals.push((-1,0)); }
        if current.y > 0 { vals.push((0,-1)); }
        let mut neighbors = LinkedList::new();
        for n in &vals {
            let n_x = ((current.x as i32) + n.0) as usize;
            let n_y = ((current.y as i32) + n.1) as usize;
            let p = Point::from((n_x, n_y));
            if self.is_pathable_point(&p) {
                neighbors.push_back(p);
            }

        }

        neighbors
    }

    fn get_weight(&self, p: Point) -> usize {
        let ref point = self.grid[p.x][p.y];
        match point {
            &Block::Weighted { weight, .. } => weight,
            _ => 0
        }
    }

    fn calc_dijkstras_path(&mut self) {
        let mut frontier = BinaryHeap::new();
        if self.start.is_none() || self.end.is_none() { return };
        let s = WeightedPoint::from((self.start.unwrap(), 0));
        frontier.push(s);

        let mut came_from = [[Point::from((0,0)); App::SIDE]; App::SIDE];
        let mut cost_so_far = [[std::usize::MAX; App::SIDE]; App::SIDE];
        cost_so_far[s.point.x][s.point.y] = 0;

        'outer: while !frontier.is_empty() {
            if let Some(current)  = frontier.pop() {
                let neighbors = self.get_neighbors(&current.point);
                let mut it = neighbors.iter();
                while let Some(neighbor) = it.next() {
                    let new_cost = cost_so_far[current.point.x][current.point.y] + self.get_weight(neighbor.clone());
                    if new_cost < cost_so_far[neighbor.x][neighbor.y] {
                        cost_so_far[neighbor.x][neighbor.y] = new_cost;
                        frontier.push(WeightedPoint::from((neighbor.clone(), new_cost)));
                        came_from[neighbor.x][neighbor.y] = current.point;
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
            let cm = came_from[current.x][current.y];
            current = Point::from((cm.x, cm.y));
            self.path.push_back(current);
        }
        self.enter_path();
    }

    fn calc_breadth_first_path(&mut self) {
        type CameFromType = (Point, bool);

        let mut frontier = LinkedList::new();

        if self.start.is_none() || self.end.is_none() { return };

        frontier.push_back(self.start.clone().unwrap());

        let mut came_from = [[(Point::from((0,0)),false); App::SIDE]; App::SIDE];

        'outer: while !frontier.is_empty() {
            if let Some(cur) = frontier.pop_front() {
                let neighbors = self.get_neighbors(&cur);
                let mut it = neighbors.iter();
                while let Some(neighbor) = it.next() {
                    if !came_from[neighbor.x][neighbor.y].1 {
                        frontier.push_back(neighbor.clone());
                        came_from[neighbor.x][neighbor.y] = (Point::from((cur.x, cur.y)), true);
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
            let cm = came_from[current.x][current.y].0;
            current = Point::from((cm.x, cm.y));
            self.path.push_back(current);
        }
        self.enter_path();
    }

    fn enter_path(&mut self) {
        {
            let mut it = self.path.iter();
            while let Some(current) = it.next() {
                match self.grid[current.x][current.y] {
                    Block::Weighted {weight: w, on_path: _ } => {
                        self.grid[current.x][current.y] = Block::Weighted { weight: w, on_path: true };
                    },
                    _ => {}
                };
            };
        }
    }

    fn click(&mut self, args: MouseButton) {
        let offset = self.window_side / App::SIDE;
        let x = self.location.x / offset as usize;
        let y = self.location.y / offset as usize;
        match args {
            mouse::MouseButton::Left => {
                if self.end.is_none() ||  self.end.clone().unwrap() != Point::from((x,y)) {
                    self.start = Some(Point::from((x,y)));
                    self.calc_dijkstras_path();
                }
            },
            mouse::MouseButton::Right => {
                if self.start.is_none() || self.start.clone().unwrap() != Point::from((x,y)) {
                    self.end = Some(Point::from((x,y)));
                    self.calc_dijkstras_path();
                }

            },
            _ => {}
        }
    }
}

fn main() {
    let opengl = OpenGL::V3_2;

    const WINDOW_SIDE: u32 = 1200;

    let mut window: Window =
        WindowSettings::new("spinning-square", [WINDOW_SIDE, WINDOW_SIDE])
    .opengl(opengl)
    .exit_on_esc(true)
    .build()
    .unwrap();



    let mut app = App {
        gl: GlGraphics::new(opengl),
        window_side: WINDOW_SIDE as usize,
        path: LinkedList::new(),
        grid: read_map(),
        location: Point::from((0, 0)),
        start: None,
        end: None,
    };

    let mut events = window.events();
    events.set_max_fps(60);
    events.set_ups(60);
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
            let temp = Point::from((c[0] as usize, c[1] as usize));
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
            Ok(i) => grid_line.push(Block::Weighted{weight: i as usize, on_path: false}),
            Err(_) => {}
        };
    }

    return grid_line;
}
