//! Export et restauration complets en un clic : profils, manettes,
//! sauvegardes de jeu, BIOS, historique… tout sauf les émulateurs, qui se
//! réinstallent tout seuls. Utile pour changer de PC.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};

/// Les dossiers jamais exportés (réinstallés automatiquement).
pub const EXCLUS: [&str; 1] = ["emulateurs"];

/// Crée l'archive .zip de tout le dossier de données (hors émulateurs).
pub fn exporter(donnees: &Path) -> Result<Vec<u8>, String> {
    let mut tampon = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut tampon);
        let options = zip::write::SimpleFileOptions::default();
        let mut pile = vec![donnees.to_path_buf()];
        while let Some(dossier) = pile.pop() {
            for e in fs::read_dir(&dossier).map_err(|e| e.to_string())?.flatten() {
                let p = e.path();
                let relatif = p
                    .strip_prefix(donnees)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");
                if EXCLUS
                    .iter()
                    .any(|x| relatif == *x || relatif.starts_with(&format!("{x}/")))
                {
                    continue;
                }
                if p.is_dir() {
                    pile.push(p);
                } else {
                    zip.start_file(relatif, options)
                        .map_err(|e| e.to_string())?;
                    zip.write_all(&fs::read(&p).map_err(|e| e.to_string())?)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(tampon.into_inner())
}

/// Restaure une archive exportée dans le dossier de données. Les fichiers
/// existants sont remplacés ; rien n'est écrit hors du dossier.
pub fn restaurer(archive: &[u8], donnees: &Path) -> Result<usize, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))
        .map_err(|_| "Ce fichier n'est pas une sauvegarde SQUAD//RETRO.".to_string())?;
    let mut n = 0;
    for i in 0..zip.len() {
        let mut e = zip.by_index(i).map_err(|e| e.to_string())?;
        let relatif = PathBuf::from(e.name());
        if relatif
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(format!("Chemin refusé dans la sauvegarde : {}", e.name()));
        }
        if e.is_dir() {
            continue;
        }
        let vers = donnees.join(relatif);
        if let Some(p) = vers.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let mut contenu = Vec::new();
        e.read_to_end(&mut contenu).map_err(|e| e.to_string())?;
        fs::write(&vers, contenu).map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_sans_les_emulateurs() {
        let a = tempfile::tempdir().unwrap();
        fs::create_dir_all(a.path().join("sauvegardes/snes")).unwrap();
        fs::write(a.path().join("sauvegardes/snes/zelda.srm"), b"save").unwrap();
        fs::create_dir_all(a.path().join("emulateurs/retroarch")).unwrap();
        fs::write(a.path().join("emulateurs/retroarch/retroarch.exe"), b"gros").unwrap();
        fs::write(a.path().join("profils.json"), b"{}").unwrap();
        let archive = exporter(a.path()).unwrap();

        let b = tempfile::tempdir().unwrap();
        assert_eq!(restaurer(&archive, b.path()).unwrap(), 2);
        assert_eq!(
            fs::read(b.path().join("sauvegardes/snes/zelda.srm")).unwrap(),
            b"save"
        );
        assert!(!b.path().join("emulateurs").exists());
        assert!(restaurer(b"pas un zip", b.path()).is_err());
    }
}
