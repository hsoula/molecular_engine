//! SDL3 viewer for the continuous reactor.
//!
//!   cargo run --release --bin acsim -- --rules data/hutton.chem --mol data/replicator.mol
//!
//! Press H in the window for the key list (also in `KEYS` below).

use std::ffi::CString;
use std::time::{Duration, Instant};

use molecular_engine::sim::molecule::Molecule;
use molecular_engine::sim::rules::{type_index, Chemistry};
use molecular_engine::sim::setup::{Config, USAGE};
use molecular_engine::sim::{Reactor, Vec2};

use sdl3::event::Event;
use sdl3::keyboard::{Keycode, Mod};
use sdl3::mouse::MouseButton;
use sdl3::pixels::{Color, FColor};
use sdl3::render::{FPoint, FRect, Vertex, WindowCanvas};

const VIEW: f32 = 800.0;
const PANEL: f32 = 380.0;
const LINE: f32 = 11.0;

const KEYS: &[&str] = &[
    "Space  pause/run      N  single step",
    "Up/Dn  steps/frame    T/G temperature +/-",
    "R  reload rules file  F  add 100 food",
    "A  auto-feed on/off   K  colour mode",
    "B  bonds on/off       L  state labels",
    "click  select molecule",
    "C copy  X cut  Del delete  (selection)",
    "V  paste clipboard at mouse (Q rotates)",
    "I  insert library molecule  Tab next",
    "S  save snapshot + molecule census",
    "P  screenshot (.bmp)  H  help  Esc quit",
];

#[derive(Clone, Copy, PartialEq)]
enum ColorMode {
    Type,
    State,
    Molecule,
}

struct App {
    cfg: Config,
    reactor: Reactor,
    library: Vec<Molecule>,
    lib_idx: usize,
    clipboard: Option<Molecule>,
    paste_angle: f32,
    /// an atom of the selected molecule (atom indices change when atoms are removed)
    selected: Option<usize>,
    paused: bool,
    single_step: bool,
    steps_per_frame: u32,
    auto_feed: bool,
    food_target: usize,
    show_bonds: bool,
    show_labels: bool,
    show_help: bool,
    color_mode: ColorMode,
    mouse: Vec2,
    messages: Vec<(String, Instant)>,
    census: Vec<(String, usize)>,
    n_molecules: usize,
    largest: usize,
    // rates
    fps: f32,
    rate_reactions: f32,
    rate_steps: f32,
    rate_t0: Instant,
    rate_step0: u64,
    rate_react0: u64,
    frames: u32,
    out_counter: u32,
}

impl App {
    fn scale(&self) -> f32 {
        VIEW / self.reactor.params.width.max(self.reactor.params.height)
    }
    fn to_screen(&self, p: Vec2) -> FPoint {
        let s = self.scale();
        FPoint::new(p.x * s, p.y * s)
    }
    fn to_world(&self, x: f32, y: f32) -> Vec2 {
        let s = self.scale();
        Vec2::new(x / s, y / s)
    }

    fn msg(&mut self, s: impl Into<String>) {
        let s = s.into();
        println!("{s}");
        self.messages.push((s, Instant::now()));
        if self.messages.len() > 6 {
            self.messages.remove(0);
        }
    }

    fn selection(&self) -> Vec<usize> {
        match self.selected {
            Some(i) if i < self.reactor.atoms.len() => self.reactor.component_of(i),
            _ => Vec::new(),
        }
    }

    fn out_path(&mut self, stem: &str, ext: &str) -> String {
        let _ = std::fs::create_dir_all("out");
        self.out_counter += 1;
        format!("out/{stem}_{:06}_{}.{ext}", self.reactor.stats.step, self.out_counter)
    }

    fn reload_rules(&mut self) {
        match Chemistry::load(&self.cfg.rules) {
            Ok(chem) => {
                let n = chem.rules.len();
                let warnings = self.reactor.set_chemistry(chem);
                self.msg(format!("reloaded {} ({n} rules)", self.cfg.rules));
                for w in warnings {
                    self.msg(format!("warning: {w}"));
                }
            }
            Err(e) => {
                // keep running with the old chemistry
                for line in e.lines() {
                    self.msg(format!("ERROR {line}"));
                }
            }
        }
    }

