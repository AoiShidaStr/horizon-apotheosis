//! World tiers of Apotheosis: one per player, chosen among those they have
//! unlocked.
//!
//! Java keeps the active tier in a player attachment with a codec, defaulting
//! to Haven and copied on death (`Attachments.WORLD_TIER`). This module does
//! the same through Horizon's server attachments on `Holder::Player`: the value
//! is saved in the player's file and copied on respawn, so it survives a
//! restart and a death.
//!
//! The unlock criteria are the Java advancements' conditions, memorised as they
//! happen: a piece of affixed gear in each of the five equipment slots at a
//! rarity, and one boss kill each (`invader`, Wither, Ender Dragon). A
//! criterion once met stays met even after the gear is taken off, and the
//! tiers it opens stay unlocked.
//!
//! The state, the unlocking, the commands and the rates live here: a tier's
//! rarity and gem-purity tables and its invader spawn chances are Java's data
//! (`rarities/*.json`, `purity_weights/weights.json`,
//! `data_maps/dimension_type/invader_spawn_rules.json`), rolled by the loot,
//! gem and invader hooks. The monsters' augments are a later step.

use horizon_client::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, EquipmentSlot, Flow, Holder, Host,
    ItemStack, LivingDeathEvent, MobKilledEvent, NativeServerPlayer, Tag, Text,
};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::sockets::Purity;
use crate::{sockets, Rng, Tier};

/// The bits of `Progress::kills`.
const KILLED_INVADER: i32 = 1;
const KILLED_WITHER: i32 = 2;
const KILLED_DRAGON: i32 = 4;

const WITHER: &str = "minecraft:wither";
const DRAGON: &str = "minecraft:ender_dragon";
const PLAYER: &str = "minecraft:player";

/// The colour of a tier's name in a message: Java's gold for an unlocked tier.
const GOLD: u32 = 0xFF_AA00;

/// The five world tiers, in Java's order: the first is active by default, each
/// next one raises the world's difficulty and the loot's quality (later steps).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldTier {
    Haven,
    Frontier,
    Ascent,
    Summit,
    Pinnacle,
}

impl WorldTier {
    pub const ALL: [WorldTier; 5] = [
        WorldTier::Haven,
        WorldTier::Frontier,
        WorldTier::Ascent,
        WorldTier::Summit,
        WorldTier::Pinnacle,
    ];

    /// The name Java's codec and commands use (`haven`, `frontier`...).
    pub fn id(self) -> &'static str {
        match self {
            WorldTier::Haven => "haven",
            WorldTier::Frontier => "frontier",
            WorldTier::Ascent => "ascent",
            WorldTier::Summit => "summit",
            WorldTier::Pinnacle => "pinnacle",
        }
    }

    /// The tier's name in English, for a client without the translation key.
    pub fn name(self) -> &'static str {
        match self {
            WorldTier::Haven => "Haven",
            WorldTier::Frontier => "Frontier",
            WorldTier::Ascent => "Ascent",
            WorldTier::Summit => "Summit",
            WorldTier::Pinnacle => "Pinnacle",
        }
    }

    /// Java's translation key for the tier's name.
    pub fn name_key(self) -> &'static str {
        match self {
            WorldTier::Haven => "text.apotheosis.world_tier.haven",
            WorldTier::Frontier => "text.apotheosis.world_tier.frontier",
            WorldTier::Ascent => "text.apotheosis.world_tier.ascent",
            WorldTier::Summit => "text.apotheosis.world_tier.summit",
            WorldTier::Pinnacle => "text.apotheosis.world_tier.pinnacle",
        }
    }

    /// The tier's name as a translated component.
    pub fn component(self) -> Text {
        Text::translate_with_fallback(self.name_key(), self.name(), &[])
    }

    /// The tier's index, in Java's order.
    pub fn index(self) -> usize {
        match self {
            WorldTier::Haven => 0,
            WorldTier::Frontier => 1,
            WorldTier::Ascent => 2,
            WorldTier::Summit => 3,
            WorldTier::Pinnacle => 4,
        }
    }

    /// The tier a name names, its case ignored (a command's argument).
    pub fn from_id(id: &str) -> Option<Self> {
        WorldTier::ALL
            .into_iter()
            .find(|tier| tier.id().eq_ignore_ascii_case(id))
    }

    /// The rarity this tier needs in all five slots, none for Haven.
    pub fn gear_rarity(self) -> Option<Tier> {
        match self {
            WorldTier::Haven => None,
            WorldTier::Frontier => Some(Tier::Common),
            WorldTier::Ascent => Some(Tier::Uncommon),
            WorldTier::Summit => Some(Tier::Rare),
            WorldTier::Pinnacle => Some(Tier::Epic),
        }
    }

    /// The boss this tier needs killed as well, none for Haven and Frontier.
    fn boss(self) -> Option<Kill> {
        match self {
            WorldTier::Haven | WorldTier::Frontier => None,
            WorldTier::Ascent => Some(Kill::Invader),
            WorldTier::Summit => Some(Kill::Wither),
            WorldTier::Pinnacle => Some(Kill::Dragon),
        }
    }
}

/// The five equipment slots the gear criteria watch, as Java's advancements
/// name them (`head`, `chest`, `legs`, `feet`, `hand`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GearSlot {
    Head,
    Chest,
    Legs,
    Feet,
    Hand,
}

impl GearSlot {
    const ALL: [GearSlot; 5] = [
        GearSlot::Head,
        GearSlot::Chest,
        GearSlot::Legs,
        GearSlot::Feet,
        GearSlot::Hand,
    ];

    fn index(self) -> usize {
        match self {
            GearSlot::Head => 0,
            GearSlot::Chest => 1,
            GearSlot::Legs => 2,
            GearSlot::Feet => 3,
            GearSlot::Hand => 4,
        }
    }

    fn equipment(self) -> EquipmentSlot {
        match self {
            GearSlot::Head => EquipmentSlot::Head,
            GearSlot::Chest => EquipmentSlot::Chest,
            GearSlot::Legs => EquipmentSlot::Legs,
            GearSlot::Feet => EquipmentSlot::Feet,
            GearSlot::Hand => EquipmentSlot::MainHand,
        }
    }
}

/// The three kills a tier asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kill {
    Invader,
    Wither,
    Dragon,
}

