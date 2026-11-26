//! La room sur le réseau, sans serveur central : l'app de l'hôte ouvre une
//! petite room (TCP, une ligne JSON par message) et les autres la rejoignent
//! avec le lien d'invitation. Marche pour la bande comme pour l'app publique.
//!
//! Le port de la room est celui du netplay + 1 (55436 par défaut). Comme
//! pour le netplay direct, l'hôte doit être joignable (même réseau, VPN type
//! Tailscale, ou port ouvert).

use crate::netplay::{Etat, Place, Room};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const PORT_ROOM: u16 = 55436;

/// Ce qu'un joueur envoie à la room.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Demande {
    Rejoindre {
        place: Place,
        mot_de_passe: Option<String>,
    },
    Pret,
    Quitter,
}

/// Ce que la room envoie aux joueurs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Nouvelle {
    /// L'état de la room. `dans_ms` : temps avant le départ (compte à rebours),
    /// relatif pour ne pas dépendre de l'heure de chaque PC.
    Room {
        room: Room,
        dans_ms: Option<u64>,
    },
    Refus {
        raison: String,
    },
}

pub fn maintenant_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn nouvelle_room(room: &Room) -> Nouvelle {
    let dans_ms = match room.etat {
        Etat::CompteARebours { depart_ms } => Some(depart_ms.saturating_sub(maintenant_ms())),
        _ => None,
    };
    Nouvelle::Room {
        room: room.clone(),
        dans_ms,
    }
}

fn envoyer(flux: &mut TcpStream, n: &Nouvelle) -> bool {
    let mut ligne = serde_json::to_string(n).unwrap_or_default();
    ligne.push('\n');
    flux.write_all(ligne.as_bytes()).is_ok()
}

type Abonnes = Arc<Mutex<Vec<(String, TcpStream)>>>;
type QuandCaChange = Arc<dyn Fn(&Room, Option<u64>) + Send + Sync>;

/// La room de l'hôte.
pub struct ServeurRoom {
    pub room: Arc<Mutex<Room>>,
    abonnes: Abonnes,
    pub port: u16,
    ecoute: TcpListener,
    on_change: QuandCaChange,
}

impl ServeurRoom {
    /// Ouvre la room. `on_change` est appelé (dans un autre fil) à chaque
    /// changement, pour mettre à jour la fenêtre de l'hôte.
    pub fn ouvrir(
        room: Room,
        port: u16,
        mot_de_passe: Option<String>,
        on_change: impl Fn(&Room, Option<u64>) + Send + Sync + 'static,
    ) -> Result<Arc<ServeurRoom>, String> {
        let ecoute = TcpListener::bind(("0.0.0.0", port))
            .map_err(|e| format!("Impossible d'ouvrir la room sur le port {port} : {e}"))?;
        let port = ecoute.local_addr().map(|a| a.port()).unwrap_or(port);
        let serveur = Arc::new(ServeurRoom {
            room: Arc::new(Mutex::new(room)),
            abonnes: Arc::new(Mutex::new(Vec::new())),
            port,
            ecoute: ecoute.try_clone().map_err(|e| e.to_string())?,
            on_change: Arc::new(on_change),
        });
        let s = serveur.clone();
        std::thread::spawn(move || {
            for flux in ecoute.incoming().flatten() {
                let s = s.clone();
                let mdp = mot_de_passe.clone();
                std::thread::spawn(move || s.servir(flux, mdp));
            }
        });
        Ok(serveur)
    }

    fn servir(&self, flux: TcpStream, mot_de_passe: Option<String>) {
        let _ = flux.set_read_timeout(Some(Duration::from_secs(60 * 60)));
        let Ok(mut ecriture) = flux.try_clone() else {
            return;
        };
        let mut pseudo: Option<String> = None;
        for ligne in BufReader::new(flux).lines() {
            let Ok(ligne) = ligne else { break };
            let Ok(demande) = serde_json::from_str::<Demande>(&ligne) else {
                continue;
            };
            match demande {
                Demande::Rejoindre {
                    place,
                    mot_de_passe: donne,
                } => {
                    if mot_de_passe.is_some() && donne != mot_de_passe {
                        envoyer(
                            &mut ecriture,
                            &Nouvelle::Refus {
                                raison: "Mot de passe incorrect.".into(),
                            },
                        );
                        break;
                    }
                    let nom = place.pseudo.clone();
                    let resultat = self.room.lock().unwrap().rejoindre(place);
                    if let Err(raison) = resultat {
                        envoyer(&mut ecriture, &Nouvelle::Refus { raison });
                        break;
                    }
                    if let Ok(copie) = ecriture.try_clone() {
                        self.abonnes.lock().unwrap().push((nom.clone(), copie));
                    }
                    pseudo = Some(nom);
                }
                Demande::Pret => {
                    if let Some(p) = &pseudo {
                        self.room.lock().unwrap().basculer_pret(p);
                    }
                }
                Demande::Quitter => break,
            }
            self.diffuser();
        }
        if let Some(p) = pseudo {
            self.room.lock().unwrap().quitter(&p);
            self.abonnes.lock().unwrap().retain(|(n, _)| *n != p);
            self.diffuser();
        }
    }

