# LUT de déflexion (Phase 4 — Bruneton 2020)

Ce document décrit la table de déflexion précalculée qui remplace l'intégration
RK4 par pixel de la Phase 3. Comme le code ne porte aucun commentaire, **toute
la physique et toutes les formules sont ici**.

## Idée

En métrique de Schwarzschild, une géodésique nulle vit dans un **plan** (celui
qui contient le centre du trou noir, la position de la caméra et la direction du
rayon). Dans ce plan, la trajectoire ne dépend que d'**un seul paramètre** : le
paramètre d'impact

```
b = |x × v| / |v|
```

(avec `v` la direction du rayon, normalisée, donc `b = |x × v|`). On précalcule
donc une fois pour toutes la forme de la trajectoire en fonction de `b`, et le
shader se contente d'une lecture de table + interpolation au lieu d'intégrer
~300 pas RK4 par pixel et par frame.

## Équation intégrée (Binet)

On n'intègre **pas** la forme à racine carrée

```
(du/dφ)² = 1/b² − u² + r_s·u³        (u = 1/r)
```

car elle a une singularité au périastre (point de rebroussement, `du/dφ = 0`).
On intègre à la place sa dérivée, l'**équation de Binet** du second ordre, qui
est lisse partout :

```
u''(φ) = −u + (3/2)·r_s·u²
```

Conditions initiales d'un rayon venant de l'infini avec paramètre d'impact `b` :
à l'infini `u → 0` et `u ≈ φ/b`, donc

```
u(0) = 0 ,   u'(0) = 1/b
```

On intègre en RK4 à pas constant `dφ` (φ croissant). `u` croît :

- **`b > b_crit`** : `u` atteint un maximum `u_t` (périastre) à `φ = δ_max`, puis
  redescend vers 0 à `φ = 2·δ_max`. Le rayon **s'échappe**, trajectoire
  symétrique autour du périastre.
- **`b < b_crit`** : pas de rebroussement, `u` croît jusqu'à l'horizon `u = 1`.
  Le rayon est **capturé**.

La **déflexion totale** (angle entre les deux asymptotes) vaut `α = 2·δ_max − π`.
En champ faible on retrouve le résultat classique `α ≈ 2·r_s/b` (test unitaire
`weak_field_deflection_matches_2_over_b`).

## Adimensionnement

L'équation de Binet est invariante d'échelle : en posant `ū = r_s·u` (donc
`ū = r_s/r`) et `b̄ = b/r_s`, elle devient

```
ū'' = −ū + (3/2)·ū²
```

