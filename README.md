# BlackHoleSimu

Simulateur de **trou noir de Schwarzschild** (non rotatif) rendu en temps réel,
visant un réalisme comparable à Gargantua (*Interstellar*) ou aux images de
l'Event Horizon Telescope. Le cœur est un *fragment shader* plein écran qui fait
du ray casting par pixel à travers l'espace-temps courbé.

Écrit en **Rust** avec **wgpu** (WebGPU natif + fallback WebGL2) et **WGSL**.
Une seule base de code, deux cibles : **natif** (`cargo run`, pour itérer vite)
et **navigateur** via **WebAssembly** (`wasm-pack`).

> **État : terminé (Phase 6 — polish).** Lentille gravitationnelle via table de
> déflexion ([Bruneton 2020](docs/deflection-lut.md)) ; disque relativiste
> physique (Novikov-Thorne + Doppler/redshift + corps noir,
> [détails](docs/disk-physics.md)) ; et post-traitement HDR : **super-sampling
> (SSAA 2×)** anti-aliasing, **bloom**, tonemap **ACES**, et un panneau de
> **sliders egui** (température, intensité, rayons, exposition, bloom, SSAA…).

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
| Régler le disque / le rendu | panneau de sliders (en haut à gauche) |

## Rendu (Phase 4)

Un triangle plein écran déclenche le *fragment shader* `blackhole.wgsl`. Pour
chaque pixel, `trace_ray` réduit le problème au **plan orbital** (centre +
caméra + rayon), calcule le **paramètre d'impact** `b = |x × v|`, puis interroge
une **table de déflexion précalculée** au lieu d'intégrer la géodésique.

La table est construite une fois au lancement (`src/physics/`) en intégrant
l'**équation de Binet** `u'' = −u + (3/2) r_s u²` (lisse, sans singularité au
périastre), adimensionnée donc indépendante de `r_s`. Elle est uploadée en deux
textures `R32Float` et lue par interpolation bilinéaire manuelle (compatible
WebGL2). Le shader en déduit, le long du rayon courbé :

1. `b < b_crit = 3√3/2·r_s` et rayon rentrant → **noir** (horizon) ;
2. croisement(s) du plan équatorial dans l'anneau `[r_in, r_out]` → **disque**
   relativiste (profil Novikov-Thorne, Doppler + redshift via `g`, couleur de
   corps noir, brillance `∝ g⁴` ; cf. [`docs/disk-physics.md`](docs/disk-physics.md)).
   Les croisements multiples donnent recto/verso et anneaux d'ordre supérieur ;
3. échappement → échantillonnage du fond stellaire dans la **direction finale
   lensée**.

Toute la dérivation (équation, adimensionnement, layout de la table, remappage
de l'axe `b`, algorithme du shader) est dans
[`docs/deflection-lut.md`](docs/deflection-lut.md).

## Architecture

```
src/
├── main.rs                 entrée native -> app::run()
├── lib.rs                  entrée WASM (#[wasm_bindgen(start)])
├── app.rs                  boucle winit, dispatch des entrées, cycle de vie
├── camera.rs               OrbitCamera (coords sphériques) + CameraController
├── scene.rs                paramètres physiques (trou noir, disque)
├── physics/
│   ├── mod.rs             ré-exports
│   ├── geodesic.rs        intégration de l'équation de Binet (+ tests)
│   ├── lut.rs             construction de la table de déflexion (+ tests)
│   └── blackbody.rs       LUT corps noir Planck × CIE → sRGB (+ tests)
├── ui.rs                  panneau de réglages egui (sliders)
└── renderer/
    ├── mod.rs             façade Renderer (orchestration d'une frame + egui)
    ├── context.rs         GpuContext : device/queue/surface, resize, frame
    ├── uniforms.rs        bloc uniforme (std140) caméra + scène
    ├── sky.rs             chargement du fond (fs natif / fetch WASM) + décodage
    ├── texture.rs         texture + sampler du fond stellaire
    ├── lut_texture.rs     upload de la table de déflexion en textures R32Float
    ├── offscreen.rs       cible HDR (résolution SSAA) + textures de bloom
    ├── post_pass.rs       bloom + composite (exposure + ACES) vers le swapchain
    └── blackhole_pass.rs  pipeline + bind groups + passe principale (→ HDR)
shaders/
├── common.wgsl             uniforms + lecture de table + trace_ray + helpers
├── blackhole.wgsl          entry points VS/FS (sortie HDR linéaire)
├── post.wgsl               bright-pass + flou gaussien séparable (bloom)
└── composite.wgsl          scène + bloom → exposure → ACES → swapchain
docs/deflection-lut.md      dérivation de la table de déflexion (Phase 4)
docs/disk-physics.md        Doppler/redshift/Novikov-Thorne/corps noir (Phase 5)
web/                        index.html + style.css + main.js (bootstrap WASM)
assets/milkyway.png         fond Voie lactée (non versionné, voir ci-dessous)
```

Post-traitement (Phase 6) : la passe principale écrit une cible HDR linéaire en
résolution SSAA, puis `post_pass` applique bloom + tonemap ACES vers le
swapchain, et egui dessine les sliders par-dessus.

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
