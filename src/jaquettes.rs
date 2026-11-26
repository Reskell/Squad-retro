//! Les jaquettes : on ne livre aucune image. Pour chaque jeu de la
//! bibliothèque, on donne l'adresse de sa jaquette dans une base publique :
//! GameTDB (d'après l'identifiant du disque) pour la GameCube et la Wii,
//! les miniatures libretro (d'après le nom No-Intro / Redump) pour le reste.

use crate::Console;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Le nom du système dans les miniatures libretro.
fn systeme_libretro(console: Console) -> &'static str {
    match console {
        Console::Nes => "Nintendo - Nintendo Entertainment System",
        Console::Snes => "Nintendo - Super Nintendo Entertainment System",
        Console::N64 => "Nintendo - Nintendo 64",
        Console::Ps1 => "Sony - PlayStation",
        Console::Gamecube => "Nintendo - GameCube",
        Console::Wii => "Nintendo - Wii",
    }
}

/// Encode un morceau de chemin d'URL (tout sauf lettres, chiffres et -._~).
fn encoder(texte: &str) -> String {
    let mut sortie = String::new();
    for o in texte.bytes() {
        if o.is_ascii_alphanumeric() || b"-._~".contains(&o) {
            sortie.push(o as char);
        } else {
            sortie.push_str(&format!("%{o:02X}"));
        }
    }
    sortie
}

/// Le nom d'un fichier sans ses extensions de jeu (« .nkit.iso », « .rvz »…).
pub fn nom_sans_extension(nom: &str) -> String {
    let mut n = nom.to_string();
    for _ in 0..2 {
        if let Some(i) = n.rfind('.') {
            let ext = n[i + 1..].to_ascii_lowercase();
            if ext.len() <= 4 && ext.chars().all(|c| c.is_ascii_alphanumeric()) && !ext.is_empty() {
                n.truncate(i);
                continue;
            }
        }
        break;
    }
    n
}

/// L'adresse de la miniature libretro d'un jeu, d'après son nom.
pub fn url_libretro(console: Console, nom: &str) -> String {
    let propre: String = nom_sans_extension(nom)
        .chars()
        .map(|c| if "&*/:`<>?\\|\"".contains(c) { '_' } else { c })
        .collect();
    format!(
        "https://thumbnails.libretro.com/{}/Named_Boxarts/{}.png",
        encoder(systeme_libretro(console)),
        encoder(&propre)
    )
}

/// L'identifiant à 6 caractères d'un disque GameCube ou Wii (« GP6E01 »),
/// lu dans son en-tête : ISO/GCM (et NKit), RVZ/WIA, WBFS, CISO.
pub fn id_disque(chemin: &Path) -> Option<String> {
    let mut f = File::open(chemin).ok()?;
    let mut debut = [0u8; 0x60];
    let lu = f.read(&mut debut).ok()?;
    let position = if lu >= 4 && (&debut[..4] == b"RVZ\x01" || &debut[..4] == b"WIA\x01") {
        // En-tête du fichier (0x48 octets), puis 4 champs de 4 octets, puis
        // l'en-tête du disque, stocké tel quel.
        0x58
    } else if lu >= 4 && &debut[..4] == b"WBFS" {
        0x200
    } else if lu >= 4 && &debut[..4] == b"CISO" {
        0x8000
    } else {
        0
    };
    let mut id = [0u8; 6];
    f.seek(SeekFrom::Start(position)).ok()?;
    f.read_exact(&mut id).ok()?;
    id.iter()
        .all(|o| o.is_ascii_uppercase() || o.is_ascii_digit())
        .then(|| String::from_utf8_lossy(&id).into_owned())
}

/// La langue de jaquette GameTDB d'après la 4e lettre de l'identifiant.
fn langue_gametdb(id: &str) -> &'static str {
    match id.as_bytes().get(3) {
        Some(b'E') | Some(b'N') => "US",
        Some(b'J') => "JA",
        Some(b'K') => "KO",
        Some(b'F') => "FR",
        Some(b'D') => "DE",
        _ => "EN",
    }
}

pub fn url_gametdb(id: &str) -> String {
    format!(
        "https://art.gametdb.com/wii/cover/{}/{}.png",
        langue_gametdb(id),
        encoder(id)
    )
}

/// La meilleure adresse de jaquette pour un fichier de jeu.
pub fn jaquette(console: Console, chemin: &Path, nom: &str) -> String {
    if matches!(console, Console::Gamecube | Console::Wii) {
        if let Some(id) = id_disque(chemin) {
            return url_gametdb(&id);
        }
    }
    url_libretro(console, nom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn noms_et_adresses() {
        assert_eq!(nom_sans_extension("Zelda (USA).nkit.iso"), "Zelda (USA)");
        assert_eq!(nom_sans_extension("Mario Party 6 (USA).rvz"), "Mario Party 6 (USA)");
        assert_eq!(nom_sans_extension("Super Mario Bros. (World)"), "Super Mario Bros. (World)");
        assert_eq!(
            url_libretro(Console::Nes, "Super Mario Bros. (World).nes"),
            "https://thumbnails.libretro.com/Nintendo%20-%20Nintendo%20Entertainment%20System/Named_Boxarts/Super%20Mario%20Bros.%20%28World%29.png"
        );
        assert!(url_libretro(Console::Snes, "A: B?.sfc").contains("A_%20B_.png"));
        assert_eq!(url_gametdb("GP6E01"), "https://art.gametdb.com/wii/cover/US/GP6E01.png");
        assert_eq!(url_gametdb("GP6P01"), "https://art.gametdb.com/wii/cover/EN/GP6P01.png");
    }

    #[test]
    fn lit_l_identifiant() {
        let dir = tempfile::tempdir().unwrap();
        let iso = dir.path().join("a.iso");
        let mut d = vec![0u8; 0x100];
        d[..6].copy_from_slice(b"GP6E01");
        File::create(&iso).unwrap().write_all(&d).unwrap();
        assert_eq!(id_disque(&iso).as_deref(), Some("GP6E01"));
        assert_eq!(jaquette(Console::Gamecube, &iso, "x"), "https://art.gametdb.com/wii/cover/US/GP6E01.png");

        let rvz = dir.path().join("b.rvz");
        let mut d = vec![0u8; 0x100];
        d[..4].copy_from_slice(b"RVZ\x01");
        d[0x58..0x5e].copy_from_slice(b"GMPE01");
        File::create(&rvz).unwrap().write_all(&d).unwrap();
        assert_eq!(id_disque(&rvz).as_deref(), Some("GMPE01"));

        let rien = dir.path().join("c.iso");
        File::create(&rien).unwrap().write_all(&[0u8; 16]).unwrap();
        assert_eq!(id_disque(&rien), None);
        assert!(jaquette(Console::Gamecube, &rien, "Jeu (USA).iso").contains("thumbnails.libretro.com"));
    }
}
