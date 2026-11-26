"""Télécharge chaque archive de manifestes/sources.json, calcule son
empreinte SHA-256 et écrit manifestes/versions-beta.json (révision + 1)."""
import hashlib, json, pathlib, sys, urllib.request

racine = pathlib.Path(__file__).resolve().parents[2] / "manifestes"
sources = json.loads((racine / "sources.json").read_text(encoding="utf-8"))["composants"]
beta_fichier = racine / "versions-beta.json"
ancienne = json.loads(beta_fichier.read_text(encoding="utf-8")) if beta_fichier.exists() else {"revision": 0}

composants, erreurs = {}, []
for nom, s in sources.items():
    try:
        requete = urllib.request.Request(s["url"], headers={"User-Agent": "SQUAD-RETRO"})
        with urllib.request.urlopen(requete, timeout=300) as r:
            octets = r.read()
        composants[nom] = {"version": s["version"], "url": s["url"], "sha256": hashlib.sha256(octets).hexdigest()}
        print(f"OK  {nom} {s['version']} ({len(octets) // 1024} Kio)")
    except Exception as e:  # noqa: BLE001
        erreurs.append(f"{nom} : {e}")
        print(f"ERREUR {nom} : {e}")

if erreurs:
    print("\nCertaines archives sont introuvables : corrige sources.json.", file=sys.stderr)
    sys.exit(1)

beta = {"revision": ancienne.get("revision", 0) + 1, "composants": composants}
beta_fichier.write_text(json.dumps(beta, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
print(f"\nversions-beta.json écrit (révision {beta['revision']}).")
