//! Lancer un jeu : vérifier que tout est là, écrire la config, préparer la
//! commande.
//!
//! RetroArch est lancé avec le bon core, la config SQUAD et son menu en mode
//! kiosque : le joueur ne voit que le jeu. Dolphin est lancé en mode batch :
//! il démarre le jeu tout de suite et se ferme avec lui.
//!
//! Le moteur prépare la commande sans la lancer : l'app la lance et suit le
//! processus, comme pour les jeux Steam.

use crate::consoles::{Console, Moteur};
use crate::emplacements::Emplacements;
use crate::netplay::Session;
use crate::profil::Profil;
use crate::{dolphin, retroarch};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use crate::retroarch::{chemin_config, extension_core};

/// Une commande prête à lancer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Commande {
    pub programme: PathBuf,
    pub arguments: Vec<String>,
}

impl Commande {
    /// La commande système, lancée depuis le dossier du programme.
    pub fn vers_process(&self) -> Command {
        let mut c = Command::new(&self.programme);
        c.args(&self.arguments);
        if let Some(dossier) = self.programme.parent() {
            if dossier.is_dir() {
                c.current_dir(dossier);
            }
        }
        c
    }
}

/// Ce qui manque pour lancer une console, en clair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "manque", rename_all = "snake_case")]
pub enum Manque {
    Retroarch,
    Core { core: String },
    Dolphin,
    Bios,
}

impl Manque {
    pub fn message(&self, console: Console) -> String {
        match self {
            Manque::Retroarch => "RetroArch n'est pas encore installé : lance l'installation des émulateurs.".into(),
            Manque::Core { core } => format!(
                "Le moteur « {core} » de la {} n'est pas installé : lance l'installation des émulateurs.",
                console.nom()
            ),
            Manque::Dolphin => "Dolphin (GameCube et Wii) n'est pas encore installé : lance l'installation des émulateurs.".into(),
            Manque::Bios => "La PlayStation a besoin de son BIOS : copie celui de ta console (scph5502.bin pour une console européenne) dans le dossier BIOS.".into(),
        }
    }

    /// Vrai si l'installation automatique peut régler le problème.
    pub fn installable(&self) -> bool {
        !matches!(self, Manque::Bios)
    }
}

/// Tout ce qui manque pour lancer une console (vide = prêt).
pub fn manques(console: Console, ou: &Emplacements) -> Vec<Manque> {
    let mut m = Vec::new();
    match console.moteur() {
        Moteur::Retroarch { core } => {
            if !ou.retroarch.is_file() {
                m.push(Manque::Retroarch);
            }
            if !retroarch::fichier_core(ou, core).is_file() {
                m.push(Manque::Core { core: core.into() });
            }
        }
        Moteur::Dolphin => {
            if !ou.dolphin.is_file() {
                m.push(Manque::Dolphin);
            }
        }
    }
    if console.demande_un_bios() && retroarch::bios_ps1(ou).is_none() {
        m.push(Manque::Bios);
    }
    m
}

/// Vérifie que tout est en place, avec un message clair sinon.
pub fn verifier(console: Console, ou: &Emplacements) -> Result<(), String> {
    match manques(console, ou).first() {
        None => Ok(()),
        Some(m) => Err(m.message(console)),
    }
}

/// Écrit la config d'une console avant de la lancer.
pub fn preparer(
    console: Console,
    ou: &Emplacements,
    profil: &Profil,
    session: Option<&Session>,
) -> std::io::Result<()> {
    match console.moteur() {
        Moteur::Retroarch { .. } => retroarch::preparer(console, ou, profil, session),
        Moteur::Dolphin => dolphin::preparer(console, ou, profil, session),
    }
}

/// La commande qui lance un jeu.
pub fn commande(
    console: Console,
    rom: &Path,
    ou: &Emplacements,
    session: Option<&Session>,
) -> Commande {
    match console.moteur() {
        Moteur::Retroarch { .. } => Commande {
            programme: ou.retroarch.clone(),
            arguments: retroarch::arguments(console, rom, ou, session),
        },
        Moteur::Dolphin => Commande {
            programme: ou.dolphin.clone(),
            arguments: dolphin::arguments(rom, ou, session),
        },
    }
}

/// Compresser une image disque sans perte : .iso GameCube/Wii → .rvz
/// (DolphinTool), .cue PS1 → .chd (chdman). L'original n'est jamais supprimé
/// par le moteur : l'app propose de le faire une fois la copie vérifiée.
pub fn conversion(
    console: Console,
    entree: &Path,
    ou: &Emplacements,
) -> Option<(Commande, PathBuf)> {
    let ext = entree.extension()?.to_str()?.to_ascii_lowercase();
    let texte = |p: &Path| p.to_string_lossy().into_owned();
    match (console, ext.as_str()) {
        (Console::Gamecube | Console::Wii, "iso" | "gcm" | "gcz" | "wbfs") => {
            let sortie = entree.with_extension("rvz");
            let arguments = [
                "convert",
                "-i",
                &texte(entree),
                "-o",
                &texte(&sortie),
                "-f",
                "rvz",
                "-b",
                "131072",
                "-c",
                "zstd",
                "-l",
                "5",
            ]
            .map(String::from)
            .to_vec();
            Some((
                Commande {
                    programme: ou.dolphin_tool(),
                    arguments,
                },
                sortie,
            ))
        }
        (Console::Ps1, "cue") => {
            let sortie = entree.with_extension("chd");
            let arguments = ["createcd", "-i", &texte(entree), "-o", &texte(&sortie)]
                .map(String::from)
                .to_vec();
            Some((
                Commande {
                    programme: ou.chdman(),
                    arguments,
                },
                sortie,
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn manques_puis_pret() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        assert_eq!(manques(Console::Snes, &ou).len(), 2);
        assert!(verifier(Console::Snes, &ou)
            .unwrap_err()
            .contains("RetroArch"));
        fs::create_dir_all(&ou.cores).unwrap();
        fs::write(&ou.retroarch, b"").unwrap();
        fs::write(retroarch::fichier_core(&ou, "snes9x"), b"").unwrap();
        assert!(verifier(Console::Snes, &ou).is_ok());
        // La PS1 a aussi besoin de son BIOS, que l'installation ne fournit pas.
        fs::write(retroarch::fichier_core(&ou, "swanstation"), b"").unwrap();
        let m = manques(Console::Ps1, &ou);
        assert_eq!(m, vec![Manque::Bios]);
        assert!(!m[0].installable());
    }

    #[test]
    fn commandes() {
        let ou = Emplacements::geres(Path::new("/d"));
        let c = commande(Console::Gamecube, Path::new("/r/mkdd.rvz"), &ou, None);
        assert_eq!(c.programme, ou.dolphin);
        assert!(c.arguments.contains(&"-b".to_string()));
        let c = commande(Console::Snes, Path::new("/r/a.sfc"), &ou, None);
        assert_eq!(c.arguments[0], "-L");
        let (conv, sortie) = conversion(Console::Gamecube, Path::new("/r/jeu.iso"), &ou).unwrap();
        assert_eq!(sortie, PathBuf::from("/r/jeu.rvz"));
        assert_eq!(conv.arguments[0], "convert");
        assert!(conversion(Console::Snes, Path::new("/r/a.sfc"), &ou).is_none());
    }
}
