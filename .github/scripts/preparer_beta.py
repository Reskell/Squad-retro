"""Télécharge chaque archive de manifestes/sources.json, calcule son
empreinte SHA-256 et écrit manifestes/versions-beta.json (révision + 1).

Les composants marqués « miroir » (cores RetroArch, dont l'adresse nightly
change tous les jours) sont copiés dans miroir/ : le workflow les publie
dans une version GitHub du dépôt, et le manifeste pointe vers cette copie
fixe. Leur version devient la date du jour."""
import datetime, hashlib, json, os, pathlib, sys, urllib.request

racine = pathlib.Path(__file__).resolve().parents[2]
manifestes = racine / "manifestes"
miroir = racine / "miroir"
sources = json.loads((manifestes / "sources.json").read_text(encoding="utf-8"))["composants"]
beta_fichier = manifestes / "versions-beta.json"
ancienne = json.loads(beta_fichier.read_text(encoding="utf-8")) if beta_fichier.exists() else {"revision": 0}
revision = ancienne.get("revision", 0) + 1
depot = os.environ.get("GITHUB_REPOSITORY", "Reskell/Squad-retro")
etiquette = f"composants-r{revision}"
aujourd_hui = datetime.date.today().strftime("%Y%m%d")

composants, erreurs = {}, []
for nom, s in sources.items():
    try:
        requete = urllib.request.Request(s["url"], headers={"User-Agent": "SQUAD-RETRO"})
        with urllib.request.urlopen(requete, timeout=300) as r:
            octets = r.read()
        url, version = s["url"], s["version"]
        if s.get("miroir"):
            miroir.mkdir(exist_ok=True)
            fichier = s["url"].rsplit("/", 1)[-1]
            (miroir / fichier).write_bytes(octets)
            url = f"https://github.com/{depot}/releases/download/{etiquette}/{fichier}"
            version = aujourd_hui
        composants[nom] = {"version": version, "url": url, "sha256": hashlib.sha256(octets).hexdigest()}
        print(f"OK  {nom} {version} ({len(octets) // 1024} Kio)")
    except Exception as e:  # noqa: BLE001
        erreurs.append(f"{nom} : {e}")
        print(f"ERREUR {nom} : {e}")

if erreurs:
    print("\nCertaines archives sont introuvables : corrige sources.json.", file=sys.stderr)
    sys.exit(1)

beta = {"revision": revision, "composants": composants}
beta_fichier.write_text(json.dumps(beta, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
(racine / "etiquette.txt").write_text(etiquette, encoding="utf-8")
print(f"\nversions-beta.json écrit (révision {revision}).")
