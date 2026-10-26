//! Rendu hors écran d'une vue de l'interface en PNG, sans fenêtre (brief § 8,
//! « vérification visuelle par l'agent »).
//!
//! ```text
//! cargo run -p planrs-ui-egui --example snapshot -- <fichier.xml | --generate N> <vue>
//!     [--row N] [--zoom day|week|month|quarter] [--size LxH] [sortie.png]
//! ```
//!
//! Vues : `welcome` (accueil, la source est ignorée), `main` (zoom ajusté),
//! `main-week`, `main-day` (défilée deux jours avant la date d'état),
//! `main-dark`, `main-en`, `tracking` (Gantt de suivi), `resources`
//! (feuille des ressources), `usage` (utilisation des ressources). Français
//! et thème clair sauf mention. `--generate N` : réseau généré sans MSPDI
//! (`planrs_tests::generate::ui_network_with_resources`, N arrondi au
//! multiple de 20 supérieur, deux liens par tâche, une ressource pour 25
//! tâches), comme le test de fluidité. `--row N` : sélectionne la ligne de
//! tâche N (depuis 0), y défile la table et le Gantt (début de la barre) ;
//! `--zoom` : échelle imposée après la
//! vue ; `--size` : fenêtre simulée (défaut 1280×720) ; `--style` : modèle
//! par défaut des préférences (`classique`, `sobre`, `contraste_eleve`,
//! `presentation`, `chantier`, `impression_nb` ; J8 lot L11), appliqué à un
//! document sans style. Sortie par défaut :
//! `planrs-snapshot-<vue>.png` dans le répertoire courant.
//!
//! Même harnais que les captures de référence (`tests/ui/common.rs`) :
//! `egui_kittest` + wgpu, 1 px par point, OS fixé, « maintenant » fixe. Un
//! adaptateur logiciel (lavapipe/llvmpipe, `DeviceType::Cpu`) est préféré
//! pour des pixels identiques à ceux de la CI ; à défaut, le premier
//! adaptateur disponible est utilisé et signalé. Toute erreur (arguments,
//! lecture, document refusé, absence d'adaptateur, écriture) est affichée sur
//! la sortie d'erreur avec un code de sortie non nul : jamais de panique
//! volontaire. Le test `snapshot_generate_and_views` (`tests/ui/desktop.rs`)
//! inclut ce fichier et prépare chaque vue sans rendu.

use std::ffi::OsString;
use std::future::Future;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use eframe::egui_wgpu::{self, RenderState, WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew, wgpu};
use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use planrs_core::Moment;
use planrs_render::style::TemplateId;
use planrs_ui_egui::timescale::TimeAxis;
use planrs_ui_egui::{
    Action, App, DocOrigin, Env, GanttStyle, LanguagePref, ThemePref, TimeScaleZoom, UiState, ViewKind,
};

/// Usage affiché en cas d'arguments invalides.
pub const USAGE: &str = "usage : cargo run -p planrs-ui-egui --example snapshot -- \
<fichier.xml | --generate N> <vue> [--row N] [--zoom day|week|month|quarter] [--size LxH] [--style <modèle>] [sortie.png]\n\
vues : welcome | main | main-week | main-day | main-dark | main-en | tracking | resources | usage\n\
modèles : classique | sobre | contraste_eleve | presentation | chantier | impression_nb";

/// Taille par défaut de la fenêtre simulée (celle des captures de référence).
pub const SIZE: (f32, f32) = (1280.0, 720.0);

/// Côté de la fenêtre simulée, en points : bornes acceptées par `--size`.
const SIDE: (u32, u32) = (320, 4096);

/// Tâches générées au plus (`--generate`).
pub const MAX_GENERATED: usize = 20_000;

/// Feuilles par récapitulative du réseau généré (20 tâches par lot).
const LEAVES: usize = 19;

/// Images au plus pour que l'interface se stabilise.
const MAX_STEPS: u64 = 16;

