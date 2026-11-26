//! Les variantes de jeux : traductions, versions coop, hacks (patchs IPS et
//! BPS), et leurs mises à jour automatiques.
//!
//! Un mod est un patch appliqué à la copie de la personne : le launcher ne
//! distribue que le patch, jamais le jeu. La variante apparaît dans la même
//! fiche que le jeu de base (« Zelda » et « Zelda [Traduction FR] »), avec son
//! propre bouton Jouer.
//!
//! Au lancement d'une variante, l'app compare sa version au catalogue des
//! mods et installe la dernière avant de jouer. La version précédente est
//! gardée : si la nouvelle plante, on revient en arrière en un clic.

use crate::consoles::Console;
use crate::installation::sha256;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Ips,
    Bps,
}

/// Un mod du catalogue (`mods.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModRetro {
    pub id: String,
    pub nom: String,
    pub console: Console,
    /// L'empreinte SHA-1 du fichier de base attendu.
    pub base_sha1: String,
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub format: Format,
    #[serde(default)]
    pub auteur: String,
    /// Les notes de version, affichées avant la mise à jour.
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CatalogueMods {
    pub revision: u32,
    pub mods: Vec<ModRetro>,
}

impl CatalogueMods {
    pub fn lire(json: &str) -> Result<CatalogueMods, String> {
        serde_json::from_str(json).map_err(|e| format!("mods.json illisible : {e}"))
    }
    /// Les mods qui s'appliquent à ce fichier.
    pub fn pour(&self, sha1: &str) -> Vec<&ModRetro> {
        self.mods
            .iter()
            .filter(|m| m.base_sha1.eq_ignore_ascii_case(sha1))
            .collect()
    }
}

/// Un mod installé chez la personne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModInstalle {
    pub version: String,
    pub fichier: PathBuf,
    #[serde(default)]
    pub precedente: Option<(String, PathBuf)>,
}

/// Les mods installés : `variantes/mods.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Installes(pub BTreeMap<String, ModInstalle>);

impl Installes {
    pub fn charger(fichier: &Path) -> Installes {
        fs::read_to_string(fichier)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }
    pub fn ecrire(&self, fichier: &Path) -> std::io::Result<()> {
        if let Some(d) = fichier.parent() {
            fs::create_dir_all(d)?;
        }
        fs::write(
            fichier,
            serde_json::to_string_pretty(self).unwrap_or_default(),
        )
    }
    /// Les mods dont une version plus récente est au catalogue.
    pub fn a_mettre_a_jour<'a>(&self, catalogue: &'a CatalogueMods) -> Vec<&'a ModRetro> {
        catalogue
            .mods
            .iter()
            .filter(|m| {
                self.0
                    .get(&m.id)
                    .map(|i| i.version != m.version)
                    .unwrap_or(false)
            })
            .collect()
    }
}

/// Le nom du fichier de la variante.
pub fn nom_variante(base: &Path, m: &ModRetro) -> String {
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("jeu");
    let titre = crate::region::titre(stem);
    let ext = base.extension().and_then(|e| e.to_str()).unwrap_or("bin");
    let propre: String = m
        .nom
        .chars()
        .filter(|c| !"\\/:*?\"<>|[]()".contains(*c))
        .collect();
    format!(
        "{titre} [{propre} v{}].{ext}",
        m.version.replace(['/', '\\'], "-")
    )
}

/// Applique un mod à la copie de base et écrit la variante dans `dossier`.
pub fn installer(
    m: &ModRetro,
    base: &Path,
    patch: &[u8],
    dossier: &Path,
    installes: &mut Installes,
) -> Result<PathBuf, String> {
    if sha256(patch) != m.sha256.to_ascii_lowercase() {
        return Err(format!(
            "Le patch de {} est abîmé ou a été modifié : rien n'a été installé.",
            m.nom
        ));
    }
    let octets =
        fs::read(base).map_err(|e| format!("Impossible de lire {} : {e}", base.display()))?;
    let resultat = appliquer(m.format, &octets, patch)?;
    fs::create_dir_all(dossier).map_err(|e| e.to_string())?;
    let fichier = dossier.join(nom_variante(base, m));
    fs::write(&fichier, resultat).map_err(|e| e.to_string())?;
    // Garder la version précédente : une seule en réserve.
    let precedente = installes
        .0
        .get(&m.id)
        .map(|i| (i.version.clone(), i.fichier.clone()));
    if let Some((_, ancien)) = installes.0.get(&m.id).and_then(|i| i.precedente.clone()) {
        if ancien != fichier {
            let _ = fs::remove_file(ancien);
        }
    }
    installes.0.insert(
        m.id.clone(),
        ModInstalle {
            version: m.version.clone(),
            fichier: fichier.clone(),
            precedente,
        },
    );
    Ok(fichier)
}

