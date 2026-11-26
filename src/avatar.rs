//! Les avatars SQUAD : un petit perso maison, décrit par quelques choix
//! (teint, coiffure, couleurs…). Le moteur ne fait que les garder propres ;
//! l'interface les dessine (en SVG, avec nos propres formes : aucune pièce
//! venue d'un jeu ou d'une console).

use serde::{Deserialize, Serialize};

/// Le nombre de choix possibles pour chaque partie de l'avatar.
pub const TEINTS: u8 = 6;
pub const VISAGES: u8 = 3;
pub const COIFFURES: u8 = 8;
pub const COULEURS_CHEVEUX: u8 = 8;
pub const YEUX: u8 = 5;
pub const BOUCHES: u8 = 5;
pub const COULEURS_TENUE: u8 = 10;
pub const ACCESSOIRES: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Avatar {
    pub teint: u8,
    pub visage: u8,
    pub coiffure: u8,
    pub cheveux: u8,
    pub yeux: u8,
    pub bouche: u8,
    pub tenue: u8,
    pub accessoire: u8,
}

impl Avatar {
    /// Ramène chaque choix dans les limites (un fichier modifié à la main ou
    /// envoyé par un autre joueur ne peut rien casser).
    pub fn nettoyer(self) -> Avatar {
        Avatar {
            teint: self.teint % TEINTS,
            visage: self.visage % VISAGES,
            coiffure: self.coiffure % COIFFURES,
            cheveux: self.cheveux % COULEURS_CHEVEUX,
            yeux: self.yeux % YEUX,
            bouche: self.bouche % BOUCHES,
            tenue: self.tenue % COULEURS_TENUE,
            accessoire: self.accessoire % ACCESSOIRES,
        }
    }

    /// Un avatar tiré d'un texte (le pseudo) : chacun a un perso différent
    /// avant même d'ouvrir le créateur.
    pub fn depuis(texte: &str) -> Avatar {
        let mut h: u32 = 2166136261;
        for o in texte.to_lowercase().bytes() {
            h ^= o as u32;
            h = h.wrapping_mul(16777619);
        }
        let mut tirer = |n: u8| {
            let v = (h % n as u32) as u8;
            h = h.rotate_left(5).wrapping_mul(2654435761);
            v
        };
        Avatar {
            teint: tirer(TEINTS),
            visage: tirer(VISAGES),
            coiffure: tirer(COIFFURES),
            cheveux: tirer(COULEURS_CHEVEUX),
            yeux: tirer(YEUX),
            bouche: tirer(BOUCHES),
            tenue: tirer(COULEURS_TENUE),
            accessoire: 0,
        }
    }
}

/// Un profil SQUAD : un pseudo et son avatar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compte {
    pub id: String,
    pub pseudo: String,
    #[serde(default)]
    pub avatar: Avatar,
}

impl Compte {
    /// Un pseudo propre : sans caractères de contrôle, 24 caractères au plus.
    pub fn pseudo_propre(pseudo: &str) -> String {
        pseudo
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(24)
            .collect()
    }
}

/// Les profils de ce PC et celui qui est utilisé.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Comptes {
    pub liste: Vec<Compte>,
    pub actif: Option<String>,
}

impl Comptes {
    pub fn actif(&self) -> Option<&Compte> {
        let id = self.actif.as_ref()?;
        self.liste.iter().find(|c| &c.id == id)
    }

    /// Ajoute ou met à jour un profil ; un id vide en crée un nouveau.
    pub fn enregistrer(&mut self, mut compte: Compte, nouvel_id: impl FnOnce() -> String) -> Result<Compte, String> {
        compte.pseudo = Compte::pseudo_propre(&compte.pseudo);
        if compte.pseudo.is_empty() {
            return Err("Choisis un pseudo.".into());
        }
        compte.avatar = compte.avatar.nettoyer();
        if self
            .liste
            .iter()
            .any(|c| c.id != compte.id && c.pseudo.eq_ignore_ascii_case(&compte.pseudo))
        {
            return Err(format!("Le pseudo « {} » est déjà pris sur ce PC.", compte.pseudo));
        }
        match self.liste.iter_mut().find(|c| !compte.id.is_empty() && c.id == compte.id) {
            Some(c) => *c = compte.clone(),
            None => {
                if self.liste.len() >= 12 {
                    return Err("12 profils au maximum sur un même PC.".into());
                }
                compte.id = nouvel_id();
                self.liste.push(compte.clone());
            }
        }
        Ok(compte)
    }

    pub fn supprimer(&mut self, id: &str) {
        self.liste.retain(|c| c.id != id);
        if self.actif.as_deref() == Some(id) {
            self.actif = None;
        }
    }
}

/// Un joueur croisé en room : il apparaît dans la plaza.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Croise {
    pub pseudo: String,
    #[serde(default)]
    pub avatar: Avatar,
    /// Le dernier jeu joué ensemble.
    #[serde(default)]
    pub jeu: String,
    /// Quand (secondes depuis 1970).
    pub vu: u64,
}

/// Garde les 30 derniers joueurs croisés, le plus récent d'abord.
pub fn noter_croise(liste: &mut Vec<Croise>, croise: Croise) {
    liste.retain(|c| !c.pseudo.eq_ignore_ascii_case(&croise.pseudo));
    liste.insert(
        0,
        Croise {
            avatar: croise.avatar.nettoyer(),
            ..croise
        },
    );
    liste.truncate(30);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nettoie_et_tire() {
        let a = Avatar { teint: 200, coiffure: 9, accessoire: 7, ..Default::default() }.nettoyer();
        assert!(a.teint < TEINTS && a.coiffure < COIFFURES && a.accessoire < ACCESSOIRES);
        assert_eq!(Avatar::depuis("Swift"), Avatar::depuis("swift"));
        assert_ne!(Avatar::depuis("Swift"), Avatar::depuis("Bront"));
    }

    #[test]
    fn comptes() {
        let mut c = Comptes::default();
        let mut n = 0;
        let mut id = || {
            n += 1;
            format!("p{n}")
        };
        let swift = c
            .enregistrer(Compte { id: String::new(), pseudo: "  Swift\n".into(), avatar: Avatar::default() }, &mut id)
            .unwrap();
        assert_eq!(swift.pseudo, "Swift");
        assert_eq!(swift.id, "p1");
        assert!(c
            .enregistrer(Compte { id: String::new(), pseudo: "swift".into(), avatar: Avatar::default() }, &mut id)
            .is_err());
        let mut modifie = swift.clone();
        modifie.avatar.tenue = 3;
        c.enregistrer(modifie, &mut id).unwrap();
        assert_eq!(c.liste.len(), 1);
        c.actif = Some("p1".into());
        assert_eq!(c.actif().unwrap().avatar.tenue, 3);
        c.supprimer("p1");
        assert!(c.actif.is_none() && c.liste.is_empty());
    }

    #[test]
    fn croises() {
        let mut l = Vec::new();
        for (p, vu) in [("Bront", 1), ("Vesper", 2), ("bront", 3)] {
            noter_croise(&mut l, Croise { pseudo: p.into(), avatar: Avatar::default(), jeu: "MP6".into(), vu });
        }
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].vu, 3);
    }
}