/// Vue demandée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// Écran d'accueil.
    Welcome,
    /// Vue principale, zoom ajusté.
    Main,
    /// Vue principale au zoom Semaine.
    MainWeek,
    /// Vue principale au zoom Jour, défilée vers la date d'état.
    MainDay,
    /// Vue principale, thème sombre.
    MainDark,
    /// Vue principale en anglais.
    MainEn,
    /// Gantt de suivi.
    Tracking,
    /// Feuille des ressources.
    Resources,
    /// Utilisation des ressources.
    Usage,
}

impl View {
    /// Vue nommée sur la ligne de commande.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "welcome" => Some(Self::Welcome),
            "main" => Some(Self::Main),
            "main-week" => Some(Self::MainWeek),
            "main-day" => Some(Self::MainDay),
            "main-dark" => Some(Self::MainDark),
            "main-en" => Some(Self::MainEn),
            "tracking" => Some(Self::Tracking),
            "resources" => Some(Self::Resources),
            "usage" => Some(Self::Usage),
            _ => None,
        }
    }

    /// Langue et thème de la vue.
    const fn prefs(self) -> (LanguagePref, ThemePref) {
        match self {
            Self::MainDark => (LanguagePref::Fr, ThemePref::Dark),
            Self::MainEn => (LanguagePref::En, ThemePref::Light),
            _ => (LanguagePref::Fr, ThemePref::Light),
        }
    }

    /// Vue principale et style du Gantt.
    const fn kind(self) -> (ViewKind, GanttStyle) {
        match self {
            Self::Tracking => (ViewKind::Gantt, GanttStyle::Tracking),
            Self::Resources => (ViewKind::ResourceSheet, GanttStyle::Standard),
            Self::Usage => (ViewKind::ResourceUsage, GanttStyle::Standard),
            _ => (ViewKind::Gantt, GanttStyle::Standard),
        }
    }
}

/// Projet à ouvrir.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// Fichier MSPDI.
    File(PathBuf),
    /// Réseau généré d'environ N tâches.
    Generate(usize),
}

/// Arguments de la ligne de commande.
#[derive(Clone, Debug, PartialEq)]
pub struct Args {
    /// Projet à ouvrir.
    pub source: Source,
    /// Vue à rendre.
    pub view: View,
    /// Nom de la vue, tel que saisi.
    pub view_name: String,
    /// Ligne à montrer et sélectionner.
    pub row: Option<usize>,
    /// Zoom imposé.
    pub zoom: Option<TimeScaleZoom>,
    /// Taille de la fenêtre simulée, en points.
    pub size: (f32, f32),
    /// Modèle par défaut des préférences (`None` : Classique).
    pub style: Option<TemplateId>,
    /// PNG à écrire.
    pub output: PathBuf,
}

/// Valeur UTF-8 de l'option `name`.
fn option_value(name: &str, value: Option<OsString>) -> Result<String, String> {
    value
        .ok_or_else(|| format!("valeur manquante après {name}"))?
        .into_string()
        .map_err(|_| format!("valeur illisible après {name}"))
}

/// Entier positif d'une option.
fn parse_count(name: &str, text: &str) -> Result<usize, String> {
    text.parse::<usize>()
        .map_err(|_| format!("{name} attend un entier, reçu « {text} »"))
}

/// Zoom nommé.
fn parse_zoom(text: &str) -> Result<TimeScaleZoom, String> {
    match text {
        "day" => Ok(TimeScaleZoom::Day),
        "week" => Ok(TimeScaleZoom::Week),
        "month" => Ok(TimeScaleZoom::Month),
        "quarter" => Ok(TimeScaleZoom::Quarter),
        other => Err(format!("zoom inconnu : {other}")),
    }
}

/// Modèle nommé (`contraste_eleve` ou `contraste-eleve`).
fn parse_style(text: &str) -> Result<TemplateId, String> {
    let key = text.replace('-', "_");
    TemplateId::ALL
        .into_iter()
        .find(|id| id.key() == key)
        .ok_or_else(|| format!("modèle inconnu : {text}"))
}