impl Kill {
    fn bit(self) -> i32 {
        match self {
            Kill::Invader => KILLED_INVADER,
            Kill::Wither => KILLED_WITHER,
            Kill::Dragon => KILLED_DRAGON,
        }
    }
}

/// What a player's unlocks have recorded so far. Every field only grows, so a
/// criterion met once stays met; nothing here says which tier is active.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Progress {
    /// The best rarity seen in each slot (`GearSlot::index`), none for a slot
    /// that has never held an affixed piece.
    best: [Option<Tier>; 5],
    /// `KILLED_*` bits of the bosses killed.
    kills: i32,
    /// The tiers unlocked, by `WorldTier::index`; Haven is always there.
    unlocked: [bool; 5],
}

impl Default for Progress {
    fn default() -> Self {
        let mut unlocked = [false; 5];
        unlocked[WorldTier::Haven.index()] = true;
        Self {
            best: [None; 5],
            kills: 0,
            unlocked,
        }
    }
}

impl Progress {
    /// Notes a piece's rarity in a slot; a lower one never takes a higher one
    /// back.
    fn note_gear(&mut self, slot: GearSlot, rarity: Option<Tier>) {
        let Some(rarity) = rarity else {
            return;
        };
        let best = &mut self.best[slot.index()];
        if best.is_none_or(|seen| rarity > seen) {
            *best = Some(rarity);
        }
    }

    fn note_kill(&mut self, kill: Kill) {
        self.kills |= kill.bit();
    }

    fn killed(&self, kill: Kill) -> bool {
        self.kills & kill.bit() != 0
    }

    /// Whether all five slots have held a piece of at least `rarity`.
    fn all_at_least(&self, rarity: Tier) -> bool {
        self.best
            .iter()
            .all(|best| best.is_some_and(|seen| seen >= rarity))
    }

    /// Whether the criteria of `tier` are met, whether or not it was granted.
    fn meets(&self, tier: WorldTier) -> bool {
        if let Some(rarity) = tier.gear_rarity()
            && !self.all_at_least(rarity)
        {
            return false;
        }
        match tier.boss() {
            Some(kill) => self.killed(kill),
            None => true,
        }
    }

    /// Grants every tier whose criteria are met and that was not granted yet,
    /// in tier order.
    fn grant(&mut self) -> Vec<WorldTier> {
        let mut newly = Vec::new();
        for tier in WorldTier::ALL {
            if !self.unlocked[tier.index()] && self.meets(tier) {
                self.unlocked[tier.index()] = true;
                newly.push(tier);
            }
        }
        newly
    }
}

/// A player's whole world-tier state, as the attachment holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    tier: WorldTier,
    progress: Progress,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tier: WorldTier::Haven,
            progress: Progress::default(),
        }
    }
}

impl State {
    /// The active tier.
    pub fn tier(&self) -> WorldTier {
        self.tier
    }

    pub fn is_unlocked(&self, tier: WorldTier) -> bool {
        self.progress.unlocked[tier.index()]
    }

    /// The unlocked tiers, in tier order.
    pub fn unlocked(&self) -> Vec<WorldTier> {
        WorldTier::ALL
            .into_iter()
            .filter(|tier| self.is_unlocked(*tier))
            .collect()
    }

    /// Activates `tier` when the player has unlocked it. A locked tier is
    /// refused and the active one is kept.
    pub fn set_active(&mut self, tier: WorldTier) -> Change {
        if !self.is_unlocked(tier) {
            return Change::Locked(tier);
        }
        if self.tier == tier {
            return Change::AlreadyActive;
        }
        self.tier = tier;
        Change::Changed(tier)
    }

    /// Activates `tier` without the unlock check, as Java's `/apoth
    /// set_world_tier` does. Returns whether the active tier changed.
    pub fn force_active(&mut self, tier: WorldTier) -> bool {
        let changed = self.tier != tier;
        self.tier = tier;
        changed
    }

    fn encode(&self) -> Compound {
        let mut data = Compound::new();
        data.insert("tier".into(), Tag::String(self.tier.id().into()));
        data.insert(
            "best".into(),
            Tag::List(
                self.progress
                    .best
                    .iter()
                    .map(|best| Tag::Int(best.map_or(0, |tier| tier as i32 + 1)))
                    .collect(),
            ),
        );
        data.insert("kills".into(), Tag::Int(self.progress.kills));
        let mut mask = 0;
        for tier in WorldTier::ALL {
            if self.is_unlocked(tier) {
                mask |= 1 << tier.index();
            }
        }
        data.insert("unlocked".into(), Tag::Int(mask));
        data
    }

    fn decode(data: &Compound) -> Option<Self> {
        let tier = match data.get(&"tier".into()) {
            Some(Tag::String(id)) => WorldTier::from_id(&id.to_string_lossy())?,
            _ => return None,
        };
        let mut best = [None; 5];
        match data.get(&"best".into()) {
            Some(Tag::List(items)) if items.len() == best.len() => {
                for (slot, item) in best.iter_mut().zip(items) {
                    let Tag::Int(rank) = item else {
                        return None;
                    };
                    *slot = match *rank {
                        0 => None,
                        rank if rank > 0 => Some(tier_at(rank - 1)?),
                        _ => return None,
                    };
                }
            }
            _ => return None,
        }
        let kills = match data.get(&"kills".into()) {
            Some(Tag::Int(kills)) => *kills,
            _ => return None,
        };
        let mask = match data.get(&"unlocked".into()) {
            Some(Tag::Int(mask)) => *mask,
            _ => return None,
        };
        let mut progress = Progress {
            best,
            kills,
            unlocked: [false; 5],
        };
        for tier in WorldTier::ALL {
            progress.unlocked[tier.index()] =
                tier == WorldTier::Haven || mask & (1 << tier.index()) != 0;
        }
        Some(Self { tier, progress })
    }
}

/// The rarity at index `0..=5` of the port's `Tier`, as `sockets` writes it.
fn tier_at(index: i32) -> Option<Tier> {
    Some(match index {
        0 => Tier::Common,
        1 => Tier::Uncommon,
        2 => Tier::Rare,
        3 => Tier::Epic,
        4 => Tier::Mythic,
        5 => Tier::Ancient,
        _ => return None,
    })
}

