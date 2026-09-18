//! Shared state behind the API: the store, the running societies, presets
//! copy, the mail sender, limiters, and the presence throttle.

use crate::actor::SocietyHandle;
use crate::limiter::RateLimiter;
use crate::mail::MailSender;
use crate::runtime::Runtime;
use crate::scheduler::{Control, Schedule};
use isms_api_types::Clock;
use isms_core::ids::CitizenId;
use isms_core::world::World;
use isms_store::{PgEventStore, SocietyRow};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

/// One running society as the API sees it.
#[derive(Clone, Debug)]
pub struct SocietyEntry {
    pub row: SocietyRow,
    pub handle: SocietyHandle,
    pub control: Arc<Control>,
}

impl SocietyEntry {
    /// The clock as it stands now (an operator may have re-anchored it).
    #[must_use]
    pub fn schedule(&self) -> Schedule {
        self.control.schedule()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub store: PgEventStore,
    pub societies: Arc<RwLock<BTreeMap<i64, SocietyEntry>>>,
    pub presets_dir: PathBuf,
    pub mail: Arc<dyn MailSender>,
    /// Public origin for links in mail and redirects, e.g. `https://isms.example`.
    pub base_url: String,
    pub limiter: Arc<RateLimiter>,
    /// Account emails allowed the `/admin` routes (`ISMS_OPERATORS`, S1.13c).
    pub operators: Arc<BTreeSet<String>>,
    /// Last tick at which each citizen was marked seen, so a `Seen` command
    /// goes to the actor at most once per tick per citizen.
    seen: Arc<Mutex<HashMap<(i64, CitizenId), u32>>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("base_url", &self.base_url)
            .field("presets_dir", &self.presets_dir)
            .finish_non_exhaustive()
    }
}

impl AppState {
    #[must_use]
    pub fn new(
        store: PgEventStore,
        societies: BTreeMap<i64, SocietyEntry>,
        presets_dir: PathBuf,
        mail: Arc<dyn MailSender>,
        base_url: String,
        operators: BTreeSet<String>,
    ) -> Self {
        AppState {
            store,
            societies: Arc::new(RwLock::new(societies)),
            presets_dir,
            mail,
            base_url,
            limiter: Arc::new(RateLimiter::default()),
            operators: Arc::new(operators),
            seen: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Everything `Runtime::start` loaded.
    #[must_use]
    pub fn entries_from(runtime: &Runtime) -> BTreeMap<i64, SocietyEntry> {
        runtime
            .societies
            .iter()
            .map(|(id, r)| {
                (
                    *id,
                    SocietyEntry {
                        row: r.row.clone(),
                        handle: r.handle.clone(),
                        control: r.control.clone(),
                    },
                )
            })
            .collect()
    }

    #[must_use]
    pub fn society(&self, id: i64) -> Option<SocietyEntry> {
        self.societies
            .read()
            .expect("societies lock")
            .get(&id)
            .cloned()
    }

    #[must_use]
    pub fn society_count(&self) -> usize {
        self.societies.read().expect("societies lock").len()
    }

    /// `true` the first time this citizen is seen at `tick`.
    pub fn first_sight_this_tick(&self, society: i64, citizen: CitizenId, tick: u32) -> bool {
        let mut seen = self.seen.lock().expect("seen lock");
        match seen.get(&(society, citizen)) {
            Some(t) if *t == tick => false,
            _ => {
                seen.insert((society, citizen), tick);
                true
            }
        }
    }
}

/// The API's clock view of a world (1-based for display, TDD 5.1).
#[must_use]
pub fn clock_of(world: &World) -> Clock {
    let tpc = world.ticks_per_cycle();
    let tick = world.meta.tick;
    Clock {
        epoch: world.meta.epoch + 1,
        cycle: tick / tpc + 1,
        tick: tick % tpc + 1,
        ticks_per_cycle: tpc,
        engine_tick: tick,
        epoch_ended: world.meta.epoch_ended.is_some(),
        epoch_ending: world.meta.epoch_ending.map(|c| c + 1),
    }
}
