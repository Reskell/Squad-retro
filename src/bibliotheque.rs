//! La bibliothèque : un dossier par console, une fiche par jeu.
//!
//! ```text
//! roms/
//!   nes/       Super Mario Bros. 3 (USA).nes
//!   snes/      Super Mario World (USA).sfc, Super Mario World (Europe).sfc
//!   gamecube/  Mario Kart - Double Dash!! (Europe).rvz
//! ```
//!
//! Chaque fichier est reconnu par son empreinte quand c'est possible (bases
//! No-Intro), sinon par son nom de fichier. Les versions d'un même jeu
//! (USA, Europe, France…) sont regroupées dans une seule fiche, et la fiche
//! désigne la version préférée selon l'ordre des régions choisi.

use crate::consoles::Console;
use crate::dat::Base;
use crate::identification::{self, Empreintes, TAILLE_MAX_EMPREINTE};
use crate::region::{self, Region, ORDRE_PAR_DEFAUT};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Une version précise d'un jeu : un fichier sur le disque.
#[derive(Debug, Clone, Serialize)]
pub struct Variante {
    pub chemin: PathBuf,
    /// Le nom officiel (base No-Intro) ou, à défaut, le nom du fichier.
    pub nom: String,
    pub region: Region,
    /// Vrai si l'empreinte a été trouvée dans une base : le fichier est intact.
    pub reconnu: bool,
    #[serde(skip)]
    pub empreintes: Option<Empreintes>,
}

/// Un jeu, toutes versions confondues.
#[derive(Debug, Clone, Serialize)]
pub struct Fiche {
    pub console: Console,
    pub titre: String,
    /// Triées de la plus préférée à la moins préférée.
    pub variantes: Vec<Variante>,
}

impl Fiche {
    /// La version à lancer par défaut.
    pub fn preferee(&self) -> &Variante {
        &self.variantes[0]
    }
}

/// Tous les jeux trouvés.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Bibliotheque {
    pub fiches: Vec<Fiche>,
}

/// Comment scanner.
pub struct Options<'a> {
    /// Les bases No-Intro / Redump chargées, par console.
    pub bases: &'a HashMap<Console, Base>,
    /// L'ordre des régions préférées.
    pub ordre: &'a [Region],
}

impl Default for Options<'_> {
    fn default() -> Self {
        static VIDE: std::sync::OnceLock<HashMap<Console, Base>> = std::sync::OnceLock::new();
        Options {
            bases: VIDE.get_or_init(HashMap::new),
            ordre: &ORDRE_PAR_DEFAUT,
        }
    }
}

impl Bibliotheque {
    /// Scanne `racine/<console>/` pour chaque console.
    pub fn scanner(racine: &Path, options: &Options) -> Bibliotheque {
        let mut fiches = Vec::new();
        for console in Console::TOUTES {
            let dossier = racine.join(console.dossier());
            fiches.extend(scanner_console(console, &dossier, options));
        }
        Bibliotheque { fiches }
    }

    /// Les fichiers identiques (même empreinte) rangés plusieurs fois : de la
    /// place perdue, à signaler. Chaque groupe contient au moins deux chemins.
    pub fn doublons(&self) -> Vec<Vec<PathBuf>> {
        let mut par_sha1: BTreeMap<&str, Vec<PathBuf>> = BTreeMap::new();
        for v in self.fiches.iter().flat_map(|f| f.variantes.iter()) {
            if let Some(e) = &v.empreintes {
                par_sha1
                    .entry(e.sha1.as_str())
                    .or_default()
                    .push(v.chemin.clone());
            }
        }
        par_sha1.into_values().filter(|g| g.len() > 1).collect()
    }

