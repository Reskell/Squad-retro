//! Le diagnostic en un clic : un rapport texte, à coller sur Discord quand
//! quelque chose ne marche pas. Il ne contient ni nom de jeu ni chemin
//! personnel au-delà du dossier SQUAD//RETRO.

use crate::consoles::Console;
use crate::emplacements::Emplacements;
use crate::installation::{self, Etat};
use crate::lancement;

pub fn rapport(ou: &Emplacements, version_app: &str, erreurs_recentes: &[String]) -> String {
    let mut r = vec![
        format!(
            "SQUAD//RETRO {version_app} — moteur {}",
            env!("CARGO_PKG_VERSION")
        ),
        format!(
            "Système : {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
        String::new(),
        "Émulateurs installés :".into(),
    ];
    let etat = Etat::charger(ou);
    if etat.composants.is_empty() {
        r.push("  (aucun)".into());
    }
    for (nom, i) in &etat.composants {
        let abimes = installation::verifier_integrite(nom, ou);
        r.push(format!(
            "  {nom} {} — {}",
            i.version,
            if abimes.is_empty() {
                "intact".to_string()
            } else {
                format!("{} fichier(s) manquant(s) ou abîmé(s)", abimes.len())
            }
        ));
    }
    r.push(String::new());
    r.push("Consoles :".into());
    for c in Console::TOUTES {
        let m = lancement::manques(c, ou);
        r.push(format!(
            "  {} — {}",
            c.nom(),
            if m.is_empty() {
                "prête".to_string()
            } else {
                m.iter()
                    .map(|x| x.message(c))
                    .collect::<Vec<_>>()
                    .join(" / ")
            }
        ));
    }
    if !erreurs_recentes.is_empty() {
        r.push(String::new());
        r.push("Dernières erreurs :".into());
        r.extend(
            erreurs_recentes
                .iter()
                .rev()
                .take(10)
                .map(|e| format!("  {e}")),
        );
    }
    r.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rapport_lisible() {
        let dir = tempfile::tempdir().unwrap();
        let ou = Emplacements::geres(dir.path());
        let t = rapport(&ou, "0.2.0", &["Impossible de lancer X".into()]);
        assert!(t.contains("(aucun)"));
        assert!(t.contains("GameCube —"));
        assert!(t.contains("Impossible de lancer X"));
    }
}