/// How a request to activate a tier went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// The tier is now active.
    Changed(WorldTier),
    /// It was already active.
    AlreadyActive,
    /// The player has not unlocked it; nothing changed.
    Locked(WorldTier),
}

/// The player attachment holding the state: saved in the player's file and
/// copied on death, like Java's `Attachments.WORLD_TIER`.
static STATE: AttachmentType<State> = AttachmentType {
    namespace: "apotheosis",
    name: "world_tier",
    default: || State::default(),
    codec: Some(AttachmentCodec {
        encode: |state| Ok(state.encode()),
        decode: |data| State::decode(data).ok_or(AttachmentError::Decode),
    }),
    copy_on_death: true,
    sync: false,
};

/// The active tiers last read, for `tier_of`, which the later steps call
/// without a host (a loot roll, a spawn's augments).
static ACTIVE: OnceLock<Mutex<HashMap<u128, WorldTier>>> = OnceLock::new();

fn active() -> &'static Mutex<HashMap<u128, WorldTier>> {
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remember(player: u128, tier: WorldTier) {
    if let Ok(mut tiers) = active().lock() {
        tiers.insert(player, tier);
    }
}

/// The player's active tier as the last hook or command read it, Haven when
/// nothing has read it yet. `tier_of_with` reads and refreshes it.
#[allow(dead_code)] // the loot, gem and monster steps read it from `loot_drops`.
pub fn tier_of(player: u128) -> WorldTier {
    active()
        .lock()
        .ok()
        .and_then(|tiers| tiers.get(&player).copied())
        .unwrap_or(WorldTier::Haven)
}

/// The player's active tier, read from their attachment. Later steps that
/// have a host should use this; it also refreshes `tier_of`'s copy.
#[allow(dead_code)] // the loot and monster steps read it from a hook's host.
pub fn tier_of_with(host: &mut dyn Host, player: u128) -> WorldTier {
    state_of(host, player).tier
}

// ---------------------------------------------------------------------------
// Rates by world tier
// ---------------------------------------------------------------------------

/// Java's `TieredWeights.Weight`: a base weight and how much one point of
/// `minecraft:luck` moves it. The values are the ones in
/// `data/apotheosis/rarities/*.json` and `purity_weights/weights.json`; a
/// field the file leaves out is zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weight {
    /// The weight with no luck at all.
    pub weight: i32,
    /// How many points of weight one point of luck adds.
    pub quality: f32,
}

impl Weight {
    const fn new(weight: i32, quality: f32) -> Self {
        Self { weight, quality }
    }

    /// Java's `Weight.getWeight` with `Weighted.wrap`'s floor:
    /// `max(0, weight + Math.round(luck * quality))`.
    pub fn at(self, luck: f32) -> i64 {
        // Java's `Math.round(float)` is `floor(x + 0.5)`, which differs from
        // Rust's `round` for a negative luck.
        let bonus = (luck * self.quality + 0.5).floor() as i64;
        (i64::from(self.weight) + bonus).max(0)
    }
}

/// The rarities a world tier rolls, in Java's order. `Tier::Ancient` is this
/// port's own extension and never comes from a tier's table.
pub const ROLLED_RARITIES: [Tier; 5] = [
    Tier::Common,
    Tier::Uncommon,
    Tier::Rare,
    Tier::Epic,
    Tier::Mythic,
];

/// Java's rarity weights, by tier (`WorldTier::index`) and by
/// `ROLLED_RARITIES` index.
const RARITY_WEIGHTS: [[Weight; 5]; 5] = [
    // Haven
    [
        Weight::new(600, 0.0),
        Weight::new(360, 2.5),
        Weight::new(40, 5.0),
        Weight::new(0, 0.0),
        Weight::new(0, 0.0),
    ],
    // Frontier
    [
        Weight::new(290, 0.0),
        Weight::new(600, 0.0),
        Weight::new(100, 5.0),
        Weight::new(10, 0.0),
        Weight::new(0, 0.0),
    ],
    // Ascent
    [
        Weight::new(100, 0.0),
        Weight::new(300, 0.0),
        Weight::new(500, 2.5),
        Weight::new(100, 5.0),
        Weight::new(0, 0.0),
    ],
    // Summit
    [
        Weight::new(0, 0.0),
        Weight::new(120, 0.0),
        Weight::new(290, 2.5),
        Weight::new(540, 5.0),
        Weight::new(50, 5.0),
    ],
    // Pinnacle
    [
        Weight::new(0, 0.0),
        Weight::new(0, 0.0),
        Weight::new(100, 0.0),
        Weight::new(650, 2.5),
        Weight::new(250, 10.0),
    ],
];

/// The purities a gem drops at, in Java's order (cracked to perfect).
pub const ROLLED_PURITIES: [Purity; 6] = Purity::ALL;

/// Java's purity weights, by tier (`WorldTier::index`) and by
/// `ROLLED_PURITIES` index.
const PURITY_WEIGHTS: [[Weight; 6]; 5] = [
    // Haven
    [
        Weight::new(550, 0.0),
        Weight::new(410, 2.5),
        Weight::new(40, 5.0),
        Weight::new(0, 0.0),
        Weight::new(0, 0.0),
        Weight::new(0, 0.0),
    ],
    // Frontier
    [
        Weight::new(230, 0.0),
        Weight::new(540, 0.0),
        Weight::new(180, 2.5),
        Weight::new(50, 1.0),
        Weight::new(0, 0.0),
        Weight::new(0, 0.0),
    ],
    // Ascent
    [
        Weight::new(20, 0.0),
        Weight::new(250, 0.0),
        Weight::new(560, 2.5),
        Weight::new(160, 5.0),
        Weight::new(10, 1.0),
        Weight::new(0, 0.0),
    ],
    // Summit
    [
        Weight::new(0, 0.0),
        Weight::new(80, 0.0),
        Weight::new(310, 2.5),
        Weight::new(580, 5.0),
        Weight::new(20, 5.0),
        Weight::new(10, 0.0),
    ],
    // Pinnacle
    [
        Weight::new(0, 0.0),
        Weight::new(0, 0.0),
        Weight::new(100, 0.0),
        Weight::new(330, 0.0),
        Weight::new(470, 0.0),
        Weight::new(100, 5.0),
    ],
];

