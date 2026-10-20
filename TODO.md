# TODO
- Data
  - [x] links database
  - [] bug order rules
  - [x] better reaction parser (text .chem files)
  - [] json exporter/loader 

- Replicator 
  - [x] wildcards rules (continuous engine, src/sim/rules.rs)
  - [x] molecule loader (.mol files, chain notation)
  - [] make the actual sims


- [] file exporter/importer:
  - [] format : id x y form state 
  - [] follow by link : id1 id2

- [x] move particles with string/links
  - [x] introduce elasticity 
  - [] create cell structure procedurally

- [] data system
  - make link system at the reactor level
  - update molecule
  - json molecule adding
  

- [x] rules loaders / chemistry loaders
  - [x] json

- [x] treat chemical rules

- [x] window with sdl 
  - [x] integration with structs
- [x] dirty system for reactor (continuous engine)
  - [x] pass through link rules
- [] kcf 
  - [] kcf loader / exporter 
  - [] python rust ?

- [] create in/out particles from cells

- [x] continuous engine (src/sim): sphere collisions, springs per type pair
- [x] connected components, molecule extraction / insertion
- [x] SDL viewer with keys and stats (acsim), headless runner (acrun)
- [] replicator cross-talk in continuous space (see README)