/// Remet la version précédente d'un mod.
pub fn retour_arriere(id: &str, installes: &mut Installes) -> Result<String, String> {
    let i = installes
        .0
        .get(id)
        .cloned()
        .ok_or("Ce mod n'est pas installé.")?;
    let (version, fichier) = i.precedente.ok_or("Pas de version précédente gardée.")?;
    if !fichier.is_file() {
        return Err("La version précédente a été supprimée.".into());
    }
    let _ = fs::remove_file(&i.fichier);
    installes.0.insert(
        id.into(),
        ModInstalle {
            version: version.clone(),
            fichier,
            precedente: None,
        },
    );
    Ok(version)
}

/// Applique un patch.
pub fn appliquer(format: Format, base: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    match format {
        Format::Ips => ips(base, patch),
        Format::Bps => bps(base, patch),
    }
}

fn ips(base: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    let faux = || "Patch IPS invalide.".to_string();
    if !patch.starts_with(b"PATCH") {
        return Err(faux());
    }
    let mut sortie = base.to_vec();
    let mut i = 5;
    loop {
        if patch.get(i..i + 3) == Some(b"EOF") {
            i += 3;
            break;
        }
        let o = patch.get(i..i + 3).ok_or_else(faux)?;
        let offset = (o[0] as usize) << 16 | (o[1] as usize) << 8 | o[2] as usize;
        let t = patch.get(i + 3..i + 5).ok_or_else(faux)?;
        let taille = (t[0] as usize) << 8 | t[1] as usize;
        i += 5;
        let (donnees, avance): (Vec<u8>, usize) = if taille == 0 {
            let r = patch.get(i..i + 3).ok_or_else(faux)?;
            let n = (r[0] as usize) << 8 | r[1] as usize;
            (vec![r[2]; n], 3)
        } else {
            (patch.get(i..i + taille).ok_or_else(faux)?.to_vec(), taille)
        };
        i += avance;
        if sortie.len() < offset + donnees.len() {
            sortie.resize(offset + donnees.len(), 0);
        }
        sortie[offset..offset + donnees.len()].copy_from_slice(&donnees);
    }
    // Extension : une taille finale sur 3 octets après EOF.
    if let Some(t) = patch.get(i..i + 3) {
        sortie.truncate((t[0] as usize) << 16 | (t[1] as usize) << 8 | t[2] as usize);
    }
    Ok(sortie)
}

fn lire_nombre(patch: &[u8], i: &mut usize) -> Result<u64, String> {
    let mut data: u64 = 0;
    let mut decalage: u64 = 1;
    loop {
        let x = *patch.get(*i).ok_or("Patch BPS tronqué.")? as u64;
        *i += 1;
        data = data
            .checked_add(
                (x & 0x7f)
                    .checked_mul(decalage)
                    .ok_or("Patch BPS invalide.")?,
            )
            .ok_or("Patch BPS invalide.")?;
        if x & 0x80 != 0 {
            break;
        }
        decalage = decalage.checked_shl(7).ok_or("Patch BPS invalide.")?;
        data = data.checked_add(decalage).ok_or("Patch BPS invalide.")?;
    }
    Ok(data)
}

fn crc32(octets: &[u8]) -> u32 {
    let mut h = crc32fast::Hasher::new();
    h.update(octets);
    h.finalize()
}

