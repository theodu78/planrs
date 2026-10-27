//! Ruban à onglets (ADR-0017, H-UI-69 à H-UI-77).
//!
//! L0 (contrat) : icônes ([`icon_data`], généré par `cargo xtask icons` ;
//! [`icons`], tracé).

#[rustfmt::skip]
pub mod icon_data;
pub mod icons;

pub use icon_data::Icon;
pub use icons::paint_icon;