    fn insert_at_mouse(&mut self, m: &Molecule) {
        let angle = self.paste_angle;
        match self.reactor.insert_molecule(m, self.mouse, angle) {
            Ok(ids) => {
                self.selected = ids.first().copied();
                self.msg(format!("inserted {} ({} atoms)", m.name, ids.len()));
            }
            Err(e) => self.msg(format!("cannot insert here: {e}")),
        }
    }

    fn save_molecule(&mut self, m: &Molecule) {
        let path = self.out_path("molecule", "mol");
        match std::fs::write(&path, m.to_text()) {
            Ok(()) => self.msg(format!("saved {path}")),
            Err(e) => self.msg(format!("cannot write {path}: {e}")),
        }
    }

    fn handle_key(&mut self, key: Keycode, keymod: Mod) -> bool {
        let shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
        match key {
            Keycode::Escape => return false,
            Keycode::Space => self.paused = !self.paused,
            Keycode::N => {
                self.paused = true;
                self.single_step = true;
            }
            Keycode::Up | Keycode::Equals | Keycode::KpPlus => {
                self.steps_per_frame = (self.steps_per_frame * 2).min(256)
            }
            Keycode::Down | Keycode::Minus | Keycode::KpMinus => {
                self.steps_per_frame = (self.steps_per_frame / 2).max(1)
            }
            Keycode::T => {
                self.reactor.params.temperature *= 1.25;
                let t = self.reactor.params.temperature;
                self.msg(format!("temperature {t:.3}"));
            }
            Keycode::G => {
                self.reactor.params.temperature /= 1.25;
                let t = self.reactor.params.temperature;
                self.msg(format!("temperature {t:.3}"));
            }
            Keycode::R => self.reload_rules(),
            Keycode::F => {
                let n = self.reactor.add_food(100);
                self.food_target += n;
                self.msg(format!("added {n} food atoms"));
            }
            Keycode::A => {
                self.auto_feed = !self.auto_feed;
                self.food_target = self.food_target.max(self.reactor.free_food_count());
                let s = if self.auto_feed { "on" } else { "off" };
                self.msg(format!("auto-feed {s} (target {} free atoms)", self.food_target));
            }
            Keycode::K => {
                self.color_mode = match self.color_mode {
                    ColorMode::Type => ColorMode::State,
                    ColorMode::State => ColorMode::Molecule,
                    ColorMode::Molecule => ColorMode::Type,
                }
            }
            Keycode::B => self.show_bonds = !self.show_bonds,
            Keycode::L => self.show_labels = !self.show_labels,
            Keycode::H => self.show_help = !self.show_help,
            Keycode::C | Keycode::X => {
                let sel = self.selection();
                if sel.is_empty() {
                    self.msg("nothing selected (click a molecule)");
                } else {
                    let m = if key == Keycode::X {
                        self.selected = None;
                        self.reactor.extract_molecule(&sel)
                    } else {
                        self.reactor.copy_molecule(&sel)
                    };
                    let verb = if key == Keycode::X { "cut" } else { "copied" };
                    self.msg(format!("{verb} {} ({} atoms)", m.name, m.atoms.len()));
                    self.save_molecule(&m);
                    self.clipboard = Some(m);
                }
            }
            Keycode::Delete | Keycode::Backspace => {
                let sel = self.selection();
                if !sel.is_empty() {
                    self.reactor.remove_atoms(&sel);
                    self.selected = None;
                    self.msg(format!("deleted {} atoms", sel.len()));
                }
            }
            Keycode::V => match self.clipboard.clone() {
                Some(m) => self.insert_at_mouse(&m),
                None => self.msg("clipboard empty (C or X a molecule first)"),
            },
            Keycode::Q => {
                self.paste_angle += std::f32::consts::FRAC_PI_4 * if shift { -1.0 } else { 1.0 };
                self.msg(format!("paste angle {:.0} deg", self.paste_angle.to_degrees()));
            }
            Keycode::I => match self.library.get(self.lib_idx).cloned() {
                Some(m) => self.insert_at_mouse(&m),
                None => self.msg("molecule library empty (--mol file.mol)"),
            },
            Keycode::Tab => {
                if !self.library.is_empty() {
                    self.lib_idx = (self.lib_idx + 1) % self.library.len();
                    let name = self.library[self.lib_idx].name.clone();
                    self.msg(format!("library: {name}"));
                }
            }
            Keycode::S => {
                let path = self.out_path("reactor", "txt");
                let mut text = self.reactor.to_snapshot();
                text += "# census (molecules with >= 2 atoms):\n";
                for (sig, n) in self.reactor.census(2) {
                    text += &format!("# {n:5} {sig}\n");
                }
                match std::fs::write(&path, text) {
                    Ok(()) => self.msg(format!("saved {path} (reload with --load)")),
                    Err(e) => self.msg(format!("cannot write {path}: {e}")),
                }
            }
            _ => {}
        }
        true
    }

