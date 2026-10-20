# Molecular engine

A 2D artificial chemistry after Hutton (2002): atoms have a **type** (`a`..`z`) and a
**state** (integer); rules rewrite the states of pairs of atoms and make or break bonds.

Two engines live in this crate:

* **grid engine** (`src/main.rs`, `reactor.rs`, ...) — the original: atoms on integer
  positions, JSON rules. Unchanged; run it with `cargo run --bin molecular_engine`.
* **continuous engine** (`src/sim/`, library) — atoms are soft spheres with continuous
  positions, bonds are springs, rules are plain text with wildcards. Binaries `acsim`
  (SDL viewer, the default) and `acrun` (headless).

```
cargo run --release                                   # acsim, Hutton replicator
cargo run --release --bin acsim -- --rules data/simple.chem --food 800 --size 50
cargo run --release --bin acrun -- --steps 100000 --every 10000 --seed 3
cargo test --lib
```

Use `--release`: the debug build is ~20x slower. `--help` lists the options
(`--rules --mol --load --food --size --insert --seed --no-feed --steps --every`).

## Chemistry files (`.chem`)

One statement per line, `#` starts a comment.

```
# reactions:   <type><state> (+|.) <type><state> -> <type><state> (+|.) <type><state> [@ p]
a0 + a1 -> a1 . a2        # '+' = not bonded, '.' = bonded
*1 + *2 -> *3 . *4        # '*' any type; in a product it keeps that reactant's type
X5 + X0 -> X7 . X6        # uppercase = type variable: both reactants have the same type
X3 + Y6 -> X2 . Y3        # different letters are independent (may still be equal)
X1 . Y1 -> Y1 . X1        # variables can move types around
b1 . *2 -> c1 . *2 @ 0.1  # concrete product type = transmutation; @ p = probability

spring * *  k=20 len=1.5             # bond springs by type pair; most specific line wins
spring a b  k=60 len=1.0 break=3.0   # break = bond breaks when stretched past this length

set temperature 0.3       # any field of reactor::Params (see below)
set food abcdef           # types used for free "food" atoms (state 0)
```

* A rule matches a pair in either order; products follow the atoms.
* When several rules match, the **first one in the file** wins. The parser warns when an
  earlier, more general rule makes a later one unreachable.
* `+`→`+` / `.`→`.` rules only change states; `+`→`.` creates a bond, `.`→`+` breaks it.
* The old notations `a_0(+)a_1` are accepted, and the old JSON rule files load directly
  (`--rules data/rules.json`; `contact: true` = bonded).
* In `acsim`, edit the file and press **R**: the rules are reloaded live. On a syntax
  error the line is reported and the previous chemistry keeps running.

`set` parameters: `radius` (0.5), `dt` (0.05), `damping` (1.0), `temperature` (0.3),
`repulsion` (40), `react_prob` (1.0, per contact per step), `react_range` (1.25: reaction
distance in units of 2r), `spring_k` (20) and `spring_len` (2r): default springs,
`max_speed` (8), `food`.

## Physics

Each step (`Reactor::step`):

1. spring forces along bonds, stiffness and rest length looked up by the bonded type pair;
2. neighbour search on a uniform grid: soft-sphere repulsion for overlapping non-bonded
   pairs; every non-bonded pair closer than `2r·react_range` is a collision candidate;
3. Langevin integration (damping + thermal noise at `temperature`), reflecting walls;
4. collision reactions (non-bonded rules) on the candidates, in random order;
5. bonded reactions, event driven: only atoms whose state or bonds changed in the
   previous step ("dirty" atoms) have their bonds checked against bonded rules.

An atom takes part in at most one reaction per step.

## Molecules (`.mol`)

Molecules are the connected components of the bond graph (`Reactor::components`,
`component_of`, `census`). They can be copied, extracted (removed) and inserted:

```
replicator: e8.a1.b1.c1.d1.f1      # chain, in rule notation ('.' or '-' = bond)

molecule ring                      # any graph, explicit relative coordinates
atom a 1 0.0 0.0
atom b 1 1.0 0.0
atom c 1 1.0 1.0
bond 0 1
bond 1 2
bond 2 0
end
```

Insertion moves free atoms that are in the way to random free spots, and fails if a
bonded atom or a wall is in the way. Molecules get a signature: linear chains as
`e8.a1.b1.f1`, anything else as `{a1*3 b2} 3b` (composition + bond count). The
"top molecules" census counts molecules by signature.

Reactor snapshots (`S` in acsim) are text: `size W H`, `atom <type> <state> <x> <y>`,
`bond i j`. Reload one with `--load file`.

## acsim keys

| key | | key | |
|---|---|---|---|
| Space | pause / run | N | single step |
| Up / Down | steps per frame ×2 / ÷2 | T / G | temperature ×1.25 / ÷1.25 |
| R | reload rules file | F | add 100 food atoms |
| A | auto-feed on/off (keeps the free food count) | K | colour by type / state / molecule |
| B | bonds on/off | L | state labels |
| click | select a molecule | C / X / Del | copy / cut / delete selection |
| V | paste clipboard at mouse | Q | rotate paste by 45° |
| I | insert library molecule at mouse | Tab | next library molecule |
| S | save snapshot + census to `out/` | P | screenshot to `out/` (.bmp) |
| H | key help | Esc | quit |

Copied / cut molecules are also written to `out/molecule_*.mol`, so they can be put in a
library file and re-inserted later (`--mol`).

## Notes on the Hutton replicator (`data/hutton.chem`)

The 8 rules of Hutton (2002) are written with variables and wildcards. In continuous
space the copy can attach on the far side of the template, where it is blocked by the
chain itself. With bond length = contact distance this stalls replication. Longer bonds
(`len=1.5`, reaction range 1.7) leave gaps an atom can pass through, and replication runs.
Two replicators that are close also exchange partners (rule 4 bonds the growing copy to
the other replicator's new atom). That gives tangles and shortened mutants such as
`e8.f1`, which out-replicate the original at higher temperature (`acrun` with
`temperature 0.6`).
