//! Version web (J8, lot L13) : garde des raccourcis (H-UI-55), fichiers
//! déposés (H-UI-56), téléchargements par genre (`SaveKind`).
//!
//! La garde elle-même (écouteur `keydown` de `apps/planrs-web`) ne tourne
//! que dans un navigateur ; sa décision est la fonction pure
//! [`web_intercepts`], testée ici sur [`KEYMAP`] et sur le périmètre rendu
//! par l'application ([`planrs_ui_egui::App::key_scope`]).

use egui::{Key, Modifiers};
use planrs_ui_egui::keymap::{
    Binding, KEYMAP, KeyScope, KeyState, Shortcut, UiBinding, WEB_NEVER, web_interceptable, web_intercepts,
};

use crate::{harness, open_demo};

/// Document ouvert, table focalisée.
const TABLE: KeyState = KeyState {
    document: true,
    table: true,
};

/// Document ouvert, table sans focus.
const DOCUMENT: KeyState = KeyState {
    document: true,
    table: false,
};

/// Ctrl/Cmd + Maj.
const COMMAND_SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::COMMAND
};

/// H-UI-55 : liste attendue des entrées retenues au navigateur ; exclues :
/// Ctrl/Cmd+N (nouvelle fenêtre), Tab et Maj+Tab, Ctrl/Cmd+Q (Quitter) ;
/// aucune entrée à W ou T (le navigateur ne les rend jamais).
#[test]
fn h_ui_55_interceptable_flags() {
    let excluded: Vec<&Shortcut> = KEYMAP
        .iter()
        .filter(|(_, shortcut, binding)| !web_interceptable(shortcut, *binding))
        .map(|(_, shortcut, _)| shortcut)
        .collect();
    let expected = [
        Shortcut::new(Modifiers::COMMAND, Key::Q),
        Shortcut::new(Modifiers::COMMAND, Key::N),
        Shortcut::new(Modifiers::SHIFT, Key::Tab),
        Shortcut::new(Modifiers::NONE, Key::Tab),
    ];
    assert_eq!(excluded, expected.iter().collect::<Vec<_>>());
    assert!(
        !KEYMAP
            .iter()
            .any(|(_, shortcut, _)| matches!(shortcut.logical_key, Key::W | Key::T)),
        "Ctrl/Cmd+W ou T dans KEYMAP : le navigateur ne les rend pas"
    );
    assert_eq!(WEB_NEVER, &[Key::N, Key::W, Key::T, Key::Tab]);
    // Entrées que le navigateur prendrait pour lui : recherche dans la
    // page, page précédente ou suivante, zoom de la page, infos de la page.
    for (modifiers, key) in [
        (Modifiers::COMMAND, Key::F),
        (Modifiers::COMMAND, Key::G),
        (COMMAND_SHIFT, Key::G),
        (Modifiers::NONE, Key::F3),
        (Modifiers::SHIFT, Key::F3),
        (Modifiers::ALT, Key::ArrowLeft),
        (Modifiers::ALT, Key::ArrowRight),
        (Modifiers::ALT, Key::Home),
        (Modifiers::COMMAND, Key::Equals),
        (Modifiers::COMMAND, Key::Minus),
        (Modifiers::COMMAND, Key::I),
        (Modifiers::COMMAND, Key::S),
        (Modifiers::COMMAND, Key::O),
    ] {
        let shortcut = Shortcut::new(modifiers, key);
        let (_, _, binding) = KEYMAP
            .iter()
            .find(|(_, s, _)| *s == shortcut)
            .unwrap_or_else(|| panic!("{shortcut:?} absent de KEYMAP"));
        assert!(web_interceptable(&shortcut, *binding), "{shortcut:?}");
    }
    assert!(!web_interceptable(
        &Shortcut::new(Modifiers::COMMAND, Key::Q),
        Binding::Ui(UiBinding::Quit)
    ));
}

/// H-UI-55 : la garde ne retient que ce que l'image lirait.
#[test]
fn h_ui_55_intercepts_follow_scope() {
    let all = KeyScope::All(TABLE);
    assert!(web_intercepts(all, Modifiers::COMMAND, Key::F));
    assert!(web_intercepts(all, Modifiers::NONE, Key::F3));
    assert!(web_intercepts(all, Modifiers::SHIFT, Key::F3));
    assert!(web_intercepts(all, COMMAND_SHIFT, Key::G));
    assert!(web_intercepts(all, Modifiers::ALT, Key::ArrowLeft));
    // Jamais : nouvelle fenêtre, onglet, presse-papiers (évènements à part).
    assert!(!web_intercepts(all, Modifiers::COMMAND, Key::N));
    assert!(!web_intercepts(all, Modifiers::COMMAND, Key::W));
    assert!(!web_intercepts(all, Modifiers::COMMAND, Key::T));
    assert!(!web_intercepts(all, Modifiers::NONE, Key::Tab));
    assert!(!web_intercepts(all, Modifiers::COMMAND, Key::C));
    assert!(!web_intercepts(all, Modifiers::COMMAND, Key::V));
    assert!(!web_intercepts(all, Modifiers::COMMAND, Key::X));
    assert!(!web_intercepts(all, Modifiers::COMMAND, Key::Q));
    // Sans document : Ctrl+F (contexte Document) reste au navigateur,
    // Ctrl+O (Global) est retenu.
    let welcome = KeyScope::All(KeyState::default());
    assert!(!web_intercepts(welcome, Modifiers::COMMAND, Key::F));
    assert!(web_intercepts(welcome, Modifiers::COMMAND, Key::O));
    // Table sans focus : la navigation (contexte Table) n'est pas lue.
    assert!(!web_intercepts(
        KeyScope::All(DOCUMENT),
        Modifiers::NONE,
        Key::Enter
    ));
    assert!(web_intercepts(KeyScope::All(TABLE), Modifiers::NONE, Key::Enter));
    // Fenêtre non modale : seuls les raccourcis gardés à côté d'elle.
    let beside = KeyScope::BesideWindow(DOCUMENT);
    assert!(web_intercepts(beside, Modifiers::COMMAND, Key::S));
    assert!(web_intercepts(beside, Modifiers::COMMAND, Key::Z));
    assert!(!web_intercepts(beside, Modifiers::COMMAND, Key::F));
    // Saisie de texte, fenêtre modale : rien.
    assert!(!web_intercepts(KeyScope::Nothing, Modifiers::COMMAND, Key::S));
    assert!(!web_intercepts(KeyScope::Nothing, Modifiers::COMMAND, Key::F));
}

/// H-UI-55 : périmètre rendu par l'application : accueil, document avec
/// table focalisée, puis barre de recherche en saisie (rien de retenu :
/// Ctrl+F y reste au navigateur, la saisie garde ses touches).
#[test]
fn h_ui_55_scope_from_app() {
    let mut harness = harness();
    harness.run();
    assert_eq!(
        harness.state().key_scope(&harness.ctx),
        KeyScope::All(KeyState::default())
    );
    open_demo(&mut harness);
    let scope = harness.state().key_scope(&harness.ctx);
    assert!(matches!(scope, KeyScope::All(keys) if keys.document), "{scope:?}");
    assert!(web_intercepts(scope, Modifiers::COMMAND, Key::F));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    assert!(harness.state().search().is_some(), "barre de recherche ouverte");
    assert_eq!(harness.state().key_scope(&harness.ctx), KeyScope::Nothing);
}
