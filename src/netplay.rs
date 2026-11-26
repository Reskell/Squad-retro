//! Le netplay et la room (vague 4).
//!
//! Une room, c'est une partie en préparation : un jeu, un hôte, des places.
//! Chacun rejoint, coche « Prêt », et quand tout le monde l'est, un compte à
//! rebours lance le jeu chez tous en même temps. Avant d'entrer, le moteur
//! vérifie que chacun a exactement le même fichier (empreinte) et les mêmes
//! versions d'émulateur : sinon le netplay se désynchronise.
//!
//! Le moteur ne transporte rien sur le réseau : la room est un état que
//! l'app partage comme elle veut (le site SQUAD//LOG pour la bande, un lien
//! d'invitation pour l'app publique). Le moteur sait seulement :
//! - faire évoluer la room (rejoindre, prêt, compte à rebours) ;
//! - fabriquer et relire un lien d'invitation ;
//! - mesurer le ping vers l'hôte ;
//! - donner à RetroArch et Dolphin ce qu'il faut pour héberger ou rejoindre.

use crate::consoles::Console;
use crate::versions::Installees;
use serde::{Deserialize, Serialize};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// Le port de netplay par défaut de RetroArch.
pub const PORT_RETROARCH: u16 = 55435;
/// Le port de netplay par défaut de Dolphin.
pub const PORT_DOLPHIN: u16 = 2626;
/// Durée du compte à rebours avant le lancement, en millisecondes.
pub const COMPTE_A_REBOURS_MS: u64 = 5_000;

/// Héberger ou rejoindre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Role {
    Heberger { port: u16 },
    Rejoindre { adresse: String, port: u16 },
}

/// Ce qu'il faut à l'émulateur pour une partie en ligne.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub role: Role,
    pub pseudo: String,
    /// Session privée : mot de passe demandé pour entrer.
    #[serde(default)]
    pub mot_de_passe: Option<String>,
    /// Serveur relais RetroArch (« madrid », « nyc »…) quand les ports ne
    /// peuvent pas être ouverts. None = connexion directe (avec UPnP).
    #[serde(default)]
    pub relais: Option<String>,
    /// Le port manette choisi dans le salon (1 = joueur 1).
    #[serde(default)]
    pub port_manette: Option<u8>,
}

impl Session {
    /// Les arguments de ligne de commande RetroArch.
    pub fn arguments_retroarch(&self) -> Vec<String> {
        let mut a = match &self.role {
            Role::Heberger { port } => {
                vec!["--host".to_string(), "--port".into(), port.to_string()]
            }
            Role::Rejoindre { adresse, port } => {
                vec![
                    "--connect".to_string(),
                    adresse.clone(),
                    "--port".into(),
                    port.to_string(),
                ]
            }
        };
        a.extend(["--nick".to_string(), self.pseudo.clone()]);
        a
    }

