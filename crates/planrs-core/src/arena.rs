//! Collection ordonnée d'entités indexée par UID (ADR-0008).
//!
//! Un `Vec` dans l'ordre d'affichage (l'ordre des ID), plus un index
//! UID → position. Sérialisée comme un tableau JSON ; la lecture refuse un UID
//! en double. L'UID d'un élément n'a pas de setter : l'index reste juste.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{Deserializer, Error as _};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::error::ModelError;
use crate::ids::EntityUid;

/// Entité qui porte un UID.
pub trait HasUid {
    /// Type de l'UID.
    type Uid: Ord + Copy;
    /// UID de l'entité.
    fn uid(&self) -> Self::Uid;
}

/// Entités dans l'ordre d'affichage, avec un index UID → position.
///
/// L'index est reconstruit en O(n log n) après [`Arena::insert`] ou
/// [`Arena::remove`] (chemin froid). L'allocation des UID (monotone, jamais
/// réutilisés) est décidée en J4.
#[derive(Clone, PartialEq, Eq)]
pub struct Arena<K, T> {
    items: Vec<T>,
    index: BTreeMap<K, usize>,
}

impl<K: fmt::Debug, T: fmt::Debug> fmt::Debug for Arena<K, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.items).finish()
    }
}

impl<K: EntityUid, T: HasUid<Uid = K>> Default for Arena<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: EntityUid, T: HasUid<Uid = K>> Arena<K, T> {
    /// Collection vide.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            index: BTreeMap::new(),
        }
    }

    /// Nombre d'éléments.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Vrai si la collection est vide.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Élément d'UID `uid`.
    #[must_use]
    pub fn get(&self, uid: K) -> Option<&T> {
        self.index.get(&uid).and_then(|&p| self.items.get(p))
    }

    /// Élément modifiable d'UID `uid` (l'UID lui-même n'est pas modifiable).
    pub fn get_mut(&mut self, uid: K) -> Option<&mut T> {
        let position = *self.index.get(&uid)?;
        self.items.get_mut(position)
    }

    /// Vrai si un élément porte l'UID `uid`.
    #[must_use]
    pub fn contains(&self, uid: K) -> bool {
        self.index.contains_key(&uid)
    }

    /// Position (0 = premier) de l'élément d'UID `uid`.
    #[must_use]
    pub fn position(&self, uid: K) -> Option<usize> {
        self.index.get(&uid).copied()
    }

    /// Élément à la position donnée.
    #[must_use]
    pub fn at(&self, position: usize) -> Option<&T> {
        self.items.get(position)
    }

    /// Éléments dans l'ordre.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// Ajoute un élément à la fin.
    ///
    /// # Errors
    ///
    /// [`ModelError::DuplicateUid`] si l'UID existe déjà.
    pub fn push(&mut self, item: T) -> Result<(), ModelError> {
        let uid = item.uid();
        if self.index.contains_key(&uid) {
            return Err(duplicate(uid));
        }
        self.index.insert(uid, self.items.len());
        self.items.push(item);
        Ok(())
    }

    /// Insère un élément à la position donnée (`position == len()` ajoute à la fin).
    ///
    /// # Errors
    ///
    /// [`ModelError::DuplicateUid`] si l'UID existe déjà,
    /// [`ModelError::PositionOutOfRange`] si `position > len()`.
    pub fn insert(&mut self, position: usize, item: T) -> Result<(), ModelError> {
        let uid = item.uid();
        if self.index.contains_key(&uid) {
            return Err(duplicate(uid));
        }
        if position > self.items.len() {
            return Err(ModelError::PositionOutOfRange { position });
        }
        self.items.insert(position, item);
        self.rebuild_index();
        Ok(())
    }

    /// Retire l'élément d'UID `uid` et le rend.
    pub fn remove(&mut self, uid: K) -> Option<T> {
        let position = self.index.get(&uid).copied()?;
        if position >= self.items.len() {
            return None;
        }
        let item = self.items.remove(position);
        self.rebuild_index();
        Some(item)
    }

    /// Plus grand UID présent.
    #[must_use]
    pub fn max_uid(&self) -> Option<K> {
        self.index.last_key_value().map(|(k, _)| *k)
    }

    fn rebuild_index(&mut self) {
        self.index = self
            .items
            .iter()
            .enumerate()
            .map(|(position, item)| (item.uid(), position))
            .collect();
    }
}

