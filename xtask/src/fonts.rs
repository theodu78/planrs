//! `cargo xtask fonts` : table des avances d'Inter pour `planrs-render`
//! (H-R11, ADR-0016 D9).
//!
//! Lit `assets/fonts/Inter-Regular.ttf` et `Inter-SemiBold.ttf` avec
//! `ttf-parser` et écrit `crates/planrs-render/src/metrics_inter.rs` :
//! cadratin (`units_per_em`) et, pour chaque graisse, la table triée
//! `(caractère, avance)` de tous les caractères de la table `cmap` (glyphe 0
//! exclu), avances en unités de police. Le rendu calcule ensuite les
//! largeurs en entiers, sans lire de police à l'exécution.
//!
//! `cargo xtask fonts --check` échoue si le fichier écrit n'est pas celui que
//! les polices donnent (fraîcheur ; même contrôle par le test
//! `metrics_inter_fresh`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Polices lues, relatives à la racine du dépôt : (constante, fichier).
const FONTS: [(&str, &str); 2] = [
    ("REGULAR", "assets/fonts/Inter-Regular.ttf"),
    ("SEMIBOLD", "assets/fonts/Inter-SemiBold.ttf"),
];

/// Fichier écrit, relatif à la racine du dépôt.
const OUTPUT: &str = "crates/planrs-render/src/metrics_inter.rs";

/// Avances par ligne dans le fichier écrit (lisibilité des diffs).
const PER_LINE: usize = 8;

pub fn run(args: &[String]) -> ExitCode {
    let check = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => {
            eprintln!("usage : cargo xtask fonts [--check]");
            return ExitCode::FAILURE;
        }
    };
    let root = repo_root();
    let generated = match generate(&root) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("xtask fonts : {error}");
            return ExitCode::FAILURE;
        }
    };
    let path = root.join(OUTPUT);
    if check {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current == generated {
            println!("xtask fonts : OK ({OUTPUT} à jour)");
            return ExitCode::SUCCESS;
        }
        eprintln!("xtask fonts : {OUTPUT} périmé, relancer `cargo xtask fonts`");
        return ExitCode::FAILURE;
    }
    match std::fs::write(&path, generated) {
        Ok(()) => {
            println!("xtask fonts : {OUTPUT} écrit");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("xtask fonts : écriture de {OUTPUT} impossible : {error}");
            ExitCode::FAILURE
        }
    }
}

/// Avances d'une police : cadratin et table triée par caractère.
struct Advances {
    units_per_em: u16,
    table: Vec<(char, u16)>,
}

fn read_font(path: &Path) -> Result<Advances, String> {
    let data = std::fs::read(path).map_err(|e| format!("{} illisible : {e}", path.display()))?;
    let face = ttf_parser::Face::parse(&data, 0)
        .map_err(|e| format!("{} : police invalide : {e}", path.display()))?;
    let mut table = Vec::new();
    // Plan multilingue de base : le sous-ensemble d'Inter n'en sort pas.
    for code in 0_u32..=0xFFFF {
        let Some(ch) = char::from_u32(code) else {
            continue;
        };
        let Some(gid) = face.glyph_index(ch) else {
            continue;
        };
        if gid.0 == 0 {
            continue;
        }
        let advance = face
            .glyph_hor_advance(gid)
            .ok_or_else(|| format!("{} : avance absente pour U+{code:04X}", path.display()))?;
        table.push((ch, advance));
    }
    Ok(Advances {
        units_per_em: face.units_per_em(),
        table,
    })
}

/// Texte de `metrics_inter.rs` pour les polices du dépôt.
fn generate(root: &Path) -> Result<String, String> {
    let mut fonts = Vec::new();
    for (name, file) in FONTS {
        fonts.push((name, file, read_font(&root.join(file))?));
    }
    let upem = fonts.first().map_or(0, |(_, _, f)| f.units_per_em);
    if upem == 0 || fonts.iter().any(|(_, _, f)| f.units_per_em != upem) {
        return Err("cadratins différents ou nuls entre les graisses".to_owned());
    }
    let mut out = String::new();
    out.push_str(
        "//! Avances des caractères d'Inter (H-R11), en unités de police.\n\
         //!\n\
         //! Fichier généré par `cargo xtask fonts` depuis `assets/fonts/` : ne pas\n\
         //! modifier à la main (fraîcheur vérifiée par `metrics_inter_fresh`).\n\n",
    );
    let _ = writeln!(out, "/// Cadratin des deux graisses, en unités de police.");
    let _ = writeln!(out, "pub(crate) const UNITS_PER_EM: u16 = {upem};\n");
    for (name, file, font) in &fonts {
        let _ = writeln!(out, "/// `{file}` : {} caractères, triés.", font.table.len());
        let _ = writeln!(out, "#[rustfmt::skip]");
        let _ = writeln!(out, "pub(crate) static {name}: &[(char, u16)] = &[");
        for chunk in font.table.chunks(PER_LINE) {
            out.push_str("   ");
            for (ch, advance) in chunk {
                let _ = write!(out, " ('\\u{{{:04x}}}', {advance}),", u32::from(*ch));
            }
            out.push('\n');
        }
        out.push_str("];\n\n");
    }
    // Une seule ligne vide finale.
    while out.ends_with("\n\n") {
        out.pop();
    }
    Ok(out)
}

fn repo_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map_or_else(|| manifest.to_path_buf(), Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "tests")]

    use super::*;

    /// Le fichier du dépôt est celui que donnent les polices du dépôt.
    #[test]
    fn metrics_inter_fresh() {
        let root = repo_root();
        let generated = generate(&root).unwrap();
        let current = std::fs::read_to_string(root.join(OUTPUT)).unwrap();
        assert!(
            current == generated,
            "{OUTPUT} périmé : relancer `cargo xtask fonts`"
        );
    }

    /// Tables triées, sans doublon, avec les caractères du français.
    #[test]
    fn metrics_inter_tables_sorted() {
        let root = repo_root();
        for (_, file) in FONTS {
            let font = read_font(&root.join(file)).unwrap();
            assert!(font.table.windows(2).all(|w| w[0].0 < w[1].0), "{file}");
            for ch in "AaÉéèêàçœ€…–Ωж".chars() {
                assert!(font.table.iter().any(|(c, _)| *c == ch), "{file} : {ch}");
            }
        }
    }
}