/// Taille `LxH` (ou `L×H`), chaque côté dans [`SIDE`].
fn parse_size(text: &str) -> Result<(f32, f32), String> {
    let invalid = || {
        format!(
            "--size attend LxH (côtés de {} à {}), reçu « {text} »",
            SIDE.0, SIDE.1
        )
    };
    let (w, h) = text.split_once(['x', 'X', '×']).ok_or_else(invalid)?;
    let side = |s: &str| {
        s.trim()
            .parse::<u32>()
            .ok()
            .filter(|v| (SIDE.0..=SIDE.1).contains(v))
            .ok_or_else(invalid)
    };
    let (w, h) = (side(w)?, side(h)?);
    // Côtés ≤ 4 096 : exacts en f32.
    #[allow(clippy::cast_precision_loss, reason = "côtés bornés à 4 096, exacts en f32")]
    Ok((w as f32, h as f32))
}

/// Lit les arguments (sans le nom du programme).
///
/// # Errors
///
/// Argument manquant, inconnu ou invalide : message à afficher avec
/// [`USAGE`].
pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
    let mut args = args.into_iter();
    let mut positional: Vec<OsString> = Vec::new();
    let mut source = None;
    let mut row = None;
    let mut zoom = None;
    let mut size = SIZE;
    let mut style = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--generate") => {
                let n = parse_count("--generate", &option_value("--generate", args.next())?)?;
                if n == 0 || n > MAX_GENERATED {
                    return Err(format!("--generate attend de 1 à {MAX_GENERATED} tâches"));
                }
                source = Some(Source::Generate(n));
            }
            Some("--row") => row = Some(parse_count("--row", &option_value("--row", args.next())?)?),
            Some("--zoom") => zoom = Some(parse_zoom(&option_value("--zoom", args.next())?)?),
            Some("--size") => size = parse_size(&option_value("--size", args.next())?)?,
            Some("--style") => style = Some(parse_style(&option_value("--style", args.next())?)?),
            Some(other) if other.starts_with("--") => return Err(format!("option inconnue : {other}")),
            _ => positional.push(arg),
        }
    }
    let mut positional = positional.into_iter();
    let source = match source {
        Some(source) => source,
        None => Source::File(
            positional
                .next()
                .map(PathBuf::from)
                .ok_or("fichier MSPDI (ou --generate N) manquant")?,
        ),
    };
    let view_name = positional
        .next()
        .ok_or("vue manquante")?
        .into_string()
        .map_err(|_| "nom de vue illisible".to_owned())?;
    let view = View::parse(&view_name).ok_or_else(|| format!("vue inconnue : {view_name}"))?;
    let output = positional.next().map_or_else(
        || PathBuf::from(format!("planrs-snapshot-{view_name}.png")),
        PathBuf::from,
    );
    if positional.next().is_some() {
        return Err("trop d'arguments".to_owned());
    }
    Ok(Args {
        source,
        view,
        view_name,
        row,
        zoom,
        size,
        style,
        output,
    })
}

/// Réveil d'un fil endormi par [`block_on`].
struct ThreadWaker(std::thread::Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Exécute un futur jusqu'au bout sur le fil courant (création de l'état
/// wgpu, asynchrone ; aucun runtime en dépendance).
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::park(),
        }
    }
}

