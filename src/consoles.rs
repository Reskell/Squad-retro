//! Les consoles de la V1 : NES, SNES, N64, PS1, GameCube, Wii.
//!
//! Chaque console sait quelles extensions de fichier elle accepte et quel
//! moteur la fait tourner. Les choix de cores sont ceux du document projet,
//! à confirmer pendant les tests de performance.

use serde::{Deserialize, Serialize};

/// Une console prise en charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Console {
    Nes,
    Snes,
    N64,
    Ps1,
    Gamecube,
    Wii,
}

/// Ce qui fait tourner une console.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Moteur {
    /// RetroArch lancé en arrière-plan, avec ce core libretro.
    Retroarch { core: &'static str },
    /// Dolphin, pour la GameCube et la Wii.
    Dolphin,
}

impl Console {
    /// Toutes les consoles, dans l'ordre d'affichage.
    pub const TOUTES: [Console; 6] = [
        Console::Nes,
        Console::Snes,
        Console::N64,
        Console::Ps1,
        Console::Gamecube,
        Console::Wii,
    ];

    /// Le nom affiché.
    pub fn nom(self) -> &'static str {
        match self {
            Console::Nes => "NES",
            Console::Snes => "Super Nintendo",
            Console::N64 => "Nintendo 64",
            Console::Ps1 => "PlayStation",
            Console::Gamecube => "GameCube",
            Console::Wii => "Wii",
        }
    }

    /// Le nom du sous-dossier de la bibliothèque (`roms/nes`, `roms/wii`…).
    pub fn dossier(self) -> &'static str {
        match self {
            Console::Nes => "nes",
            Console::Snes => "snes",
            Console::N64 => "n64",
            Console::Ps1 => "ps1",
            Console::Gamecube => "gamecube",
            Console::Wii => "wii",
        }
    }

    /// Les extensions acceptées, en minuscules et sans le point.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Console::Nes => &["nes"],
            Console::Snes => &["sfc", "smc"],
            Console::N64 => &["z64", "n64", "v64"],
            Console::Ps1 => &["chd", "cue", "pbp"],
            Console::Gamecube => &["rvz", "iso", "gcm", "gcz"],
            Console::Wii => &["rvz", "wbfs", "iso"],
        }
    }

    /// Le moteur par défaut.
    pub fn moteur(self) -> Moteur {
        match self {
            Console::Nes => Moteur::Retroarch { core: "mesen" },
            Console::Snes => Moteur::Retroarch { core: "snes9x" },
            Console::N64 => Moteur::Retroarch {
                core: "mupen64plus_next",
            },
            Console::Ps1 => Moteur::Retroarch {
                core: "swanstation",
            },
            Console::Gamecube | Console::Wii => Moteur::Dolphin,
        }
    }

    /// Vrai si la console a besoin d'un BIOS issu de sa propre console.
    pub fn demande_un_bios(self) -> bool {
        matches!(self, Console::Ps1)
    }

    /// Retrouve une console d'après le nom de son dossier.
    pub fn depuis_dossier(nom: &str) -> Option<Console> {
        Console::TOUTES
            .into_iter()
            .find(|c| c.dossier().eq_ignore_ascii_case(nom))
    }

    /// Vrai si ce fichier a une extension acceptée par la console.
    pub fn accepte(self, chemin: &std::path::Path) -> bool {
        chemin
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| {
                let e = e.to_ascii_lowercase();
                self.extensions().contains(&e.as_str())
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn extensions_et_dossiers() {
        assert!(Console::Snes.accepte(Path::new("Zelda.SFC")));
        assert!(!Console::Snes.accepte(Path::new("Zelda.nes")));
        assert_eq!(Console::depuis_dossier("GameCube"), Some(Console::Gamecube));
        assert_eq!(Console::depuis_dossier("dreamcast"), None);
    }

    #[test]
    fn moteurs() {
        assert_eq!(Console::Wii.moteur(), Moteur::Dolphin);
        assert_eq!(Console::Nes.moteur(), Moteur::Retroarch { core: "mesen" });
        assert!(Console::Ps1.demande_un_bios());
    }
}