fn bps(base: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    if !patch.starts_with(b"BPS1") || patch.len() < 4 + 12 {
        return Err("Patch BPS invalide.".into());
    }
    let fin = patch.len() - 12;
    let lire_u32 =
        |a: usize| u32::from_le_bytes([patch[a], patch[a + 1], patch[a + 2], patch[a + 3]]);
    if crc32(&patch[..patch.len() - 4]) != lire_u32(patch.len() - 4) {
        return Err("Patch BPS abîmé.".into());
    }
    if crc32(base) != lire_u32(fin) {
        return Err(
            "Ce patch ne correspond pas à ta copie du jeu (autre version ou autre région).".into(),
        );
    }
    let mut i = 4;
    let taille_source = lire_nombre(patch, &mut i)? as usize;
    let taille_cible = lire_nombre(patch, &mut i)? as usize;
    let meta = lire_nombre(patch, &mut i)? as usize;
    i += meta;
    if taille_source != base.len() || taille_cible > 512 * 1024 * 1024 {
        return Err("Ce patch ne correspond pas à ta copie du jeu.".into());
    }
    let mut sortie: Vec<u8> = Vec::with_capacity(taille_cible);
    let (mut source_rel, mut cible_rel): (i64, i64) = (0, 0);
    let invalide = || "Patch BPS invalide.".to_string();
    while i < fin {
        let data = lire_nombre(patch, &mut i)?;
        let longueur = ((data >> 2) + 1) as usize;
        match data & 3 {
            0 => {
                let debut = sortie.len();
                sortie.extend_from_slice(base.get(debut..debut + longueur).ok_or_else(invalide)?);
            }
            1 => {
                sortie.extend_from_slice(patch.get(i..i + longueur).ok_or_else(invalide)?);
                i += longueur;
            }
            mode => {
                let d = lire_nombre(patch, &mut i)? as i64;
                let decal = if d & 1 == 1 { -(d >> 1) } else { d >> 1 };
                if mode == 2 {
                    source_rel += decal;
                    let s = usize::try_from(source_rel).map_err(|_| invalide())?;
                    sortie.extend_from_slice(base.get(s..s + longueur).ok_or_else(invalide)?);
                    source_rel += longueur as i64;
                } else {
                    cible_rel += decal;
                    for _ in 0..longueur {
                        let t = usize::try_from(cible_rel).map_err(|_| invalide())?;
                        let o = *sortie.get(t).ok_or_else(invalide)?;
                        sortie.push(o);
                        cible_rel += 1;
                    }
                }
            }
        }
    }
    if sortie.len() != taille_cible || crc32(&sortie) != lire_u32(fin + 4) {
        return Err("Le résultat du patch est faux : ta copie ne correspond pas.".into());
    }
    Ok(sortie)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nombre(mut n: u64, sortie: &mut Vec<u8>) {
        loop {
            let x = (n & 0x7f) as u8;
            n >>= 7;
            if n == 0 {
                sortie.push(0x80 | x);
                return;
            }
            sortie.push(x);
            n -= 1;
        }
    }

    /// Un patch BPS : copie 4 octets de la source, écrit « !! », recopie 2 octets déjà écrits.
    fn patch_bps(base: &[u8], cible: &[u8]) -> Vec<u8> {
        let mut p = b"BPS1".to_vec();
        nombre(base.len() as u64, &mut p);
        nombre(cible.len() as u64, &mut p);
        nombre(0, &mut p);
        nombre(((4 - 1) << 2) as u64, &mut p); // SourceRead 4
        nombre((((2 - 1) << 2) | 1) as u64, &mut p); // TargetRead 2
        p.extend_from_slice(b"!!");
        nombre((((2 - 1) << 2) | 3) as u64, &mut p); // TargetCopy 2 depuis 0
        nombre(0, &mut p);
        p.extend_from_slice(&crc32(base).to_le_bytes());
        p.extend_from_slice(&crc32(cible).to_le_bytes());
        let c = crc32(&p);
        p.extend_from_slice(&c.to_le_bytes());
        p
    }

    #[test]
    fn bps_fonctionne() {
        let base = b"ABCDEFGH";
        let cible = b"ABCD!!AB";
        let p = patch_bps(base, cible);
        assert_eq!(bps(base, &p).unwrap(), cible);
        assert!(bps(b"ZZZZZZZZ", &p)
            .unwrap_err()
            .contains("ne correspond pas"));
    }

    #[test]
    fn ips_fonctionne() {
        let mut p = b"PATCH".to_vec();
        p.extend_from_slice(&[0, 0, 1, 0, 2, b'x', b'y']); // offset 1, 2 octets
        p.extend_from_slice(&[0, 0, 6, 0, 0, 0, 3, b'z']); // RLE à 6, 3 fois z
        p.extend_from_slice(b"EOF");
        assert_eq!(ips(b"abcdef", &p).unwrap(), b"axydefzzz");
    }

    #[test]
    fn installe_met_a_jour_et_revient() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("Jeu (France).sfc");
        fs::write(&base, b"abcdef").unwrap();
        let mut p = b"PATCH".to_vec();
        p.extend_from_slice(&[0, 0, 0, 0, 1, b'Z']);
        p.extend_from_slice(b"EOF");
        let mut m = ModRetro {
            id: "jeu-coop".into(),
            nom: "Coop".into(),
            console: Console::Snes,
            base_sha1: "x".into(),
            version: "1.0".into(),
            url: "u".into(),
            sha256: sha256(&p),
            format: Format::Ips,
            auteur: String::new(),
            notes: String::new(),
        };
        let mut installes = Installes::default();
        let v1 = installer(&m, &base, &p, &dir.path().join("variantes"), &mut installes).unwrap();
        assert_eq!(fs::read(&v1).unwrap(), b"Zbcdef");
        assert!(v1.to_string_lossy().contains("Jeu [Coop v1.0].sfc"));

        let mut catalogue = CatalogueMods {
            revision: 2,
            mods: vec![m.clone()],
        };
        catalogue.mods[0].version = "1.1".into();
        assert_eq!(installes.a_mettre_a_jour(&catalogue).len(), 1);

        m.version = "1.1".into();
        installer(&m, &base, &p, &dir.path().join("variantes"), &mut installes).unwrap();
        assert_eq!(retour_arriere("jeu-coop", &mut installes).unwrap(), "1.0");
        assert!(installes.0["jeu-coop"].fichier.is_file());
    }
}
