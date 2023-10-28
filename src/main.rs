
use std::io;
use std::io::*;
use std::process;
use std::thread;
use std::time;
use rand::Rng;

use ctrlc;
use bitvec::prelude::*;
pub use crossterm::*;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Options {
    /// generations per second
    #[arg(short, default_value_t = 10.0)]
    t_hz: f64,

    /// rulestring in B/S notation
    #[arg(short, long, default_value_t = ("B3/S23").to_string())]
    rulestring: String,
}

// TODO: clean up better somehow
fn cleanup() {
    let mut w = io::stdout();
    let _ = w.flush();
    let _ = execute!(w, style::ResetColor,
                     cursor::Show,
                     terminal::LeaveAlternateScreen);
    let _ = w.flush();
    process::exit(0);
}

// TODO: better error messages

fn parse_tab(s: &str, prefix: Option<char>) -> BitArray {
    let mut chars = s.chars();
    match prefix {
        Some(p) => {
            let c = chars.next().expect("Invalid rulestring");
            if c != p { panic!("Invalid rulestring"); }
        }
        _ => {},
    }

    let mut result: BitArr!(for 10) = BitArray::ZERO;
    for n in chars.map(|c| c.to_digit(10).unwrap()){
        result.set(n as usize, true)
    }
    result
}

fn main() -> std::io::Result<()> {
    let options = Options::parse();

    let (b, s) = options.rulestring.split_once('/').expect("Invalid rulestring");
    let birth_tab    = parse_tab(b, Some('B'));
    let survival_tab = parse_tab(s, Some('S'));
    
    ctrlc::set_handler(cleanup).expect("Couldn't set ctrl-c handler");

    let mut w = io::stdout();
    
    queue!(w, terminal::EnterAlternateScreen,
           style::ResetColor,
           terminal::Clear(terminal::ClearType::All),
           cursor::Hide)?;

    let (term_width, term_height) = terminal::size()?;

    let width:  usize = (term_width  - 1) as usize;
    let height: usize = (term_height - 1) as usize;

    let vec_size = (width*height) as usize;
    
    let mut cells1 = BitVec::<usize, Msb0>::new();
    let mut cells2 = BitVec::<usize, Msb0>::new();
    cells1.resize(vec_size, false);
    cells2.resize(vec_size, false);

    let mut cells = &mut cells1;
    let mut new_cells = &mut cells2;
    
    let mut rng = rand::thread_rng();
    
    for mut cell in cells.iter_mut() {
        *cell = rng.gen();
    }
    
    let delay = time::Duration::from_secs_f64(1.0 / options.t_hz);
    
    let mut start = time::Instant::now();
    loop {
        // draw
        {
            queue!(w, cursor::MoveTo(0,0))?;
            for y in 0..height {
                for x in 0..width {
                    if cells[x + y*width] {
                        queue!(w, style::SetAttribute(style::Attribute::Reverse))?;
                        w.write(b" ")?;
                        queue!(w, style::SetAttribute(style::Attribute::Reset))?;
                    } else {
                        w.write(b" ")?;
                    }
                }
                queue!(w, cursor::MoveToNextLine(1))?;
            }
            
            w.flush()?;
        }

        let elapsed = start.elapsed();
        if elapsed < delay {
            thread::sleep(delay - elapsed);
        }
        start = time::Instant::now();

        // update
        for y in 0..height {
            for x in 0..width {
                let mut neighbor_count: usize = 0;
                for dy in -1..2 {
                    for dx in -1..2 {
                        let nx = (x + width.wrapping_add(dx as usize)) % width;
                        let ny = (y + height.wrapping_add(dy as usize)) % height;
                        neighbor_count += cells[nx + ny*width] as usize;
                    }
                }
                let index = (x + y*width) as usize;
                if cells[index] {
                    neighbor_count -= 1;
                    new_cells.set(index, survival_tab[neighbor_count]);
                } else {
                    new_cells.set(index, birth_tab[neighbor_count]);
                }
            }
        }
        (cells, new_cells) = (new_cells, cells);
    }
}