fn duplicate<K: EntityUid>(uid: K) -> ModelError {
    ModelError::DuplicateUid {
        entity: K::ENTITY,
        uid: uid.raw(),
    }
}

impl<'a, K, T> IntoIterator for &'a Arena<K, T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<K: EntityUid, T: HasUid<Uid = K>> TryFrom<Vec<T>> for Arena<K, T> {
    type Error = ModelError;
    fn try_from(items: Vec<T>) -> Result<Self, Self::Error> {
        let mut index = BTreeMap::new();
        for (position, item) in items.iter().enumerate() {
            let uid = item.uid();
            if index.insert(uid, position).is_some() {
                return Err(duplicate(uid));
            }
        }
        Ok(Self { items, index })
    }
}

impl<K, T> From<Arena<K, T>> for Vec<T> {
    fn from(arena: Arena<K, T>) -> Self {
        arena.items
    }
}

impl<K, T: Serialize> Serialize for Arena<K, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.items.serialize(serializer)
    }
}

impl<'de, K: EntityUid, T: HasUid<Uid = K> + Deserialize<'de>> Deserialize<'de> for Arena<K, T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let items = Vec::<T>::deserialize(deserializer)?;
        Self::try_from(items).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use serde::{Deserialize, Serialize};

    use super::{Arena, HasUid};
    use crate::error::{EntityKind, ModelError};
    use crate::ids::ResourceUid;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Item {
        uid: ResourceUid,
        name: String,
    }

    impl HasUid for Item {
        type Uid = ResourceUid;
        fn uid(&self) -> ResourceUid {
            self.uid
        }
    }

    fn item(uid: u32, name: &str) -> Item {
        Item {
            uid: ResourceUid::new(uid),
            name: name.to_owned(),
        }
    }

    fn names(a: &Arena<ResourceUid, Item>) -> Vec<&str> {
        a.iter().map(|i| i.name.as_str()).collect()
    }

    #[test]
    fn push_insert_remove_keep_index() {
        let mut a = Arena::new();
        assert!(a.is_empty());
        a.push(item(5, "a")).unwrap();
        a.push(item(2, "b")).unwrap();
        assert_eq!(
            a.push(item(5, "dup")),
            Err(ModelError::DuplicateUid {
                entity: EntityKind::Resource,
                uid: 5
            })
        );
        a.insert(0, item(9, "c")).unwrap();
        assert_eq!(names(&a), ["c", "a", "b"]);
        assert_eq!(a.position(ResourceUid::new(5)), Some(1));
        assert_eq!(a.position(ResourceUid::new(2)), Some(2));
        assert_eq!(a.at(0).unwrap().name, "c");
        assert_eq!(
            a.insert(4, item(10, "x")),
            Err(ModelError::PositionOutOfRange { position: 4 })
        );
        assert!(a.insert(1, item(2, "dup")).is_err());
        a.insert(3, item(10, "d")).unwrap();
        assert_eq!(a.max_uid(), Some(ResourceUid::new(10)));
        assert_eq!(a.remove(ResourceUid::new(5)).unwrap().name, "a");
        assert_eq!(a.remove(ResourceUid::new(5)), None);
        assert_eq!(names(&a), ["c", "b", "d"]);
        assert_eq!(a.position(ResourceUid::new(2)), Some(1));
        assert_eq!(a.position(ResourceUid::new(10)), Some(2));
        assert!(!a.contains(ResourceUid::new(5)));
        a.get_mut(ResourceUid::new(2)).unwrap().name = "B".to_owned();
        assert_eq!(a.get(ResourceUid::new(2)).unwrap().name, "B");
        assert_eq!(a.len(), 3);
        assert_eq!(Arena::<ResourceUid, Item>::new().max_uid(), None);
    }

    #[test]
    fn serde_is_an_array_and_refuses_duplicates() {
        let a = Arena::try_from(vec![item(3, "a"), item(1, "b")]).unwrap();
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, r#"[{"uid":3,"name":"a"},{"uid":1,"name":"b"}]"#);
        assert!(
            serde_json::from_str::<Arena<ResourceUid, Item>>(
                r#"[{"uid":3,"name":"a"},{"uid":3,"name":"b"}]"#
            )
            .is_err()
        );
        assert_eq!(
            Arena::try_from(vec![item(3, "a"), item(3, "b")]),
            Err(ModelError::DuplicateUid {
                entity: EntityKind::Resource,
                uid: 3
            })
        );
        let v: Vec<Item> = a.into();
        assert_eq!(v.len(), 2);
    }
}
