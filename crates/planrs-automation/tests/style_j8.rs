//! Style dans le document (lot L6 de J8, ADR-0016 D7) : `style.apply`,
//! `style.get`, `style.list`, `style.set_bar_role`,
//! `style.reset_to_template`, `style.save_as_template` ; annulation,
//! `dry_run`, écriture et lecture de `<?planrs-style?>` (H-R12), modèle par
//! défaut (H-UI-43), aller-retour (P56).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]

use planrs_automation::command::style::{self, HEADLESS_DEFAULT};
use planrs_automation::error::codes;
use planrs_automation::session::DocOrigin;
use planrs_automation::text::Language;
use planrs_automation::{AnchorSpec, ApplyMode, Batch, CallEnv, NewProject, Session, call_with};
use planrs_mspdi::write::WriteOptions;
use planrs_mspdi::{ImportCode, style_pi};
use planrs_render::style::{BarRole, Rgb, TemplateId, Variant, template};
use proptest::prelude::*;
use serde_json::{Value, json};

fn now() -> planrs_core::Moment {
    "2026-10-19T08:00".parse().unwrap()
}

fn env() -> CallEnv {
    CallEnv::new(now(), Language::Fr)
}

fn session() -> Session {
    let init = NewProject {
        name: "P".to_owned(),
        anchor: AnchorSpec::Start(now()),
        lang: None,
    };
    let mut s = Session::new_project(init, now()).unwrap();
    let batch = Batch::from_json(json!({
        "format_version": 1,
        "ops": [
            { "op": "task.add", "name": "Fondations", "duration": { "text": "5j" } },
            { "op": "task.add", "name": "Murs", "duration": { "text": "10j" } }
        ]
    }))
    .unwrap();
    s.apply(&batch, ApplyMode::Commit).unwrap();
    s
}

fn call(s: &mut Session, id: &str, params: Value) -> Value {
    call_with(s, &env(), id, params).unwrap_or_else(|e| panic!("{id} : {e:?}"))
}

fn payload(s: &Session) -> Option<String> {
    style_pi::entry(&s.project().extras).map(|e| e.payload.clone())
}

fn xml(s: &Session) -> String {
    String::from_utf8(s.save_bytes(&WriteOptions::default()).xml).unwrap()
}

fn reopen(bytes: &[u8]) -> (Session, Vec<planrs_mspdi::ImportDiagnostic>) {
    let origin = DocOrigin {
        name: "p.xml".to_owned(),
        path: None,
    };
    Session::open_bytes(bytes, origin, now(), Language::Fr).unwrap()
}

