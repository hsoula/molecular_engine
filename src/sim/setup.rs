//! Command-line configuration shared by `acsim` and `acrun`.

use super::molecule::Molecule;
use super::reactor::Reactor;
use super::rules::Chemistry;
use super::vec2::Vec2;

#[derive(Clone, Debug)]
pub struct Config {
    pub rules: String,
    pub molecules: Option<String>,
    pub load: Option<String>,
    pub food: usize,
    pub size: f32,
    pub seed: u64,
    /// how many copies of the first molecule to insert at start
    pub insert: usize,
    pub steps: u64,
    pub every: u64,
    pub auto_feed: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rules: "data/hutton.chem".into(),
            molecules: Some("data/replicator.mol".into()),
            load: None,
            food: 500,
            size: 40.0,
            seed: 42,
            insert: 1,
            steps: 20000,
            every: 1000,
            auto_feed: true,
        }
    }
}

pub const USAGE: &str = "options:
  --rules <file>      chemistry (.chem text, or legacy .json)   [data/hutton.chem]
  --mol <file>        molecule library (.mol)                    [data/replicator.mol]
  --load <file>       start from a reactor snapshot instead of a random soup
  --food <n>          free atoms (state 0) at start               [500]
  --size <s>          box size (world units, atom radius 0.5)    [40]
  --insert <k>        copies of the first library molecule        [1]
  --seed <n>          random seed                                 [42]
  --no-feed           do not top up free food automatically
  --steps <n>         (acrun) number of steps                     [20000]
  --every <n>         (acrun) report interval                     [1000]";

impl Config {
    pub fn from_args() -> Result<Config, String> {
        let mut c = Config::default();
        let mut args = std::env::args().skip(1);
        while let Some(a) = args.next() {
            let mut val = || args.next().ok_or_else(|| format!("{a} needs a value"));
            match a.as_str() {
                "--rules" => c.rules = val()?,
                "--mol" => c.molecules = Some(val()?),
                "--load" => c.load = Some(val()?),
                "--food" => c.food = num(&val()?)?,
                "--size" => c.size = num(&val()?)?,
                "--insert" => c.insert = num(&val()?)?,
                "--seed" => c.seed = num(&val()?)?,
                "--steps" => c.steps = num(&val()?)?,
                "--every" => c.every = num(&val()?)?,
                "--no-feed" => c.auto_feed = false,
                "-h" | "--help" => return Err(USAGE.into()),
                _ => return Err(format!("unknown option {a}\n{USAGE}")),
            }
        }
        Ok(c)
    }

    /// Build the reactor and load the molecule library. Returns warnings too.
    pub fn build(&self) -> Result<(Reactor, Vec<Molecule>, Vec<String>), String> {
        let chem = Chemistry::load(&self.rules)?;
        let mut warnings = Vec::new();
        let mut reactor = match &self.load {
            Some(path) => {
                let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
                Reactor::from_snapshot(&text, Chemistry::new(), self.seed)?
            }
            None => Reactor::new(self.size, self.size, Chemistry::new(), self.seed),
        };
        warnings.extend(reactor.set_chemistry(chem));
        let spacing = reactor.params.contact_dist();
        let library = match &self.molecules {
            Some(path) => match std::fs::read_to_string(path) {
                Ok(text) => Molecule::parse_file(&text, spacing).map_err(|e| format!("{path}: {e}"))?,
                Err(e) => {
                    warnings.push(format!("{path}: {e}"));
                    Vec::new()
                }
            },
            None => Vec::new(),
        };
        if self.load.is_none() {
            if let Some(m) = library.first() {
                let (w, h) = (reactor.params.width, reactor.params.height);
                for k in 0..self.insert {
                    // spread the seeds on a horizontal line
                    let at = Vec2::new(w * (k as f32 + 1.0) / (self.insert as f32 + 1.0), h * 0.5);
                    if let Err(e) = reactor.insert_molecule(m, at, 0.0) {
                        warnings.push(format!("insert {}: {e}", m.name));
                    }
                }
            }
            let placed = reactor.add_food(self.food);
            if placed < self.food {
                warnings.push(format!("box full: placed {placed} of {} food atoms", self.food));
            }
        }
        Ok((reactor, library, warnings))
    }
}

fn num<T: std::str::FromStr>(s: &str) -> Result<T, String> {
    s.parse().map_err(|_| format!("bad number `{s}`"))
}
