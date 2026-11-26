//! Les consoles : celles de la V1 (NES, SNES, N64, PS1, GameCube, Wii) et
//! celles ajoutées ensuite (Game Boy, GBA, DS, Master System, Mega Drive,
//! PC Engine, Saturn, Dreamcast, PSP, PS2, arcade).
//!
//! Chaque console sait quelles extensions de fichier elle accepte et quel
//! moteur la fait tourner. Les choix de cores sont ceux du document projet,
//! à confirmer pendant les tests de performance.

use serde::{Deserialize, Serialize};

/// Une console prise en charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Console {
    Nes,
    Snes,
    N64,
    Ps1,
    Gamecube,
    Wii,
    Gb,
    Gba,
    Nds,
    Mastersystem,
    Megadrive,
    Pcengine,
    Saturn,
    Dreamcast,
    Psp,
    Ps2,
    Arcade,
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
    pub const TOUTES: [Console; 17] = [
        Console::Nes,
        Console::Snes,
        Console::N64,
        Console::Ps1,
        Console::Gamecube,
        Console::Wii,
        Console::Gb,
        Console::Gba,
        Console::Nds,
        Console::Mastersystem,
        Console::Megadrive,
        Console::Pcengine,
        Console::Saturn,
        Console::Dreamcast,
        Console::Psp,
        Console::Ps2,
        Console::Arcade,
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
            Console::Gb => "Game Boy (Color)",
            Console::Gba => "Game Boy Advance",
            Console::Nds => "Nintendo DS",
            Console::Mastersystem => "Master System",
            Console::Megadrive => "Mega Drive",
            Console::Pcengine => "PC Engine",
            Console::Saturn => "Saturn",
            Console::Dreamcast => "Dreamcast",
            Console::Psp => "PSP",
            Console::Ps2 => "PlayStation 2",
            Console::Arcade => "Arcade",
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
            Console::Gb => "gb",
            Console::Gba => "gba",
            Console::Nds => "nds",
            Console::Mastersystem => "mastersystem",
            Console::Megadrive => "megadrive",
            Console::Pcengine => "pcengine",
            Console::Saturn => "saturn",
            Console::Dreamcast => "dreamcast",
            Console::Psp => "psp",
            Console::Ps2 => "ps2",
            Console::Arcade => "arcade",
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
            Console::Gb => &["gb", "gbc"],
            Console::Gba => &["gba"],
            Console::Nds => &["nds"],
            Console::Mastersystem => &["sms"],
            Console::Megadrive => &["md", "gen", "smd", "bin"],
            Console::Pcengine => &["pce"],
            Console::Saturn => &["chd", "cue"],
            Console::Dreamcast => &["chd", "gdi", "cdi"],
            Console::Psp => &["iso", "cso", "chd"],
            Console::Ps2 => &["iso", "chd"],
            Console::Arcade => &["zip"],
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
            Console::Gb => Moteur::Retroarch { core: "gambatte" },
            Console::Gba => Moteur::Retroarch { core: "mgba" },
            Console::Nds => Moteur::Retroarch { core: "melondsds" },
            Console::Mastersystem | Console::Megadrive => Moteur::Retroarch {
                core: "genesis_plus_gx",
            },
            Console::Pcengine => Moteur::Retroarch {
                core: "mednafen_pce_fast",
            },
            Console::Saturn => Moteur::Retroarch {
                core: "mednafen_saturn",
            },
            Console::Dreamcast => Moteur::Retroarch { core: "flycast" },
            Console::Psp => Moteur::Retroarch { core: "ppsspp" },
            Console::Ps2 => Moteur::Retroarch { core: "pcsx2" },
            Console::Arcade => Moteur::Retroarch { core: "fbneo" },
        }
    }

    /// Vrai si la console a besoin d'un BIOS issu de sa propre console.
    pub fn demande_un_bios(self) -> bool {
        !self.bios().is_empty()
    }

    /// Les BIOS acceptés (un seul suffit), dans le dossier BIOS, avec leur
    /// taille exacte. Vide = pas de BIOS demandé.
    pub fn bios(self) -> &'static [(&'static str, u64)] {
        match self {
            Console::Ps1 => &[
                ("scph5502.bin", 524_288),
                ("scph5501.bin", 524_288),
                ("scph5500.bin", 524_288),
                ("scph1001.bin", 524_288),
                ("scph7502.bin", 524_288),
            ],
            Console::Saturn => &[("sega_101.bin", 524_288), ("mpr-17933.bin", 524_288)],
            // PCSX2 cherche ses BIOS dans bios/pcsx2/bios.
            Console::Ps2 => &[
                ("pcsx2/bios/SCPH-70004_BIOS_V12_EUR_200.BIN", 4_194_304),
                ("pcsx2/bios/SCPH-70012_BIOS_V12_USA_200.BIN", 4_194_304),
                ("pcsx2/bios/SCPH-39004_BIOS_V7_EUR_160.BIN", 4_194_304),
                ("pcsx2/bios/SCPH-39001_BIOS_V7_USA_160.BIN", 4_194_304),
            ],
            _ => &[],
        }
    }

    /// Le message qui dit quel BIOS copier.
    pub fn aide_bios(self) -> &'static str {
        match self {
            Console::Ps1 => "La PlayStation a besoin de son BIOS : copie celui de ta console (scph5502.bin pour une console européenne) dans le dossier BIOS.",
            Console::Saturn => "La Saturn a besoin de son BIOS : copie celui de ta console (sega_101.bin ou mpr-17933.bin) dans le dossier BIOS.",
            Console::Ps2 => "La PlayStation 2 a besoin de son BIOS : copie celui de ta console dans le dossier BIOS, sous-dossier pcsx2/bios.",
            _ => "",
        }
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
        assert_eq!(Console::depuis_dossier("dreamcast"), Some(Console::Dreamcast));
        assert_eq!(Console::depuis_dossier("amiga"), None);
        assert!(Console::Gba.accepte(Path::new("Zelda.gba")));
    }

    #[test]
    fn moteurs() {
        assert_eq!(Console::Wii.moteur(), Moteur::Dolphin);
        assert_eq!(Console::Nes.moteur(), Moteur::Retroarch { core: "mesen" });
        assert!(Console::Ps1.demande_un_bios() && Console::Ps2.demande_un_bios());
        assert!(!Console::Gba.demande_un_bios());
        // Deux consoles ne partagent pas le même dossier.
        let mut d: Vec<&str> = Console::TOUTES.iter().map(|c| c.dossier()).collect();
        d.sort();
        d.dedup();
        assert_eq!(d.len(), Console::TOUTES.len());
    }
}
