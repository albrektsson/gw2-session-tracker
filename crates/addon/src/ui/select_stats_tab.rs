use nexus::imgui::{TreeNodeFlags, Ui};
use std::{
    cell::RefCell,
    path::Path,
    sync::{Arc, Mutex},
};
use session_tracker_core::{
    config::save_config,
    stats::{
        select_all, select_ids, toggle_stat, unselect_all, unselect_ids, Category, StatDef,
        STAT_CATALOG, SUPERCATEGORIES,
    },
    sync::lock_recover,
};
use session_tracker_net::state::AppState;

use super::settings_window::config_from_state;

thread_local! {
    static SEARCH_FILTER: RefCell<String> = const { RefCell::new(String::new()) };
}

fn persist(state: &AppState, addon_dir: &Path) {
    let config = config_from_state(state);
    if let Err(err) = save_config(addon_dir, &config) {
        log::warn!("failed to save session tracker config: {err}");
    }
}

fn category_display_name(category: Category) -> &'static str {
    match category {
        Category::Misc => "Misc",
        Category::Currency => "Currencies",
        Category::Festival => "Festival",
        Category::Wvw => "WvW",
        Category::Pvp => "PvP",
        Category::OpenWorld => "Open World",
        Category::Fractal => "Fractal",
        Category::Raid => "Raid",
        Category::Strike => "Strike Mission",
        Category::BasicCraftingMaterials => "Basic Crafting Materials",
        Category::IntermediateCraftingMaterials => "Intermediate Crafting Materials",
        Category::AdvancedCraftingMaterials => "Advanced Crafting Materials",
        Category::AscendedMaterials => "Ascended Materials",
        Category::GemstonesAndJewels => "Gemstones and Jewels",
        Category::CookingMaterials => "Cooking Materials",
        Category::CookingIngredients => "Cooking Ingredients",
        Category::ScribingMaterials => "Scribing Materials",
        Category::FestiveMaterials => "Festive Materials",
    }
}

fn stats_in_category(category: Category, needle: &str) -> Vec<&'static StatDef> {
    STAT_CATALOG
        .iter()
        .filter(|s| s.categories.contains(&category))
        .filter(|s| needle.is_empty() || s.display_name.to_lowercase().contains(needle))
        .collect()
}

pub fn render_select_stats_tab(ui: &Ui, shared: &Arc<Mutex<AppState>>, addon_dir: &Path) {
    SEARCH_FILTER.with(|filter| {
        let mut query = filter.borrow_mut();
        ui.input_text("##stat_search", &mut query)
            .hint("Search stats...")
            .build();

        if ui.button("Select all") {
            let mut state = lock_recover(shared);
            select_all(&mut state.selected_stats);
            persist(&state, addon_dir);
        }
        ui.same_line();
        if ui.button("Unselect all") {
            let mut state = lock_recover(shared);
            unselect_all(&mut state.selected_stats);
            persist(&state, addon_dir);
        }

        ui.separator();

        let needle = query.to_lowercase();
        let mut state = lock_recover(shared);

        if let Some(timer) = STAT_CATALOG.iter().find(|s| s.id == "session_timer")
            && (needle.is_empty() || timer.display_name.to_lowercase().contains(&needle))
        {
            let mut checked = state.selected_stats.iter().any(|id| id == timer.id);
            if ui.checkbox(timer.display_name, &mut checked) {
                toggle_stat(&mut state.selected_stats, timer.id);
                persist(&state, addon_dir);
            }
            ui.separator();
        }

        for (supercategory_name, subcategories) in SUPERCATEGORIES {
            if !ui.collapsing_header(*supercategory_name, TreeNodeFlags::DEFAULT_OPEN) {
                continue;
            }
            ui.indent();

            for &category in *subcategories {
                let stats = stats_in_category(category, &needle);
                if stats.is_empty() {
                    continue;
                }

                let name = category_display_name(category);
                let selected_count = stats
                    .iter()
                    .filter(|s| state.selected_stats.iter().any(|id| id == s.id))
                    .count();
                // "###name" pins the widget's identity to the stable
                // category name, decoupled from the displayed text - the
                // displayed part includes selected_count, which changes on
                // every toggle. Without the "###" split, ImGui derives
                // identity from the whole label by default, so a changing
                // count would make it treat every toggle as a brand new
                // (closed) header instead of the same one staying open.
                let header_label =
                    format!("{name} ({selected_count}/{} selected)###{name}", stats.len());

                if !ui.collapsing_header(header_label, TreeNodeFlags::empty()) {
                    continue;
                }
                ui.indent();

                let ids: Vec<&str> = stats.iter().map(|s| s.id).collect();
                if ui.button(format!("Select all##{name}_select")) {
                    select_ids(&mut state.selected_stats, &ids);
                    persist(&state, addon_dir);
                }
                ui.same_line();
                if ui.button(format!("Unselect all##{name}_unselect")) {
                    unselect_ids(&mut state.selected_stats, &ids);
                    persist(&state, addon_dir);
                }

                for stat in &stats {
                    let mut checked = state.selected_stats.iter().any(|id| id == stat.id);
                    // "##name_id" disambiguates the widget id: the same stat
                    // can render in several categories (e.g. a currency in
                    // both "Currencies" and "WvW"), and ImGui derives widget
                    // identity from the label text by default.
                    if ui.checkbox(format!("{}##{name}_{}", stat.display_name, stat.id), &mut checked) {
                        toggle_stat(&mut state.selected_stats, stat.id);
                        persist(&state, addon_dir);
                    }
                }

                ui.unindent();
            }

            ui.unindent();
        }
    });
}