    fn update(&mut self) {
        let steps = if self.single_step {
            self.single_step = false;
            1
        } else if self.paused {
            0
        } else {
            self.steps_per_frame
        };
        for _ in 0..steps {
            self.reactor.step();
        }
        if self.auto_feed && steps > 0 {
            let missing = self.food_target.saturating_sub(self.reactor.free_food_count());
            if missing > 0 {
                self.reactor.add_food(missing.min(50));
            }
        }
        self.frames += 1;
        if self.frames % 15 == 1 {
            let comps = self.reactor.components(2);
            self.n_molecules = comps.len();
            self.largest = comps.iter().map(|c| c.len()).max().unwrap_or(0);
            self.census = self.reactor.census(2);
        }
        let dt = self.rate_t0.elapsed().as_secs_f32();
        if dt >= 1.0 {
            let st = &self.reactor.stats;
            self.rate_steps = (st.step - self.rate_step0) as f32 / dt;
            self.rate_reactions = (st.reactions - self.rate_react0) as f32 / dt;
            self.fps = self.frames as f32 / dt;
            self.frames = 0;
            self.rate_t0 = Instant::now();
            self.rate_step0 = st.step;
            self.rate_react0 = st.reactions;
        }
        self.messages.retain(|(_, t)| t.elapsed() < Duration::from_secs(8));
    }

    // ------------------------------------------------------------------ drawing

    fn atom_color(&self, i: usize, labels: &[u32]) -> FColor {
        let a = &self.reactor.atoms[i];
        let (h, v) = match self.color_mode {
            ColorMode::Type => (type_index(a.form) as f32 / 6.0 * 0.83, if a.state == 0 { 0.45 } else { 1.0 }),
            ColorMode::State => (a.state.rem_euclid(10) as f32 * 0.1, 1.0),
            ColorMode::Molecule => {
                if a.bonds.is_empty() {
                    (0.0, 0.0)
                } else {
                    (((labels[i] as u64).wrapping_mul(2654435761) % 1000) as f32 / 1000.0, 1.0)
                }
            }
        };
        if self.color_mode == ColorMode::Molecule && v == 0.0 {
            return FColor { r: 0.3, g: 0.3, b: 0.3, a: 1.0 };
        }
        hsv(h, 0.75, v)
    }