/// Natural-spawn invader chances in 10 000, by dimension (overworld, Nether,
/// End) and tier. Java's overworld and End rows are equal; a dimension the
/// data gives no row (a mod's) takes the overworld's.
const INVADER_CHANCES: [[u64; 5]; 3] = [
    // Overworld
    [0, 180, 200, 250, 300],
    // Nether
    [0, 250, 270, 300, 350],
    // End
    [0, 180, 200, 250, 300],
];

/// The rarity table of a tier, common first.
pub fn rarity_weights(tier: WorldTier) -> &'static [Weight; 5] {
    &RARITY_WEIGHTS[tier.index()]
}

/// The purity table of a tier, cracked first.
pub fn purity_weights(tier: WorldTier) -> &'static [Weight; 6] {
    &PURITY_WEIGHTS[tier.index()]
}

/// An index drawn from `weights`, each entry's chance proportional to its
/// weight; the first entry when every one of them weighs zero.
fn weighted(weights: &[i64], rand: &mut Rng) -> usize {
    let total: u64 = weights.iter().map(|weight| *weight as u64).sum();
    if total == 0 {
        return 0;
    }
    let mut roll = rand.below(total);
    for (index, weight) in weights.iter().enumerate() {
        let weight = *weight as u64;
        if roll < weight {
            return index;
        }
        roll -= weight;
    }
    weights.len() - 1
}

/// The rarity an affixed piece of `tier` rolls at `luck`.
pub fn roll_rarity(tier: WorldTier, luck: f32, rand: &mut Rng) -> Tier {
    let weights: Vec<i64> = rarity_weights(tier).iter().map(|w| w.at(luck)).collect();
    ROLLED_RARITIES[weighted(&weights, rand)]
}

/// The purity a gem of `tier` rolls at `luck`, with no gem constraint.
pub fn roll_purity(tier: WorldTier, luck: f32, rand: &mut Rng) -> Purity {
    let weights: Vec<i64> = purity_weights(tier).iter().map(|w| w.at(luck)).collect();
    ROLLED_PURITIES[weighted(&weights, rand)]
}

/// The purity a gem of `tier` rolls at `luck` between the gem's own floor and
/// the drop's ceiling: a purity outside that range never weighs, and when every
/// allowed purity weighs zero, the plain roll is clamped into the range.
pub fn roll_gem_purity(
    tier: WorldTier,
    luck: f32,
    floor: Purity,
    ceiling: Purity,
    rand: &mut Rng,
) -> Purity {
    let ceiling = ceiling.max(floor);
    let weights: Vec<i64> = purity_weights(tier)
        .iter()
        .enumerate()
        .map(|(index, weight)| {
            let purity = ROLLED_PURITIES[index];
            if purity >= floor && purity <= ceiling {
                weight.at(luck)
            } else {
                0
            }
        })
        .collect();
    if weights.iter().any(|weight| *weight > 0) {
        return ROLLED_PURITIES[weighted(&weights, rand)];
    }
    roll_purity(tier, luck, rand).clamp(floor, ceiling)
}

/// The chance in 10 000 that a natural spawn near a `tier` player becomes an
/// invader; zero at Haven.
pub fn invader_chance(tier: WorldTier, dimension: &str) -> u64 {
    let row = match dimension {
        "minecraft:the_nether" => 1,
        "minecraft:the_end" => 2,
        _ => 0,
    };
    INVADER_CHANCES[row][tier.index()]
}

/// The player nearest a point of `dimension`, from the server's online list.
pub fn nearest_player(host: &mut dyn Host, dimension: &str, pos: [f64; 3]) -> Option<u128> {
    let distance = |player: &NativeServerPlayer| {
        let [x, y, z] = player.position;
        (x - pos[0]).powi(2) + (y - pos[1]).powi(2) + (z - pos[2]).powi(2)
    };
    host.server_players()
        .into_iter()
        .filter(|player| player.dimension == dimension)
        .min_by(|a, b| distance(a).total_cmp(&distance(b)))
        .map(|player| player.uuid)
}

/// The tier that governs a drop in `dimension` at `pos`: the killer's when the
/// loot names one, else the nearest player's, else Haven.
pub fn tier_for_drop(
    host: &mut dyn Host,
    killer: Option<u128>,
    dimension: &str,
    pos: [f64; 3],
) -> WorldTier {
    match killer.or_else(|| nearest_player(host, dimension, pos)) {
        Some(player) => tier_of_with(host, player),
        None => WorldTier::Haven,
    }
}

/// Reads a player's state, remembering its active tier.
fn state_of(host: &mut dyn Host, player: u128) -> State {
    let state = STATE.get(host, Holder::Player(player)).unwrap_or_default();
    remember(player, state.tier);
    state
}

/// Saves a player's state; false when the attachment refused it.
fn store(host: &mut dyn Host, player: u128, state: &State) -> bool {
    let stored = STATE
        .set(host, Holder::Player(player), state.clone())
        .is_ok();
    if stored {
        remember(player, state.tier);
    }
    stored
}

/// Reads a player's state, lets `note` record what happened, grants the tiers
/// the criteria now meet, saves when anything changed and tells the player of
/// each tier they just unlocked.
fn observe(host: &mut dyn Host, player: u128, note: impl FnOnce(&mut State)) {
    let mut state = match STATE.get(host, Holder::Player(player)) {
        Ok(state) => state,
        Err(_) => return,
    };
    let before = state.clone();
    note(&mut state);
    let newly = state.progress.grant();
    if state != before {
        if !store(host, player, &state) {
            return;
        }
    } else {
        remember(player, state.tier);
    }
    for tier in newly {
        announce(host, player, tier);
    }
}

fn announce(host: &mut dyn Host, player: u128, tier: WorldTier) {
    let line = Text::translate_with_fallback(
        "apotheosis.world_tier.unlocked",
        "World tier unlocked: %s",
        &[tier.component()],
    )
    .with_color(GOLD);
    if host.acting_player().is_some_and(|actor| actor.uuid == player) {
        host.feedback(line);
    } else {
        let _ = host.tell(player, line);
    }
}

/// The rarity of an affixed piece, as `sockets` recorded it when it rolled
/// the affixes. Plain gear is no criterion.
fn rarity_of(stack: &ItemStack) -> Option<Tier> {
    sockets::tier_of(stack)
}

