# BlackHoleSimu

Simulateur de **trou noir de Schwarzschild** (non rotatif) rendu en temps réel,
visant un réalisme comparable à Gargantua (*Interstellar*) ou aux images de
l'Event Horizon Telescope. Le cœur est un *fragment shader* plein écran qui fait
du ray casting par pixel à travers l'espace-temps courbé.

Écrit en **Rust** avec **wgpu** (WebGPU natif + fallback WebGL2) et **WGSL**.
Une seule base de code, deux cibles : **natif** (`cargo run`, pour itérer vite)
et **navigateur** via **WebAssembly** (`wasm-pack`).

> **État : Phase 1 terminée.** Le squelette compile et tourne sur les deux
> cibles : fenêtre wgpu + triangle plein écran affichant une couleur unie
> animée. La physique arrive aux phases suivantes (voir la feuille de route).

## Effets visés (feuille de route physique)

- Lentille gravitationnelle correcte (arrière du disque visible au-dessus et en dessous)
- Anneau de photons à 1.5 r_s
- Beaming relativiste (un côté du disque plus brillant)
- Redshift gravitationnel
- Émission de corps noir (température du disque → couleur via loi de Planck)
- Fond étoilé (HDRI Voie lactée) distordu par la lentille

## Prérequis

- **Rust** (toolchain stable, gérée par `rustup`)
- Cible WASM : `rustup target add wasm32-unknown-unknown`
- **wasm-pack** (`brew install wasm-pack` ou `cargo install wasm-pack`)
- Un GPU/navigateur compatible WebGPU ou WebGL2

> Sur macOS, si Rust a été installé via Homebrew (`brew install rust`), il ne
> fournit pas la `std` pour `wasm32`. Migrer vers `rustup` :
> `brew uninstall rust && brew install rustup && rustup default stable`.
> Les proxies `cargo`/`rustc` de la formule Homebrew `rustup` sont dans
> `/opt/homebrew/opt/rustup/bin` — pense à l'ajouter au `PATH`.

## Build & exécution

### Natif (recommandé pour le dev)

```bash
cargo run            # build debug + lance la fenêtre
cargo run --release  # version optimisée
RUST_LOG=info cargo run   # avec les logs
```

### Navigateur (WASM)

```bash
# Compile le module WASM dans web/pkg
wasm-pack build --target web --out-dir web/pkg

# Sert le dossier web/ (le WASM doit être servi en HTTP, pas en file://)
cd web && python3 -m http.server 8080
# puis ouvrir http://localhost:8080
```

## Contrôles

| Action | Entrée |
| --- | --- |
| (Phase 1) aucun | — |
| Orbite caméra | clic-glisser *(à partir de la phase 2)* |
| Zoom | molette *(à partir de la phase 2)* |

## Architecture

```
src/
├── main.rs            entrée native -> app::run()
├── lib.rs             entrée WASM (#[wasm_bindgen(start)])
├── app.rs            boucle d'évènements winit + cycle de vie
└── renderer/
    ├── mod.rs
    └── pipeline.rs   setup wgpu (device, surface, pipeline, uniforms) + rendu
shaders/
└── fullscreen.wgsl   triangle plein écran (deviendra le ray caster)
web/                  index.html + style.css + main.js (bootstrap WASM)
```

Phases à venir : `physics/` (métrique Schwarzschild, RK4, disque Novikov-Thorne),
`renderer/precompute.rs` (table de déflexion à la Bruneton), `renderer/textures.rs`
(HDRI, LUT corps noir), `camera.rs` (caméra orbitale).

## Tests

```bash
cargo test
```

Les tests unitaires couvriront la physique : métrique, intégrateur RK4 (vérifié
sur des orbites circulaires connues), transformation Doppler. *(à partir de la
phase 3.)*

## Références scientifiques

- **E. Bruneton (2020)** — *Real-time High-Quality Rendering of Non-Rotating
  Black Holes*. Table de déflexion précalculée indexée par paramètre d'impact.
- **O. James, E. von Tunzelmann, P. Franklin, K. Thorne (2015)** —
  *Gravitational Lensing by Spinning Black Holes in Astrophysics, and in the
  Movie Interstellar*, Classical and Quantum Gravity 32, 065001.
- **Event Horizon Telescope Collaboration (2019, 2022)** — images de M87* et
  Sgr A* (benchmark visuel).

## Licence

MIT — voir [LICENSE](LICENSE).