#[test]
fn style_apply_undo_redo() {
    let mut s = session();
    let before = s.project().clone();
    let bytes_before = xml(&s);
    assert!(!bytes_before.contains("planrs-style"));
    s.mark_saved();
    let out = call(&mut s, "style.apply", json!({ "template": "presentation" }));
    assert_eq!(out["committed"], true);
    assert!(s.is_dirty(), "document marqué modifié");
    assert_eq!(payload(&s).as_deref(), Some(r#"{"base":"presentation","v":1}"#));
    assert!(xml(&s).contains("<?planrs-style {\"base\":\"presentation\",\"v\":1}?>"));
    // Une entrée d'annulation par appel.
    call(&mut s, "history.undo", json!({}));
    assert_eq!(s.project(), &before);
    assert_eq!(xml(&s), bytes_before);
    call(&mut s, "history.redo", json!({}));
    assert_eq!(payload(&s).as_deref(), Some(r#"{"base":"presentation","v":1}"#));
    // Surcharge seule : fusionnée avec le style du document.
    call(
        &mut s,
        "style.apply",
        json!({ "patch": { "density": "compact" } }),
    );
    let doc = style::doc_style(s.project()).unwrap();
    assert_eq!(doc.base, TemplateId::Presentation);
    assert_eq!(doc.over.density, Some(planrs_render::style::Density::Compact));
    // Retour au défaut : plus rien dans le fichier.
    call(&mut s, "style.apply", json!({ "reset": true }));
    assert!(payload(&s).is_none());
    assert_eq!(xml(&s), bytes_before);
    call(&mut s, "history.undo", json!({}));
    assert!(style::doc_style(s.project()).unwrap().over.density.is_some());
}

#[test]
fn style_apply_dry_run() {
    let mut s = session();
    let before = s.project().clone();
    let generation = s.generation();
    let out = call(
        &mut s,
        "style.apply",
        json!({ "template": "sobre", "dry_run": true }),
    );
    assert_eq!(out["committed"], false);
    assert_eq!(s.project(), &before);
    assert_eq!(s.generation(), generation);
    let out = call(
        &mut s,
        "style.set_bar_role",
        json!({ "role": "critical", "color": "#AA0000", "dry_run": true }),
    );
    assert_eq!(out["committed"], false);
    assert_eq!(s.project(), &before);
}

#[test]
fn style_apply_refusals_leave_session_intact() {
    let mut s = session();
    let before = s.project().clone();
    for (params, code) in [
        (json!({}), codes::PARAM_INVALID),
        (
            json!({ "reset": true, "template": "sobre" }),
            codes::PARAM_INVALID,
        ),
        (json!({ "patch": { "unknown": 1 } }), codes::STYLE_INVALID),
        (
            json!({ "patch": { "font": { "family": "inter", "size16": 999 } } }),
            codes::STYLE_INVALID,
        ),
        (
            json!({ "patch": { "name": "x".repeat(65) } }),
            codes::STYLE_INVALID,
        ),
        (json!({ "template": "nope" }), codes::PARAM_INVALID),
    ] {
        let err = call_with(&mut s, &env(), "style.apply", params.clone()).unwrap_err();
        assert_eq!(err.code, code, "{params}");
    }
    assert_eq!(s.project(), &before);
}

/// H-UI-59, H-UI-60 : un geste sur un type de barre = une entrée
/// d'annulation ; les autres rôles et variantes ne bougent pas.
#[test]
fn r_style_set_bar_role_undo() {
    let mut s = session();
    call(&mut s, "style.apply", json!({ "template": "sobre" }));
    let sobre = template(TemplateId::Sobre);
    let history = s.history().entries.len();
    call(
        &mut s,
        "style.set_bar_role",
        json!({ "role": "critical", "color": "#AA0000", "thickness": 60, "ends": "square",
                "text_left": "duration", "text_right": null }),
    );
    assert_eq!(s.history().entries.len(), history + 1);
    let st = style::effective_style(s.project(), HEADLESS_DEFAULT);
    let paint = st.tokens(Variant::Light).bars[&BarRole::Critical];
    assert_eq!(paint.fill, Rgb(0xAA, 0, 0));
    // Contour gardé (celui du modèle), sombre et rôle voisin inchangés.
    assert_eq!(
        paint.outline,
        sobre.tokens(Variant::Light).bars[&BarRole::Critical].outline
    );
    assert_eq!(
        st.tokens(Variant::Dark).bars[&BarRole::Critical],
        sobre.tokens(Variant::Dark).bars[&BarRole::Critical]
    );
    assert_eq!(st.bars[&BarRole::Task], sobre.bars[&BarRole::Task]);
    let shape = st.bars[&BarRole::Critical];
    assert_eq!((shape.height_pct, shape.radius16), (60, 0));
    assert_eq!(shape.pattern, sobre.bars[&BarRole::Critical].pattern);
    let texts = st.texts[&BarRole::Critical];
    assert_eq!(texts.left, Some(planrs_render::style::BarField::Duration));
    assert_eq!(texts.right, None);
    // Second geste sur un autre rôle : le premier reste.
    call(
        &mut s,
        "style.set_bar_role",
        json!({ "role": "task", "color": "#004488", "variant": "dark" }),
    );
    let st = style::effective_style(s.project(), HEADLESS_DEFAULT);
    assert_eq!(
        st.tokens(Variant::Dark).bars[&BarRole::Task].fill,
        Rgb(0, 0x44, 0x88)
    );
    assert_eq!(
        st.tokens(Variant::Light).bars[&BarRole::Critical].fill,
        Rgb(0xAA, 0, 0)
    );
    // Ctrl+Z : couleur d'avant (H-UI-60).
    call(&mut s, "history.undo", json!({}));
    call(&mut s, "history.undo", json!({}));
    assert_eq!(style::effective_style(s.project(), HEADLESS_DEFAULT), sobre);
    // Sans style : part du modèle `base` (défaut Classique).
    let mut fresh = session();
    call(
        &mut fresh,
        "style.set_bar_role",
        json!({ "role": "task", "color": "#123456", "base": "chantier" }),
    );
    assert_eq!(
        style::doc_style(fresh.project()).unwrap().base,
        TemplateId::Chantier
    );
    // Refus : aucun réglage, couleur mal formée, épaisseur hors bornes.
    for params in [
        json!({ "role": "task" }),
        json!({ "role": "task", "color": "red" }),
        json!({ "role": "task", "thickness": 5 }),
        json!({ "role": "task", "variant": "dark" }),
    ] {
        let err = call_with(&mut s, &env(), "style.set_bar_role", params.clone()).unwrap_err();
        assert_eq!(err.code, codes::PARAM_INVALID, "{params}");
    }
}

/// H-UI-61 « Revenir au modèle » : toutes les surcharges, ou celles d'un
/// type de barre ; le modèle de base reste, une entrée d'annulation.
#[test]
fn r_style_reset_to_template() {
    let mut s = session();
    call(
        &mut s,
        "style.apply",
        json!({ "template": "sobre", "patch": { "density": "airy" } }),
    );
    call(
        &mut s,
        "style.set_bar_role",
        json!({ "role": "critical", "color": "#AA0000" }),
    );
    call(
        &mut s,
        "style.set_bar_role",
        json!({ "role": "task", "thickness": 50, "text_inside": "percent" }),
    );
    call(&mut s, "style.reset_to_template", json!({ "role": "critical" }));
    let doc = style::doc_style(s.project()).unwrap();
    assert!(
        doc.over.colors.is_none(),
        "couleur critique retirée, variantes vides retirées"
    );
    assert!(doc.over.bars.contains_key(&BarRole::Task));
    assert_eq!(doc.over.density, Some(planrs_render::style::Density::Airy));
    call(&mut s, "style.reset_to_template", json!({}));
    let doc = style::doc_style(s.project()).unwrap();
    assert_eq!(doc.base, TemplateId::Sobre);
    assert!(doc.over.is_empty());
    assert_eq!(payload(&s).as_deref(), Some(r#"{"base":"sobre","v":1}"#));
    call(&mut s, "history.undo", json!({}));
    assert!(!style::doc_style(s.project()).unwrap().over.is_empty());
    // Sans style : rien à faire, rien d'écrit.
    let mut fresh = session();
    call(&mut fresh, "style.reset_to_template", json!({}));
    assert!(payload(&fresh).is_none());
}

#[test]
fn style_get_and_list_views() {
    let mut s = session();
    let v = call(&mut s, "style.get", json!({}));
    assert_eq!(v["source"], "default");
    assert_eq!(v["base"], "classique");
    assert_eq!(v["resolved"]["name"], template(TemplateId::Classique).name);
    let l = call(&mut s, "style.list", json!({}));
    assert_eq!(l["templates"].as_array().unwrap().len(), 6);
    assert_eq!(l["templates"][1]["id"], "sobre");
    assert_eq!(l["document"], Value::Null);
    assert_eq!(l["default"], "classique");
    call(
        &mut s,
        "style.apply",
        json!({ "template": "chantier", "patch": { "density": "airy" } }),
    );
    let v = call(&mut s, "style.get", json!({}));
    assert_eq!(v["source"], "document");
    assert_eq!(v["base"], "chantier");
    assert_eq!(v["over"], json!({ "density": "airy" }));
    assert_eq!(v["resolved"]["density"], "airy");
    let l = call(&mut s, "style.list", json!({}));
    assert_eq!(l["document"], json!({ "base": "chantier", "customized": true }));
}

/// H-UI-61 : une couleur qui fait tomber un contraste sous H-UI-45 est
/// signalée, avec la couleur conforme la plus proche.
#[test]
fn style_get_reports_contrast_issue_with_suggestion() {
    let mut s = session();
    call(&mut s, "style.apply", json!({ "template": "sobre" }));
    let v = call(&mut s, "style.get", json!({}));
    assert_eq!(v["contrast_issues"], json!([]), "Sobre conforme");
    call(
        &mut s,
        "style.set_bar_role",
        json!({ "role": "task", "color": "#FFF59D" }),
    );
    let v = call(&mut s, "style.get", json!({}));
    let issues = v["contrast_issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0]["role"], "task");
    assert_eq!(issues[0]["variant"], "light");
    assert_eq!(issues[0]["min"], 3000);
    let suggested = Rgb::parse(issues[0]["suggestion"].as_str().unwrap()).unwrap();
    assert!(planrs_render::style::contrast_ratio(suggested, Rgb(0xFF, 0xFF, 0xFF)) >= 3000);
}

#[test]
fn style_save_as_template_reads_only() {
    let mut s = session();
    call(&mut s, "style.apply", json!({ "template": "presentation" }));
    let before = s.project().clone();
    let generation = s.generation();
    let v = call(
        &mut s,
        "style.save_as_template",
        json!({ "name": "  Client A  " }),
    );
    assert_eq!(
        v,
        json!({ "name": "Client A", "style": { "v": 1, "base": "presentation" } })
    );
    assert_eq!(s.project(), &before);
    assert_eq!(s.generation(), generation);
    for name in ["", "   ", &"x".repeat(65)] {
        let err = call_with(&mut s, &env(), "style.save_as_template", json!({ "name": name })).unwrap_err();
        assert_eq!(err.code, codes::PARAM_INVALID);
    }
}

/// `render.gantt` sans `style` dessine le style du document.
#[test]
fn render_gantt_uses_document_style() {
    let mut s = session();
    let classique = call(&mut s, "render.gantt", json!({}));
    assert!(
        classique["svg"]
            .as_str()
            .unwrap()
            .contains("font-family:sans-serif")
    );
    call(&mut s, "style.apply", json!({ "template": "sobre" }));
    let doc = call(&mut s, "render.gantt", json!({}));
    let sobre = call(&mut s, "render.gantt", json!({ "style": "sobre" }));
    assert_eq!(doc["svg"], sobre["svg"]);
    assert!(doc["svg"].as_str().unwrap().contains("Inter,sans-serif"));
}

/// H-R12 : version inconnue à la lecture ⇒ ignorée, diagnostic
/// `style.ignored`, modèle par défaut.
#[test]
fn h_r12_unknown_version_ignored() {
    let s = session();
    let base = xml(&s);
    for bad in [
        r#"{"base":"sobre","v":2}"#,
        r#"{"base":"sobre","v":1,"extra":true}"#,
        r#"{"base":"inconnu","v":1}"#,
        "pas du json",
    ] {
        let text = base.replacen("<Project", &format!("<?planrs-style {bad}?>\n<Project"), 1);
        let (opened, diags) = reopen(text.as_bytes());
        assert!(payload(&opened).is_none(), "{bad}");
        let d = diags
            .iter()
            .find(|d| d.code.code() == "style.ignored")
            .unwrap_or_else(|| panic!("{bad} : {diags:?}"));
        assert_eq!(
            d.code,
            ImportCode::StyleIgnored {
                reason: style_pi::StyleIgnoredReason::Invalid
            }
        );
        assert_eq!(
            style::effective_style(opened.project(), HEADLESS_DEFAULT),
            template(HEADLESS_DEFAULT)
        );
        // Réécrit sans instruction.
        assert_eq!(xml(&opened), base, "{bad}");
    }
}

/// P56 (aller-retour) : MSPDI avec style lu puis écrit = mêmes octets ; le
/// style relu égale le style écrit.
#[test]
fn p56_styled_session_roundtrip_same_bytes() {
    let mut s = session();
    call(
        &mut s,
        "style.apply",
        json!({ "template": "contraste_eleve", "patch": { "density": "compact" } }),
    );
    call(
        &mut s,
        "style.set_bar_role",
        json!({ "role": "milestone", "color": "#102030", "text_right": "finish" }),
    );
    let first = xml(&s);
    let (opened, diags) = reopen(first.as_bytes());
    assert!(
        diags.iter().all(|d| d.code.code() != "style.ignored"),
        "{diags:?}"
    );
    assert_eq!(style::doc_style(opened.project()), style::doc_style(s.project()));
    assert_eq!(xml(&opened), first);
    // Préférence « ne jamais écrire » : fichier sans style, octets J7.
    let off = WriteOptions {
        style: false,
        ..WriteOptions::default()
    };
    let plain = String::from_utf8(opened.save_bytes(&off).xml).unwrap();
    assert!(!plain.contains("planrs-style"));
}

/// H-UI-43 : document sans style ⇒ modèle par défaut de l'appelant.
#[test]
fn h_ui_43_default_template_for_unstyled_document() {
    let mut s = session();
    assert_eq!(
        style::effective_style(s.project(), TemplateId::Sobre),
        template(TemplateId::Sobre)
    );
    call(&mut s, "style.apply", json!({ "template": "presentation" }));
    assert_eq!(
        style::effective_style(s.project(), TemplateId::Sobre),
        template(TemplateId::Presentation)
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// P56 (lecture) : toute charge d'instruction donne le style du fichier
    /// (valide) ou le modèle par défaut avec diagnostic, jamais de panique.
    #[test]
    fn p56_any_pi_payload_opens(text in "[ -~]{0,200}", valid in any::<bool>()) {
        let s = session();
        let base = xml(&s);
        let body = if valid {
            r#"{"base":"sobre","over":{"density":"airy"},"v":1}"#.to_owned()
        } else {
            text.replace("?>", "")
        };
        let doc = base.replacen("<Project", &format!("<?planrs-style {body}?>\n<Project"), 1);
        let (opened, diags) = reopen(doc.as_bytes());
        let ignored = diags.iter().any(|d| d.code.code() == "style.ignored");
        match style::doc_style(opened.project()) {
            Some(_) => prop_assert!(!ignored),
            None => prop_assert!(ignored || body.trim().is_empty() && payload(&opened).is_none()),
        }
        if valid {
            prop_assert!(!ignored);
        }
    }
}
