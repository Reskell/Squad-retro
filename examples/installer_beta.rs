//! Vérification de bout en bout d'un manifeste : télécharge chaque composant,
//! l'installe avec le moteur dans un dossier temporaire, puis vérifie que
//! chaque console est prête (hors BIOS). Lancé par l'action « Préparer la
//! bêta » sur un PC Windows, avant que quiconque n'installe ces versions.
//!
//! `cargo run --example installer_beta -- manifestes/versions-beta.json`

use squad_retro::installation;
use squad_retro::lancement::{manques, Manque};
use squad_retro::versions::{Action, Installees, Validees};
use squad_retro::{Console, Emplacements};
use std::io::Read;

fn main() {
    let chemin = std::env::args().nth(1).unwrap_or_else(|| "manifestes/versions-beta.json".into());
    let validees = Validees::lire(&std::fs::read_to_string(&chemin).expect("manifeste introuvable")).expect("manifeste illisible");
    let dossier = std::env::temp_dir().join("squad-retro-verification");
    let _ = std::fs::remove_dir_all(&dossier);
    let ou = Emplacements::geres(&dossier);
    let mut echecs = 0;
    for action in validees.a_faire(&Installees::new()) {
        let (Action::Installer { nom, composant } | Action::MettreAJour { nom, composant, .. }) = action;
        let mut octets = Vec::new();
        match ureq::get(&composant.url).call() {
            Ok(r) => {
                r.into_reader().read_to_end(&mut octets).expect("lecture");
            }
            Err(e) => {
                println!("ÉCHEC {nom} : téléchargement : {e}");
                echecs += 1;
                continue;
            }
        }
        match installation::installer(&nom, &composant, &octets, &ou) {
            Ok(i) => println!("OK    {nom} {} ({} fichiers)", i.version, i.fichiers.len()),
            Err(e) => {
                println!("ÉCHEC {nom} : {e}");
                echecs += 1;
            }
        }
    }
    for c in Console::TOUTES {
        let m: Vec<Manque> = manques(c, &ou).into_iter().filter(|m| *m != Manque::Bios).collect();
        if m.is_empty() {
            println!("PRÊTE {}", c.nom());
        } else {
            println!("ÉCHEC {} : {}", c.nom(), m.iter().map(|x| x.message(c)).collect::<Vec<_>>().join(" / "));
            echecs += 1;
        }
    }
    if echecs > 0 {
        std::process::exit(1);
    }
}
