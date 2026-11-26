//! Où vit tout ce que SQUAD//RETRO installe et écrit.
//!
//! ```text
//! <données>/
//!   emulateurs/retroarch/   retroarch.exe, cores/   (installés par SQUAD)
//!   emulateurs/dolphin/     Dolphin.exe, DolphinTool.exe
//!   emulateurs/outils/      chdman.exe
//!   configs/                une config RetroArch par console
//!   dolphin/                le dossier utilisateur de Dolphin (option -u)
//!   bios/                   les BIOS de la personne (PS1)
//!   sauvegardes/<console>/  sauvegardes en jeu et instantanées
//! ```
//!
//! Par défaut tout est géré par SQUAD (`geres`). La vague 1 permettait de
//! pointer un RetroArch déjà installé : c'est toujours possible
//! (`avec_retroarch`), utile pour tester.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Emplacements {
    /// L'exécutable RetroArch.
    pub retroarch: PathBuf,
    /// Le dossier des cores libretro.
    pub cores: PathBuf,
    /// L'exécutable Dolphin.
    pub dolphin: PathBuf,
    /// Le dossier des données de SQUAD//RETRO.
    pub donnees: PathBuf,
}

fn exe(nom: &str) -> String {
    if cfg!(windows) {
        format!("{nom}.exe")
    } else {
        nom.to_string()
    }
}

impl Emplacements {
    /// Tout installé et géré par SQUAD, dans `donnees`.
    pub fn geres(donnees: &Path) -> Emplacements {
        let emulateurs = donnees.join("emulateurs");
        Emplacements {
            retroarch: emulateurs.join("retroarch").join(exe("retroarch")),
            cores: emulateurs.join("retroarch").join("cores"),
            dolphin: emulateurs.join("dolphin").join(if cfg!(windows) {
                "Dolphin.exe"
            } else {
                "dolphin-emu"
            }),
            donnees: donnees.to_path_buf(),
        }
    }

    /// Un RetroArch déjà installé par la personne ; le reste reste géré.
    pub fn avec_retroarch(retroarch: &Path, donnees: &Path) -> Emplacements {
        let dossier = retroarch.parent().unwrap_or(Path::new("."));
        Emplacements {
            retroarch: retroarch.to_path_buf(),
            cores: dossier.join("cores"),
            ..Emplacements::geres(donnees)
        }
    }

    /// Ancien nom (vague 1).
    pub fn depuis_retroarch(retroarch: &Path, donnees: &Path) -> Emplacements {
        Emplacements::avec_retroarch(retroarch, donnees)
    }

    pub fn emulateurs(&self) -> PathBuf {
        self.donnees.join("emulateurs")
    }
    pub fn configs(&self) -> PathBuf {
        self.donnees.join("configs")
    }
    pub fn dossier_dolphin(&self) -> PathBuf {
        self.donnees.join("dolphin")
    }
    pub fn bios(&self) -> PathBuf {
        self.donnees.join("bios")
    }
    pub fn sauvegardes(&self) -> PathBuf {
        self.donnees.join("sauvegardes")
    }
    /// L'outil de conversion de Dolphin (vers .rvz).
    pub fn dolphin_tool(&self) -> PathBuf {
        let dossier = self.dolphin.parent().unwrap_or(Path::new("."));
        dossier.join(exe("DolphinTool"))
    }
    /// L'outil de conversion des disques PS1 (vers .chd).
    pub fn chdman(&self) -> PathBuf {
        self.emulateurs().join("outils").join(exe("chdman"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemins_geres() {
        let ou = Emplacements::geres(Path::new("/d"));
        assert!(ou.retroarch.starts_with("/d/emulateurs/retroarch"));
        assert_eq!(ou.cores, PathBuf::from("/d/emulateurs/retroarch/cores"));
        assert_eq!(ou.bios(), PathBuf::from("/d/bios"));
        let perso = Emplacements::avec_retroarch(Path::new("/RA/retroarch.exe"), Path::new("/d"));
        assert_eq!(perso.cores, PathBuf::from("/RA/cores"));
        assert_eq!(perso.dolphin, ou.dolphin);
    }
}