/// The acting player's five slots, every tick: a piece already worn when the
/// world opened counts too, unlike a hook that only sees a change.
pub fn player_tick(host: &mut dyn Host) {
    let Some(player) = host.acting_player() else {
        return;
    };
    if STATE
        .get(host, Holder::Player(player.uuid))
        .is_ok_and(|state| state.progress.unlocked.iter().all(|unlocked| *unlocked))
    {
        return;
    }
    let mut seen = [None; 5];
    for slot in GearSlot::ALL {
        seen[slot.index()] = host
            .player_equipment(player.uuid, slot.equipment())
            .as_ref()
            .and_then(rarity_of);
    }
    observe(host, player.uuid, |state| {
        for slot in GearSlot::ALL {
            state.progress.note_gear(slot, seen[slot.index()]);
        }
    });
}

/// The kill a death is, when the criteria watch it.
fn kill_of(host: &mut dyn Host, kind: &str, entity: Option<u32>) -> Option<Kill> {
    if kind == WITHER {
        return Some(Kill::Wither);
    }
    if kind == DRAGON {
        return Some(Kill::Dragon);
    }
    if entity.is_some_and(|id| crate::affix_effects::is_boss(host, id)) {
        return Some(Kill::Invader);
    }
    None
}

/// The player who dealt a death's last blow, from the player entity id the
/// hook gives (`0` when it was not a player).
fn killer(host: &mut dyn Host, attacker: u32) -> Option<u128> {
    if attacker == 0 {
        return None;
    }
    let mut players = host.server_players();
    if let Some(actor) = host.acting_player() {
        players.push(actor);
    }
    players
        .into_iter()
        .find(|player| player.entity_id > 0 && player.entity_id as u32 == attacker)
        .map(|player| player.uuid)
}

/// The acting player's blow killed a mob: credit the invader, Wither or dragon
/// criteria to them.
pub fn mob_killed(host: &mut dyn Host, event: &MobKilledEvent<'_>) {
    if event.kind == PLAYER {
        return;
    }
    let Some(kill) = kill_of(host, event.kind, Some(event.entity)) else {
        return;
    };
    observe(host, event.player, |state| state.progress.note_kill(kill));
}

/// A mob is about to die. `mob_killed` credits a blow the acting player dealt;
/// a kill from range names the killer by entity id instead, so it is credited
/// here. A death the hook gives no player for (a mob's arrow, the void, a
/// single-player player's arrow) unlocks nothing.
pub fn living_death(host: &mut dyn Host, event: &LivingDeathEvent<'_>) -> Flow {
    if event.kind == PLAYER {
        return Flow::Continue;
    }
    let Some(kill) = kill_of(host, event.kind, Some(event.entity)) else {
        return Flow::Continue;
    };
    if let Some(player) = killer(host, event.attacker) {
        observe(host, player, |state| state.progress.note_kill(kill));
    }
    Flow::Continue
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// A command line's plan, once parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Show the active tier and the unlocked ones.
    Show,
    /// Activate a tier for the acting player.
    Activate(WorldTier),
    /// Set another player's tier, operators only.
    Remote { target: String, tier: WorldTier },
}

/// Why a command line cannot run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// The words do not form a usage.
    Usage,
    /// A tier name no tier has.
    UnknownTier,
    /// `set_world_tier` below operator level 2.
    NotOperator,
}

/// Whether an operator level may run `/apoth set_world_tier`, Java's
/// `requires(c -> c.hasPermission(2))`.
pub fn is_operator(permission: u8) -> bool {
    permission >= 2
}

/// Plans `/apoth world_tier ...` from the words after `apoth`.
pub fn plan(words: &[&str], permission: u8) -> Result<Action, Rejection> {
    let Some((root, rest)) = words.split_first() else {
        return Err(Rejection::Usage);
    };
    match *root {
        "world_tier" => match rest {
            [] => Ok(Action::Show),
            [tier] => WorldTier::from_id(tier)
                .map(Action::Activate)
                .ok_or(Rejection::UnknownTier),
            _ => Err(Rejection::Usage),
        },
        "set_world_tier" => {
            if !is_operator(permission) {
                return Err(Rejection::NotOperator);
            }
            match rest {
                [target, tier] => WorldTier::from_id(tier)
                    .map(|tier| Action::Remote {
                        target: (*target).to_owned(),
                        tier,
                    })
                    .ok_or(Rejection::UnknownTier),
                _ => Err(Rejection::Usage),
            }
        }
        _ => Err(Rejection::Usage),
    }
}

/// Runs a parsed `plan` for the acting player, answering in their chat.
pub fn command(host: &mut dyn Host, player: &NativeServerPlayer, words: &[&str]) -> Flow {
    match plan(words, player.permission) {
        Ok(Action::Show) => show(host, player.uuid),
        Ok(Action::Activate(tier)) => activate(host, player, tier),
        Ok(Action::Remote { target, tier }) => set_other(host, player, &target, tier),
        Err(rejection) => host.feedback(rejection_line(rejection)),
    }
    Flow::Cancel
}

fn show(host: &mut dyn Host, player: u128) {
    let state = state_of(host, player);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.command.world_tier.show_active",
        "World tier: %s",
        &[state.tier().component()],
    ));
    host.feedback(Text::translate_with_fallback(
        "apotheosis.command.world_tier.show_unlocked",
        "Unlocked: %s",
        &[names(&state.unlocked())],
    ));
}

fn activate(host: &mut dyn Host, player: &NativeServerPlayer, tier: WorldTier) {
    let mut state = state_of(host, player.uuid);
    match state.set_active(tier) {
        Change::Locked(tier) => host.feedback(Text::translate_with_fallback(
            "apotheosis.world_tier.locked",
            "%s is not unlocked yet.",
            &[tier.component()],
        )),
        Change::AlreadyActive => host.feedback(Text::translate_with_fallback(
            "apotheosis.command.world_tier.already",
            "Your world tier is already %s.",
            &[tier.component()],
        )),
        Change::Changed(tier) => {
            if store(host, player.uuid, &state) {
                host.feedback(Text::translate_with_fallback(
                    "apotheosis.command.world_tier.changed",
                    "World tier set to %s.",
                    &[tier.component()],
                ));
            }
        }
    }
}