    /// Recherche sur toutes les consoles : chaque mot doit apparaître dans le
    /// titre, sans tenir compte des majuscules ni des accents.
    pub fn rechercher(&self, texte: &str) -> Vec<&Fiche> {
        let mots: Vec<String> = texte.split_whitespace().map(simplifier).collect();
        if mots.is_empty() {
            return Vec::new();
        }
        self.fiches
            .iter()
            .filter(|f| {
                let titre = simplifier(&f.titre);
                mots.iter().all(|m| titre.contains(m.as_str()))
            })
            .collect()
    }

    /// Un jeu au hasard parmi ceux qui passent le filtre. `graine` vient de
    /// l'app (l'heure, par exemple) : le moteur reste sans hasard caché.
    pub fn au_hasard(&self, filtre: impl Fn(&Fiche) -> bool, graine: u64) -> Option<&Fiche> {
        let candidats: Vec<&Fiche> = self.fiches.iter().filter(|f| filtre(f)).collect();
        if candidats.is_empty() {
            return None;
        }
        // Mélange de la graine (splitmix64) pour éviter de toujours tomber au début.
        let mut z = graine.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Some(candidats[(z % candidats.len() as u64) as usize])
    }

    /// Les fiches d'une console.
    pub fn de(&self, console: Console) -> impl Iterator<Item = &Fiche> {
        self.fiches.iter().filter(move |f| f.console == console)
    }
}

/// Scanne le dossier d'une console (et ses sous-dossiers directs).
pub fn scanner_console(console: Console, dossier: &Path, options: &Options) -> Vec<Fiche> {
    let mut fichiers = Vec::new();
    lister(dossier, console, 0, &mut fichiers);

    let mut par_titre: BTreeMap<String, Vec<Variante>> = BTreeMap::new();
    for chemin in fichiers {
        let variante = identifier(&chemin, console, options);
        par_titre
            .entry(region::titre(&variante.nom).to_lowercase())
            .or_default()
            .push(variante);
    }

    par_titre
        .into_values()
        .map(|mut variantes| {
            variantes.sort_by_key(|v| (region::rang(v.region, options.ordre), !v.reconnu));
            Fiche {
                console,
                titre: region::titre(&variantes[0].nom),
                variantes,
            }
        })
        .collect()
}