    /// Les lignes de config RetroArch pour cette session.
    pub fn config_retroarch(&self) -> Vec<String> {
        let mut l = vec![
            r#"netplay_nat_traversal = "true""#.to_string(),
            // L'indicateur de qualité de connexion (ping) pendant la partie.
            r#"netplay_ping_show = "true""#.into(),
            format!(r#"netplay_nickname = "{}""#, echapper(&self.pseudo)),
        ];
        match &self.relais {
            Some(r) => {
                l.push(r#"netplay_use_mitm_server = "true""#.into());
                l.push(format!(r#"netplay_mitm_server = "{}""#, echapper(r)));
            }
            None => l.push(r#"netplay_use_mitm_server = "false""#.into()),
        }
        if let Some(m) = &self.mot_de_passe {
            l.push(format!(r#"netplay_password = "{}""#, echapper(m)));
        }
        // Un invité demande le port manette choisi dans le salon.
        if let (Role::Rejoindre { .. }, Some(p)) = (&self.role, self.port_manette) {
            if (1..=16).contains(&p) {
                l.push(format!(r#"netplay_request_device_p{p} = "true""#));
            }
        }
        l
    }

    /// La section [NetPlay] de Dolphin.ini. Dolphin n'a pas de commande pour
    /// rejoindre directement : la fenêtre NetPlay s'ouvre pré-remplie.
    pub fn netplay_dolphin(&self) -> String {
        let mut l = vec![
            "[NetPlay]".to_string(),
            format!("Nickname = {}", self.pseudo),
        ];
        match &self.role {
            Role::Heberger { port } => {
                l.push("TraversalChoice = direct".into());
                l.push(format!("HostPort = {port}"));
                l.push("UseUPNP = True".into());
            }
            Role::Rejoindre { adresse, port } => {
                l.push("TraversalChoice = direct".into());
                l.push(format!("Address = {adresse}"));
                l.push(format!("ConnectPort = {port}"));
            }
        }
        l.join("\n") + "\n"
    }
}

fn echapper(texte: &str) -> String {
    texte.replace(['"', '\n', '\r'], "")
}

/// Une place dans la room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    pub pseudo: String,
    pub pret: bool,
    /// Les versions installées chez ce joueur.
    #[serde(default)]
    pub versions: Installees,
    /// L'empreinte SHA-1 de son fichier de jeu.
    pub sha1: String,
    /// Son avatar SQUAD (absent chez les versions plus anciennes).
    #[serde(default)]
    pub avatar: Option<crate::avatar::Avatar>,
    /// Son port manette (1 = joueur 1), attribué à l'entrée, échangeable.
    #[serde(default)]
    pub port: Option<u8>,
}

/// Un message du chat de la room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRoom {
    pub pseudo: String,
    pub texte: String,
}

/// Où en est la room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "etat", rename_all = "snake_case")]
pub enum Etat {
    Attente,
    /// Le jeu démarre chez tous à cet instant (ms depuis 1970).
    CompteARebours {
        depart_ms: u64,
    },
    EnJeu,
}

/// Une partie en préparation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub console: Console,
    pub titre: String,
    /// L'empreinte du fichier de l'hôte : tout le monde doit avoir le même.
    pub sha1: String,
    pub hote: String,
    pub max: usize,
    /// Vrai si un mot de passe est demandé (le mot de passe ne voyage pas ici).
    pub privee: bool,
    pub places: Vec<Place>,
    pub etat: Etat,
    /// Le chat (les 50 derniers messages).
    #[serde(default)]
    pub messages: Vec<MessageRoom>,
    /// Les joueurs exclus par l'hôte : ils ne peuvent pas revenir.
    #[serde(default)]
    pub exclus: Vec<String>,
    /// Les places gardées pour des potes précis.
    #[serde(default)]
    pub reserves: Vec<String>,
}

/// Les composants dont la version doit être identique, par console.
pub fn composants_requis(console: Console) -> Vec<String> {
    match console.moteur() {
        crate::consoles::Moteur::Retroarch { core } => {
            vec!["retroarch".into(), format!("core:{core}")]
        }
        crate::consoles::Moteur::Dolphin => vec!["dolphin".into()],
    }
}

impl Room {
    pub fn nouvelle(
        id: &str,
        console: Console,
        titre: &str,
        hote: Place,
        max: usize,
        privee: bool,
    ) -> Room {
        Room {
            id: id.into(),
            console,
            titre: titre.into(),
            sha1: hote.sha1.clone(),
            hote: hote.pseudo.clone(),
            max: max.max(2),
            privee,
            places: vec![Place { port: Some(1), ..hote }],
            etat: Etat::Attente,
            messages: Vec::new(),
            exclus: Vec::new(),
            reserves: Vec::new(),
        }
    }

    fn port_libre(&self) -> Option<u8> {
        (1..=self.max.min(16) as u8).find(|n| !self.places.iter().any(|p| p.port == Some(*n)))
    }

    /// Un message dans le chat (200 caractères au plus, 50 messages gardés).
    pub fn dire(&mut self, pseudo: &str, texte: &str) -> bool {
        let texte: String = texte.trim().chars().filter(|c| !c.is_control()).take(200).collect();
        if texte.is_empty() || !self.places.iter().any(|p| p.pseudo == pseudo) {
            return false;
        }
        self.messages.push(MessageRoom { pseudo: pseudo.into(), texte });
        let trop = self.messages.len().saturating_sub(50);
        self.messages.drain(..trop);
        true
    }

    /// Prend un port manette ; s'il est pris, les deux joueurs échangent.
    pub fn choisir_port(&mut self, pseudo: &str, port: u8) -> Result<(), String> {
        if port == 0 || port as usize > self.max.min(16) {
            return Err(format!("Les ports vont de 1 à {}.", self.max.min(16)));
        }
        let Some(i) = self.places.iter().position(|p| p.pseudo == pseudo) else {
            return Err("Tu n'es pas dans la room.".into());
        };
        let ancien = self.places[i].port;
        if let Some(autre) = self.places.iter_mut().find(|p| p.port == Some(port)) {
            autre.port = ancien;
        }
        self.places[i].port = Some(port);
        Ok(())
    }

    /// L'hôte exclut un joueur : il sort et ne peut plus revenir.
    pub fn exclure(&mut self, pseudo: &str) -> bool {
        if pseudo == self.hote || !self.places.iter().any(|p| p.pseudo == pseudo) {
            return false;
        }
        self.quitter(pseudo);
        self.exclus.push(pseudo.to_lowercase());
        true
    }

    /// L'hôte garde une place pour un pote (ou la libère s'il la gardait déjà).
    pub fn reserver(&mut self, pseudo: &str) -> Result<(), String> {
        let p = pseudo.trim();
        if p.is_empty() {
            return Ok(());
        }
        if let Some(i) = self.reserves.iter().position(|r| r.eq_ignore_ascii_case(p)) {
            self.reserves.remove(i);
            return Ok(());
        }
        if self.reserves.len() + 1 >= self.max {
            return Err("Plus de place à réserver.".into());
        }
        self.reserves.push(p.chars().take(24).collect());
        Ok(())
    }

    /// Fait entrer un joueur, après vérification du fichier et des versions.
    pub fn rejoindre(&mut self, joueur: Place) -> Result<(), String> {
        if self.etat != Etat::Attente {
            return Err("La partie a déjà commencé.".into());
        }
        if self.places.iter().any(|p| p.pseudo == joueur.pseudo) {
            return Err(format!("{} est déjà dans la room.", joueur.pseudo));
        }
        if self.places.len() >= self.max {
            return Err("La room est complète.".into());
        }
        if self.exclus.contains(&joueur.pseudo.to_lowercase()) {
            return Err("L'hôte t'a retiré de cette room.".into());
        }
        let reserve = self.reserves.iter().any(|r| r.eq_ignore_ascii_case(&joueur.pseudo));
        let gardees = self
            .reserves
            .iter()
            .filter(|r| !self.places.iter().any(|p| p.pseudo.eq_ignore_ascii_case(r)))
            .count();
        if !reserve && self.max - self.places.len() <= gardees {
            return Err("Les places restantes sont réservées à des potes de l'hôte.".into());
        }
        if joueur.sha1 != self.sha1 {
            return Err(format!(
                "Ton fichier de {} n'est pas le même que celui de {} (autre version ou autre région). \
                 Il faut exactement le même pour jouer ensemble.",
                self.titre, self.hote
            ));
        }
        let hote = &self.places[0];
        for c in composants_requis(self.console) {
            if hote.versions.get(&c) != joueur.versions.get(&c) {
                return Err(format!(
                    "Ta version de {c} ({}) n'est pas celle de {} ({}). Lance la mise à jour puis réessaie.",
                    joueur.versions.get(&c).map(String::as_str).unwrap_or("absente"),
                    self.hote,
                    hote.versions.get(&c).map(String::as_str).unwrap_or("absente"),
                ));
            }
        }
        let port = self.port_libre();
        self.places.push(Place { port, ..joueur });
        Ok(())
    }

    pub fn quitter(&mut self, pseudo: &str) {
        self.places.retain(|p| p.pseudo != pseudo);
        if let Etat::CompteARebours { .. } = self.etat {
            self.etat = Etat::Attente;
        }
    }

    /// Coche ou décoche « Prêt ». Annule le compte à rebours si quelqu'un se décoche.
    pub fn basculer_pret(&mut self, pseudo: &str) -> bool {
        let Some(p) = self.places.iter_mut().find(|p| p.pseudo == pseudo) else {
            return false;
        };
        p.pret = !p.pret;
        let pret = p.pret;
        if !pret {
            if let Etat::CompteARebours { .. } = self.etat {
                self.etat = Etat::Attente;
            }
        }
        pret
    }

    pub fn tous_prets(&self) -> bool {
        self.places.len() >= 2 && self.places.iter().all(|p| p.pret)
    }

    /// L'hôte lance le compte à rebours, si tout le monde est prêt.
    pub fn lancer_compte_a_rebours(&mut self, maintenant_ms: u64) -> Result<u64, String> {
        if !self.tous_prets() {
            return Err("Tout le monde n'est pas prêt.".into());
        }
        let depart = maintenant_ms + COMPTE_A_REBOURS_MS;
        self.etat = Etat::CompteARebours { depart_ms: depart };
        Ok(depart)
    }

    /// Passe en jeu quand l'heure du départ est passée.
    pub fn avancer(&mut self, maintenant_ms: u64) -> bool {
        if let Etat::CompteARebours { depart_ms } = self.etat {
            if maintenant_ms >= depart_ms {
                self.etat = Etat::EnJeu;
                return true;
            }
        }
        false
    }
}

/// Un lien d'invitation à coller sur Discord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invitation {
    pub room: String,
    pub console: Console,
    pub titre: String,
    pub adresse: String,
    pub port: u16,
    pub privee: bool,
}

/// Le préfixe des liens : l'app s'enregistre pour l'ouvrir.
pub const SCHEMA: &str = "squadretro://rejoindre";

impl Invitation {
    pub fn lien(&self) -> String {
        format!(
            "{SCHEMA}?room={}&console={}&titre={}&adresse={}&port={}&privee={}",
            encoder(&self.room),
            self.console.dossier(),
            encoder(&self.titre),
            encoder(&self.adresse),
            self.port,
            if self.privee { 1 } else { 0 }
        )
    }

    pub fn lire(lien: &str) -> Result<Invitation, String> {
        let invalide = || "Ce lien d'invitation n'est pas valide.".to_string();
        let requete = lien
            .strip_prefix(SCHEMA)
            .and_then(|r| r.strip_prefix('?'))
            .ok_or_else(invalide)?;
        let mut champs = std::collections::HashMap::new();
        for paire in requete.split('&') {
            let (k, v) = paire.split_once('=').ok_or_else(invalide)?;
            champs.insert(k, decoder(v).ok_or_else(invalide)?);
        }
        let champ = |k: &str| champs.get(k).cloned().ok_or_else(invalide);
        Ok(Invitation {
            room: champ("room")?,
            console: Console::depuis_dossier(&champ("console")?).ok_or_else(invalide)?,
            titre: champ("titre")?,
            adresse: champ("adresse")?,
            port: champ("port")?.parse().map_err(|_| invalide())?,
            privee: champ("privee")? == "1",
        })
    }
}

fn encoder(texte: &str) -> String {
    texte
        .bytes()
        .map(|o| match o {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (o as char).to_string()
            }
            _ => format!("%{o:02X}"),
        })
        .collect()
}

fn decoder(texte: &str) -> Option<String> {
    let octets = texte.as_bytes();
    let mut sortie = Vec::with_capacity(octets.len());
    let mut i = 0;
    while i < octets.len() {
        if octets[i] == b'%' {
            let hex = std::str::from_utf8(octets.get(i + 1..i + 3)?).ok()?;
            sortie.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            sortie.push(octets[i]);
            i += 1;
        }
    }
    String::from_utf8(sortie).ok()
}

/// Mesure le temps de connexion à l'hôte (une approximation du ping).
pub fn mesurer_ping(adresse: &str, port: u16, delai: Duration) -> Result<Duration, String> {
    let cible = (adresse, port)
        .to_socket_addrs()
        .map_err(|_| format!("Adresse introuvable : {adresse}"))?
        .next()
        .ok_or_else(|| format!("Adresse introuvable : {adresse}"))?;
    let debut = Instant::now();
    TcpStream::connect_timeout(&cible, delai).map_err(|_| "L'hôte ne répond pas.".to_string())?;
    Ok(debut.elapsed())
}

/// Ce que vaut un ping pour jouer.
pub fn qualite(ping: Duration) -> &'static str {
    match ping.as_millis() {
        0..=40 => "Excellente",
        41..=80 => "Bonne",
        81..=130 => "Moyenne : un peu de décalage possible",
        _ => "Mauvaise : la partie risque de saccader",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joueur(pseudo: &str, sha1: &str, dolphin: &str) -> Place {
        let mut versions = Installees::new();
        versions.insert("dolphin".into(), dolphin.into());
        Place {
            pseudo: pseudo.into(),
            pret: false,
            versions,
            sha1: sha1.into(),
            avatar: None,
            port: None,
        }
    }

    #[test]
    fn room_complete() {
        let mut room = Room::nouvelle(
            "r1",
            Console::Gamecube,
            "Mario Kart",
            joueur("Swift", "aaa", "2512"),
            4,
            false,
        );
        assert!(room
            .rejoindre(joueur("Tibo", "bbb", "2512"))
            .unwrap_err()
            .contains("fichier"));
        assert!(room
            .rejoindre(joueur("Tibo", "aaa", "2509"))
            .unwrap_err()
            .contains("version"));
        room.rejoindre(joueur("Tibo", "aaa", "2512")).unwrap();
        assert!(room.lancer_compte_a_rebours(0).is_err());
        room.basculer_pret("Swift");
        room.basculer_pret("Tibo");
        let depart = room.lancer_compte_a_rebours(1_000).unwrap();
        assert_eq!(depart, 6_000);
        room.basculer_pret("Tibo");
        assert_eq!(room.etat, Etat::Attente);
        room.basculer_pret("Tibo");
        room.lancer_compte_a_rebours(1_000).unwrap();
        assert!(!room.avancer(5_999));
        assert!(room.avancer(6_000));
        assert!(room.rejoindre(joueur("Late", "aaa", "2512")).is_err());
    }

    #[test]
    fn lien_aller_retour() {
        let inv = Invitation {
            room: "r1".into(),
            console: Console::Snes,
            titre: "Super Mario World & co".into(),
            adresse: "90.12.34.56".into(),
            port: 55435,
            privee: true,
        };
        let lien = inv.lien();
        assert!(lien.starts_with("squadretro://rejoindre?"));
        assert_eq!(Invitation::lire(&lien).unwrap(), inv);
        assert!(Invitation::lire("https://exemple.com").is_err());
    }

    #[test]
    fn arguments_et_config() {
        let s = Session {
            role: Role::Rejoindre {
                adresse: "1.2.3.4".into(),
                port: 55435,
            },
            pseudo: "Swift".into(),
            mot_de_passe: Some("x\"y".into()),
            relais: Some("madrid".into()),
            port_manette: None,
        };
        assert_eq!(
            s.arguments_retroarch()[..2],
            ["--connect".to_string(), "1.2.3.4".into()]
        );
        let c = s.config_retroarch().join("\n");
        assert!(c.contains(r#"netplay_mitm_server = "madrid""#));
        assert!(c.contains(r#"netplay_password = "xy""#));
        assert!(s.netplay_dolphin().contains("Address = 1.2.3.4"));
    }

    #[test]
    fn ping_local() {
        let ecoute = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = ecoute.local_addr().unwrap().port();
        let p = mesurer_ping("127.0.0.1", port, Duration::from_secs(1)).unwrap();
        assert_eq!(qualite(p), "Excellente");
    }
}

#[cfg(test)]
mod tests_salon {
    use super::*;

    fn joueur(p: &str) -> Place {
        Place { pseudo: p.into(), pret: false, versions: Installees::new(), sha1: "s".into(), avatar: None, port: None }
    }

    #[test]
    fn chat_ports_exclusion_reserves() {
        let mut r = Room::nouvelle("r", Console::Gamecube, "Mario Party 6", joueur("Swift"), 4, false);
        assert_eq!(r.places[0].port, Some(1));
        r.rejoindre(joueur("Bront")).unwrap();
        assert_eq!(r.places[1].port, Some(2));
        // Échange de ports.
        r.choisir_port("Bront", 1).unwrap();
        assert_eq!((r.places[0].port, r.places[1].port), (Some(2), Some(1)));
        assert!(r.choisir_port("Bront", 9).is_err());
        // Chat.
        assert!(r.dire("Bront", "  salut\\n "));
        assert!(!r.dire("Inconnu", "coucou"));
        assert!(!r.dire("Bront", "   "));
        for i in 0..60 {
            r.dire("Swift", &format!("m{i}"));
        }
        assert_eq!(r.messages.len(), 50);
        // Réservations : 4 places, 2 prises, 2 réservées → un inconnu est refusé.
        r.reserver("Vesper").unwrap();
        r.reserver("Lucie").unwrap();
        assert!(r.rejoindre(joueur("Tibo")).unwrap_err().contains("réservées"));
        r.rejoindre(joueur("vesper")).unwrap();
        assert_eq!(r.places[2].port, Some(3));
        // Exclusion définitive.
        assert!(r.exclure("Bront"));
        assert!(!r.exclure("Swift"));
        assert!(r.rejoindre(joueur("Bront")).unwrap_err().contains("retiré"));
    }

    #[test]
    fn port_demande_par_l_invite() {
        let s = Session {
            role: Role::Rejoindre { adresse: "1.2.3.4".into(), port: 55435 },
            pseudo: "Bront".into(),
            mot_de_passe: None,
            relais: None,
            port_manette: Some(2),
        };
        assert!(s.config_retroarch().iter().any(|l| l == r#"netplay_request_device_p2 = "true""#));
    }
}