/// État wgpu sans fenêtre : adaptateur logiciel préféré, sinon le premier.
fn render_state() -> Result<RenderState, String> {
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup
        .instance_descriptor
        .backends
        .remove(wgpu::Backends::BROWSER_WEBGPU);
    setup.native_adapter_selector = Some(Arc::new(
        |adapters: &[wgpu::Adapter], _surface: Option<&wgpu::Surface<'_>>| {
            let cpu = adapters
                .iter()
                .find(|a| a.get_info().device_type == wgpu::DeviceType::Cpu);
            match cpu.or_else(|| adapters.first()) {
                Some(adapter) => {
                    let info = adapter.get_info();
                    if info.device_type != wgpu::DeviceType::Cpu {
                        eprintln!(
                            "snapshot : aucun adaptateur logiciel, rendu par « {} » ({:?}) : \
                             les pixels peuvent différer de la CI",
                            info.name, info.backend
                        );
                    }
                    Ok(adapter.clone())
                }
                None => Err("aucun adaptateur wgpu (installer mesa-vulkan-drivers pour lavapipe)".to_owned()),
            }
        },
    ));
    let setup = WgpuSetup::CreateNew(setup);
    let instance = block_on(setup.new_instance());
    let config = WgpuConfiguration {
        wgpu_setup: setup,
        ..WgpuConfiguration::default()
    };
    block_on(RenderState::create(
        &config,
        &instance,
        None,
        egui_wgpu::RendererOptions::PREDICTABLE,
    ))
    .map_err(|error| format!("initialisation wgpu impossible : {error}"))
}

/// Laisse l'interface se stabiliser (au plus [`MAX_STEPS`] images).
fn settle(harness: &mut Harness<'static, App>) -> Result<(), String> {
    harness
        .try_run()
        .map(|_| ())
        .map_err(|error| format!("l'interface ne se stabilise pas : {error}"))
}

/// « Maintenant » fixe : date d'état de l'exemple, ou début du réseau
/// généré (celui du test de fluidité).
fn fixed_now(source: &Source) -> Result<Moment, String> {
    let text = match source {
        Source::File(_) => "2027-01-15T08:00",
        Source::Generate(_) => "2026-01-05T08:00",
    };
    text.parse()
        .map_err(|error| format!("date fixe invalide : {error}"))
}

/// Ouvre la source dans l'application.
fn open(app: &mut App, source: &Source) -> Result<(), String> {
    match source {
        Source::File(path) => {
            let bytes = std::fs::read(path).map_err(|error| format!("{} : {error}", path.display()))?;
            let origin = DocOrigin {
                name: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: path.to_str().map(str::to_owned),
                demo: false,
            };
            app.open_bytes(&bytes, origin);
        }
        Source::Generate(n) => {
            let summaries = n.div_ceil(LEAVES + 1).max(1);
            let tasks = summaries.saturating_mul(LEAVES + 1);
            let resources = (tasks / 25).max(1);
            let project = planrs_tests::generate::ui_network_with_resources(
                summaries,
                LEAVES,
                tasks.saturating_mul(2),
                resources,
            );
            let origin = DocOrigin {
                name: format!("réseau généré ({tasks} tâches)"),
                path: None,
                demo: false,
            };
            app.open_project(project, origin);
        }
    }
    if app.document.is_none() {
        let detail = app
            .messages()
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>()
            .join(" ; ");
        return Err(format!("document refusé : {detail}"));
    }
    Ok(())
}