    fn draw(&self, canvas: &mut WindowCanvas) {
        canvas.set_draw_color(Color::RGB(12, 12, 18));
        canvas.clear();
        let r = &self.reactor;
        let s = self.scale();
        canvas.set_draw_color(Color::RGB(40, 40, 55));
        let _ = canvas.draw_rect(FRect::new(0.0, 0.0, r.params.width * s, r.params.height * s));

        // bonds
        if self.show_bonds {
            canvas.set_draw_color(Color::RGB(200, 200, 200));
            for (i, a) in r.atoms.iter().enumerate() {
                for &j in &a.bonds {
                    if j as usize > i {
                        let _ = canvas.draw_line(self.to_screen(a.pos), self.to_screen(r.atoms[j as usize].pos));
                    }
                }
            }
        }

        // atoms as filled circles in one geometry batch
        let labels = if self.color_mode == ColorMode::Molecule { r.component_labels() } else { Vec::new() };
        let sel = self.selection();
        let rad = r.params.radius * s;
        let seg = if rad > 6.0 { 14 } else { 8 };
        let mut verts: Vec<Vertex> = Vec::with_capacity((r.atoms.len() + sel.len()) * (seg + 1));
        let mut idx: Vec<i32> = Vec::with_capacity((r.atoms.len() + sel.len()) * seg * 3);
        let white = FColor { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        for &i in &sel {
            push_disc(&mut verts, &mut idx, self.to_screen(r.atoms[i].pos), rad * 1.45, white, seg);
        }
        for i in 0..r.atoms.len() {
            let c = self.atom_color(i, &labels);
            // bonded atoms drawn slightly smaller so bonds stay visible
            let k = if r.atoms[i].bonds.is_empty() { 0.9 } else { 0.75 };
            push_disc(&mut verts, &mut idx, self.to_screen(r.atoms[i].pos), rad * k, c, seg);
        }
        let _ = canvas.render_geometry(&verts, None, &idx[..]);

        if self.show_labels && rad >= 4.0 {
            canvas.set_draw_color(Color::RGB(0, 0, 0));
            for a in &r.atoms {
                let p = self.to_screen(a.pos);
                let t = if rad >= 9.0 { format!("{}{}", a.form, a.state) } else { a.state.to_string() };
                let w = 8.0 * t.len() as f32;
                text(canvas, p.x - w / 2.0, p.y - 4.0, &t);
            }
        }

        // paste preview
        if let Some(m) = &self.clipboard {
            canvas.set_draw_color(Color::RGB(90, 90, 120));
            for a in &m.atoms {
                let p = self.to_screen(self.mouse + a.pos.rotated(self.paste_angle));
                let _ = canvas.draw_rect(FRect::new(p.x - rad, p.y - rad, 2.0 * rad, 2.0 * rad));
            }
        }

        self.draw_panel(canvas);
    }

    fn draw_panel(&self, canvas: &mut WindowCanvas) {
        let r = &self.reactor;
        let x0 = VIEW + 10.0;
        canvas.set_draw_color(Color::RGB(24, 24, 34));
        let _ = canvas.fill_rect(FRect::new(VIEW, 0.0, PANEL, VIEW));
        let mut cur = Cursor { x: x0, y: 10.0 };
        let head = Color::RGB(255, 210, 120);
        let norm = Color::RGB(220, 220, 220);
        let dim = Color::RGB(150, 150, 160);
        let state = if self.paused { "PAUSED" } else { "running" };
        cur.line(canvas, head, "MOLECULAR ENGINE  (continuous)");
        cur.line(canvas, norm, &format!("step {:<10} t = {:.1}", r.stats.step, r.stats.step as f32 * r.params.dt));
        cur.line(canvas, norm, &format!("{state}  {} steps/frame  {:.0} fps", self.steps_per_frame, self.fps));
        cur.line(canvas, norm, &format!("{:.0} steps/s  {:.0} reactions/s", self.rate_steps, self.rate_reactions));
        cur.y += 4.0;
        cur.line(canvas, head, "REACTOR");
        cur.line(canvas, norm, &format!("atoms {}  bonds {}  free food {}", r.atoms.len(), r.bond_count(), r.free_food_count()));
        cur.line(canvas, norm, &format!("molecules {}  largest {}", self.n_molecules, self.largest));
        cur.line(canvas, norm, &format!("reactions {}  made {} broken {}", r.stats.reactions, r.stats.bonds_made, r.stats.bonds_broken));
        cur.line(canvas, norm, &format!("T {:.3} (kinetic {:.3})  contacts {}", r.params.temperature, r.kinetic_temperature(), r.stats.collisions));
        let feed = if self.auto_feed { format!("on, target {}", self.food_target) } else { "off".into() };
        cur.line(canvas, norm, &format!("auto-feed {feed}"));
        cur.y += 4.0;
        cur.line(canvas, head, "TOP MOLECULES");
        for (sig, n) in self.census.iter().take(8) {
            cur.line(canvas, norm, &clip(&format!("{n:4} {sig}"), 45));
        }
        cur.y += 4.0;
        cur.line(canvas, head, &clip(&format!("RULES  {}", self.cfg.rules), 45));
        for (rule, n) in r.chem.rules.iter().zip(&r.stats.per_rule).take(16) {
            cur.line(canvas, norm, &clip(&format!("{n:7} {rule}"), 45));
        }
        if r.chem.rules.len() > 16 {
            cur.line(canvas, dim, &format!("   ... {} more", r.chem.rules.len() - 16));
        }
        cur.y += 4.0;
        let sel = self.selection();
        if !sel.is_empty() {
            cur.line(canvas, head, &format!("SELECTION  {} atoms", sel.len()));
            let sig = r.copy_molecule(&sel).signature();
            for chunk in sig.as_bytes().chunks(44).take(3) {
                cur.line(canvas, norm, &String::from_utf8_lossy(chunk));
            }
        }
        let lib = self.library.get(self.lib_idx).map_or("-".to_string(), |m| m.name.clone());
        cur.line(canvas, dim, &clip(&format!("library [I]: {lib}"), 45));
        if let Some(m) = &self.clipboard {
            cur.line(canvas, dim, &clip(&format!("clipboard [V]: {}", m.name), 45));
        }
        cur.y += 4.0;
        for (m, _) in &self.messages {
            let c = if m.starts_with("ERROR") { Color::RGB(255, 110, 110) } else { Color::RGB(140, 220, 140) };
            cur.line(canvas, c, &clip(m, 45));
        }
        if self.show_help {
            let mut yk = VIEW - 10.0 - LINE * KEYS.len() as f32;
            canvas.set_draw_color(dim);
            for k in KEYS {
                text(canvas, x0, yk, k);
                yk += LINE;
            }
        } else {
            canvas.set_draw_color(dim);
            text(canvas, x0, VIEW - 10.0 - LINE, "H: keys");
        }
    }
}

struct Cursor {
    x: f32,
    y: f32,
}

impl Cursor {
    fn line(&mut self, canvas: &mut WindowCanvas, c: Color, s: &str) {
        canvas.set_draw_color(c);
        text(canvas, self.x, self.y, s);
        self.y += LINE;
    }
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { s.chars().take(n - 1).collect::<String>() + "~" }
}

/// SDL3 built-in 8x8 debug font, drawn with the current draw colour.
fn text(canvas: &mut WindowCanvas, x: f32, y: f32, s: &str) {
    if let Ok(c) = CString::new(s) {
        unsafe {
            sdl3::sys::render::SDL_RenderDebugText(canvas.raw(), x, y, c.as_ptr());
        }
    }
}

fn push_disc(verts: &mut Vec<Vertex>, idx: &mut Vec<i32>, c: FPoint, r: f32, color: FColor, seg: usize) {
    let base = verts.len() as i32;
    let zero = FPoint::new(0.0, 0.0);
    verts.push(Vertex { position: c, color, tex_coord: zero });
    for k in 0..seg {
        let a = k as f32 / seg as f32 * std::f32::consts::TAU;
        verts.push(Vertex { position: FPoint::new(c.x + r * a.cos(), c.y + r * a.sin()), color, tex_coord: zero });
    }
    for k in 0..seg as i32 {
        idx.extend_from_slice(&[base, base + 1 + k, base + 1 + (k + 1) % seg as i32]);
    }
}

fn hsv(h: f32, s: f32, v: f32) -> FColor {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor();
    let f = h - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i as i32 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    FColor { r, g, b, a: 1.0 }
}

fn save_screenshot(canvas: &WindowCanvas, path: &str) -> Result<(), String> {
    let surf = canvas.read_pixels(None).map_err(|e| e.to_string())?;
    let c = CString::new(path).map_err(|e| e.to_string())?;
    // (sdl3 0.14's Surface::save_bmp reports success as an error, so call SDL directly)
    let ok = unsafe { sdl3::sys::surface::SDL_SaveBMP(surf.raw(), c.as_ptr()) };
    if ok { Ok(()) } else { Err(sdl3::get_error().to_string()) }
}

fn main() {
    let cfg = match Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}\n(acsim) keys: press H in the window");
            std::process::exit(if e == USAGE { 0 } else { 2 });
        }
    };
    let (reactor, library, warnings) = match cfg.build() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    let food_target = reactor.free_food_count();
    let mut app = App {
        auto_feed: cfg.auto_feed,
        cfg,
        reactor,
        library,
        lib_idx: 0,
        clipboard: None,
        paste_angle: 0.0,
        selected: None,
        paused: false,
        single_step: false,
        steps_per_frame: 16,
        food_target,
        show_bonds: true,
        show_labels: false,
        show_help: true,
        color_mode: ColorMode::Type,
        mouse: Vec2::ZERO,
        messages: Vec::new(),
        census: Vec::new(),
        n_molecules: 0,
        largest: 0,
        fps: 0.0,
        rate_reactions: 0.0,
        rate_steps: 0.0,
        rate_t0: Instant::now(),
        rate_step0: 0,
        rate_react0: 0,
        frames: 0,
        out_counter: 0,
    };
    for w in warnings {
        app.msg(format!("warning: {w}"));
    }
    let n_rules = app.reactor.chem.rules.len();
    let rules_file = app.cfg.rules.clone();
    app.msg(format!("{rules_file}: {n_rules} rules"));

    // optional: --screenshot-after <frames> <file.bmp>, for headless checks
    let shot: Option<(u32, String)> = std::env::var("ACSIM_SCREENSHOT")
        .ok()
        .and_then(|v| v.split_once(':').map(|(n, p)| (n.parse().unwrap_or(60), p.to_string())));

    let sdl = sdl3::init().expect("SDL init");
    let video = sdl.video().expect("SDL video");
    let window = video
        .window("molecular engine - continuous", (VIEW + PANEL) as u32, VIEW as u32)
        .position_centered()
        .build()
        .expect("window");
    let mut canvas = window.into_canvas();
    let mut events = sdl.event_pump().expect("event pump");
    let frame_time = Duration::from_micros(1_000_000 / 60);
    let mut frame_no: u32 = 0;

    'running: loop {
        let t_frame = Instant::now();
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode: Some(k), keymod, .. } => {
                    if k == Keycode::P {
                        let path = app.out_path("screen", "bmp");
                        match save_screenshot(&canvas, &path) {
                            Ok(()) => app.msg(format!("saved {path}")),
                            Err(e) => app.msg(format!("ERROR screenshot: {e}")),
                        }
                    } else if !app.handle_key(k, keymod) {
                        break 'running;
                    }
                }
                Event::MouseMotion { x, y, .. } => app.mouse = app.to_world(x, y),
                Event::MouseButtonDown { mouse_btn: MouseButton::Left, x, y, .. } => {
                    if x < VIEW {
                        let p = app.to_world(x, y);
                        app.selected = app.reactor.atom_at(p);
                    }
                }
                _ => {}
            }
        }
        app.update();
        app.draw(&mut canvas);
        frame_no += 1;
        if let Some((n, path)) = &shot {
            if frame_no == *n {
                if let Err(e) = save_screenshot(&canvas, path) {
                    eprintln!("screenshot: {e}");
                }
                break 'running;
            }
        }
        canvas.present();
        if let Some(rest) = frame_time.checked_sub(t_frame.elapsed()) {
            std::thread::sleep(rest);
        }
    }
}
