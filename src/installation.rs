//! L'installation et la mise à jour des émulateurs (vague 3).
//!
//! L'app télécharge l'archive indiquée par le fichier des versions validées
//! et la passe au moteur, qui :
//! 1. refuse tout fichier dont l'empreinte SHA-256 ne correspond pas ;
//! 2. décompresse (.zip ou .7z) dans un dossier temporaire, sans jamais
//!    écrire hors de ce dossier ;
//! 3. range l'ancienne version de côté (retour arrière en un clic si la
//!    nouvelle plante), en gardant ce qui doit survivre (les cores de
//!    RetroArch, par exemple) ;
//! 4. note la liste des fichiers et leur taille, pour vérifier plus tard
//!    que rien n'a été abîmé ou supprimé.

use crate::emplacements::Emplacements;
use crate::versions::{Composant, Installees};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};

/// Comment un composant s'installe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cible {
    /// Remplace tout un dossier (en gardant certains sous-dossiers).
    Dossier {
        dossier: PathBuf,
        garder: Vec<&'static str>,
    },
    /// Ajoute ses fichiers dans un dossier existant (un core, un outil).
    Fichiers { dossier: PathBuf },
}

/// Où va un composant, d'après son nom dans versions.json.
pub fn cible(nom: &str, ou: &Emplacements) -> Option<Cible> {
    let emulateurs = ou.emulateurs();
    match nom {
        "retroarch" => Some(Cible::Dossier {
            dossier: emulateurs.join("retroarch"),
            garder: vec!["cores", "shaders"],
        }),
        "dolphin" => Some(Cible::Dossier {
            dossier: emulateurs.join("dolphin"),
            garder: vec![],
        }),
        "shaders" => Some(Cible::Dossier {
            dossier: emulateurs.join("retroarch").join("shaders"),
            garder: vec![],
        }),
        "chdman" => Some(Cible::Fichiers {
            dossier: emulateurs.join("outils"),
        }),
        n if n.starts_with("core:") => Some(Cible::Fichiers {
            dossier: ou.cores.clone(),
        }),
        _ => None,
    }
}

/// Ce que le moteur retient d'un composant installé.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installe {
    pub version: String,
    /// Fichiers écrits (relatifs au dossier cible, avec des « / ») et leur taille.
    pub fichiers: BTreeMap<String, u64>,
    /// La version précédente, gardée pour le retour arrière.
    #[serde(default)]
    pub precedente: Option<String>,
}

/// L'état de tout ce qui est installé, dans `installees.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Etat {
    pub composants: BTreeMap<String, Installe>,
}

impl Etat {
    pub fn fichier(ou: &Emplacements) -> PathBuf {
        ou.emulateurs().join("installees.json")
    }
    pub fn charger(ou: &Emplacements) -> Etat {
        fs::read_to_string(Etat::fichier(ou))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }
    pub fn ecrire(&self, ou: &Emplacements) -> std::io::Result<()> {
        fs::create_dir_all(ou.emulateurs())?;
        fs::write(
            Etat::fichier(ou),
            serde_json::to_string_pretty(self).unwrap_or_default(),
        )
    }
    /// Nom → version, pour comparer aux versions validées.
    pub fn versions(&self) -> Installees {
        self.composants
            .iter()
            .map(|(n, i)| (n.clone(), i.version.clone()))
            .collect()
    }
}

pub fn sha256(octets: &[u8]) -> String {
    Sha256::digest(octets)
        .iter()
        .map(|o| format!("{o:02x}"))
        .collect()
}