/// En minuscules, sans accents ni ponctuation : « Pokémon : Rouge » → « pokemon rouge ».
pub fn simplifier(texte: &str) -> String {
    let mut s = String::new();
    for c in texte.to_lowercase().chars() {
        let c = match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        };
        s.push(c);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn lister(dossier: &Path, console: Console, profondeur: u8, sortie: &mut Vec<PathBuf>) {
    let Ok(entrees) = fs::read_dir(dossier) else {
        return;
    };
    let mut entrees: Vec<_> = entrees.flatten().map(|e| e.path()).collect();
    entrees.sort();
    for chemin in entrees {
        if chemin.is_dir() {
            if profondeur == 0 {
                lister(&chemin, console, 1, sortie);
            }
        } else if console.accepte(&chemin) {
            sortie.push(chemin);
        }
    }
}

fn identifier(chemin: &Path, console: Console, options: &Options) -> Variante {
    let nom_fichier = chemin
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();

    let empreintes = if identification::empreinte_utile(console)
        && fs::metadata(chemin)
            .map(|m| m.len() <= TAILLE_MAX_EMPREINTE)
            .unwrap_or(false)
    {
        identification::empreintes(chemin, console).ok()
    } else {
        None
    };

    let trouve = empreintes.as_ref().and_then(|e| {
        options
            .bases
            .get(&console)
            .and_then(|base| base.chercher(&e.sha1, &e.crc32))
    });

    let nom = trouve.map(|t| t.nom.clone()).unwrap_or(nom_fichier);
    Variante {
        chemin: chemin.to_path_buf(),
        region: Region::depuis_nom(&nom),
        reconnu: trouve.is_some(),
        nom,
        empreintes,
    }
}

/// Charge les bases rangées dans un dossier : `nes.dat`, `snes.dat`… (le nom
/// du dossier de la console). Une base absente ou illisible est ignorée : les
/// jeux de cette console sont alors reconnus par leur nom de fichier.
pub fn charger_bases(dossier: &Path) -> HashMap<Console, Base> {
    let mut bases = HashMap::new();
    for console in Console::TOUTES {
        let fichier = dossier.join(format!("{}.dat", console.dossier()));
        if let Ok(texte) = fs::read_to_string(&fichier) {
            if let Ok(base) = Base::lire(&texte) {
                bases.insert(console, base);
            }
        }
    }
    bases
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;

    #[test]
    fn doublons_recherche_hasard() {
        let dir = tempfile::tempdir().unwrap();
        let snes = dir.path().join("snes");
        fs::create_dir_all(snes.join("copies")).unwrap();
        write(snes.join("Pokémon Rouge (France).sfc"), b"meme").unwrap();
        write(
            snes.join("copies").join("Pokemon Rouge (France) copie.sfc"),
            b"meme",
        )
        .unwrap();
        write(snes.join("Zelda (Europe).sfc"), b"autre").unwrap();
        let b = Bibliotheque::scanner(dir.path(), &Options::default());
        assert_eq!(b.doublons().len(), 1);
        assert_eq!(b.rechercher("pokemon").len(), 2);
        assert_eq!(b.rechercher("ZELDA eur").len(), 0);
        assert_eq!(b.rechercher("zelda").len(), 1);
        assert!(b.au_hasard(|f| f.titre.contains("Zelda"), 42).is_some());
        assert!(b.au_hasard(|_| false, 1).is_none());
    }

    #[test]
    fn charge_les_bases_presentes() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path().join("nes.dat"),
            r#"<datafile><game name="X (USA)"><rom crc="1" sha1="2"/></game></datafile>"#,
        )
        .unwrap();
        write(dir.path().join("snes.dat"), "pas du xml <<<").unwrap();
        let bases = charger_bases(dir.path());
        assert!(bases.contains_key(&Console::Nes));
        assert!(!bases.contains_key(&Console::N64));
    }

    #[test]
    fn regroupe_et_prefere_la_france() {
        let dir = tempfile::tempdir().unwrap();
        let snes = dir.path().join("snes");
        fs::create_dir_all(&snes).unwrap();
        write(snes.join("Jeu A (USA).sfc"), b"usa").unwrap();
        write(snes.join("Jeu A (France).sfc"), b"fra").unwrap();
        write(snes.join("Jeu B (Europe).smc"), b"eur").unwrap();
        write(snes.join("notes.txt"), b"pas un jeu").unwrap();

        let biblio = Bibliotheque::scanner(dir.path(), &Options::default());
        let fiches: Vec<_> = biblio.de(Console::Snes).collect();
        assert_eq!(fiches.len(), 2);
        let a = fiches.iter().find(|f| f.titre == "Jeu A").unwrap();
        assert_eq!(a.variantes.len(), 2);
        assert_eq!(a.preferee().region, Region::France);
    }

    #[test]
    fn reconnait_par_empreinte() {
        let dir = tempfile::tempdir().unwrap();
        let snes = dir.path().join("snes");
        fs::create_dir_all(&snes).unwrap();
        write(snes.join("fichier renomme.sfc"), b"abc").unwrap();

        let dat = r#"<datafile><game name="Vrai Nom (Europe)">
            <rom name="x.sfc" size="3" crc="352441c2" sha1="a9993e364706816aba3e25717850c26c9cd0d89d"/>
        </game></datafile>"#;
        let mut bases = HashMap::new();
        bases.insert(Console::Snes, Base::lire(dat).unwrap());
        let options = Options {
            bases: &bases,
            ordre: &ORDRE_PAR_DEFAUT,
        };

        let biblio = Bibliotheque::scanner(dir.path(), &options);
        let fiche = biblio.de(Console::Snes).next().unwrap();
        assert_eq!(fiche.titre, "Vrai Nom");
        assert!(fiche.preferee().reconnu);
        assert_eq!(fiche.preferee().region, Region::Europe);
    }
}