/// Prépare la vue demandée dans le harnais.
fn prepare(harness: &mut Harness<'static, App>, args: &Args) -> Result<(), String> {
    settle(harness)?;
    if args.view == View::Welcome {
        return Ok(());
    }
    open(harness.state_mut(), &args.source)?;
    settle(harness)?;
    let (kind, style) = args.view.kind();
    harness.state_mut().dispatch(Action::SetView(kind));
    harness.state_mut().dispatch(Action::SetGanttStyle(style));
    settle(harness)?;
    match args.view {
        View::MainWeek => {
            harness.state_mut().dispatch(Action::SetZoom(TimeScaleZoom::Week));
            settle(harness)?;
        }
        View::MainDay => {
            harness.state_mut().dispatch(Action::SetZoom(TimeScaleZoom::Day));
            settle(harness)?;
            // Abscisse de la date d'état par l'axe de l'interface (temps →
            // pixels), deux jours de marge à gauche.
            let target = harness.state().document.as_ref().map(|doc| {
                let axis = TimeAxis::new(doc.range, TimeScaleZoom::Day, 0.0);
                axis.x(doc.project().effective_status_date()) - 2.0 * axis.px_per_day()
            });
            if let Some(target) = target.filter(|t| t.is_finite()) {
                let delta = target.max(0.0) - harness.state().view().gantt_scroll_x;
                harness.state_mut().dispatch(Action::ScrollGanttX(delta));
                settle(harness)?;
            }
        }
        _ => {}
    }
    if let Some(zoom) = args.zoom {
        harness.state_mut().dispatch(Action::SetZoom(zoom));
        settle(harness)?;
    }
    if let Some(row) = args.row {
        let (uid, bar) = harness
            .state()
            .rows()
            .and_then(|rows| rows.row(row))
            .map(|r| (r.uid, r.bar))
            .ok_or_else(|| format!("--row {row} : ligne absente"))?;
        harness.state_mut().dispatch(Action::Select(Some(uid)));
        harness.state_mut().dispatch(Action::ScrollToRow(row));
        settle(harness)?;
        // Gantt : défilé jusqu'au début de la barre (axe de l'interface,
        // deux jours de marge à gauche).
        let app = harness.state();
        let target = bar.zip(app.document.as_ref()).map(|((start, _), doc)| {
            let axis = TimeAxis::new(doc.range, app.state.zoom, 0.0);
            axis.x(start) - 2.0 * axis.px_per_day()
        });
        if let Some(target) = target.filter(|t| t.is_finite()) {
            let delta = target.max(0.0) - app.view().gantt_scroll_x;
            harness.state_mut().dispatch(Action::ScrollGanttX(delta));
            settle(harness)?;
        }
    }
    Ok(())
}

/// Harnais prêt à rendre la vue demandée : `renderer` absent, aucun rendu
/// possible (préparation seule, tests).
///
/// # Errors
///
/// Document refusé, ligne absente, interface instable.
pub fn prepared(args: &Args, renderer: Option<WgpuTestRenderer>) -> Result<Harness<'static, App>, String> {
    let (language, theme) = args.view.prefs();
    let app = App::new(
        UiState {
            language,
            theme,
            default_template: args.style.unwrap_or(TemplateId::Classique),
            ..UiState::default()
        },
        Env::fixed(fixed_now(&args.source)?),
    );
    let egui_theme = if theme == ThemePref::Dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    let mut builder = Harness::builder()
        .with_size(egui::vec2(args.size.0, args.size.1))
        .with_pixels_per_point(1.0)
        .with_theme(egui_theme)
        .with_os(egui::os::OperatingSystem::Nix)
        .with_max_steps(MAX_STEPS);
    if let Some(renderer) = renderer {
        builder = builder.renderer(renderer);
    }
    let mut harness = builder.build_ui_state(planrs_ui_egui::ui, app);
    prepare(&mut harness, args)?;
    harness.remove_cursor();
    settle(&mut harness)?;
    Ok(harness)
}

/// Rend la vue et écrit le PNG.
///
/// # Errors
///
/// Toute erreur de préparation, de rendu ou d'écriture.
pub fn run(args: &Args) -> Result<(), String> {
    let renderer = WgpuTestRenderer::from_render_state(render_state()?);
    let mut harness = prepared(args, Some(renderer))?;
    let image = harness.render()?;
    image
        .save(&args.output)
        .map_err(|error| format!("{} : {error}", args.output.display()))?;
    let app = harness.state();
    let tasks = app.document.as_ref().map_or(0, |doc| doc.project().tasks.len());
    println!(
        "{} : vue {}, {}×{} px, {tasks} tâches, zoom {:?}, lignes {:?}",
        args.output.display(),
        args.view_name,
        image.width(),
        image.height(),
        app.state.zoom,
        app.last_frame.table_rows,
    );
    Ok(())
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args_os().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("snapshot : {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("snapshot : {error}");
            ExitCode::FAILURE
        }
    }
}