/// Installe (ou met à jour) un composant à partir de son archive téléchargée.
pub fn installer(
    nom: &str,
    composant: &Composant,
    archive: &[u8],
    ou: &Emplacements,
) -> Result<Installe, String> {
    if sha256(archive) != composant.sha256.to_ascii_lowercase() {
        return Err(format!(
            "Le téléchargement de {nom} est abîmé ou a été modifié (empreinte différente) : rien n'a été installé."
        ));
    }
    let cible = cible(nom, ou).ok_or_else(|| format!("Composant inconnu : {nom}"))?;
    // Un dossier temporaire propre à cette installation : deux installations
    // lancées en même temps ne se marchent plus dessus.
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let temporaire = ou.emulateurs().join(format!(
        ".installation-{}-{}-{unique}",
        nom.replace(':', "-"),
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&temporaire);
    fs::create_dir_all(&temporaire).map_err(|e| e.to_string())?;
    if let Err(e) = decompresser(archive, &composant.url, &temporaire) {
        let _ = fs::remove_dir_all(&temporaire);
        return Err(e);
    }
    let source = dossier_utile(&temporaire);

    let mut etat = Etat::charger(ou);
    let precedente = etat.composants.get(nom).map(|i| i.version.clone());
    let fichiers = match &cible {
        Cible::Dossier { dossier, garder } => {
            if let Err(e) = remplacer_dossier(&source, dossier, garder, nom, precedente.as_deref(), ou) {
                let _ = fs::remove_dir_all(&temporaire);
                return Err(e);
            }
            inventaire(dossier, garder)
        }
        Cible::Fichiers { dossier } => {
            fs::create_dir_all(dossier).map_err(|e| e.to_string())?;
            let ajoutes = inventaire(&source, &[]);
            let anciennes = anciennes(ou, nom, precedente.as_deref());
            for chemin in ajoutes.keys() {
                let depuis = source.join(chemin);
                let vers = dossier.join(chemin);
                if vers.is_file() {
                    let copie = anciennes.join(chemin);
                    if let Some(p) = copie.parent() {
                        let _ = fs::create_dir_all(p);
                    }
                    let _ = fs::copy(&vers, &copie);
                }
                if let Some(p) = vers.parent() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
                fs::copy(&depuis, &vers)
                    .map_err(|e| format!("Impossible d'écrire {} : {e}", vers.display()))?;
            }
            ajoutes
        }
    };
    let _ = fs::remove_dir_all(&temporaire);

    let installe = Installe {
        version: composant.version.clone(),
        fichiers,
        precedente,
    };
    etat.composants.insert(nom.to_string(), installe.clone());
    etat.ecrire(ou).map_err(|e| e.to_string())?;
    Ok(installe)
}

fn anciennes(ou: &Emplacements, nom: &str, version: Option<&str>) -> PathBuf {
    ou.emulateurs().join("anciennes").join(format!(
        "{}-{}",
        nom.replace(':', "-"),
        version.unwrap_or("inconnue")
    ))
}

fn remplacer_dossier(
    source: &Path,
    dossier: &Path,
    garder: &[&str],
    nom: &str,
    precedente: Option<&str>,
    ou: &Emplacements,
) -> Result<(), String> {
    if dossier.exists() {
        let rangement = anciennes(ou, nom, precedente);
        let _ = fs::remove_dir_all(&rangement);
        fs::create_dir_all(rangement.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::rename(dossier, &rangement).map_err(|e| {
            format!("Impossible de ranger l'ancienne version (un jeu tourne-t-il ?) : {e}")
        })?;
        let copie = copier_dossier(source, dossier).and_then(|_| {
            for g in garder {
                let ancien = rangement.join(g);
                if ancien.exists() && !dossier.join(g).exists() {
                    copier_dossier(&ancien, &dossier.join(g))?;
                }
            }
            Ok(())
        });
        if let Err(e) = copie {
            // Échec en cours de route : on remet l'ancien dossier tel quel.
            let _ = fs::remove_dir_all(dossier);
            let _ = fs::rename(&rangement, dossier);
            return Err(e);
        }
    } else if let Err(e) = copier_dossier(source, dossier) {
        let _ = fs::remove_dir_all(dossier);
        return Err(e);
    }
    Ok(())
}

/// Revient à la version précédente d'un composant (si elle a été gardée).
pub fn retour_arriere(nom: &str, ou: &Emplacements) -> Result<String, String> {
    let mut etat = Etat::charger(ou);
    let actuel = etat
        .composants
        .get(nom)
        .cloned()
        .ok_or_else(|| format!("{nom} n'est pas installé."))?;
    let precedente = actuel
        .precedente
        .clone()
        .ok_or_else(|| format!("Pas de version précédente de {nom} à remettre."))?;
    let rangement = anciennes(ou, nom, Some(&precedente));
    if !rangement.exists() {
        return Err(format!(
            "La version {precedente} de {nom} n'a pas été gardée."
        ));
    }
    match cible(nom, ou).ok_or_else(|| format!("Composant inconnu : {nom}"))? {
        Cible::Dossier { dossier, garder } => {
            let echec = ou
                .emulateurs()
                .join(format!(".echec-{}", nom.replace(':', "-")));
            let _ = fs::remove_dir_all(&echec);
            fs::rename(&dossier, &echec).map_err(|e| e.to_string())?;
            copier_dossier(&rangement, &dossier)?;
            for g in &garder {
                if echec.join(g).exists() && !dossier.join(g).exists() {
                    copier_dossier(&echec.join(g), &dossier.join(g))?;
                }
            }
            let _ = fs::remove_dir_all(&echec);
            etat.composants.insert(
                nom.into(),
                Installe {
                    version: precedente.clone(),
                    fichiers: inventaire(&dossier, &garder),
                    precedente: None,
                },
            );
        }
        Cible::Fichiers { dossier } => {
            for chemin in inventaire(&rangement, &[]).keys() {
                fs::copy(rangement.join(chemin), dossier.join(chemin))
                    .map_err(|e| e.to_string())?;
            }
            etat.composants.insert(
                nom.into(),
                Installe {
                    version: precedente.clone(),
                    fichiers: actuel.fichiers,
                    precedente: None,
                },
            );
        }
    }
    etat.ecrire(ou).map_err(|e| e.to_string())?;
    Ok(precedente)
}

/// Les composants installés dont des fichiers manquent ou sont abîmés :
/// à réinstaller même si leur version est la bonne.
pub fn a_reparer(ou: &Emplacements) -> Vec<String> {
    Etat::charger(ou)
        .composants
        .keys()
        .filter(|n| !verifier_integrite(n, ou).is_empty())
        .cloned()
        .collect()
}

/// Les fichiers d'un composant manquants ou de taille changée (vide = intact).
pub fn verifier_integrite(nom: &str, ou: &Emplacements) -> Vec<String> {
    let etat = Etat::charger(ou);
    let (Some(installe), Some(cible)) = (etat.composants.get(nom), cible(nom, ou)) else {
        return vec![format!("{nom} n'est pas installé")];
    };
    let dossier = match cible {
        Cible::Dossier { dossier, .. } | Cible::Fichiers { dossier } => dossier,
    };
    installe
        .fichiers
        .iter()
        .filter(|(chemin, taille)| {
            fs::metadata(dossier.join(chemin))
                .map(|m| m.len() != **taille)
                .unwrap_or(true)
        })
        .map(|(chemin, _)| chemin.clone())
        .collect()
}

fn decompresser(archive: &[u8], url: &str, dossier: &Path) -> Result<(), String> {
    let est_7z = archive.starts_with(b"7z\xBC\xAF\x27\x1C") || url.ends_with(".7z");
    if est_7z {
        let racine = dossier.to_path_buf();
        sevenz_rust2::decompress_with_extract_fn(
            Cursor::new(archive),
            dossier,
            |entree, lecteur, chemin| {
                if !chemin.starts_with(&racine) {
                    return Err(sevenz_rust2::Error::Other("chemin hors du dossier".into()));
                }
                sevenz_rust2::default_entry_extract_fn(entree, lecteur, chemin)
            },
        )
        .map_err(|e| format!("Archive 7z illisible : {e}"))
    } else {
        let mut zip = zip::ZipArchive::new(Cursor::new(archive))
            .map_err(|e| format!("Archive zip illisible : {e}"))?;
        for i in 0..zip.len() {
            let mut entree = zip.by_index(i).map_err(|e| e.to_string())?;
            let relatif = PathBuf::from(entree.name());
            if relatif
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err(format!("Chemin refusé dans l'archive : {}", entree.name()));
            }
            let vers = dossier.join(&relatif);
            if entree.is_dir() {
                fs::create_dir_all(&vers).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(p) = vers.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut contenu = Vec::new();
            entree
                .read_to_end(&mut contenu)
                .map_err(|e| e.to_string())?;
            fs::write(&vers, contenu).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

/// Beaucoup d'archives contiennent un seul dossier (« RetroArch-Win64/ ») :
/// on descend dedans.
fn dossier_utile(dossier: &Path) -> PathBuf {
    let entrees: Vec<PathBuf> = fs::read_dir(dossier)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    if entrees.len() == 1 && entrees[0].is_dir() {
        entrees[0].clone()
    } else {
        dossier.to_path_buf()
    }
}

fn copier_dossier(depuis: &Path, vers: &Path) -> Result<(), String> {
    fs::create_dir_all(vers).map_err(|e| e.to_string())?;
    for entree in fs::read_dir(depuis).map_err(|e| e.to_string())?.flatten() {
        let chemin = entree.path();
        let cible = vers.join(entree.file_name());
        if chemin.is_dir() {
            copier_dossier(&chemin, &cible)?;
        } else {
            fs::copy(&chemin, &cible)
                .map_err(|e| format!("Impossible d'écrire {} : {e}", cible.display()))?;
        }
    }
    Ok(())
}

/// Tous les fichiers d'un dossier, relatifs, avec leur taille (hors `garder`).
pub fn inventaire(dossier: &Path, garder: &[&str]) -> BTreeMap<String, u64> {
    fn parcourir(racine: &Path, d: &Path, garder: &[&str], sortie: &mut BTreeMap<String, u64>) {
        let Ok(it) = fs::read_dir(d) else { return };
        for e in it.flatten() {
            let p = e.path();
            let relatif = p
                .strip_prefix(racine)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            if d == racine && garder.contains(&relatif.as_str()) {
                continue;
            }
            if p.is_dir() {
                parcourir(racine, &p, garder, sortie);
            } else if let Ok(m) = e.metadata() {
                sortie.insert(relatif, m.len());
            }
        }
    }
    let mut sortie = BTreeMap::new();
    parcourir(dossier, dossier, garder, &mut sortie);
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_avec(fichiers: &[(&str, &[u8])]) -> Vec<u8> {
        let mut tampon = Cursor::new(Vec::new());
        {
            let mut z = zip::ZipWriter::new(&mut tampon);
            for (nom, contenu) in fichiers {
                z.start_file(*nom, zip::write::SimpleFileOptions::default())
                    .unwrap();
                z.write_all(contenu).unwrap();
            }
            z.finish().unwrap();
        }
        tampon.into_inner()
    }

    fn composant(version: &str, archive: &[u8], url: &str) -> Composant {
        Composant {
            version: version.into(),
            url: url.into(),
            sha256: sha256(archive),
        }
    }

    #[test]
    fn installe_met_a_jour_et_revient() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        let v1 = zip_avec(&[
            ("RetroArch-Win64/retroarch.exe", b"v1"),
            ("RetroArch-Win64/info.txt", b"1"),
        ]);
        installer("retroarch", &composant("1.0", &v1, "x.zip"), &v1, &ou).unwrap();
        assert_eq!(
            fs::read(ou.emulateurs().join("retroarch/retroarch.exe")).unwrap(),
            b"v1"
        );

        // Un core installé entre-temps doit survivre à la mise à jour.
        let core = zip_avec(&[("snes9x_libretro.dll", b"core")]);
        installer(
            "core:snes9x",
            &composant("1.63", &core, "c.zip"),
            &core,
            &ou,
        )
        .unwrap();
        assert!(ou.cores.join("snes9x_libretro.dll").is_file());

        let v2 = zip_avec(&[("RetroArch-Win64/retroarch.exe", b"v2")]);
        let i = installer("retroarch", &composant("2.0", &v2, "x.zip"), &v2, &ou).unwrap();
        assert_eq!(i.precedente.as_deref(), Some("1.0"));
        assert!(ou.cores.join("snes9x_libretro.dll").is_file());
        assert!(verifier_integrite("retroarch", &ou).is_empty());

        assert_eq!(retour_arriere("retroarch", &ou).unwrap(), "1.0");
        assert_eq!(
            fs::read(ou.emulateurs().join("retroarch/retroarch.exe")).unwrap(),
            b"v1"
        );
        assert!(ou.cores.join("snes9x_libretro.dll").is_file());
        assert_eq!(
            Etat::charger(&ou).versions().get("retroarch").unwrap(),
            "1.0"
        );
    }

    #[test]
    fn repere_ce_qui_est_a_reparer() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        let core = zip_avec(&[("mesen_libretro.dll", b"core")]);
        installer("core:mesen", &composant("1", &core, "c.zip"), &core, &ou).unwrap();
        assert!(a_reparer(&ou).is_empty());
        fs::remove_file(ou.cores.join("mesen_libretro.dll")).unwrap();
        assert_eq!(a_reparer(&ou), vec!["core:mesen".to_string()]);
        // La réinstaller répare.
        installer("core:mesen", &composant("1", &core, "c.zip"), &core, &ou).unwrap();
        assert!(a_reparer(&ou).is_empty());
        // Aucun dossier temporaire ne traîne.
        let restes: Vec<_> = fs::read_dir(ou.emulateurs())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with(".installation"))
            .collect();
        assert!(restes.is_empty());
    }

    #[test]
    fn refuse_empreinte_et_chemins() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        let a = zip_avec(&[("a.exe", b"x")]);
        let mut mauvais = composant("1", &a, "a.zip");
        mauvais.sha256 = "00".into();
        assert!(installer("dolphin", &mauvais, &a, &ou)
            .unwrap_err()
            .contains("empreinte"));
        let piege = zip_avec(&[("../../evil.exe", b"x")]);
        assert!(installer("dolphin", &composant("1", &piege, "p.zip"), &piege, &ou).is_err());
    }

    #[test]
    fn integrite_signale_un_fichier_abime() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        let a = zip_avec(&[("Dolphin-x64/Dolphin.exe", b"abc")]);
        installer("dolphin", &composant("2512", &a, "d.zip"), &a, &ou).unwrap();
        fs::write(
            ou.emulateurs().join("dolphin/Dolphin.exe"),
            "abimé!".as_bytes(),
        )
        .unwrap();
        assert_eq!(
            verifier_integrite("dolphin", &ou),
            vec!["Dolphin.exe".to_string()]
        );
    }

    #[test]
    fn archive_7z() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src/Dolphin-x64");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("Dolphin.exe"), b"7z!").unwrap();
        let archive = dir.path().join("d.7z");
        sevenz_rust2::compress_to_path(dir.path().join("src"), &archive).unwrap();
        let octets = fs::read(&archive).unwrap();
        let ou = Emplacements::geres(&dir.path().join("donnees"));
        installer("dolphin", &composant("1", &octets, "d.7z"), &octets, &ou).unwrap();
        assert_eq!(
            fs::read(ou.emulateurs().join("dolphin/Dolphin.exe")).unwrap(),
            b"7z!"
        );
    }
}
