extern crate sdl3;
use sdl3::pixels::Color;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use std::time::Duration;

mod chemistry;
mod compound;
mod rule;
mod rulec;
mod snip_tests;
mod reactor;
mod atom;

use crate::compound::{new_compound_from_text, Compound};
use crate::rule::Rule;
use crate::rulec::RuleC;
use crate::rule::new_rule_from_text;
use crate::atom::Atom;
use crate::reactor::Reactor;

use std::io::Write;
use rand;
use sdl3::rect::{Point, Rect};
use serde_json::json;

fn display_window_loop(reactor : &mut Reactor, tmax:i32) {

    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl3 demo", 800, 800)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    // fills the canvas with the color we set in `set_draw_color`.
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut t = 0;

    'running: loop {

        t += 1 ;
        //canvas.clear();
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        // fills the canvas with the color we set in `set_draw_color`.
        canvas.clear();
        //
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        if t > tmax {
            break 'running;
        }
        // The rest of the game loop goes here...
        let a = reactor.move_all_atoms();
        reactor.check_linked_rule();
        println!("{} {}", t, a);
        for i in reactor.atoms.iter() {
            let x = i.x;
            let y = i.y;
            let s = i.state;
            //println!("x: {}, y: {}", x, y);
            let	step = (800 / reactor.get_w()) as u32;
            let ax = (step as i32) * x;
            let ay = (step as i32) * y;
            canvas.set_draw_color(Color::RGB(255, 210, (s * 255 /4) as u8));
            // A draw a rectangle which almost fills our window with it !
            canvas.fill_rect(Rect::new(ax, ay, step, step));

            for ix in 0..i.link.len() {
                let x = i.x;
                let y = i.y;
                let fx = reactor.atoms[i.link[ix] as usize].x;
                let fy = reactor.atoms[i.link[ix] as usize].y;
                let x1 = x * (step as i32)  + (step as i32) /2;
                let y1 = y * (step as i32)  + (step as i32) /2;
                let	x2 = fx * (step as i32)  + (step as i32) /2;
                let	y2 = fy * (step as i32)  + (step as i32) /2;

                canvas.set_draw_color(Color::RGB(255, 255, 255));
                canvas.draw_line(Point::new(x1,y1), Point::new(x2, y2)).unwrap()
            }
        }


        canvas.present();
        //canvas.clear();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 120));
    }
}
fn main() {

    let n = 100;
    let w = 100;
    let h = 100;
    let mut reactor = Reactor::new(w, h, n);
    // let c = Compound{form: 'a', state: 0};
    // let d = Compound{form: 'a', state: 1};
    // let r = RuleC{contact:false, a1:c, a2:d};
    // println!("{}", r.get_key());
    // let c1 = Compound{form: 'a', state: 1};
    // let d1 = Compound{form: 'a', state: 2};
    // let r1 = RuleC{contact:true, a1:c1, a2:d1};
    // let r0 = Rule{substrate:r, product:r1, id:0};
    let s = "a_0(+)a_1->a_1(.)a_2";
    let r0 = new_rule_from_text(s.to_string(),0);
    reactor.add_rule(r0);
    let s = "a_2(+)a_0->a_3(.)a_1";
    let r0 = new_rule_from_text(s.to_string(),1);
    reactor.add_rule(r0);


    let s = "a_3(.)a_1->a_2(.)a_4";
    let r0 = new_rule_from_text(s.to_string(),1);
    reactor.add_rule(r0);

    //
    // let c = Compound{form: 'a', state: 1};
    // let d = Compound{form: 'a', state: 2};
    // let r = RuleC{contact:true, a1:c, a2:d};
    // println!("{}", r.get_key());
    // let c1 = Compound{form: 'a', state: 4};
    // let d1 = Compound{form: 'a', state: 1};
    // let r1 = RuleC{contact:false, a1:c1, a2:d1};
    // let r0 = Rule{substrate:r, product:r1, id:1};
    //reactor.add_rule(r0);

    reactor.fill_random();
    for i in 0..n {
        reactor.atoms[i as usize].form = 'a';
        reactor.atoms[i as usize].state = 0;
    }
    for i in 0..4 {
        reactor.atoms[i as usize].form = 'a';
        reactor.atoms[i as usize].state = 1;
    }

    // for i in 0..100 {
    //     let x = reactor.move_all_atoms();
    //     println!("{} {}", i, x);
    // }

    //let s = json!(r0);
    //println!("{}", s);
    display_window_loop(&mut reactor, 10000000);

}

