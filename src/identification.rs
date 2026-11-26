//! L'empreinte d'un fichier de jeu : CRC32 et SHA-1.
//!
//! Les bases No-Intro calculent l'empreinte des jeux NES sans les 16 octets
//! de l'en-tête iNES : on les retire aussi, sinon aucun jeu NES n'est reconnu.
//! Les images disque (.rvz, .chd…) sont compressées : leur empreinte ne
//! correspond pas à Redump, elles sont alors reconnues par leur nom de
//! fichier (voir `bibliotheque`).

use crate::consoles::Console;
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Les empreintes d'un fichier, en hexadécimal minuscule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empreintes {
    pub crc32: String,
    pub sha1: String,
}

/// Au-delà de cette taille, on ne calcule pas l'empreinte (images disque).
pub const TAILLE_MAX_EMPREINTE: u64 = 128 * 1024 * 1024;

/// Vrai si on sait calculer une empreinte comparable aux bases pour cette console.
pub fn empreinte_utile(console: Console) -> bool {
    matches!(console, Console::Nes | Console::Snes | Console::N64)
}

/// Calcule les empreintes d'un fichier.
pub fn empreintes(chemin: &Path, console: Console) -> io::Result<Empreintes> {
    let mut donnees = Vec::new();
    File::open(chemin)?.read_to_end(&mut donnees)?;
    Ok(empreintes_octets(&donnees, console))
}

/// Calcule les empreintes d'un contenu déjà lu.
pub fn empreintes_octets(donnees: &[u8], console: Console) -> Empreintes {
    let utiles = match console {
        Console::Nes if a_un_entete_ines(donnees) => &donnees[16..],
        Console::Snes if donnees.len() % 1024 == 512 => &donnees[512..], // en-tête de copieur
        _ => donnees,
    };
    let n64 = if console == Console::N64 {
        Some(vers_z64(utiles))
    } else {
        None
    };
    let utiles = n64.as_deref().unwrap_or(utiles);

    let mut crc = crc32fast::Hasher::new();
    crc.update(utiles);
    let mut sha = Sha1::new();
    sha.update(utiles);
    Empreintes {
        crc32: format!("{:08x}", crc.finalize()),
        sha1: hex(&sha.finalize()),
    }
}

fn a_un_entete_ines(donnees: &[u8]) -> bool {
    donnees.len() > 16 && &donnees[..4] == b"NES\x1a"
}

/// No-Intro référence les jeux N64 en ordre « .z64 » (gros-boutiste).
/// Les .v64 (octets inversés deux à deux) et .n64 (petit-boutiste) sont remis
/// dans cet ordre avant le calcul.
fn vers_z64(donnees: &[u8]) -> Vec<u8> {
    let mut sortie = donnees.to_vec();
    if donnees.len() < 4 {
        return sortie;
    }
    match &donnees[..4] {
        [0x37, 0x80, 0x40, 0x12] => {
            for i in (0..sortie.len() / 2 * 2).step_by(2) {
                sortie.swap(i, i + 1);
            }
        }
        [0x40, 0x12, 0x37, 0x80] => {
            for i in (0..sortie.len() / 4 * 4).step_by(4) {
                sortie[i..i + 4].reverse();
            }
        }
        _ => {}
    }
    sortie
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retire_l_entete_ines() {
        let mut avec = b"NES\x1a".to_vec();
        avec.extend_from_slice(&[0u8; 12]);
        avec.extend_from_slice(b"contenu");
        let a = empreintes_octets(&avec, Console::Nes);
        let b = empreintes_octets(b"contenu", Console::Nes);
        assert_eq!(a, b);
    }

    #[test]
    fn empreinte_connue() {
        // CRC32 et SHA-1 de « abc ».
        let e = empreintes_octets(b"abc", Console::Snes);
        assert_eq!(e.crc32, "352441c2");
        assert_eq!(e.sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn n64_remis_en_ordre() {
        let z64 = [0x80, 0x37, 0x12, 0x40, 1, 2, 3, 4];
        let v64 = [0x37, 0x80, 0x40, 0x12, 2, 1, 4, 3];
        let n64 = [0x40, 0x12, 0x37, 0x80, 4, 3, 2, 1];
        let a = empreintes_octets(&z64, Console::N64);
        assert_eq!(a, empreintes_octets(&v64, Console::N64));
        assert_eq!(a, empreintes_octets(&n64, Console::N64));
    }
}