    /// Envoie l'état de la room à tout le monde.
    pub fn diffuser(&self) {
        let room = self.room.lock().unwrap().clone();
        let n = nouvelle_room(&room);
        self.abonnes
            .lock()
            .unwrap()
            .retain_mut(|(_, f)| envoyer(f, &n));
        let dans = if let Nouvelle::Room { dans_ms, .. } = n {
            dans_ms
        } else {
            None
        };
        (self.on_change)(&room, dans);
    }

    /// L'hôte coche ou décoche « Prêt ».
    pub fn pret_hote(&self) {
        let hote = self.room.lock().unwrap().hote.clone();
        self.room.lock().unwrap().basculer_pret(&hote);
        self.diffuser();
    }

    /// L'hôte lance le compte à rebours.
    pub fn lancer(&self) -> Result<u64, String> {
        self.room
            .lock()
            .unwrap()
            .lancer_compte_a_rebours(maintenant_ms())?;
        self.diffuser();
        Ok(crate::netplay::COMPTE_A_REBOURS_MS)
    }

    /// Ferme la room : les joueurs sont déconnectés.
    pub fn fermer(&self) {
        for (_, f) in self.abonnes.lock().unwrap().drain(..) {
            let _ = f.shutdown(Shutdown::Both);
        }
        // Débloque la boucle d'écoute en se connectant une dernière fois.
        let _ = self.ecoute.set_nonblocking(true);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

/// Un joueur dans la room de quelqu'un d'autre.
pub struct ClientRoom {
    flux: Mutex<TcpStream>,
}

impl ClientRoom {
    /// Rejoint une room. `on_nouvelle` est appelé à chaque message de la room.
    pub fn rejoindre(
        adresse: &str,
        port: u16,
        place: Place,
        mot_de_passe: Option<String>,
        on_nouvelle: impl Fn(Nouvelle) + Send + 'static,
    ) -> Result<Arc<ClientRoom>, String> {
        let mut flux = TcpStream::connect((adresse, port))
            .map_err(|_| "La room ne répond pas : l'hôte est-il joignable ?".to_string())?;
        let mut ligne = serde_json::to_string(&Demande::Rejoindre {
            place,
            mot_de_passe,
        })
        .unwrap_or_default();
        ligne.push('\n');
        flux.write_all(ligne.as_bytes())
            .map_err(|e| e.to_string())?;
        let lecture = flux.try_clone().map_err(|e| e.to_string())?;
        std::thread::spawn(move || {
            for l in BufReader::new(lecture).lines() {
                let Ok(l) = l else { break };
                if let Ok(n) = serde_json::from_str::<Nouvelle>(&l) {
                    on_nouvelle(n);
                }
            }
        });
        Ok(Arc::new(ClientRoom {
            flux: Mutex::new(flux),
        }))
    }

    fn envoyer(&self, d: &Demande) -> Result<(), String> {
        let mut ligne = serde_json::to_string(d).unwrap_or_default();
        ligne.push('\n');
        self.flux
            .lock()
            .unwrap()
            .write_all(ligne.as_bytes())
            .map_err(|_| "La room est fermée.".to_string())
    }

    pub fn pret(&self) -> Result<(), String> {
        self.envoyer(&Demande::Pret)
    }

    pub fn quitter(&self) {
        let _ = self.envoyer(&Demande::Quitter);
        let _ = self.flux.lock().unwrap().shutdown(Shutdown::Both);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consoles::Console;
    use crate::versions::Installees;
    use std::sync::mpsc;

    fn place(p: &str) -> Place {
        Place {
            pseudo: p.into(),
            pret: false,
            versions: Installees::new(),
            sha1: "aaa".into(),
        }
    }

    #[test]
    fn deux_joueurs_jusqu_au_compte_a_rebours() {
        let room = Room::nouvelle("r", Console::Snes, "Jeu", place("Hote"), 4, true);
        let serveur = ServeurRoom::ouvrir(room, 0, Some("secret".into()), |_, _| {}).unwrap();

        let (tx, rx) = mpsc::channel();
        let refus = mpsc::channel::<Nouvelle>();
        let r = ClientRoom::rejoindre(
            "127.0.0.1",
            serveur.port,
            place("Mauvais"),
            Some("faux".into()),
            move |n| {
                let _ = refus.0.send(n);
            },
        );
        assert!(r.is_ok());
        assert!(matches!(
            refus.1.recv_timeout(Duration::from_secs(2)).unwrap(),
            Nouvelle::Refus { .. }
        ));

        let client = ClientRoom::rejoindre(
            "127.0.0.1",
            serveur.port,
            place("Tibo"),
            Some("secret".into()),
            move |n| {
                let _ = tx.send(n);
            },
        )
        .unwrap();
        let premiere = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(&premiere, Nouvelle::Room { room, .. } if room.places.len() == 2));

        client.pret().unwrap();
        let _ = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        serveur.pret_hote();
        let _ = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        serveur.lancer().unwrap();
        let n = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(n, Nouvelle::Room { dans_ms: Some(ms), .. } if ms > 4_000));
        client.quitter();
        serveur.fermer();
    }
}
