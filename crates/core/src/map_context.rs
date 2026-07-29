#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapGroup {
    Wvw,
    Pvp,
    Pve,
}

// Mirrors gw2_mumble::map_type (not a dependency of this crate - see
// crates/core/Cargo.toml). Values per the GW2 MumbleLink API, stable.
mod map_type {
    pub const PVP: u32 = 2;
    pub const GVG: u32 = 3;
    pub const INSTANCE: u32 = 4;
    pub const PVE: u32 = 5;
    pub const TOURNAMENT: u32 = 6;
    pub const USER_TOURNAMENT: u32 = 8;
    pub const WVW_ETERNAL_BATTLEGROUNDS: u32 = 9;
    pub const WVW_BLUE_BORDERLANDS: u32 = 10;
    pub const WVW_GREEN_BORDERLANDS: u32 = 11;
    pub const WVW_RED_BORDERLANDS: u32 = 12;
    pub const WVW_REWARD: u32 = 13;
    pub const WVW_OBSIDIAN_SANCTUM: u32 = 14;
    pub const WVW_EDGE_OF_THE_MISTS: u32 = 15;
    pub const PVE_MINI: u32 = 16;
    pub const BIG_BATTLE: u32 = 17;
    pub const WVW_LOUNGE: u32 = 18;
}

/// `None` for character creation/tutorial/auto-redirect/unrecognized ids -
/// no mode list applies, only Global renders.
pub fn map_group_for(map_type_id: u32) -> Option<MapGroup> {
    use map_type as mt;
    match map_type_id {
        mt::PVP | mt::GVG | mt::TOURNAMENT | mt::USER_TOURNAMENT | mt::BIG_BATTLE => Some(MapGroup::Pvp),
        mt::WVW_ETERNAL_BATTLEGROUNDS
        | mt::WVW_BLUE_BORDERLANDS
        | mt::WVW_GREEN_BORDERLANDS
        | mt::WVW_RED_BORDERLANDS
        | mt::WVW_REWARD
        | mt::WVW_OBSIDIAN_SANCTUM
        | mt::WVW_EDGE_OF_THE_MISTS
        | mt::WVW_LOUNGE => Some(MapGroup::Wvw),
        mt::PVE | mt::PVE_MINI | mt::INSTANCE => Some(MapGroup::Pve),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pvp_arena_maps_to_pvp() {
        assert_eq!(map_group_for(map_type::PVP), Some(MapGroup::Pvp));
    }

    #[test]
    fn big_battle_maps_to_pvp() {
        assert_eq!(map_group_for(map_type::BIG_BATTLE), Some(MapGroup::Pvp));
    }

    #[test]
    fn eternal_battlegrounds_maps_to_wvw() {
        assert_eq!(map_group_for(map_type::WVW_ETERNAL_BATTLEGROUNDS), Some(MapGroup::Wvw));
    }

    #[test]
    fn borderlands_map_to_wvw() {
        assert_eq!(map_group_for(map_type::WVW_BLUE_BORDERLANDS), Some(MapGroup::Wvw));
        assert_eq!(map_group_for(map_type::WVW_GREEN_BORDERLANDS), Some(MapGroup::Wvw));
        assert_eq!(map_group_for(map_type::WVW_RED_BORDERLANDS), Some(MapGroup::Wvw));
    }

    #[test]
    fn wvw_lounge_maps_to_wvw() {
        assert_eq!(map_group_for(map_type::WVW_LOUNGE), Some(MapGroup::Wvw));
    }

    #[test]
    fn open_world_maps_to_pve() {
        assert_eq!(map_group_for(map_type::PVE), Some(MapGroup::Pve));
    }

    #[test]
    fn mini_map_maps_to_pve() {
        assert_eq!(map_group_for(map_type::PVE_MINI), Some(MapGroup::Pve));
    }

    #[test]
    fn instance_maps_to_pve() {
        // Fractals, raids, strikes, dungeons, and story instances are all
        // `INSTANCE` - this grouping is coarse by design (see plan).
        assert_eq!(map_group_for(map_type::INSTANCE), Some(MapGroup::Pve));
    }

    #[test]
    fn character_creation_has_no_map_group() {
        assert_eq!(map_group_for(1), None);
    }

    #[test]
    fn unrecognized_map_type_has_no_map_group() {
        assert_eq!(map_group_for(9999), None);
    }
}
