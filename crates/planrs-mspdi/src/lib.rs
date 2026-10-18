//! Import et export du format MSPDI XML (Microsoft Project Data Interchange).
//!
//! MSPDI est le format natif de planrs en V1 : il garantit l'échange avec
//! MS Project. L'implémentation suit le schéma XML public de Microsoft
//! (`mspdi_pj15.xsd`, Project 2013 SDK, cité par URL et empreinte, jamais
//! commité) et le comportement observé sur des fichiers enregistrés par
//! MS Project (clean-room, règle 1). Aucun flottant : les décimales des
//! durées sont lues en entiers et arrondies par la règle unique R-001.
//!
//! État (J2, ADR-0009 ; J3, ADR-0010) : lecture ([`read`]) avec capture
//! opaque des éléments que l'écrivain ne régénère pas (R-128). Lecture événementielle
//! quick-xml, arbre borné par sous-arbre (pile explicite, limites de taille,
//! de profondeur, d'éléments et de texte ; DOCTYPE refusé), correspondance
//! typée indulgente : une valeur illisible devient un diagnostic et reçoit sa
//! valeur par défaut, jamais un échec global. Les valeurs calculées par
//! MS Project vont dans [`RecordedSchedule`], jamais dans le `Project` :
//! l'effacement exigé par l'ADR-0004 est structurel.
//!
//! Modules :
//! - [`scalar`] : booléens, entiers, dates, heures, durées ISO 8601,
//!   dixièmes de minute (R-070, R-072, R-073, R-075) ;
//! - [`codes`] : tables des codes numériques MSPDI vers les types du cœur ;
//! - [`error`] : erreurs structurelles typées, positions ;
//! - [`diag`] : diagnostics d'import ;
//! - [`written`] : ensembles W des balises régénérées par l'écrivain,
//!   contrat de la capture opaque (R-128).

#![deny(
    clippy::float_arithmetic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

pub mod codes;
pub mod diag;
pub mod error;
pub mod scalar;
pub mod written;

mod calendar;
mod fields;
mod project;
mod recorded;
mod task;
mod write;
mod xml;

use planrs_core::{Moment, Project, Subject};

pub use diag::{ImportCode, ImportDiagnostic, LinkIgnoredReason};
pub use error::{Location, MspdiError};
pub use recorded::{RecordedSchedule, RecordedTask, Tenths};

/// Plafond de la profondeur d'imbrication, quelle que soit
/// [`Limits::max_depth`] : la pile des éléments ouverts reste petite, et
/// aucune valeur réglée par l'appelant ne la rend dangereuse (règle 10).
pub const MAX_DEPTH: u16 = 256;

/// Limites de lecture : protègent contre un document hostile ou corrompu.
///
/// Mémoire : les éléments sautés ne coûtent qu'un compteur ; seuls les
/// éléments retenus (champs lus des tâches, calendriers, affectations et du
/// projet) sont construits, environ 100 octets chacun, bornés par
/// `max_retained` ; chaque charge opaque capturée (R-128) compte aussi
/// comme un élément retenu, et leur total est borné par
/// `max_opaque_bytes` ; les diagnostics d'import sont plafonnés (10 000 au
/// total, 1 000 par code). Avec les valeurs par défaut, la mémoire de la
/// lecture reste de l'ordre de quelques centaines de Mio au pire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Taille maximale de l'entrée, en octets.
    pub max_bytes: u64,
    /// Profondeur maximale d'imbrication, plafonnée à [`MAX_DEPTH`].
    pub max_depth: u16,
    /// Nombre maximal d'éléments (retenus ou sautés).
    pub max_elements: u64,
    /// Nombre maximal d'éléments retenus (construits en mémoire).
    pub max_retained: u64,
    /// Longueur maximale du texte d'un élément, en octets.
    pub max_text_bytes: usize,
    /// Total maximal des charges opaques capturées (R-128), en octets ; au
    /// premier dépassement, cette capture et toutes les suivantes sont
    /// abandonnées (`mspdi.truncated`, [`SourceInfo::opaque_dropped`]).
    pub max_opaque_bytes: u64,
}

impl Default for Limits {
    /// 256 Mio, 64 niveaux, 20 000 000 éléments dont 3 000 000 retenus
    /// (environ 60 000 tâches de 50 champs, charges opaques comprises),
    /// 16 Mio de texte, 64 Mio de charges opaques.
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_depth: 64,
            max_elements: 20_000_000,
            max_retained: 3_000_000,
            max_text_bytes: 16 * 1024 * 1024,
            max_opaque_bytes: 64 * 1024 * 1024,
        }
    }
}

/// Options de lecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReadOptions {
    /// « Maintenant » injecté (règle 5) : date courante du projet si
    /// `CurrentDate` est absent, début du projet si `StartDate` l'est.
    pub current_date: Moment,
    /// Limites.
    pub limits: Limits,
}

impl ReadOptions {
    /// Options avec les limites par défaut.
    #[must_use]
    pub fn new(current_date: Moment) -> Self {
        Self {
            current_date,
            limits: Limits::default(),
        }
    }
}

