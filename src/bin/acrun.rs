//! Headless run of the continuous reactor: prints stats and the molecule census.
//!
//!   cargo run --release --bin acrun -- --steps 50000 --every 5000

use molecular_engine::sim::setup::Config;
use std::time::Instant;

fn main() {
    let cfg = match Config::from_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let (mut r, _library, warnings) = match cfg.build() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    for w in warnings {
        eprintln!("warning: {w}");
    }
    let food_target = r.free_food_count();
    println!("{} rules, {} atoms, box {}x{}", r.chem.rules.len(), r.atoms.len(), r.params.width, r.params.height);
    let t0 = Instant::now();
    for s in 1..=cfg.steps {
        r.step();
        if cfg.auto_feed && s % 10 == 0 {
            let missing = food_target.saturating_sub(r.free_food_count());
            r.add_food(missing);
        }
        if s % cfg.every == 0 || s == cfg.steps {
            let comps = r.components(2);
            let largest = comps.iter().map(|c| c.len()).max().unwrap_or(0);
            println!(
                "step {:>7}  atoms {:>5}  bonds {:>5}  molecules {:>4}  largest {:>3}  reactions {:>8}  kT {:.3}  ({:.0} steps/s)",
                s,
                r.atoms.len(),
                r.bond_count(),
                comps.len(),
                largest,
                r.stats.reactions,
                r.kinetic_temperature(),
                s as f64 / t0.elapsed().as_secs_f64()
            );
            for (sig, n) in r.census(2).iter().take(8) {
                println!("    {n:>4} x {sig}");
            }
        }
    }
    println!("rule firings:");
    for (rule, n) in r.chem.rules.iter().zip(&r.stats.per_rule) {
        println!("  {n:>8}  {rule}");
    }
}
