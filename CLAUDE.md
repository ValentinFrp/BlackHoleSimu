# CLAUDE.md

Guide pour travailler sur ce dépôt. Voir `README.md` pour la doc utilisateur.

## Projet

Simulateur de trou noir de Schwarzschild rendu en temps réel. Ray casting par
pixel dans un fragment shader plein écran (espace-temps courbé). Objectif :
réalisme type Gargantua / EHT. Approche optimisée visée : **Bruneton 2020**
(table de déflexion précalculée CPU, indexée par paramètre d'impact, au lieu
d'intégrer la géodésique par pixel chaque frame).

## Stack (imposée — ne pas changer sans accord explicite de l'utilisateur)

- Rust 2021, **wgpu 29** (WebGPU natif + fallback WebGL2), **winit 0.30**, WGSL
- wasm-bindgen + wasm-pack pour le navigateur
- Pas de moteur de jeu (pas de Bevy) : accès direct à wgpu
- **Un seul crate, deux cibles** : `lib` (cdylib pour WASM + rlib) et `bin`
  natif (`src/main.rs`) qui réutilise la lib. Pas de cargo workspace.

## Commandes

```bash
cargo run                  # natif (debug)
cargo run --release        # natif optimisé
cargo build                # vérif compilation natif
cargo test                 # tests (physique surtout)
wasm-pack build --target web --out-dir web/pkg   # build navigateur -> web/pkg
cd web && python3 -m http.server 8080            # servir le web
```

### Gotcha toolchain (macOS de l'utilisateur)

`cargo`/`rustc` sont des **proxies rustup** installés par la formule Homebrew
`rustup`, situés dans `/opt/homebrew/opt/rustup/bin` (formule *keg-only*, pas
dans `~/.cargo/bin`). Ce chemin a été ajouté à `~/.zshrc` et `~/.zprofile`. Si
une commande `cargo` échoue avec « command not found » dans un shell non
interactif, préfixer :

```bash
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
```

Le Rust Homebrew d'origine (sans `std` wasm32) a été désinstallé au profit de
rustup. `brew autoremove` étant actif chez l'utilisateur, un `brew uninstall`
peut emporter des dépendances orphelines — prévenir avant d'en lancer.

## Spécificités d'API wgpu 29 (a changé vs versions ≤ 25)

Caler tout nouveau code sur l'exemple officiel `hello_triangle` du tag de la
version. Pièges rencontrés :

- `wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(display_handle)))`
- `surface.get_current_texture()` renvoie un **enum** `CurrentSurfaceTexture`
  (`Success` / `Suboptimal` / `Timeout` / `Occluded` / `Outdated` / `Lost` /
  `Validation`), **pas** un `Result`.
- `DeviceDescriptor` a les champs `experimental_features`, `memory_hints`, `trace`.
- `PipelineLayoutDescriptor.bind_group_layouts: &[Option<&BindGroupLayout>]`
  (envelopper dans `Some(...)`), et champ `immediate_size`.
- `RenderPipelineDescriptor` / `RenderPassDescriptor` ont `multiview_mask`.
- `RenderPassColorAttachment` a `depth_slice`.

## Architecture

```
src/main.rs            entrée native -> blackhole_simu::app::run()
src/lib.rs             entrée WASM (#[wasm_bindgen(start)]) + déclaration modules
src/app.rs             boucle winit (ApplicationHandler), init GPU async via spawn()
src/renderer/mod.rs    réexporte WgpuState
src/renderer/pipeline.rs  device/surface/uniforms/pipeline + render()
shaders/fullscreen.wgsl   triangle plein écran (deviendra le ray caster)
web/                   index.html + style.css + main.js (bootstrap WASM)
```

L'init GPU est asynchrone : exécutée dans `resumed()` via `spawn()` (pollster en
natif, `spawn_local` en WASM), résultat renvoyé dans la boucle par l'évènement
`AppAction::Ready`. Uniforms (`time`, `resolution`) mis à jour chaque frame.

## Conventions

- **Aucun commentaire dans le code** (préférence utilisateur). Toute la doc,
  surtout la physique et les formules, va dans des fichiers `.md` détaillés.
  Compenser l'absence de commentaires par un nommage clair.
- **Tests unitaires** pour la physique : métrique, RK4 (vérifié sur orbites
  circulaires connues), transformation Doppler.
- **Commit à la fin de chaque phase**, message clair (`phase N: ...`).
- Devant un choix technique non trivial (ex. storage buffer vs uniform pour la
  LUT), **exposer les options et demander** à l'utilisateur.
- Cibles perf : 60 fps en 1080p (desktop moyen / M1), 30 fps en 720p (mobile),
  bundle WASM gzipé < 2 Mo (hors HDRI). Si une cible est ratée, proposer des
  arbitrages avant de couper dans la qualité visuelle.

## Feuille de route (phases)

1. ✅ Squelette qui compile natif + WASM (couleur unie animée).
2. Trou noir naïf : caméra orbitale, sphère noire, disque plat, skybox HDRI.
3. Géodésiques RK4 en temps réel dans le shader.
4. LUT de déflexion (Bruneton) — le shader interroge la table.
5. Physique fine : Doppler `g`, intensité `g^4`, profil T(r), LUT corps noir.
6. Polish : super-sampling sur l'anneau, bloom HDR, tonemap ACES, UI sliders.