/// Provenance du document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceInfo {
    /// `SaveVersion` (14 pour MS Project 2010 et suivants).
    pub save_version: Option<i64>,
    /// Espace de noms déclaré par la racine.
    pub namespace: Option<String>,
    /// Sections d'interface présentes (`Views`, `Tables`, `Filters`, `Groups`,
    /// `Maps`, `Reports`, `Drawings`, `DataLinks`, `VBAProjects`) : signature d'un
    /// enregistrement par MS Project.
    pub ui_sections: Vec<String>,
    /// Éléments sautés (Resources, Views, contenu des affectations, champs
    /// non lus des tâches et des calendriers…).
    pub skipped_elements: u64,
    /// Diagnostics d'import supprimés au-delà des plafonds
    /// ([`diag::MAX_DIAGNOSTICS`], [`diag::MAX_DIAGNOSTICS_PER_CODE`]) ;
    /// un diagnostic [`ImportCode::Truncated`] les signale aussi.
    pub dropped_diagnostics: u64,
    /// Total des charges opaques gardées dans les `Extras` (R-128), en
    /// octets ; au plus [`Limits::max_opaque_bytes`].
    pub opaque_bytes: u64,
    /// Charges opaques abandonnées (plafond, entité non résolue), dans
    /// l'ordre du document ; au plus [`MAX_OPAQUE_LOSSES`] entrées.
    pub opaque_dropped: Vec<OpaqueLoss>,
    /// Nombre total de charges opaques abandonnées, listées ou non.
    pub opaque_dropped_total: u64,
}

/// Origine des entrées d'`Extras` capturées par la lecture MSPDI (R-128).
pub const EXTRAS_ORIGIN: &str = "mspdi";

/// Nombre maximal de pertes de charges opaques listées dans
/// [`SourceInfo::opaque_dropped`] ; au-delà, seul
/// [`SourceInfo::opaque_dropped_total`] croît.
pub const MAX_OPAQUE_LOSSES: usize = 1_000;

/// Cause de l'abandon d'une charge opaque (R-128).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpaqueLossReason {
    /// Plafond atteint ([`Limits::max_opaque_bytes`] ou
    /// [`Limits::max_retained`]) : cette capture et toutes les suivantes
    /// sont abandonnées (`mspdi.truncated`).
    Cap,
    /// Entité non résolue dans la charge (`&nbsp;`…) : réémise telle quelle,
    /// elle rendrait le document mal formé (`mspdi.inconsistent` signale
    /// l'entité).
    UnresolvedEntity,
    /// Bornes de la charge hors de l'entrée (jamais attendu).
    OutOfBounds,
}

/// Charge opaque abandonnée (R-128).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpaqueLoss {
    /// Entité porteuse : projet, tâche ou calendrier ; `None` si son UID
    /// n'a pas pu être lu.
    pub subject: Option<Subject>,
    /// Clé : nom qualifié de l'élément, tel qu'écrit.
    pub key: String,
    /// Taille de l'élément, balise englobante comprise, en octets.
    pub bytes: u64,
    /// Cause.
    pub reason: OpaqueLossReason,
    /// Position du début de l'élément.
    pub at: Location,
}

/// Document MSPDI lu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MspdiDocument {
    /// Entrées seules (ADR-0008), options CPM comprises.
    pub project: Project,
    /// Valeurs calculées telles qu'enregistrées : jamais dans `project`.
    pub recorded: RecordedSchedule,
    /// Provenance.
    pub source: SourceInfo,
    /// Diagnostics d'import, puis ceux de `Project::validate` (enveloppés).
    pub diagnostics: Vec<ImportDiagnostic>,
}

/// BOM UTF-8.
const BOM_UTF8: &[u8] = &[0xEF, 0xBB, 0xBF];

/// Lit un document MSPDI. Aucune E/S : l'entrée est en octets (testable,
/// wasm).
///
/// # Errors
///
/// [`MspdiError`] seulement si le document est structurellement
/// inexploitable ; tout le reste devient un diagnostic d'import.
pub fn read(input: &[u8], options: &ReadOptions) -> Result<MspdiDocument, MspdiError> {
    let limits = options.limits;
    let size = u64::try_from(input.len()).unwrap_or(u64::MAX);
    if size > limits.max_bytes {
        return Err(MspdiError::TooLarge {
            size,
            max: limits.max_bytes,
        });
    }
    if matches!(input, [0xFE, 0xFF, ..] | [0xFF, 0xFE, ..]) {
        return Err(MspdiError::Utf16NotSupported);
    }
    let body = input.strip_prefix(BOM_UTF8).unwrap_or(input);
    if body.is_empty() {
        return Err(MspdiError::Empty);
    }
    let text = std::str::from_utf8(body).map_err(|e| MspdiError::InvalidUtf8 {
        at: error::locate_in_bytes(body, e.valid_up_to()),
    })?;
    let lines = error::LineIndex::new(text);
    let mut sink = diag::Sink::new(&lines);
    let mut raw = xml::read_document(text, limits, &lines, &mut sink)?;
    let mut budget = std::mem::take(&mut raw.budget);
    let project::Built {
        project,
        recorded,
        save_version,
        mut losses,
    } = project::build(&raw, &mut budget, options, &mut sink);
    budget.report(&mut sink);
    losses.sort_by_key(|l| l.at);
    let source = SourceInfo {
        save_version,
        namespace: raw.namespace.clone(),
        ui_sections: raw.ui_sections.clone(),
        skipped_elements: raw.skipped_elements,
        dropped_diagnostics: sink.dropped,
        opaque_bytes: budget.kept_bytes,
        opaque_dropped: losses
            .into_iter()
            .map(|l| OpaqueLoss {
                subject: l.subject,
                key: l.key,
                bytes: l.bytes,
                reason: l.reason,
                at: lines.locate(l.at),
            })
            .collect(),
        opaque_dropped_total: budget.dropped_total,
    };
    let mut diagnostics = sink.finish();
    diagnostics.extend(
        project
            .validate()
            .finish()
            .into_iter()
            .map(ImportDiagnostic::core),
    );
    Ok(MspdiDocument {
        project,
        recorded,
        source,
        diagnostics,
    })
}