fn set_other(host: &mut dyn Host, actor: &NativeServerPlayer, target: &str, tier: WorldTier) {
    let Some(player) = resolve(host, actor, target) else {
        host.feedback(Text::translate_with_fallback(
            "apotheosis.command.set_world_tier.missing_player",
            "No player named %s is online.",
            &[Text::literal(target.to_owned())],
        ));
        return;
    };
    let mut state = state_of(host, player.uuid);
    let changed = state.force_active(tier);
    if changed && !store(host, player.uuid, &state) {
        return;
    }
    remember(player.uuid, tier);
    host.feedback(Text::translate_with_fallback(
        "apotheosis.command.set_world_tier.done",
        "Set %s's world tier to %s.",
        &[
            Text::literal(player.name.clone()),
            tier.component(),
        ],
    ));
}

/// The player a name names: the acting one or an online guest. A vanilla
/// server's offline players are not reachable from a mod's hook.
fn resolve(
    host: &mut dyn Host,
    actor: &NativeServerPlayer,
    name: &str,
) -> Option<NativeServerPlayer> {
    if actor.name.eq_ignore_ascii_case(name) {
        return Some(actor.clone());
    }
    host.server_players()
        .into_iter()
        .find(|player| player.name.eq_ignore_ascii_case(name))
}

fn rejection_line(rejection: Rejection) -> Text {
    match rejection {
        Rejection::Usage => Text::translate_with_fallback(
            "apotheosis.command.world_tier.usage",
            "Usage: /apoth world_tier [haven|frontier|ascent|summit|pinnacle], /apoth set_world_tier <player> <tier>",
            &[],
        ),
        Rejection::UnknownTier => Text::translate_with_fallback(
            "apotheosis.command.world_tier.unknown_tier",
            "Unknown world tier: choose haven, frontier, ascent, summit or pinnacle.",
            &[],
        ),
        Rejection::NotOperator => Text::translate_with_fallback(
            "apotheosis.command.set_world_tier.denied",
            "Only operators can set another player's world tier.",
            &[],
        ),
    }
}

