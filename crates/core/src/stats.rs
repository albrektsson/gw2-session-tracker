use crate::api::ApiSnapshot;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatSource {
    Achievement(u32),
    WvwRank,
    Deaths,
    Kdr,
    Currency(u32),
    PvpRank,
    PvpWins,
    PvpLosses,
    PvpRankingPoints,
    PvpRankedWins,
    PvpRankedLosses,
    PvpUnrankedWins,
    PvpUnrankedLosses,
    PvpCustomWins,
    PvpCustomLosses,
    PvpKdr,
    Item(u32),
}

/// A stat's browsing category in the Select Stats picker. Purely a UI
/// grouping concern - a stat can belong to more than one (e.g. every
/// currency is in `Currency`, and some are *also* cross-tagged into the
/// activity that earns them, like `Wvw` or `Pvp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Misc,
    Currency,
    Festival,
    Wvw,
    Pvp,
    OpenWorld,
    Fractal,
    Raid,
    Strike,
    BasicCraftingMaterials,
    IntermediateCraftingMaterials,
    AdvancedCraftingMaterials,
    AscendedMaterials,
    GemstonesAndJewels,
    CookingMaterials,
    CookingIngredients,
    ScribingMaterials,
    FestiveMaterials,
}

/// UI grouping of categories into supercategories - not attached to
/// individual stats. (display name, subcategories)
pub const SUPERCATEGORIES: &[(&str, &[Category])] = &[
    ("General", &[Category::Misc, Category::Currency, Category::Festival]),
    ("Competitive", &[Category::Wvw, Category::Pvp]),
    ("PvE", &[Category::OpenWorld, Category::Fractal, Category::Raid, Category::Strike]),
    (
        "Material Storage",
        &[
            Category::BasicCraftingMaterials,
            Category::IntermediateCraftingMaterials,
            Category::AdvancedCraftingMaterials,
            Category::AscendedMaterials,
            Category::GemstonesAndJewels,
            Category::CookingMaterials,
            Category::CookingIngredients,
            Category::ScribingMaterials,
            Category::FestiveMaterials,
        ],
    ),
];

#[derive(Debug, Clone, Copy)]
pub struct StatDef {
    pub id: &'static str,
    pub display_name: &'static str,
    pub source: StatSource,
    pub categories: &'static [Category],
    /// A render.guildwars2.com icon URL. Real per-stat icons for
    /// currencies/items (which have one via the GW2 API); a shared
    /// WvW/PvP achievement-category icon for stats with no natural icon
    /// of their own (achievements have no `icon` field at all via the
    /// API, and computed/account-field stats like Deaths/KDR/Rank have
    /// no GW2 icon either). `None` only for stats outside both WvW and
    /// PvP with no natural icon (currently none - kept as a fallback for
    /// future non-WvW/PvP computed stats).
    pub icon_url: Option<&'static str>,
}

/// Shared fallback icon for WvW-flavored stats with no icon of their own
/// (the "World vs World" achievement category icon).
pub const WVW_FALLBACK_ICON_URL: &str =
    "https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png";

/// Shared fallback icon for PvP-flavored stats with no icon of their own
/// (the "PvP Conqueror" achievement category icon, reused by GW2 itself
/// across several PvP achievement categories).
pub const PVP_FALLBACK_ICON_URL: &str =
    "https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png";

/// A PvP rank tier's badge icon, matching what the game shows next to the
/// player's name (verified against the live `/v2/pvp/ranks` API).
pub struct PvpRankTier {
    pub min_rank: u32,
    pub icon_url: &'static str,
}

pub const PVP_RANK_TIERS: &[PvpRankTier] = &[
    PvpRankTier { min_rank: 1, icon_url: "https://render.guildwars2.com/file/592A4144FE1B6904CD0C69230840B8C21A0C36F7/347222.png" }, // Rabbit
    PvpRankTier { min_rank: 10, icon_url: "https://render.guildwars2.com/file/DECD0D647C9433CC2128BF2F6FE5A5185513EE59/347223.png" }, // Deer
    PvpRankTier { min_rank: 20, icon_url: "https://render.guildwars2.com/file/69F7561A34530F9A6366A0C3ECC0E508EF7451E6/347224.png" }, // Dolyak
    PvpRankTier { min_rank: 30, icon_url: "https://render.guildwars2.com/file/B0920D9F3A07276854FDC4CD364AD3DDF6387061/347225.png" }, // Wolf
    PvpRankTier { min_rank: 40, icon_url: "https://render.guildwars2.com/file/F6CB4002B278077FA5BA434D49776D69CA042EA0/347226.png" }, // Tiger
    PvpRankTier { min_rank: 50, icon_url: "https://render.guildwars2.com/file/5169E9ED4A0DADF06835F87737540A02C39AB534/347227.png" }, // Bear
    PvpRankTier { min_rank: 60, icon_url: "https://render.guildwars2.com/file/65001A2EAE08A564126CCD73AA14FB9A250B2C66/347228.png" }, // Shark
    PvpRankTier { min_rank: 70, icon_url: "https://render.guildwars2.com/file/3DD50A531B472791C07AE9A57840A118F1B8F3E9/347229.png" }, // Phoenix
    PvpRankTier { min_rank: 80, icon_url: "https://render.guildwars2.com/file/A2ACA72B379FCC4C4D7CE3D59DED073196485B5B/347230.png" }, // Dragon
];

/// Resolves the tier for a live PvP rank value (already includes rank
/// rollovers, which can exceed the top tier's nominal range - those still
/// resolve to the top tier, matching how the game keeps showing the max
/// badge past that point). `min_rank` on the result is stable per tier,
/// useful as a cache/texture identifier key that only changes when the
/// player actually moves to a different tier.
pub fn pvp_rank_tier(rank: u32) -> &'static PvpRankTier {
    PVP_RANK_TIERS
        .iter()
        .rev()
        .find(|t| rank >= t.min_rank)
        .unwrap_or(&PVP_RANK_TIERS[0])
}

/// Resolves just the badge icon for a live PvP rank value.
pub fn pvp_rank_icon_url(rank: u32) -> &'static str {
    pvp_rank_tier(rank).icon_url
}

use Category::{Currency as Cur, Festival, Fractal, Misc, OpenWorld, Pvp, Raid, Strike, Wvw};

