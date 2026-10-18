//! Analyse minimale du XSD : arbre des éléments `xsd:*` (noms locaux,
//! attributs), séquences des éléments de `Project`.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// Élément du XSD.
#[derive(Debug, Default)]
pub struct Node {
    /// Nom local (`element`, `sequence`, `choice`, `complexType`…).
    pub name: String,
    /// Attributs.
    pub attrs: Vec<(String, String)>,
    /// Enfants éléments.
    pub children: Vec<Node>,
}

impl Node {
    /// Valeur d'un attribut.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|c| c.name == name)
    }
}

fn node(e: &BytesStart<'_>) -> Node {
    Node {
        name: e.local_name().as_ref().to_owned(),
        attrs: e
            .attributes()
            .flatten()
            .map(|a| (a.key.local_name().as_ref().to_owned(), a.value.to_string()))
            .collect(),
        children: Vec::new(),
    }
}

/// Arbre du document (racine `schema`).
pub fn parse(text: &str) -> Node {
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Node> = vec![Node::default()];
    loop {
        match reader.read_event().expect("XSD bien formé") {
            Event::Start(e) => stack.push(node(&e)),
            Event::Empty(e) => {
                let n = node(&e);
                stack.last_mut().unwrap().children.push(n);
            }
            Event::End(_) => {
                let done = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(done);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let mut doc = stack.pop().unwrap();
    doc.children.pop().expect("racine")
}

/// Élément global `Project`.
pub fn project_element(schema: &Node) -> Option<&Node> {
    schema
        .children
        .iter()
        .find(|c| c.name == "element" && c.attr("name") == Some("Project"))
}

/// Éléments déclarés dans la séquence d'un élément (groupes `choice` et
/// `sequence` imbriqués aplatis).
fn declared(element: &Node) -> Vec<&Node> {
    fn walk<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
        for c in &n.children {
            match c.name.as_str() {
                "element" => out.push(c),
                "choice" | "sequence" => walk(c, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    if let Some(seq) = element.child("complexType").and_then(|t| t.child("sequence")) {
        walk(seq, &mut out);
    }
    out
}

/// Élément au chemin `a/b/c` sous `from` (chemin vide : `from`).
pub fn descend<'a>(from: &'a Node, path: &str) -> Option<&'a Node> {
    let mut cur = from;
    for part in path.split('/').filter(|p| !p.is_empty()) {
        cur = declared(cur).into_iter().find(|e| e.attr("name") == Some(part))?;
    }
    Some(cur)
}

/// Un élément d'une séquence, tel que P27 le compare.
#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    /// Nom.
    pub name: String,
    /// `minOccurs`.
    pub min: u8,
    /// `maxOccurs` > 1 ou non borné.
    pub many: bool,
    /// `maxLength` de la restriction de l'élément.
    pub max_len: Option<u16>,
}

/// Séquence d'un élément.
pub fn sequence(element: &Node) -> Vec<Item> {
    declared(element)
        .into_iter()
        .map(|e| {
            let max_len = e
                .child("simpleType")
                .and_then(|t| t.child("restriction"))
                .and_then(|r| r.child("maxLength"))
                .and_then(|m| m.attr("value"))
                .map(|v| v.parse().expect("maxLength entier"));
            Item {
                name: e.attr("name").expect("élément nommé").to_owned(),
                min: e.attr("minOccurs").map_or(1, |v| v.parse().expect("minOccurs")),
                many: e.attr("maxOccurs").is_some_and(|v| v != "1"),
                max_len,
            }
        })
        .collect()
}
