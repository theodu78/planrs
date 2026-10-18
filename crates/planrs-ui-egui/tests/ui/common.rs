//! Harnais des captures de référence : mode de comparaison lu dans
//! `PLANRS_UI_SNAPSHOTS`, rendu wgpu limité à un adaptateur logiciel.
//!
//! - variable absente ou `0` : aucun rendu (le `LazyRenderer` par défaut de
//!   kittest n'est jamais initialisé, donc aucun GPU n'est touché) ; le
//!   scénario et ses assertions s'exécutent, une ligne sur stderr signale la
//!   capture non comparée. Cas des jobs `checks`, `tests-matrix` et des
//!   postes macOS et Windows.
//! - `1` : rendu par un adaptateur `DeviceType::Cpu` (lavapipe/llvmpipe)
//!   exclusivement, et comparaison avec `tests/snapshots/{nom}.png`
//!   (seuils dans `kittest.toml`). Cas du job CI `ui`.
//! - toute autre valeur : échec explicite.

use std::sync::Arc;

use eframe::egui_wgpu::{WgpuSetup, WgpuSetupCreateNew, wgpu};
use egui_kittest::wgpu::WgpuTestRenderer;
use egui_kittest::{Harness, LazyRenderer};
use planrs_ui_egui::{App, Env, LanguagePref, ThemePref, UiState};

/// Mode de comparaison des captures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotMode {
    /// Pas de rendu, pixels non comparés.
    Off,
    /// Rendu logiciel et comparaison exigés.
    Required,
}

/// Lit `PLANRS_UI_SNAPSHOTS` ; panique sur une valeur autre que `0` ou `1`.
pub fn snapshot_mode() -> SnapshotMode {
    match std::env::var("PLANRS_UI_SNAPSHOTS").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("0") => SnapshotMode::Off,
        Ok("1") => SnapshotMode::Required,
        other => panic!("PLANRS_UI_SNAPSHOTS doit valoir 0 ou 1 (ou être absente), reçu {other:?}"),
    }
}

/// Rendu des captures par un adaptateur logiciel uniquement : un GPU
/// matériel produirait des références incompatibles avec la CI.
/// `WGPU_BACKEND` reste respecté (la CI fixe `vulkan`) ; `WGPU_ADAPTER_NAME`
/// est sans effet, le sélecteur ci-dessous le court-circuite.
fn software_renderer() -> LazyRenderer {
    LazyRenderer::new(|| {
        let mut setup = WgpuSetupCreateNew::without_display_handle();
        setup
            .instance_descriptor
            .backends
            .remove(wgpu::Backends::BROWSER_WEBGPU);
        setup.native_adapter_selector = Some(Arc::new(
            |adapters: &[wgpu::Adapter], _surface: Option<&wgpu::Surface<'_>>| {
                adapters
                    .iter()
                    .find(|a| a.get_info().device_type == wgpu::DeviceType::Cpu)
                    .cloned()
                    .ok_or_else(|| {
                        "aucun adaptateur wgpu logiciel (lavapipe/llvmpipe) : installer \
                         mesa-vulkan-drivers, ou lancer sans PLANRS_UI_SNAPSHOTS=1"
                            .to_owned()
                    })
            },
        ));
        WgpuTestRenderer::from_setup(WgpuSetup::CreateNew(setup))
    })
}

/// Application de capture : langue et thème forcés (jamais « système »).
pub fn app(language: LanguagePref, theme: ThemePref) -> App {
    let state = UiState {
        language,
        theme,
        ..UiState::default()
    };
    App::new(state, Env::fixed(crate::now()))
}

/// Harnais de capture 1280×720, échelle 1, thème egui cohérent avec
/// `UiState.theme`, OS fixé ; rendu logiciel en mode `Required` seulement.
pub fn harness(language: LanguagePref, theme: ThemePref) -> Harness<'static, App> {
    let egui_theme = match theme {
        ThemePref::Dark => egui::Theme::Dark,
        ThemePref::Light | ThemePref::System => egui::Theme::Light,
    };
    let builder = Harness::builder()
        .with_size(egui::vec2(1280.0, 720.0))
        .with_pixels_per_point(1.0)
        .with_theme(egui_theme)
        .with_os(egui::os::OperatingSystem::Nix)
        // `ScrollToRow` anime le défilement : 10 images mesurées avant stabilité.
        .with_max_steps(16);
    let builder = match snapshot_mode() {
        SnapshotMode::Required => builder.renderer(software_renderer()),
        SnapshotMode::Off => builder,
    };
    builder.build_ui_state(planrs_ui_egui::ui, app(language, theme))
}

/// Compare la capture `name` (mode `Required`) ou signale qu'elle ne l'est
/// pas (mode `Off`).
#[track_caller]
pub fn snapshot(harness: &mut Harness<'static, App>, name: &str) {
    // Même scénario dans les deux modes : pointeur retiré (pas de survol).
    harness.remove_cursor();
    harness.run();
    match snapshot_mode() {
        SnapshotMode::Required => harness.snapshot(name),
        SnapshotMode::Off => {
            eprintln!("capture « {name} » non comparée : PLANRS_UI_SNAPSHOTS absent");
        }
    }
}