const CORE_STATS: &[StatDef] = &[
    // WvW
    StatDef { id: "kills", display_name: "Kills", source: StatSource::Achievement(283), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "deaths", display_name: "Deaths", source: StatSource::Deaths, categories: &[Wvw, Pvp, Misc], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "kdr", display_name: "KDR", source: StatSource::Kdr, categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "wvw_rank", display_name: "WvW Rank", source: StatSource::WvwRank, categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "supply_repair", display_name: "Supply (Repair)", source: StatSource::Achievement(306), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "dolyaks_killed", display_name: "Dolyaks Killed", source: StatSource::Achievement(288), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "dolyaks_escorted", display_name: "Dolyaks Escorted", source: StatSource::Achievement(285), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "camps_captured", display_name: "Camps Captured", source: StatSource::Achievement(291), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "camps_defended", display_name: "Camps Defended", source: StatSource::Achievement(310), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "towers_captured", display_name: "Towers Captured", source: StatSource::Achievement(297), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "towers_defended", display_name: "Towers Defended", source: StatSource::Achievement(322), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "keeps_captured", display_name: "Keeps Captured", source: StatSource::Achievement(300), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "keeps_defended", display_name: "Keeps Defended", source: StatSource::Achievement(316), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "castles_captured", display_name: "Castles Captured", source: StatSource::Achievement(294), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "castles_defended", display_name: "Castles Defended", source: StatSource::Achievement(313), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "objectives_captured", display_name: "Objectives Captured", source: StatSource::Achievement(303), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    StatDef { id: "objectives_defended", display_name: "Objectives Defended", source: StatSource::Achievement(319), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/2BBA251A24A2C1A0A305D561580449AF5B55F54F/338457.png") },
    // PvP
    StatDef { id: "pvp_kills", display_name: "PvP Kills", source: StatSource::Achievement(239), categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_kdr", display_name: "PvP KDR", source: StatSource::PvpKdr, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_rank", display_name: "PvP Rank", source: StatSource::PvpRank, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_ranking_points", display_name: "PvP Ranking Points", source: StatSource::PvpRankingPoints, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_wins", display_name: "PvP Total Wins", source: StatSource::PvpWins, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_losses", display_name: "PvP Total Losses", source: StatSource::PvpLosses, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_ranked_wins", display_name: "PvP Ranked Wins", source: StatSource::PvpRankedWins, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_ranked_losses", display_name: "PvP Ranked Losses", source: StatSource::PvpRankedLosses, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_unranked_wins", display_name: "PvP Unranked Wins", source: StatSource::PvpUnrankedWins, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_unranked_losses", display_name: "PvP Unranked Losses", source: StatSource::PvpUnrankedLosses, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_custom_wins", display_name: "PvP Custom Wins", source: StatSource::PvpCustomWins, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    StatDef { id: "pvp_custom_losses", display_name: "PvP Custom Losses", source: StatSource::PvpCustomLosses, categories: &[Pvp], icon_url: Some("https://render.guildwars2.com/file/7F4E2835316DE912B1493CCF500A9D5CF4A83B4A/42676.png") },
    // cross-tagged into the activity category that earns them)
    StatDef { id: "gold", display_name: "Gold", source: StatSource::Currency(1), categories: &[Cur, Wvw, Pvp, Fractal, Raid, OpenWorld, Strike], icon_url: Some("https://render.guildwars2.com/file/98457F504BA2FAC8457F532C4B30EDC23929ACF9/619316.png") },
    StatDef { id: "karma", display_name: "Karma", source: StatSource::Currency(2), categories: &[Cur, Wvw, OpenWorld], icon_url: Some("https://render.guildwars2.com/file/94953FA23D3E0D23559624015DFEA4CFAA07F0E5/155026.png") },
    StatDef { id: "laurels", display_name: "Laurels", source: StatSource::Currency(3), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/A1BD345AD9192C3A585BE2F6CB0617C5A797A1E2/619317.png") },
    StatDef { id: "gems", display_name: "Gems", source: StatSource::Currency(4), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/220061640ECA41C0577758030357221B4ECCE62C/502065.png") },
    StatDef { id: "fractal_relic", display_name: "Fractal Relic", source: StatSource::Currency(7), categories: &[Cur, Fractal], icon_url: Some("https://render.guildwars2.com/file/0204DAD0D40674035F9F5F5270043C3207EEA7E8/619320.png") },
    StatDef { id: "badges_of_honor", display_name: "Badges of Honor", source: StatSource::Currency(15), categories: &[Cur, Wvw], icon_url: Some("https://render.guildwars2.com/file/AC3178E7BD066BC597F9D4247848E6033A047EDE/699004.png") },
    StatDef { id: "guild_commendation", display_name: "Guild Commendation", source: StatSource::Currency(16), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/1DB12CF3E9D8D3CF77DA805B2008D417EA32C172/699005.png") },
    StatDef { id: "transmutation_charge", display_name: "Transmutation Charge", source: StatSource::Currency(18), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/66A42690C2EA05235C4090372B975117F60EE7E3/779571.png") },
    StatDef { id: "airship_part", display_name: "Airship Part", source: StatSource::Currency(19), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/07EC38BCBE979B7F4557057F51785F0930075F35/1029833.png") },
    StatDef { id: "ley_line_crystal", display_name: "Ley Line Crystal", source: StatSource::Currency(20), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/DD02946A3AB4076C500836533F67303EE464A6AC/1206837.png") },
    StatDef { id: "lump_of_aurillium", display_name: "Lump of Aurillium", source: StatSource::Currency(22), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/5D74D6D3D8179FC49D572975CC5E0CC701025508/1206836.png") },
    StatDef { id: "spirit_shard", display_name: "Spirit Shard", source: StatSource::Currency(23), categories: &[Cur, OpenWorld], icon_url: Some("https://render.guildwars2.com/file/0AD608DE7FDEE0B909905C0AF9401321CF65CD94/1010701.png") },
    StatDef { id: "pristine_fractal_relic", display_name: "Pristine Fractal Relic", source: StatSource::Currency(24), categories: &[Cur, Fractal], icon_url: Some("https://render.guildwars2.com/file/77B0F842ED036D71E46B80570D6CFE25CB4C0677/619321.png") },
    StatDef { id: "geode", display_name: "Geode", source: StatSource::Currency(25), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/11C6D8AB5FD04866BF3D751771F90C392739B5DF/1010699.png") },
    StatDef { id: "wvw_skirmish_tickets", display_name: "WvW Skirmish Claim Tickets", source: StatSource::Currency(26), categories: &[Cur, Wvw], icon_url: Some("https://render.guildwars2.com/file/0F911F55FF800FC0589970FF9137BF050699392F/1010702.png") },
    StatDef { id: "bandit_crest", display_name: "Bandit Crest", source: StatSource::Currency(27), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/B8E963AF6D4C66237D44194A6570A1492D1F5EE4/1010697.png") },
    StatDef { id: "magnetite_shard", display_name: "Magnetite Shard", source: StatSource::Currency(28), categories: &[Cur, Raid], icon_url: Some("https://render.guildwars2.com/file/95F8F70D97186780CE45B54600B90BE356D8EA7C/1302741.png") },
    StatDef { id: "provisioner_token", display_name: "Provisioner Token", source: StatSource::Currency(29), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/5CF6FC0B62554EBECAC6752AAFA2B8D80F726077/1302745.png") },
    StatDef { id: "pvp_league_tickets", display_name: "PvP League Tickets", source: StatSource::Currency(30), categories: &[Cur, Pvp], icon_url: Some("https://render.guildwars2.com/file/DB05F5F20B94A3D12AF11041F60E194D66584957/1313248.png") },
    StatDef { id: "proof_of_heroics", display_name: "Proof of Heroics", source: StatSource::Currency(31), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/0D971EF54A6FA5146AC9F60A0702DD640CBB8011/1207142.png") },
    StatDef { id: "unbound_magic", display_name: "Unbound Magic", source: StatSource::Currency(32), categories: &[Cur, OpenWorld], icon_url: Some("https://render.guildwars2.com/file/55CBF5154BC749F0BE7B01F9C75C04F2CD4BC561/1465799.png") },
    StatDef { id: "ascended_shards_of_glory", display_name: "Ascended Shards of Glory", source: StatSource::Currency(33), categories: &[Cur, Pvp], icon_url: Some("https://render.guildwars2.com/file/1AAD230BF65AE92C000E0C18D26D404022092C35/1614710.png") },
    StatDef { id: "trade_contract", display_name: "Trade Contract", source: StatSource::Currency(34), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/DEC2276FB1D5637BE563F618A012DEE63402170C/1767459.png") },
    StatDef { id: "elegy_mosaic", display_name: "Elegy Mosaic", source: StatSource::Currency(35), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/4C5D08BCD7F55B250B413F98A9FB4C323E3B0F93/1766338.png") },
    StatDef { id: "testimony_of_desert_heroics", display_name: "Testimony of Desert Heroics", source: StatSource::Currency(36), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/29A2465AE376091D3871B42D14D90B453060B69C/1822313.png") },
    StatDef { id: "exalted_key", display_name: "Exalted Key", source: StatSource::Currency(37), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/D8A351CBB13B24E49F9755BD290F710964B8FD99/1203022.png") },
    StatDef { id: "machete", display_name: "Machete", source: StatSource::Currency(38), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/2539DA0C2F1259A20DEE6707B8D8BCDA050372E2/1203080.png") },
    StatDef { id: "bandit_skeleton_key", display_name: "Bandit Skeleton Key", source: StatSource::Currency(40), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/332FF8697F27CEC0A599A008C901ACD8273E0C1E/65724.png") },
    StatDef { id: "pact_crowbar", display_name: "Pact Crowbar", source: StatSource::Currency(41), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/CC21772A9131AEAF4C18EE90172AE2A8DB1850BA/1203099.png") },
    StatDef { id: "vial_of_chak_acid", display_name: "Vial of Chak Acid", source: StatSource::Currency(42), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/B60BE97A3899A1D36CE81CB3C896CACACCA79237/1203046.png") },
    StatDef { id: "zephyrite_lockpick", display_name: "Zephyrite Lockpick", source: StatSource::Currency(43), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/A6F57A2C946B2A02062AC0A9452703505CF8B3BE/831466.png") },
    StatDef { id: "traders_key", display_name: "Trader's Key", source: StatSource::Currency(44), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/7CD20D4B072AD23F97202F05E4130CF8A4B069C5/69116.png") },
    StatDef { id: "volatile_magic", display_name: "Volatile Magic", source: StatSource::Currency(45), categories: &[Cur, OpenWorld], icon_url: Some("https://render.guildwars2.com/file/57F51A1F62E3FBB7B5E02CBD7C9717371D1CC8F2/1894697.png") },
    StatDef { id: "pvp_tournament_voucher", display_name: "PvP Tournament Voucher", source: StatSource::Currency(46), categories: &[Cur, Pvp], icon_url: Some("https://render.guildwars2.com/file/C30D1480E3383F999DE87921706D9D39E49CDC7A/2010217.png") },
    StatDef { id: "racing_medallion", display_name: "Racing Medallion", source: StatSource::Currency(47), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/555601A503A5DCFCC90C421AECF1ACEF70B2AFAC/2069367.png") },
    StatDef { id: "mistborn_key", display_name: "Mistborn Key", source: StatSource::Currency(49), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/2C47C1C30F61DB3A743BA47FB7E70808156A9009/2140520.png") },
    StatDef { id: "festival_token", display_name: "Festival Token", source: StatSource::Currency(50), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/63E1A0F023D101045B5BA2331C289327687FC7E3/797790.png") },
    StatDef { id: "cache_key", display_name: "Cache Key", source: StatSource::Currency(51), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/7D7CAD2BF4527006CFE6BE3DBF306353260DC8B2/2292743.png") },
    StatDef { id: "green_prophet_shard", display_name: "Green Prophet Shard", source: StatSource::Currency(53), categories: &[Cur, Strike], icon_url: Some("https://render.guildwars2.com/file/BFEDF6110C69BE3521010E7F23B5147919B90FA9/2270834.png") },
    StatDef { id: "blue_prophet_crystal", display_name: "Blue Prophet Crystal", source: StatSource::Currency(54), categories: &[Cur, Strike], icon_url: Some("https://render.guildwars2.com/file/2F531F63F5A9A93C624A3CDDC73F9078C00E5097/2270830.png") },
    StatDef { id: "green_prophet_crystal", display_name: "Green Prophet Crystal", source: StatSource::Currency(55), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/361BEAAEB637D90905060E4A59365118C7A6790D/2270831.png") },
    StatDef { id: "blue_prophet_shard", display_name: "Blue Prophet Shard", source: StatSource::Currency(57), categories: &[Cur, Strike], icon_url: Some("https://render.guildwars2.com/file/A0ECC4DEE6E84062649EA464079AE46B61089130/2270833.png") },
    StatDef { id: "war_supplies", display_name: "War Supplies", source: StatSource::Currency(58), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/3A930379AABCB10BDBE42F0FA436F29DD023C274/2293273.png") },
    StatDef { id: "unstable_fractal_essence", display_name: "Unstable Fractal Essence", source: StatSource::Currency(59), categories: &[Cur, Fractal], icon_url: Some("https://render.guildwars2.com/file/AD0AF3F321EB250B5FB4C44A7B3392CD1208921A/1202328.png") },
    StatDef { id: "tyrian_defense_seal", display_name: "Tyrian Defense Seal", source: StatSource::Currency(60), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/A004AF632A0ABA29ACFAB475AF1197B5BF80D9D9/2351816.png") },
    StatDef { id: "research_note", display_name: "Research Note", source: StatSource::Currency(61), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/BC2EACDA78BC950263F901E8EBA6E74D08E97E23/2596976.png") },
    StatDef { id: "unusual_coin", display_name: "Unusual Coin", source: StatSource::Currency(62), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/662D371A6C145EDDDA2DE6B53BC05F5F08CE386F/2596977.png") },
    StatDef { id: "astral_acclaim", display_name: "Astral Acclaim", source: StatSource::Currency(63), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/1856A01E331452E4C14E4C9CF4F818E3FAEF9B79/3124964.png") },
    StatDef { id: "jade_sliver", display_name: "Jade Sliver", source: StatSource::Currency(64), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/90FF7B7D0CBBC031B04F7E0A18576D7EBECB1C90/2596975.png") },
    StatDef { id: "testimony_of_jade_heroics", display_name: "Testimony of Jade Heroics", source: StatSource::Currency(65), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/7BFCF20F62C7DA650DFA69723E0B4F00E4F0ACF8/2597023.png") },
    StatDef { id: "ancient_coin", display_name: "Ancient Coin", source: StatSource::Currency(66), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/482B14330E349A01766C03412599B42DA103963D/3123700.png") },
    StatDef { id: "canach_coins", display_name: "Canach Coins", source: StatSource::Currency(67), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/F5FDD02592E10C3E63CF64D44713220FB6C70D03/2596973.png") },
    StatDef { id: "imperial_favor", display_name: "Imperial Favor", source: StatSource::Currency(68), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/D312AE6DB1AE2ECC2494772DF1D312D13B501D4E/2596974.png") },
    StatDef { id: "tales_of_dungeon_delving", display_name: "Tales of Dungeon Delving", source: StatSource::Currency(69), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/37CCE672250A3170B71760949C4C9C9B186517B1/619327.png") },
    StatDef { id: "legendary_insight", display_name: "Legendary Insight", source: StatSource::Currency(70), categories: &[Cur, Raid], icon_url: Some("https://render.guildwars2.com/file/6D33B7387BAF2E2CC9B5D37D1D1B01246AB6FA22/1302744.png") },
    StatDef { id: "jade_miners_keycard", display_name: "Jade Miner's Keycard", source: StatSource::Currency(71), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/5E741B943ACA14FBAEFD2B75CEEDAF4750BA20DA/3004443.png") },
    StatDef { id: "static_charge", display_name: "Static Charge", source: StatSource::Currency(72), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/314EF613F04029E9A8354FB8D79A72EDC36DDF10/3123702.png") },
    StatDef { id: "pinch_of_stardust", display_name: "Pinch of Stardust", source: StatSource::Currency(73), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/22DE50C72BCD2610345EA67F5E4032057EA2ABE5/3123701.png") },
    StatDef { id: "calcified_gasp", display_name: "Calcified Gasp", source: StatSource::Currency(75), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/0BF799CCA1DD262AE1F3B867900A002D080B56C3/3188140.png") },
    StatDef { id: "ursus_oblige", display_name: "Ursus Oblige", source: StatSource::Currency(76), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/0CCE7302D97D26A695BDD40863EED11D0B221DE7/3593176.png") },
    StatDef { id: "gaeting_crystal", display_name: "Gaeting Crystal", source: StatSource::Currency(77), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/0C05B2E6F012B209C3095CDD6AF5F84B0BA9CC3A/3442797.png") },
    StatDef { id: "fine_rift_essence", display_name: "Fine Rift Essence", source: StatSource::Currency(78), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/41D633F8F0CCFAD7FDADEF7CE84BF7C312AA1B49/3630022.png") },
    StatDef { id: "rare_rift_essence", display_name: "Rare Rift Essence", source: StatSource::Currency(79), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/A6012206459C56680D1BD4D23E0B706F0B0AE40D/3630024.png") },
    StatDef { id: "masterwork_rift_essence", display_name: "Masterwork Rift Essence", source: StatSource::Currency(80), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/E0A96441F8405ABEF06114BE750154583CF3B1D2/3630023.png") },
    StatDef { id: "antiquated_ducat", display_name: "Antiquated Ducat", source: StatSource::Currency(81), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/F891B355BC31BD7B4103FE5DF9ACB2FCF928F4CB/3710051.png") },
    StatDef { id: "testimony_of_castoran_heroics", display_name: "Testimony of Castoran Heroics", source: StatSource::Currency(82), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/D6BC79630AFF64123ED10DF9D065CCA657E0767E/3710074.png") },
    StatDef { id: "aether_rich_sap", display_name: "Aether-Rich Sap", source: StatSource::Currency(83), categories: &[Cur], icon_url: Some("https://render.guildwars2.com/file/79F23C52AF0AA29A976877285FF904BCA2D122FE/3710050.png") },
    // material storage), not lifetime totals, hence "(current)" in the name.
    StatDef { id: "heavy_loot_bag", display_name: "Heavy Loot Bag (current)", source: StatSource::Item(8920), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/EBC5CEC199D1E51B02756A1C796A65E9D24F04B5/63171.png") },
    StatDef { id: "memory_of_battle", display_name: "Memory of Battle (current)", source: StatSource::Item(71581), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/E4D0455D2EFDB0DFC008F4564B38D6545901A05B/1206833.png") },
    StatDef { id: "emblem_of_the_avenger", display_name: "Emblem of the Avenger (current)", source: StatSource::Item(93075), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/C756AF2212663BAAB926E1EE064CFE7E2FF0EE97/2270824.png") },
    StatDef { id: "emblem_of_the_conqueror", display_name: "Emblem of the Conqueror (current)", source: StatSource::Item(93146), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/D548CB0A04FA28BC76A91A6D8045E31A06DD417B/2270825.png") },
    StatDef { id: "grandmaster_mark_shard", display_name: "Grandmaster Mark Shard (current)", source: StatSource::Item(87557), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/6746AD0BC7EEB85962D9D05B6B7E59ECCE7C726D/1986161.png") },
    StatDef { id: "skirmish_chest_1", display_name: "Skirmish Chest Tier 1 (current)", source: StatSource::Item(84966), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/1878EFD0E96164B9E6CCB765E9C4453204139744/1701123.png") },
    StatDef { id: "skirmish_chest_2", display_name: "Skirmish Chest Tier 2 (current)", source: StatSource::Item(96536), categories: &[Wvw], icon_url: Some("https://render.guildwars2.com/file/1878EFD0E96164B9E6CCB765E9C4453204139744/1701123.png") },
    StatDef { id: "unidentified_gear_common", display_name: "Piece of Common Unidentified Gear (current)", source: StatSource::Item(85016), categories: &[Misc], icon_url: Some("https://render.guildwars2.com/file/E37A036C10C33E4242E568690CB2EA55AA65B915/1938436.png") },
    StatDef { id: "unidentified_gear", display_name: "Piece of Unidentified Gear (current)", source: StatSource::Item(84731), categories: &[Misc], icon_url: Some("https://render.guildwars2.com/file/B147379DFC5430E207FCB742804E199EDF727719/1766400.png") },
    StatDef { id: "unidentified_gear_rare", display_name: "Piece of Rare Unidentified Gear (current)", source: StatSource::Item(83008), categories: &[Misc], icon_url: Some("https://render.guildwars2.com/file/EF63A10BD2317CECCEA63A3B7E6555550B414C4E/1766399.png") },
    StatDef { id: "essence_of_luck_fine", display_name: "Essence of Luck, Fine (current)", source: StatSource::Item(45175), categories: &[Misc], icon_url: Some("https://render.guildwars2.com/file/1BF5D192EE5DAF97A7F4090461C450DA00F8FFAC/631148.png") },
    StatDef { id: "essence_of_luck_masterwork", display_name: "Essence of Luck, Masterwork (current)", source: StatSource::Item(45176), categories: &[Misc], icon_url: Some("https://render.guildwars2.com/file/07450110280000435FBA2B4BE57DE6DCE86E22AC/631149.png") },
    StatDef { id: "essence_of_luck_rare", display_name: "Essence of Luck, Rare (current)", source: StatSource::Item(45177), categories: &[Misc], icon_url: Some("https://render.guildwars2.com/file/4FA2D6CEF9039B402F2695CF2E740B4CF6F50753/631150.png") },
    StatDef { id: "essence_of_luck_exotic", display_name: "Essence of Luck, Exotic (current)", source: StatSource::Item(45178), categories: &[Misc], icon_url: Some("https://render.guildwars2.com/file/DAB46301D2175B2CAAC4BACBA02F6A0A2F1DBEB8/631151.png") },
    StatDef { id: "essence_of_luck_legendary", display_name: "Essence of Luck, Legendary (current)", source: StatSource::Item(45179), categories: &[Misc], icon_url: Some("https://render.guildwars2.com/file/DB6E0EFF01587F1C44DD131DCBB34BA7D27CC7EF/631152.png") },
    StatDef { id: "mystic_coin", display_name: "Mystic Coin (current)", source: StatSource::Item(19976), categories: &[Fractal, Raid], icon_url: Some("https://render.guildwars2.com/file/AB0317DF5B0E1BA47436A5420248660765154C08/62864.png") },
    StatDef { id: "fractal_encryption", display_name: "Fractal Encryption (current)", source: StatSource::Item(75919), categories: &[Fractal], icon_url: Some("https://render.guildwars2.com/file/0FE0E9AA9080D07A2A0EE3141A1FEBBC0DC5F819/1200196.png") },
    StatDef { id: "coffer_of_the_dragon_ball_champion", display_name: "Coffer of the Dragon Ball Champion (current)", source: StatSource::Item(68617), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/08B96751629476BBF8D20B524803BE6914936B4D/947640.png") },
    StatDef { id: "little_lucky_envelope", display_name: "Little Lucky Envelope (current)", source: StatSource::Item(68645), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/4D0D1A0832670F033701CC65AA34E4AE6047E406/947656.png") },
    StatDef { id: "divine_lucky_envelope", display_name: "Divine Lucky Envelope (current)", source: StatSource::Item(68646), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/9D94B96446F269662F6ACC2531394A06C0E03951/947657.png") },
    StatDef { id: "dragon_ball_champions_divine_lucky_envelope", display_name: "Dragon Ball Champion's Divine Lucky Envelope (current)", source: StatSource::Item(68647), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/9D94B96446F269662F6ACC2531394A06C0E03951/947657.png") },
    StatDef { id: "lucky_red_bag", display_name: "Lucky Red Bag (current)", source: StatSource::Item(94653), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/D5ED66E2CE457F9EB1B02978D80FC7A23A6E5E97/2393755.png") },
    StatDef { id: "token_of_the_celestial_champion", display_name: "Token of the Celestial Champion (current)", source: StatSource::Item(92659), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/67711D644F17915EBAA63B7A30DF53B264E6121F/2242654.png") },
    StatDef { id: "token_of_the_celestial_champion_fragment", display_name: "Token of the Celestial Champion Fragment (current)", source: StatSource::Item(94668), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/064494A1C6F2B47F366690F91816B246F61A9A58/2393784.png") },
    StatDef { id: "token_of_the_dragon_ball_champion", display_name: "Token of the Dragon Ball Champion (current)", source: StatSource::Item(68618), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/BAC1DA6A6AABA57F7D2D2F4FD35AA80828640791/1341448.png") },
    StatDef { id: "bauble", display_name: "Bauble (current)", source: StatSource::Item(39752), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/AE5AE77B1B291BA6A418B9F80B43CF3D437D0806/561983.png") },
    StatDef { id: "bauble_bubble", display_name: "Bauble Bubble (current)", source: StatSource::Item(41886), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/231E351E042921F8BE94B2E3717D2DF734D4B9F4/561863.png") },
    StatDef { id: "continue_coin", display_name: "Continue Coin (current)", source: StatSource::Item(41824), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/669B427B9735DC61EF3A566064083F1DC37E9D5A/561825.png") },
    StatDef { id: "crimson_assassin_token", display_name: "Crimson Assassin Token (current)", source: StatSource::Item(80890), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/DDE59EA1676B099CE10C0F2A1BF8704EFDCB73D2/1663867.png") },
    StatDef { id: "fancy_furniture_coin", display_name: "Fancy Furniture Coin (current)", source: StatSource::Item(78062), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/D9DD3377179B516544117DC5F54B044320E20EC3/1418362.png") },
    StatDef { id: "dragon_coffer", display_name: "Dragon Coffer (current)", source: StatSource::Item(43357), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/3CA03D7273EE0469B12A40BDEE5DE0E5064F3E70/591468.png") },
    StatDef { id: "trick_or_treat_bag", display_name: "Trick-or-Treat Bag (current)", source: StatSource::Item(36038), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/E6017363449406DEE3DD3B80263AA2A91716F1DE/499375.png") },
    StatDef { id: "wintersday_gift", display_name: "Wintersday Gift (current)", source: StatSource::Item(77604), categories: &[Festival], icon_url: Some("https://render.guildwars2.com/file/420CE6DF804B1429750A115F9B6CEDA504A036F4/526220.png") },
];

/// The full stat catalog: the hand-curated `CORE_STATS` plus the
/// generated Material Storage entries. Built once on first access.
pub static STAT_CATALOG: std::sync::LazyLock<Vec<StatDef>> = std::sync::LazyLock::new(|| {
    CORE_STATS
        .iter()
        .copied()
        .chain(crate::material_storage::MATERIAL_STORAGE_STATS.iter().copied())
        .collect()
});

pub fn compute_lifetime_values(snapshot: &ApiSnapshot) -> HashMap<&'static str, f64> {
    let mut values = HashMap::new();
    for stat in STAT_CATALOG.iter() {
        let value = match stat.source {
            StatSource::Achievement(id) => {
                snapshot.achievements.get(&id).copied().unwrap_or(0) as f64
            }
            StatSource::WvwRank => snapshot.wvw_rank as f64,
            StatSource::Deaths => snapshot.total_deaths as f64,
            StatSource::Currency(id) => snapshot.currencies.get(&id).copied().unwrap_or(0) as f64,
            StatSource::PvpRank => snapshot.pvp_rank as f64,
            StatSource::PvpWins => snapshot.pvp_wins as f64,
            StatSource::PvpLosses => snapshot.pvp_losses as f64,
            StatSource::PvpRankingPoints => snapshot.pvp_ranking_points as f64,
            StatSource::PvpRankedWins => snapshot.pvp_ranked_wins as f64,
            StatSource::PvpRankedLosses => snapshot.pvp_ranked_losses as f64,
            StatSource::PvpUnrankedWins => snapshot.pvp_unranked_wins as f64,
            StatSource::PvpUnrankedLosses => snapshot.pvp_unranked_losses as f64,
            StatSource::Item(id) => snapshot.items.get(&id).copied().unwrap_or(0) as f64,
            // computed below once their inputs are known
            StatSource::Kdr
            | StatSource::PvpKdr
            | StatSource::PvpCustomWins
            | StatSource::PvpCustomLosses => continue,
        };
        values.insert(stat.id, value);
    }

    let kills = values.get("kills").copied().unwrap_or(0.0);
    let deaths = values.get("deaths").copied().unwrap_or(0.0);
    let kdr = if deaths > 0.0 { kills / deaths } else { kills };
    values.insert("kdr", kdr);

    let pvp_kills = values.get("pvp_kills").copied().unwrap_or(0.0);
    let pvp_kdr = if deaths > 0.0 { pvp_kills / deaths } else { pvp_kills };
    values.insert("pvp_kdr", pvp_kdr);

    let pvp_custom_wins = snapshot.pvp_wins as f64
        - snapshot.pvp_ranked_wins as f64
        - snapshot.pvp_unranked_wins as f64;
    let pvp_custom_losses = snapshot.pvp_losses as f64
        - snapshot.pvp_ranked_losses as f64
        - snapshot.pvp_unranked_losses as f64;
    values.insert("pvp_custom_wins", pvp_custom_wins);
    values.insert("pvp_custom_losses", pvp_custom_losses);

    values
}

/// Whether `id` should never be allowed to regress between polls. The GW2
/// API occasionally reports a lower value than the previous poll for
/// achievement progress and death counts, then self-corrects on the next
/// poll - a documented, transient API bug rather than a real drop.
/// Nothing else is guarded: currencies and items legitimately decrease
/// (spending, salvaging), and computed ratios like KDR legitimately drop
/// too (dying without a kill).
pub fn is_regression_guarded(id: &str) -> bool {
    STAT_CATALOG
        .iter()
        .any(|s| s.id == id && matches!(s.source, StatSource::Achievement(_) | StatSource::Deaths))
}

/// Resolves persisted selected-stat ids into catalog entries, in the
/// user's chosen order. Ids that no longer exist in `STAT_CATALOG` (e.g. a
/// stat later removed from the catalog) are silently dropped.
pub fn resolve_selected_stats(selected_ids: &[String]) -> Vec<&'static StatDef> {
    selected_ids
        .iter()
        .filter_map(|id| STAT_CATALOG.iter().find(|s| s.id == id.as_str()))
        .collect()
}

/// Toggles `id` in `selected`: appends if absent, removes if present.
/// No-op if `id` isn't a valid `STAT_CATALOG` id.
pub fn toggle_stat(selected: &mut Vec<String>, id: &str) {
    if !STAT_CATALOG.iter().any(|s| s.id == id) {
        return;
    }
    match selected.iter().position(|s| s == id) {
        Some(pos) => {
            selected.remove(pos);
        }
        None => selected.push(id.to_string()),
    }
}

/// Selects every stat in the catalog, in catalog order.
pub fn select_all(selected: &mut Vec<String>) {
    *selected = STAT_CATALOG.iter().map(|s| s.id.to_string()).collect();
}

/// Clears the selection.
pub fn unselect_all(selected: &mut Vec<String>) {
    selected.clear();
}

/// Adds every id in `ids` to `selected` that isn't already present
/// (appended in `ids` order). Used for "select all in category" buttons,
/// where `ids` is a subset of the catalog rather than all of it.
pub fn select_ids(selected: &mut Vec<String>, ids: &[&str]) {
    for id in ids {
        if !selected.iter().any(|s| s == id) {
            selected.push(id.to_string());
        }
    }
}

/// Removes every id in `ids` from `selected`. Used for "unselect all in
/// category" buttons.
pub fn unselect_ids(selected: &mut Vec<String>, ids: &[&str]) {
    selected.retain(|s| !ids.contains(&s.as_str()));
}

/// Swaps `id` with its predecessor in `selected`. No-op if `id` is
/// already first, or isn't present.
pub fn move_stat_up(selected: &mut [String], id: &str) {
    if let Some(pos) = selected.iter().position(|s| s == id) {
        if pos > 0 {
            selected.swap(pos, pos - 1);
        }
    }
}

/// Swaps `id` with its successor in `selected`. No-op if `id` is already
/// last, or isn't present.
pub fn move_stat_down(selected: &mut [String], id: &str) {
    if let Some(pos) = selected.iter().position(|s| s == id) {
        if pos + 1 < selected.len() {
            selected.swap(pos, pos + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ApiSnapshot;
    use std::collections::HashMap as StdHashMap;

    fn snapshot(wvw_rank: u64, achievements: &[(u32, u64)], total_deaths: u64) -> ApiSnapshot {
        let mut map = StdHashMap::new();
        for (id, value) in achievements {
            map.insert(*id, *value);
        }
        ApiSnapshot {
            wvw_rank,
            achievements: map,
            total_deaths,
            currencies: StdHashMap::new(),
            pvp_rank: 0,
            pvp_wins: 0,
            pvp_losses: 0,
            pvp_ranking_points: 0,
            pvp_ranked_wins: 0,
            pvp_ranked_losses: 0,
            pvp_unranked_wins: 0,
            pvp_unranked_losses: 0,
            items: StdHashMap::new(),
        }
    }

    #[test]
    fn maps_achievement_ids_to_stat_values() {
        let snap = snapshot(0, &[(283, 500), (306, 12000)], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["kills"], 500.0);
        assert_eq!(values["supply_repair"], 12000.0);
    }

    #[test]
    fn missing_achievement_defaults_to_zero() {
        let snap = snapshot(0, &[], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["dolyaks_killed"], 0.0);
    }

    #[test]
    fn computes_kdr_from_kills_and_deaths() {
        let snap = snapshot(0, &[(283, 100)], 25);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["kdr"], 4.0);
    }

    #[test]
    fn kdr_falls_back_to_kills_when_no_deaths() {
        let snap = snapshot(0, &[(283, 7)], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["kdr"], 7.0);
    }

    #[test]
    fn maps_wvw_rank_and_deaths() {
        let snap = snapshot(1500, &[], 42);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["wvw_rank"], 1500.0);
        assert_eq!(values["deaths"], 42.0);
    }

    #[test]
    fn catalog_has_eight_hundred_eight_stats() {
        // 17 WvW + 12 PvP + 67 currencies + 33 items + 679 material storage items
        assert_eq!(STAT_CATALOG.len(), 808);
    }

    #[test]
    fn catalog_has_sixty_seven_currencies() {
        let count = STAT_CATALOG
            .iter()
            .filter(|s| matches!(s.source, StatSource::Currency(_)))
            .count();
        assert_eq!(count, 67);
    }

    #[test]
    fn catalog_has_seven_hundred_twelve_items() {
        // 33 non-material-storage items + 679 material storage items
        let count = STAT_CATALOG
            .iter()
            .filter(|s| matches!(s.source, StatSource::Item(_)))
            .count();
        assert_eq!(count, 712);
    }

    #[test]
    fn material_storage_has_nine_categories_matching_live_api_counts() {
        let expected: &[(Category, usize)] = &[
            (Category::BasicCraftingMaterials, 79),
            (Category::IntermediateCraftingMaterials, 65),
            (Category::AdvancedCraftingMaterials, 157),
            (Category::AscendedMaterials, 58),
            (Category::GemstonesAndJewels, 60),
            (Category::CookingMaterials, 135),
            (Category::CookingIngredients, 37),
            (Category::ScribingMaterials, 69),
            (Category::FestiveMaterials, 19),
        ];
        for &(category, count) in expected {
            let actual = STAT_CATALOG
                .iter()
                .filter(|s| s.categories.contains(&category))
                .count();
            assert_eq!(actual, count, "{category:?} expected {count}, got {actual}");
        }
    }

    #[test]
    fn pile_of_soybeans_appears_exactly_once() {
        // Present in both Cooking Materials and Cooking Ingredients on the
        // live API; the generator dedups to the first category (order).
        let matches = STAT_CATALOG
            .iter()
            .filter(|s| matches!(s.source, StatSource::Item(97105)))
            .count();
        assert_eq!(matches, 1);
    }

    #[test]
    fn item_stats_are_labeled_current_not_lifetime() {
        for stat in STAT_CATALOG.iter().filter(|s| matches!(s.source, StatSource::Item(_))) {
            assert!(
                stat.display_name.ends_with("(current)"),
                "{} should be labeled (current)",
                stat.display_name
            );
        }
    }

    #[test]
    fn mystic_coin_is_tagged_fractal_and_raid() {
        let mystic_coin = STAT_CATALOG.iter().find(|s| s.id == "mystic_coin").unwrap();
        assert!(mystic_coin.categories.contains(&Category::Fractal));
        assert!(mystic_coin.categories.contains(&Category::Raid));
    }

    #[test]
    fn currency_and_item_stats_have_icon_urls() {
        let gold = STAT_CATALOG.iter().find(|s| s.id == "gold").unwrap();
        assert!(gold.icon_url.unwrap().starts_with("https://render.guildwars2.com"));

        let heavy_loot_bag = STAT_CATALOG.iter().find(|s| s.id == "heavy_loot_bag").unwrap();
        assert!(heavy_loot_bag.icon_url.unwrap().starts_with("https://render.guildwars2.com"));

        let material_item = STAT_CATALOG.iter().find(|s| s.id == "lump_of_tin").unwrap();
        assert!(material_item.icon_url.unwrap().starts_with("https://render.guildwars2.com"));
    }

    #[test]
    fn achievement_and_computed_wvw_stats_fall_back_to_wvw_category_icon() {
        // GW2's /v2/achievements has no icon field at all, and computed/
        // account-field stats have no natural GW2 icon either - both fall
        // back to the shared WvW achievement-category icon.
        for id in ["kills", "deaths", "kdr", "wvw_rank"] {
            let stat = STAT_CATALOG.iter().find(|s| s.id == id).unwrap();
            assert_eq!(stat.icon_url, Some(WVW_FALLBACK_ICON_URL), "{id}");
        }
    }

    #[test]
    fn achievement_and_computed_pvp_stats_fall_back_to_pvp_category_icon() {
        for id in ["pvp_kills", "pvp_kdr", "pvp_rank", "pvp_ranking_points"] {
            let stat = STAT_CATALOG.iter().find(|s| s.id == id).unwrap();
            assert_eq!(stat.icon_url, Some(PVP_FALLBACK_ICON_URL), "{id}");
        }
    }

    #[test]
    fn pvp_rank_icon_url_resolves_correct_tier() {
        assert_eq!(pvp_rank_icon_url(1), PVP_RANK_TIERS[0].icon_url); // Rabbit
        assert_eq!(pvp_rank_icon_url(9), PVP_RANK_TIERS[0].icon_url); // still Rabbit
        assert_eq!(pvp_rank_icon_url(10), PVP_RANK_TIERS[1].icon_url); // Deer
        assert_eq!(pvp_rank_icon_url(80), PVP_RANK_TIERS[8].icon_url); // Dragon
        assert_eq!(pvp_rank_icon_url(200), PVP_RANK_TIERS[8].icon_url); // still Dragon, past top tier
    }

    #[test]
    fn pvp_rank_icon_url_handles_zero_rank() {
        // Below the lowest tier's min_rank (1) shouldn't happen in
        // practice, but must not panic.
        assert_eq!(pvp_rank_icon_url(0), PVP_RANK_TIERS[0].icon_url);
    }

    #[test]
    fn maps_item_ids_to_stat_values() {
        let mut snap = snapshot(0, &[], 0);
        snap.items.insert(8920, 12);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["heavy_loot_bag"], 12.0);
    }

    #[test]
    fn missing_item_defaults_to_zero() {
        let snap = snapshot(0, &[], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["memory_of_battle"], 0.0);
    }

    #[test]
    fn deaths_is_tagged_wvw_pvp_and_misc() {
        let deaths = STAT_CATALOG.iter().find(|s| s.id == "deaths").unwrap();
        assert!(deaths.categories.contains(&Category::Wvw));
        assert!(deaths.categories.contains(&Category::Pvp));
        assert!(deaths.categories.contains(&Category::Misc));
    }

    #[test]
    fn gold_is_cross_tagged_into_every_activity_category() {
        let gold = STAT_CATALOG.iter().find(|s| s.id == "gold").unwrap();
        for cat in [
            Category::Currency,
            Category::Wvw,
            Category::Pvp,
            Category::Fractal,
            Category::Raid,
            Category::OpenWorld,
            Category::Strike,
        ] {
            assert!(gold.categories.contains(&cat), "gold missing {cat:?}");
        }
    }

    #[test]
    fn laurels_is_currency_only() {
        let laurels = STAT_CATALOG.iter().find(|s| s.id == "laurels").unwrap();
        assert_eq!(laurels.categories, &[Category::Currency]);
    }

    #[test]
    fn maps_currency_ids_to_stat_values() {
        let mut snap = snapshot(0, &[], 0);
        snap.currencies.insert(1, 100001);
        snap.currencies.insert(4, 50);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["gold"], 100001.0);
        assert_eq!(values["gems"], 50.0);
    }

    #[test]
    fn missing_currency_defaults_to_zero() {
        let snap = snapshot(0, &[], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["karma"], 0.0);
    }

    #[test]
    fn maps_pvp_rank_wins_and_losses() {
        let mut snap = snapshot(0, &[], 0);
        snap.pvp_rank = 45;
        snap.pvp_wins = 120;
        snap.pvp_losses = 80;
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_rank"], 45.0);
        assert_eq!(values["pvp_wins"], 120.0);
        assert_eq!(values["pvp_losses"], 80.0);
    }

    #[test]
    fn maps_pvp_ranking_points_and_ranked_unranked_splits() {
        let mut snap = snapshot(0, &[], 0);
        snap.pvp_ranking_points = 300;
        snap.pvp_ranked_wins = 10;
        snap.pvp_ranked_losses = 4;
        snap.pvp_unranked_wins = 30;
        snap.pvp_unranked_losses = 20;
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_ranking_points"], 300.0);
        assert_eq!(values["pvp_ranked_wins"], 10.0);
        assert_eq!(values["pvp_ranked_losses"], 4.0);
        assert_eq!(values["pvp_unranked_wins"], 30.0);
        assert_eq!(values["pvp_unranked_losses"], 20.0);
    }

    #[test]
    fn computes_pvp_custom_wins_and_losses_as_remainder() {
        let mut snap = snapshot(0, &[], 0);
        snap.pvp_wins = 120;
        snap.pvp_losses = 80;
        snap.pvp_ranked_wins = 10;
        snap.pvp_ranked_losses = 4;
        snap.pvp_unranked_wins = 30;
        snap.pvp_unranked_losses = 20;
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_custom_wins"], 80.0); // 120 - 10 - 30
        assert_eq!(values["pvp_custom_losses"], 56.0); // 80 - 4 - 20
    }

    #[test]
    fn computes_pvp_kdr_from_pvp_kills_achievement_and_shared_deaths() {
        let snap = snapshot(0, &[(239, 50)], 25);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_kills"], 50.0);
        assert_eq!(values["pvp_kdr"], 2.0);
    }

    #[test]
    fn pvp_kdr_falls_back_to_kills_when_no_deaths() {
        let snap = snapshot(0, &[(239, 7)], 0);
        let values = compute_lifetime_values(&snap);
        assert_eq!(values["pvp_kdr"], 7.0);
    }

    #[test]
    fn resolve_selected_stats_preserves_order() {
        let selected = vec!["kdr".to_string(), "kills".to_string()];
        let resolved = resolve_selected_stats(&selected);
        assert_eq!(resolved.iter().map(|s| s.id).collect::<Vec<_>>(), vec!["kdr", "kills"]);
    }

    #[test]
    fn resolve_selected_stats_skips_unknown_ids() {
        let selected = vec!["kills".to_string(), "not_a_real_stat".to_string(), "deaths".to_string()];
        let resolved = resolve_selected_stats(&selected);
        assert_eq!(resolved.iter().map(|s| s.id).collect::<Vec<_>>(), vec!["kills", "deaths"]);
    }

    #[test]
    fn resolve_selected_stats_empty_input_yields_empty_output() {
        assert!(resolve_selected_stats(&[]).is_empty());
    }

    #[test]
    fn toggle_stat_adds_then_removes() {
        let mut selected = vec![];
        toggle_stat(&mut selected, "kills");
        assert_eq!(selected, vec!["kills"]);
        toggle_stat(&mut selected, "kills");
        assert!(selected.is_empty());
    }

    #[test]
    fn toggle_stat_ignores_unknown_id() {
        let mut selected = vec![];
        toggle_stat(&mut selected, "not_a_real_stat");
        assert!(selected.is_empty());
    }

    #[test]
    fn select_ids_adds_only_missing_ids() {
        let mut selected = vec!["kills".to_string()];
        select_ids(&mut selected, &["kills", "deaths", "kdr"]);
        assert_eq!(selected, vec!["kills", "deaths", "kdr"]);
    }

    #[test]
    fn unselect_ids_removes_only_listed_ids() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string(), "kdr".to_string()];
        unselect_ids(&mut selected, &["deaths", "kdr"]);
        assert_eq!(selected, vec!["kills"]);
    }

    #[test]
    fn select_all_yields_full_catalog_in_order() {
        let mut selected = vec![];
        select_all(&mut selected);
        assert_eq!(selected, STAT_CATALOG.iter().map(|s| s.id.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn unselect_all_clears() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        unselect_all(&mut selected);
        assert!(selected.is_empty());
    }

    #[test]
    fn move_stat_up_swaps_with_predecessor() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string(), "kdr".to_string()];
        move_stat_up(&mut selected, "kdr");
        assert_eq!(selected, vec!["kills", "kdr", "deaths"]);
    }

    #[test]
    fn move_stat_down_swaps_with_successor() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string(), "kdr".to_string()];
        move_stat_down(&mut selected, "kills");
        assert_eq!(selected, vec!["deaths", "kills", "kdr"]);
    }

    #[test]
    fn move_stat_up_is_noop_when_already_first() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_up(&mut selected, "kills");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }

    #[test]
    fn move_stat_up_is_noop_for_absent_id() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_up(&mut selected, "kdr");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }

    #[test]
    fn move_stat_down_is_noop_when_already_last() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_down(&mut selected, "deaths");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }

    #[test]
    fn move_stat_down_is_noop_for_absent_id() {
        let mut selected = vec!["kills".to_string(), "deaths".to_string()];
        move_stat_down(&mut selected, "kdr");
        assert_eq!(selected, vec!["kills", "deaths"]);
    }
}
