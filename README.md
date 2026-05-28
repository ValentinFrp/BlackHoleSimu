# BlackHoleSimu

Simulateur de **trou noir de Schwarzschild** (non rotatif) rendu en temps réel,
visant un réalisme comparable à Gargantua (*Interstellar*) ou aux images de
l'Event Horizon Telescope. Le cœur est un *fragment shader* plein écran qui fait
du ray casting par pixel à travers l'espace-temps courbé.

Écrit en **Rust** avec **wgpu** (WebGPU natif + fallback WebGL2) et **WGSL**.
Une seule base de code, deux cibles : **natif** (`cargo run`, pour itérer vite)
et **navigateur** via **WebAssembly** (`wasm-pack`).

> **État : Phase 2 (trou noir naïf).** Caméra orbitale, sphère noire (horizon),
> disque d'accrétion plat coloré et fond étoilé HDRI. Les rayons sont encore
> **droits** : pas de lentille gravitationnelle (elle arrive en Phase 3 avec
> l'intégration des géodésiques).

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
| Orbiter autour du trou noir | clic gauche + glisser |
| Zoom (rapprocher / éloigner) | molette |

## Rendu (Phase 2)

Un triangle plein écran déclenche le *fragment shader* `blackhole.wgsl`, qui
lance pour chaque pixel un rayon depuis la caméra. Trois issues :

1. le rayon coupe l'horizon (sphère de rayon `r_s`) → **noir** ;
2. il coupe le plan équatorial dans l'anneau `[r_in, r_out]` → **disque** coloré
   (dégradé radial provisoire ; le vrai corps noir relativiste vient en Phase 5) ;
3. sinon il part à l'infini → on échantillonne le **HDRI** (projection
   équirectangulaire) dans la direction du rayon.

À ce stade les rayons sont rectilignes ; la courbure de l'espace-temps (lentille,
anneau de photons, beaming, redshift) est ajoutée aux phases suivantes.

## Architecture

```
src/
├── main.rs                 entrée native -> app::run()
├── lib.rs                  entrée WASM (#[wasm_bindgen(start)])
├── app.rs                  boucle winit, dispatch des entrées, cycle de vie
├── camera.rs               OrbitCamera (coords sphériques) + CameraController
├── scene.rs                paramètres physiques (trou noir, disque)
└── renderer/
    ├── mod.rs              façade Renderer (orchestration d'une frame)
    ├── context.rs          GpuContext : device/queue/surface, resize, frame
    ├── uniforms.rs         bloc uniforme (std140) caméra + scène
    ├── hdri.rs             chargement HDRI (fs natif / fetch WASM) + décodage
    ├── texture.rs          texture + sampler du fond HDRI
    └── blackhole_pass.rs   pipeline + bind groups + enregistrement de la passe
shaders/
├── common.wgsl             uniforms + helpers math (rayons, intersections, sky)
└── blackhole.wgsl          entry points VS/FS (concaténé après common.wgsl)
web/                        index.html + style.css + main.js (bootstrap WASM)
assets/milkyway.hdr         fond Voie lactée (non versionné, voir ci-dessous)
```

Phases à venir : `physics/` (métrique Schwarzschild, RK4, disque Novikov-Thorne),
`renderer/precompute.rs` (table de déflexion à la Bruneton), LUT corps noir.

## Assets

Le fond étoilé attendu en `assets/milkyway.png` est une carte du ciel
équirectangulaire (4096×2048). Le chargeur accepte aussi le Radiance `.hdr`
(adapter `SKY_SOURCE` dans `renderer/mod.rs`). Le fichier n'est pas versionné ;
s'il est absent, le rendu bascule sur un fond sombre uni.

Le fond fourni par défaut est le **Milky Way panorama** de l'ESO :

> Crédit : **ESO/S. Brunier** — [eso0932a](https://www.eso.org/public/images/eso0932a/),
> sous licence [Creative Commons CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
> Réduit en 4096×2048 et converti en PNG pour ce projet.

En natif, le chemin est résolu en absolu (racine du crate) : `cargo run`
fonctionne quel que soit le dossier courant. Pour le web, le serveur sert `web/`,
donc un symlink `web/assets -> ../assets` expose l'asset sans le dupliquer :

```bash
ln -s ../assets web/assets   # à créer une fois
```

## Tests

```bash
cargo test
```

Actuellement : validation du shader WGSL via naga (parse + validation) sans GPU.
À partir de la Phase 3 s'ajouteront les tests de physique : métrique, intégrateur
RK4 (vérifié sur des orbites circulaires connues), transformation Doppler.

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