**sans aucune dépendance en `r_s`**. La table est donc calculée une fois pour
`r_s = 1`, et le shader convertit : `b̄ = b/r_s`, `ū = r_s/r`, `r = r_s/ū`. Elle
reste valable quel que soit `r_s` (utile quand un slider l'exposera en Phase 6).

Constantes adimensionnées :

- horizon : `ū = 1` (r = r_s)
- sphère de photons : `ū = 2/3` (r = 1.5·r_s)
- paramètre d'impact critique : `b̄_crit = 3√3/2 ≈ 2.598076`

## Contenu de la table

Deux textures `R32Float` (lues en `textureLoad`, **interpolation bilinéaire
manuelle** dans le shader — pas besoin de la feature `float32-filterable`, donc
compatible WebGL2) :

- `lut_u` : `N_B × N_PHI`, texel `(i, j)` = `ū` le long de la trajectoire `b̄_i`,
  à l'angle `φ = φ_max(b̄_i)·j/(N_PHI−1)`. La colonne stocke **toute** la
  trajectoire (échappement : montée + descente symétrique ; capture : montée
  jusqu'à l'horizon).
- `lut_phi_max` : `N_B × 1`, `φ_max(b̄_i)` = angle total de la trajectoire
  (`2·δ_max` si échappement, `φ` à l'horizon si capture). Sert à dénormaliser
  l'axe φ.

`φ` est normalisé par `φ_max` colonne par colonne (sinon la divergence de
`φ_max` près de `b̄_crit` gâcherait la résolution).

### Remappage de l'axe `b̄`

`b̄_crit` est un point **intérieur** de la plage, et c'est là que la déflexion
diverge (anneau de photons). On concentre donc la résolution juste au-dessus :

```
i ≤ SPLIT :  b̄ = b̄_crit · (i / SPLIT)                          (capture, linéaire)
i > SPLIT :  e = (i − SPLIT)/(N_B − 1 − SPLIT) ;  b̄ = b̄_crit + (B_MAX − b̄_crit)·e²
```

Continu en `i = SPLIT` (`b̄ = b̄_crit`). L'inverse (utilisé dans le shader) :

```
b̄ ≤ b̄_crit :  fi = (b̄/b̄_crit) · SPLIT
b̄ > b̄_crit :  fi = SPLIT + √((b̄ − b̄_crit)/(B_MAX − b̄_crit)) · (N_B − 1 − SPLIT)
```

### Singularité en `b̄_crit`

À `b̄ = b̄_crit` exactement, `φ_max → ∞` (le rayon s'enroule indéfiniment sur la
sphère de photons). L'intégration est plafonnée à `MAX_PHI = 12π` ; au-delà, la
trajectoire est traitée comme capturée. Le cœur de l'anneau de photons (un fil
infiniment fin) devient donc noir — approximation acceptable.

### Au-delà de `B_MAX`

Pour `b̄ ≥ B_MAX = 64`, la déflexion est `< 2/64 ≈ 0.03 rad` et le rayon passe à
plus de `64·r_s` du centre (très loin du disque, `r_out = 11·r_s`). Le shader
court-circuite : rayon droit, échantillonnage direct du ciel.

## Constantes partagées Rust ↔ WGSL

Ces valeurs sont dupliquées (pas de préprocesseur de shader) et **doivent rester
synchronisées** entre `src/physics/lut.rs`, `src/physics/geodesic.rs` et
`shaders/common.wgsl` :

| Constante | Valeur | Rôle |
| --- | --- | --- |
| `N_B` | 1024 | résolution axe `b̄` |
| `N_PHI` | 512 | résolution axe `φ` |
| `SPLIT` | 256 | index où `b̄ = b̄_crit` |
| `B_CRIT` | 2.598076211353316 | `3√3/2` |
| `B_MAX` | 64.0 | `b̄` max tabulé |

## Algorithme du shader (`trace_ray`)

Pour chaque pixel, à partir de `origin` (caméra) et `dir` (direction du rayon) :

1. **Base du plan** : `radial = origin/|origin|`, `cos_psi = radial·dir`,
   `tangent = normalize(dir − cos_psi·radial)` (sens initial de progression de
   l'angle balayé θ). `sin_psi = |dir − cos_psi·radial|`.
   - Cas dégénéré `sin_psi ≈ 0` (rayon radial) : droit dans le trou → noir si
     rentrant, ciel sinon.
2. **Paramètre d'impact** : `b̄ = |origin|·sin_psi / r_s`. Si `b̄ ≥ B_MAX` →
   rayon droit, ciel.
3. **Lecture table** : `fi = lut_index_from_b(b̄)`, `φ_max = fetch_phi_max(fi)`.
   `captured = b̄ < b̄_crit`. `inward = cos_psi < 0`.
4. **Angle de la caméra sur la trajectoire** `φ_camera` : recherche par
   dichotomie de `φ` tel que `ū(φ) = r_s/r_camera`, sur la branche montante
   (`[0, φ_max/2]` si échappement, `[0, φ_max]` si capture).
5. **Angle total balayé** depuis la caméra :
   `θ_total = φ_max − φ_camera` (rentrant) ou `φ_camera` (sortant).
   La correspondance angle balayé θ ↔ angle table φ est
   `φ(θ) = φ_camera + travel_sign·θ` avec `travel_sign = +1` (rentrant) / `−1`
   (sortant).
6. **Croisements du disque** (plan équatorial `y = 0`). Le point sur la
   trajectoire est `P(θ) = r(θ)·(cos θ·radial + sin θ·tangent)`, donc
   `P(θ).y = 0` ⇔ `tan θ = −radial.y / tangent.y`. Solutions `θ = θ₀ + k·π`,
   `k = 0, 1, 2…` On parcourt les croisements par θ croissant (du plus proche au
   plus loin) tant que `θ ≤ θ_total` ; pour chacun on lit `ū` via la table,
   `r = r_s/ū`, et si `r ∈ [r_in, r_out]` → **disque** (opaque, on s'arrête au
   premier croisement valide). Gère naturellement les images multiples du disque
   (recto/verso, anneaux d'ordre supérieur) via `k > 0`.
7. Sinon, si `captured && inward` → **horizon** (noir).
8. Sinon → **échappement** : direction finale
   `normalize(cos(θ_total)·radial + sin(θ_total)·tangent)`, échantillonnée dans
   la carte du ciel.

## Coût et pistes d'optimisation

Par pixel : ~`CAMERA_PHI_ITERATIONS` (20) itérations de dichotomie + boucle de
croisements du disque, chacune faisant 4 `textureLoad` (bilinéaire). À comparer
aux ~300 pas RK4 × 4 évaluations d'accélération de la Phase 3.

Si la cible 60 fps / 1080p n'est pas tenue, la dichotomie de l'étape 4 peut être
supprimée en stockant aussi la table **directe** `Φ(ū, b̄)` (angle depuis
l'infini en fonction de `ū`), qui donne `φ_camera` en une lecture. C'est un
compromis mémoire (≈ 2 Mo de plus) vs calcul — à arbitrer après mesure.
