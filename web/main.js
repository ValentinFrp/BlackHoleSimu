import init from "./pkg/blackhole_simu.js";

init().catch((err) => {
  console.error("Échec d'initialisation du module WASM :", err);
});
