//! Rendu hors écran d'une vue de l'interface en PNG, sans fenêtre (brief § 8,
//! « vérification visuelle par l'agent »).
//!
//! ```text
//! cargo run -p planrs-ui-egui --example snapshot -- <fichier.xml> <vue> [sortie.png]
//! ```
//!
//! Vues : `welcome` (accueil, le fichier est ignoré), `main` (zoom ajusté),
//! `main-week`, `main-day` (défilée deux jours avant la date d'état),
//! `main-dark`, `main-en`. Français et thème clair sauf mention. Sortie par
//! défaut : `planrs-snapshot-<vue>.png` dans le répertoire courant.
//!
//! Même harnais que les captures de référence (`tests/ui/common.rs`) :
//! `egui_kittest` + wgpu, 1280×720, 1 px par point, OS fixé, « maintenant »
//! fixe. Un adaptateur logiciel (lavapipe/llvmpipe, `DeviceType::Cpu`) est
//! préféré pour des pixels identiques à ceux de la CI ; à défaut, le premier
//! adaptateur disponible est utilisé et signalé. Toute erreur (arguments,
//! lecture, document refusé, absence d'adaptateur, écriture) est affichée sur
//! la sortie d'erreur avec un code de sortie non nul : jamais de panique
//! volontaire.

use std::future::Future;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

use eframe::egui_wgpu::{self, RenderState, WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew, wgpu};
use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use planrs_core::Moment;
use planrs_ui_egui::timescale::TimeAxis;
use planrs_ui_egui::{Action, App, DocOrigin, Env, LanguagePref, ThemePref, TimeScaleZoom, UiState};

/// Usage affiché en cas d'arguments invalides.
const USAGE: &str = "usage : cargo run -p planrs-ui-egui --example snapshot -- <fichier.xml> <vue> [sortie.png]\n\
vues : welcome | main | main-week | main-day | main-dark | main-en";

/// Taille de la fenêtre simulée (celle des captures de référence).
const SIZE: (f32, f32) = (1280.0, 720.0);

/// Images au plus pour que l'interface se stabilise.
const MAX_STEPS: u64 = 16;

/// Vue demandée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
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
}

impl View {
    /// Vue nommée sur la ligne de commande.
    fn parse(name: &str) -> Option<Self> {
        match name {
            "welcome" => Some(Self::Welcome),
            "main" => Some(Self::Main),
            "main-week" => Some(Self::MainWeek),
            "main-day" => Some(Self::MainDay),
            "main-dark" => Some(Self::MainDark),
            "main-en" => Some(Self::MainEn),
            _ => None,
        }
    }

    /// Langue et thème de la vue.
    const fn prefs(self) -> (LanguagePref, ThemePref) {
        match self {
            Self::MainDark => (LanguagePref::Fr, ThemePref::Dark),
            Self::MainEn => (LanguagePref::En, ThemePref::Light),
            Self::Welcome | Self::Main | Self::MainWeek | Self::MainDay => {
                (LanguagePref::Fr, ThemePref::Light)
            }
        }
    }
}

/// Arguments de la ligne de commande.
struct Args {
    /// Fichier MSPDI.
    input: PathBuf,
    /// Vue à rendre.
    view: View,
    /// Nom de la vue, tel que saisi.
    view_name: String,
    /// PNG à écrire.
    output: PathBuf,
}

/// Lit les arguments : deux obligatoires, un facultatif.
fn parse_args() -> Result<Args, String> {
    let mut args = std::env::args_os().skip(1);
    let input = args.next().map(PathBuf::from).ok_or("fichier MSPDI manquant")?;
    let view_name = args
        .next()
        .ok_or("vue manquante")?
        .into_string()
        .map_err(|_| "nom de vue illisible".to_owned())?;
    let view = View::parse(&view_name).ok_or_else(|| format!("vue inconnue : {view_name}"))?;
    let output = args.next().map_or_else(
        || PathBuf::from(format!("planrs-snapshot-{view_name}.png")),
        PathBuf::from,
    );
    if args.next().is_some() {
        return Err("trop d'arguments".to_owned());
    }
    Ok(Args {
        input,
        view,
        view_name,
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

/// Prépare la vue demandée dans le harnais.
fn prepare(harness: &mut Harness<'static, App>, args: &Args) -> Result<(), String> {
    settle(harness)?;
    if args.view == View::Welcome {
        return Ok(());
    }
    let bytes = std::fs::read(&args.input).map_err(|error| format!("{} : {error}", args.input.display()))?;
    let origin = DocOrigin {
        name: args
            .input
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: args.input.to_str().map(str::to_owned),
        demo: false,
    };
    harness.state_mut().open_bytes(&bytes, origin);
    if harness.state().document.is_none() {
        let detail = harness
            .state()
            .messages()
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>()
            .join(" ; ");
        return Err(format!("document refusé : {detail}"));
    }
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
        View::Welcome | View::Main | View::MainDark | View::MainEn => {}
    }
    Ok(())
}

/// Rend la vue et écrit le PNG.
fn run(args: &Args) -> Result<(), String> {
    let (language, theme) = args.view.prefs();
    let now: Moment = "2027-01-15T08:00"
        .parse()
        .map_err(|error| format!("date fixe invalide : {error}"))?;
    let app = App::new(
        UiState {
            language,
            theme,
            ..UiState::default()
        },
        Env::fixed(now),
    );
    let egui_theme = if theme == ThemePref::Dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    let renderer = WgpuTestRenderer::from_render_state(render_state()?);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(SIZE.0, SIZE.1))
        .with_pixels_per_point(1.0)
        .with_theme(egui_theme)
        .with_os(egui::os::OperatingSystem::Nix)
        .with_max_steps(MAX_STEPS)
        .renderer(renderer)
        .build_ui_state(planrs_ui_egui::ui, app);
    prepare(&mut harness, args)?;
    harness.remove_cursor();
    settle(&mut harness)?;
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
    let args = match parse_args() {
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
