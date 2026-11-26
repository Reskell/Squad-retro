//! Les bases No-Intro (cartouches) et Redump (disques), au format .dat.
//!
//! Ce sont des fichiers XML « Logiqx » :
//!
//! ```xml
//! <datafile>
//!   <game name="Super Mario World (USA)">
//!     <rom name="Super Mario World (USA).sfc" size="524288" crc="b19ed489" sha1="6b47bb75..."/>
//!   </game>
//! </datafile>
//! ```
//!
//! On en tire un index empreinte → nom du jeu. Les fichiers .dat ne sont pas
//! fournis ici : ils se téléchargent sur les sites de No-Intro et Redump.

use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;

/// Un jeu connu de la base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entree {
    pub nom: String,
    pub taille: Option<u64>,
}

/// L'index d'une base : par SHA-1 d'abord, par CRC32 en secours.
#[derive(Debug, Default, Clone)]
pub struct Base {
    par_sha1: HashMap<String, Entree>,
    par_crc: HashMap<String, Entree>,
}

impl Base {
    /// Lit le contenu d'un fichier .dat.
    pub fn lire(xml: &str) -> Result<Base, String> {
        let mut lecteur = Reader::from_str(xml);
        lecteur.config_mut().trim_text(true);
        let mut base = Base::default();
        let mut jeu_courant: Option<String> = None;

        loop {
            match lecteur.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => match e.name().as_ref() {
                    b"game" | b"machine" => {
                        jeu_courant = attribut(&e, b"name");
                    }
                    b"rom" => {
                        let Some(nom) = jeu_courant.clone() else { continue };
                        let entree = Entree {
                            nom,
                            taille: attribut(&e, b"size").and_then(|t| t.parse().ok()),
                        };
                        if let Some(sha1) = attribut(&e, b"sha1") {
                            base.par_sha1.insert(sha1.to_ascii_lowercase(), entree.clone());
                        }
                        if let Some(crc) = attribut(&e, b"crc") {
                            base.par_crc.insert(crc.to_ascii_lowercase(), entree);
                        }
                    }
                    _ => {}
                },
                Ok(Event::End(e)) if matches!(e.name().as_ref(), b"game" | b"machine") => {
                    jeu_courant = None;
                }
                Ok(Event::Eof) => break,
                Err(err) => return Err(format!("fichier .dat illisible : {err}")),
                _ => {}
            }
        }
        Ok(base)
    }

    /// Retrouve un jeu d'après ses empreintes (en minuscules).
    pub fn chercher(&self, sha1: &str, crc: &str) -> Option<&Entree> {
        self.par_sha1.get(sha1).or_else(|| self.par_crc.get(crc))
    }

    /// Le nombre de fichiers connus.
    pub fn len(&self) -> usize {
        self.par_sha1.len().max(self.par_crc.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn attribut(e: &quick_xml::events::BytesStart, cle: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == cle)
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXEMPLE: &str = r#"<?xml version="1.0"?>
<datafile>
  <header><name>Exemple</name></header>
  <game name="Jeu A (USA)">
    <description>Jeu A (USA)</description>
    <rom name="Jeu A (USA).sfc" size="4" crc="ABCD1234" sha1="AAAA"/>
  </game>
  <game name="Jeu A (France)">
    <rom name="Jeu A (France).sfc" size="4" crc="00001111" sha1="bbbb"/>
  </game>
</datafile>"#;

    #[test]
    fn lit_une_base() {
        let base = Base::lire(EXEMPLE).unwrap();
        assert_eq!(base.len(), 2);
        assert_eq!(base.chercher("aaaa", "").unwrap().nom, "Jeu A (USA)");
        assert_eq!(base.chercher("zzzz", "00001111").unwrap().nom, "Jeu A (France)");
        assert!(base.chercher("zzzz", "ffffffff").is_none());
    }
}