/// The tiers' names as one translated line: `Haven, Frontier`.
fn names(tiers: &[WorldTier]) -> Text {
    let mut line = Text::literal("");
    for (index, tier) in tiers.iter().enumerate() {
        if index > 0 {
            line = line.append(Text::literal(", "));
        }
        line = line.append(tier.component());
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state with every slot filled at `rarity` and no kills yet.
    fn geared(rarity: Tier) -> State {
        let mut state = State::default();
        for slot in GearSlot::ALL {
            state.progress.note_gear(slot, Some(rarity));
        }
        state.progress.grant();
        state
    }

    fn unlocked(state: &State) -> Vec<WorldTier> {
        state.unlocked()
    }

    #[test]
    fn haven_is_active_and_unlocked_by_default() {
        let state = State::default();
        assert_eq!(state.tier(), WorldTier::Haven);
        assert_eq!(unlocked(&state), vec![WorldTier::Haven]);
        assert!(!state.is_unlocked(WorldTier::Frontier));
        assert_eq!(WorldTier::from_id("PINNACLE"), Some(WorldTier::Pinnacle));
        assert_eq!(WorldTier::from_id("nope"), None);
    }

    #[test]
    fn frontier_needs_a_piece_in_every_slot() {
        let mut state = State::default();
        for slot in [GearSlot::Head, GearSlot::Chest, GearSlot::Legs, GearSlot::Feet] {
            state.progress.note_gear(slot, Some(Tier::Common));
        }
        assert!(state.progress.grant().is_empty(), "four slots are not enough");
        assert!(!state.is_unlocked(WorldTier::Frontier));

        state.progress.note_gear(GearSlot::Hand, Some(Tier::Uncommon));
        assert_eq!(state.progress.grant(), vec![WorldTier::Frontier]);
        assert!(state.is_unlocked(WorldTier::Frontier));
        // A higher rarity in one slot stands for the lower one.
        assert!(state.progress.all_at_least(Tier::Common));
    }

    #[test]
    fn a_slot_keeps_its_best_rarity() {
        let mut state = State::default();
        state.progress.note_gear(GearSlot::Head, Some(Tier::Rare));
        state.progress.note_gear(GearSlot::Head, Some(Tier::Common));
        state.progress.note_gear(GearSlot::Head, None);
        assert_eq!(state.progress.best[GearSlot::Head.index()], Some(Tier::Rare));
        assert!(!state.progress.all_at_least(Tier::Uncommon));
    }

    #[test]
    fn criteria_are_memorised_apart() {
        // One slot at common, the rest empty: nothing yet. Taking the piece
        // off does not lose what was seen, and the other four complete it
        // later.
        let mut state = State::default();
        state.progress.note_gear(GearSlot::Head, Some(Tier::Common));
        state.progress.note_gear(GearSlot::Head, None);
        assert!(state.progress.grant().is_empty());
        for slot in [GearSlot::Chest, GearSlot::Legs, GearSlot::Feet, GearSlot::Hand] {
            state.progress.note_gear(slot, Some(Tier::Common));
        }
        assert_eq!(state.progress.grant(), vec![WorldTier::Frontier]);
    }

    #[test]
    fn each_boss_unlocks_the_tier_above_its_gear() {
        // Ascent: uncommon in all five and an invader.
        let mut state = geared(Tier::Uncommon);
        assert_eq!(unlocked(&state), vec![WorldTier::Haven, WorldTier::Frontier]);
        state.progress.note_kill(Kill::Invader);
        assert_eq!(state.progress.grant(), vec![WorldTier::Ascent]);

        // Summit: rare in all five and the Wither. Better gear alone stops at
        // Frontier while a boss is missing.
        let mut state = geared(Tier::Rare);
        assert_eq!(unlocked(&state), vec![WorldTier::Haven, WorldTier::Frontier]);
        state.progress.note_kill(Kill::Wither);
        // The criteria are independent, as Java's advancements are: the rare
        // set and the Wither open Summit, while Ascent still wants an invader.
        assert_eq!(state.progress.grant(), vec![WorldTier::Summit]);
        assert!(!state.is_unlocked(WorldTier::Ascent));

        // Pinnacle: epic in all five and the dragon.
        let mut state = geared(Tier::Epic);
        assert_eq!(unlocked(&state), vec![WorldTier::Haven, WorldTier::Frontier]);
        state.progress.note_kill(Kill::Dragon);
        assert_eq!(state.progress.grant(), vec![WorldTier::Pinnacle]);
    }

    #[test]
    fn a_kill_without_the_gear_unlocks_nothing() {
        let mut state = State::default();
        state.progress.note_kill(Kill::Invader);
        state.progress.note_kill(Kill::Wither);
        state.progress.note_kill(Kill::Dragon);
        assert!(state.progress.grant().is_empty());
        assert_eq!(unlocked(&state), vec![WorldTier::Haven]);
    }

    #[test]
    fn a_tier_is_unlocked_once_and_announced_once() {
        let mut state = geared(Tier::Epic);
        state.progress.note_kill(Kill::Wither);
        state.progress.note_kill(Kill::Dragon);
        let first = state.progress.grant();
        assert!(first.contains(&WorldTier::Pinnacle));
        assert!(state.progress.grant().is_empty(), "a second check regrants");
    }

    #[test]
    fn a_locked_tier_is_refused() {
        let mut state = State::default();
        assert_eq!(
            state.set_active(WorldTier::Summit),
            Change::Locked(WorldTier::Summit)
        );
        assert_eq!(state.tier(), WorldTier::Haven);

        let mut state = geared(Tier::Common);
        assert_eq!(state.set_active(WorldTier::Summit), Change::Locked(WorldTier::Summit));
        assert_eq!(
            state.set_active(WorldTier::Frontier),
            Change::Changed(WorldTier::Frontier)
        );
        assert_eq!(state.tier(), WorldTier::Frontier);
        assert_eq!(
            state.set_active(WorldTier::Frontier),
            Change::AlreadyActive
        );
    }

    #[test]
    fn an_operator_bypasses_the_unlock_check() {
        let mut state = State::default();
        assert!(state.force_active(WorldTier::Pinnacle));
        assert_eq!(state.tier(), WorldTier::Pinnacle);
        assert!(!state.is_unlocked(WorldTier::Pinnacle), "unlocks are untouched");
        assert!(!state.force_active(WorldTier::Pinnacle));
    }

    #[test]
    fn only_level_two_and_above_is_an_operator() {
        assert!(!is_operator(0));
        assert!(!is_operator(1));
        assert!(is_operator(2));
        assert!(is_operator(4));
    }

    #[test]
    fn command_lines_plan_and_refuse() {
        assert_eq!(plan(&["world_tier"], 0), Ok(Action::Show));
        assert_eq!(
            plan(&["world_tier", "Summit"], 0),
            Ok(Action::Activate(WorldTier::Summit))
        );
        assert_eq!(
            plan(&["world_tier", "nope"], 0),
            Err(Rejection::UnknownTier)
        );
        assert_eq!(plan(&["world_tier", "summit", "x"], 0), Err(Rejection::Usage));

        // The operator command is refused below level 2, whatever the words.
        assert_eq!(
            plan(&["set_world_tier", "Bob", "summit"], 1),
            Err(Rejection::NotOperator)
        );
        assert_eq!(
            plan(&["set_world_tier", "Bob", "summit"], 2),
            Ok(Action::Remote {
                target: "Bob".to_owned(),
                tier: WorldTier::Summit
            })
        );
        assert_eq!(
            plan(&["set_world_tier", "Bob", "nope"], 4),
            Err(Rejection::UnknownTier)
        );
        assert_eq!(plan(&["set_world_tier", "Bob"], 4), Err(Rejection::Usage));
        assert_eq!(plan(&["salvage"], 0), Err(Rejection::Usage));
    }

    #[test]
    fn state_round_trips_through_its_codec() {
        let mut state = State::default();
        for (slot, rarity) in [
            (GearSlot::Head, Tier::Mythic),
            (GearSlot::Chest, Tier::Epic),
            (GearSlot::Legs, Tier::Rare),
            (GearSlot::Feet, Tier::Common),
            (GearSlot::Hand, Tier::Ancient),
        ] {
            state.progress.note_gear(slot, Some(rarity));
        }
        state.progress.note_kill(Kill::Invader);
        state.progress.note_kill(Kill::Dragon);
        state.progress.grant();
        state.set_active(WorldTier::Ascent);

        let encoded = state.encode();
        assert_eq!(State::decode(&encoded), Some(state.clone()));
    }

    #[test]
    fn a_default_state_round_trips() {
        let state = State::default();
        assert_eq!(State::decode(&state.encode()), Some(state));
    }

    #[test]
    fn broken_data_is_refused() {
        let mut data = Compound::new();
        assert_eq!(State::decode(&data), None, "an empty compound");

        data.insert("tier".into(), Tag::String("haven".into()));
        data.insert("best".into(), Tag::List(vec![Tag::Int(0); 4]));
        data.insert("kills".into(), Tag::Int(0));
        data.insert("unlocked".into(), Tag::Int(1));
        assert_eq!(State::decode(&data), None, "four slots");

        data.insert("best".into(), Tag::List(vec![Tag::Int(9); 5]));
        assert_eq!(State::decode(&data), None, "a rarity out of range");

        data.insert("best".into(), Tag::List(vec![Tag::Int(0); 5]));
        data.insert("tier".into(), Tag::String("olympus".into()));
        assert_eq!(State::decode(&data), None, "an unknown tier");

        data.insert("tier".into(), Tag::String("haven".into()));
        assert_eq!(
            State::decode(&data).map(|state| state.unlocked()),
            Some(vec![WorldTier::Haven]),
            "Haven is unlocked even when its bit is absent"
        );
    }

    #[test]
    fn tier_of_starts_at_haven() {
        assert_eq!(tier_of(0x1234_5678_9abc_def0), WorldTier::Haven);
    }

    /// The raw `(weight, quality)` pairs of a table, for an exact comparison.
    fn pairs(weights: &[Weight]) -> Vec<(i32, f32)> {
        weights.iter().map(|w| (w.weight, w.quality)).collect()
    }

    #[test]
    fn the_rarity_table_is_javas() {
        assert_eq!(
            pairs(rarity_weights(WorldTier::Haven)),
            vec![(600, 0.0), (360, 2.5), (40, 5.0), (0, 0.0), (0, 0.0)]
        );
        assert_eq!(
            pairs(rarity_weights(WorldTier::Frontier)),
            vec![(290, 0.0), (600, 0.0), (100, 5.0), (10, 0.0), (0, 0.0)]
        );
        assert_eq!(
            pairs(rarity_weights(WorldTier::Ascent)),
            vec![(100, 0.0), (300, 0.0), (500, 2.5), (100, 5.0), (0, 0.0)]
        );
        assert_eq!(
            pairs(rarity_weights(WorldTier::Summit)),
            vec![(0, 0.0), (120, 0.0), (290, 2.5), (540, 5.0), (50, 5.0)]
        );
        assert_eq!(
            pairs(rarity_weights(WorldTier::Pinnacle)),
            vec![(0, 0.0), (0, 0.0), (100, 0.0), (650, 2.5), (250, 10.0)]
        );
    }

    #[test]
    fn the_purity_table_is_javas() {
        assert_eq!(
            pairs(purity_weights(WorldTier::Haven)),
            vec![(550, 0.0), (410, 2.5), (40, 5.0), (0, 0.0), (0, 0.0), (0, 0.0)]
        );
        assert_eq!(
            pairs(purity_weights(WorldTier::Frontier)),
            vec![(230, 0.0), (540, 0.0), (180, 2.5), (50, 1.0), (0, 0.0), (0, 0.0)]
        );
        assert_eq!(
            pairs(purity_weights(WorldTier::Ascent)),
            vec![(20, 0.0), (250, 0.0), (560, 2.5), (160, 5.0), (10, 1.0), (0, 0.0)]
        );
        assert_eq!(
            pairs(purity_weights(WorldTier::Summit)),
            vec![(0, 0.0), (80, 0.0), (310, 2.5), (580, 5.0), (20, 5.0), (10, 0.0)]
        );
        assert_eq!(
            pairs(purity_weights(WorldTier::Pinnacle)),
            vec![(0, 0.0), (0, 0.0), (100, 0.0), (330, 0.0), (470, 0.0), (100, 5.0)]
        );
    }

    #[test]
    fn luck_moves_only_the_weights_with_a_quality() {
        assert_eq!(Weight::new(360, 2.5).at(2.0), 365);
        assert_eq!(Weight::new(40, 5.0).at(1.0), 45);
        assert_eq!(Weight::new(600, 0.0).at(4.0), 600, "quality zero never moves");
        assert_eq!(Weight::new(40, 5.0).at(-2.0), 30, "unlucky lowers it too");
        assert_eq!(Weight::new(1, 1.0).at(-5.0), 0, "never below zero");
        // Java's `Math.round` rounds half up: 0.2 * 2.5 -> 1.
        assert_eq!(Weight::new(410, 2.5).at(0.2), 411);
        // Nothing common moves at Haven, since its quality is zero; the rest
        // do not fall when luck rises.
        let haven = rarity_weights(WorldTier::Haven);
        assert_eq!(haven[0].at(3.0), haven[0].at(0.0));
        for index in 1..5 {
            assert!(haven[index].at(3.0) >= haven[index].at(0.0));
        }
    }

    #[test]
    fn haven_never_rolls_epic_mythic_or_ancient() {
        let mut rand = Rng::new(5);
        for _ in 0..4000 {
            let rarity = roll_rarity(WorldTier::Haven, 0.0, &mut rand);
            assert!(rarity <= Tier::Rare, "Haven rolled {rarity:?}");
            assert_ne!(rarity, Tier::Ancient);
        }
    }

    #[test]
    fn pinnacle_never_rolls_common_or_ancient() {
        let mut rand = Rng::new(9);
        for _ in 0..4000 {
            let rarity = roll_rarity(WorldTier::Pinnacle, 0.0, &mut rand);
            assert!(rarity >= Tier::Rare, "Pinnacle rolled {rarity:?}");
            assert_ne!(rarity, Tier::Ancient);
        }
    }

    #[test]
    fn a_gem_floor_is_never_crossed() {
        let mut rand = Rng::new(21);
        for tier in WorldTier::ALL {
            for boss in [false, true] {
                let ceiling = if boss { Purity::Perfect } else { Purity::Flawless };
                for _ in 0..500 {
                    let purity = roll_gem_purity(tier, 0.0, Purity::Flawless, ceiling, &mut rand);
                    assert!(purity >= Purity::Flawless, "{tier:?} rolled {purity:?}");
                    assert!(purity <= ceiling);
                }
            }
        }
    }

    #[test]
    fn a_gem_whose_allowed_purities_all_weigh_zero_takes_the_nearest() {
        // Haven weighs nothing from normal up, so a normal floor lands there.
        let mut rand = Rng::new(31);
        for _ in 0..500 {
            assert_eq!(
                roll_gem_purity(WorldTier::Haven, 0.0, Purity::Normal, Purity::Perfect, &mut rand),
                Purity::Normal
            );
        }
    }

    #[test]
    fn invader_chances_follow_the_tier_and_dimension() {
        for dimension in ["minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"] {
            assert_eq!(invader_chance(WorldTier::Haven, dimension), 0, "Haven is safe");
        }
        assert_eq!(invader_chance(WorldTier::Frontier, "minecraft:overworld"), 180);
        assert_eq!(invader_chance(WorldTier::Ascent, "minecraft:overworld"), 200);
        assert_eq!(invader_chance(WorldTier::Summit, "minecraft:overworld"), 250);
        assert_eq!(invader_chance(WorldTier::Pinnacle, "minecraft:overworld"), 300);
        assert_eq!(invader_chance(WorldTier::Frontier, "minecraft:the_nether"), 250);
        assert_eq!(invader_chance(WorldTier::Ascent, "minecraft:the_nether"), 270);
        assert_eq!(invader_chance(WorldTier::Summit, "minecraft:the_nether"), 300);
        assert_eq!(invader_chance(WorldTier::Pinnacle, "minecraft:the_nether"), 350);
        assert_eq!(invader_chance(WorldTier::Frontier, "minecraft:the_end"), 180);
        assert_eq!(invader_chance(WorldTier::Pinnacle, "minecraft:the_end"), 300);
        // A mod's dimension takes the overworld's row.
        assert_eq!(invader_chance(WorldTier::Pinnacle, "mymod:void"), 300);
    }
}
