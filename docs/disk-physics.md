# Physique du disque d'accrétion (Phase 5)

Au croisement d'un rayon avec le plan équatorial dans l'anneau `[r_in, r_out]`,
on calcule la couleur observée du disque à partir de la relativité, et non plus
d'un dégradé arbitraire. Le code ne portant pas de commentaires, toute la
dérivation est ici. Voir aussi `docs/deflection-lut.md` pour la table de
déflexion qui fournit le point d'impact.

## Vue d'ensemble : tout tient dans le facteur `g`

`g = ν_observée / ν_émise` est le rapport de fréquence entre l'émission (dans le
référentiel local de la matière du disque) et la réception (caméra). Il combine
le **Doppler** (rotation du disque) et le **redshift gravitationnel**.

Deux conséquences exploitées :

1. Un **corps noir décalé reste un corps noir** : un spectre de Planck à
   température `T` vu avec un facteur `g` est un spectre de Planck à `T' = g·T`.
   → la **couleur** observée est celle du corps noir à `g·T(r)`.
2. L'invariant de Liouville `I_ν / ν³ = cste` le long d'une géodésique nulle, et
   la loi de Stefan-Boltzmann `I ∝ T⁴`, donnent une **brillance bolométrique**
   observée `∝ g⁴ · T(r)⁴`.

D'où le shading final au point d'impact :

```
couleur = corps_noir(g · T(r)) × ( g⁴ · (T(r)/T_pic)⁴ × DISK_INTENSITY )
```

(`T(r)/T_pic` garde la dynamique en O(1) ; `DISK_INTENSITY` + `exposure` + ACES
règlent l'échelle.)

## Profil de température T(r) — Novikov-Thorne

Disque mince, condition de couple nul au bord interne (Shakura-Sunyaev /
Novikov-Thorne, forme effective) :

```
T(r) = T_pic · S(r) / S_pic ,   S(r) = [ (r_in/r)³ · (1 − √(r_in/r)) ]^(1/4)
```

- `S(r_in) = 0` : bord interne **froid** (signature du disque relativiste).
- `S` est maximal en `r = (49/36)·r_in ≈ 1.361·r_in`, où
  `S_pic = ((36/49)³ / 7)^(1/4) ≈ 0.4877986` (constante `NT_PEAK` du shader).
- `T_pic` est exposé par `Scene::disk.peak_temperature` (défaut **6500 K**,
  uniform `disk_temperature`). À 6500 K le bord externe tombe vers ~3500 K
  (orangé) et le décalage Doppler pousse le côté qui s'approche vers le
  blanc-bleu et celui qui s'éloigne vers le rouge — d'où une couleur lisible.

## Vitesse orbitale de la matière

Orbite circulaire géodésique de Schwarzschild. Vitesse **mesurée localement**
par un observateur statique :

```
β(r) = √( (r_s/2) / (r − r_s) )
```

(azimutale, dans le plan équatorial), `γ = 1/√(1−β²)`. Valable pour `r > 1.5 r_s` ;
le disque commençant à `r_in = 3 r_s` (ISCO), on reste bien dans le domaine.

## Direction locale du photon

Le tracé se fait **caméra → disque** ; le photon physique va dans l'autre sens.
La direction du photon dans le plan orbital se dérive analytiquement de la table
sans différences finies, via l'intégrale première de la géodésique
(adimensionnée, `ū = r_s/r`) :

```
(dū/dφ)² = 1/b̄² − ū² + ū³
```

- signe de `dū/dφ` : `+` sur la branche montante (`φ < φ_max/2`, ou capture),
  `−` après le périastre.
- `dr/dθ = −(r_s/ū²)·(dū/dφ)·travel_sign` (avec `θ` l'angle balayé depuis la
  caméra, `φ = φ_camera + travel_sign·θ`).

Au croisement (angle balayé `θ`), avec `radial`/`tangent` la base du plan
orbital :

```
r̂        = cos θ · radial + sin θ · tangent      (direction radiale du disque, équatoriale)
t̂        = −sin θ · radial + cos θ · tangent     (perpendiculaire dans le plan orbital)
photon   = (dr/dθ) · r̂ + r · t̂                  (direction coordonnée du tracé)
```

### Correction de tétrade (repère propre)

`photon` est une direction en **coordonnées** (espace tracé comme euclidien).
Pour le Doppler il faut la direction dans le **repère orthonormé local** de
l'observateur statique. En Schwarzschild :

- composante **radiale** : longueur propre `dr/√(1−r_s/r)` → on multiplie la part
  radiale par `1/√(1−r_s/r)` ;
- composantes **transverses** (azimutale `r dφ`, verticale `r dθ_polaire`) :
  déjà propres (`g_φφ = g_θθ = r²`), inchangées.

La direction d'**émission** (vers la caméra) est `k̂ = normalize(−photon_propre)`.

## Le facteur g

```
g_doppler  = 1 / ( γ · (1 − β · (φ̂ · k̂)) )           φ̂ = DISK_SPIN · (ŷ × r̂)
g_gravité  = √( (1 − r_s/r) / (1 − r_s/r_caméra) )
g          = g_gravité · g_doppler
```

- `φ̂ · k̂ > 0` (matière venant vers la caméra) → `g > 1` : **bleui, plus
  brillant**. L'autre côté : **rougi, assombri**. Avec l'exposant `g⁴` sur la
  brillance, l'asymétrie est forte (beaming **physiquement exact**, choix de
  cette phase).
- `DISK_SPIN = ±1` fixe le sens de rotation.

## Corps noir → couleur

LUT 1D (`src/physics/blackbody.rs`, texture `Rgba16Float` de `BB_N = 1024`,
`T ∈ [1000, 40000] K`) :

1. spectre de Planck `B(λ, T)` ;
2. intégré sur 380–780 nm contre les **fonctions colorimétriques CIE 1931**
   (approximation gaussienne multi-lobes de *Wyman, Sloan & Shirley 2013*, pour
   éviter d'embarquer les tables) → XYZ ;
3. normalisation `Y = 1` (la luminance est portée séparément par la brillance) ;
4. XYZ → **sRGB linéaire** (matrice standard), composantes négatives (hors gamut)
   coupées à 0.

Le shader lit la LUT à `g·T(r)` (interpolation linéaire manuelle, `textureLoad`).

Tests (`cargo test`) : 2000 K rougeâtre (`R > B`), 20000 K bleuté (`B > R`),
6500 K ≈ neutre.

## Constantes du shader (réglage)

| Constante | Rôle |
| --- | --- |
| `NT_PEAK` | `S_pic` de Novikov-Thorne (≈ 0.4878) |
| `DISK_SPIN` | sens de rotation (±1) |
| `DISK_INTENSITY` | échelle de brillance globale du disque |
| `BB_N`, `BB_T_MIN`, `BB_T_MAX` | doivent matcher `src/physics/blackbody.rs` |
