use nexus::imgui::{TreeNodeFlags, Ui};
use std::cell::RefCell;
use session_tracker_core::category::{Category, SUPERCATEGORIES};
use session_tracker_core::stats::{StatDef, STAT_CATALOG};
use session_tracker_net::state::StatListKind;

use crate::app_handle::AppHandle;
use super::stat_icon::render_stat_icon;

thread_local! {
    static SEARCH_FILTER: RefCell<String> = const { RefCell::new(String::new()) };
}

// Stats with no `Category` at all (client-computed, not browsable by
// activity type) are pinned above the category tree instead of inside it -
// `stats_in_category` below would never match them otherwise.
const PINNED_STAT_IDS: &[&str] = &["session_timer", "distance_traveled", "combat_time"];

const ICON_SIZE: f32 = 16.0;

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
        .filter(|s| !PINNED_STAT_IDS.contains(&s.id))
        .filter(|s| s.categories.contains(&category))
        .filter(|s| needle.is_empty() || s.display_name.to_lowercase().contains(needle))
        .collect()
}

pub fn render_select_stats_tab(ui: &Ui, app: &AppHandle) {
    if let Some(_tabs) = ui.tab_bar("stat-list-scope") {
        for kind in StatListKind::ALL {
            if let Some(_tab) = ui.tab_item(kind.label()) {
                render_select_stats_editor(ui, app, kind);
            }
        }
    }
}

fn render_select_stats_editor(ui: &Ui, app: &AppHandle, kind: StatListKind) {
    let label = kind.label();
    let cache_dir = session_tracker_net::icon_cache::cache_dir(app.addon_dir());
    SEARCH_FILTER.with(|filter| {
        let mut query = filter.borrow_mut();
        ui.input_text("##stat_search", &mut query)
            .hint("Search stats...")
            .build();

        if ui.button(format!("Select all##{label}_all_select")) {
            app.select_all(kind);
        }
        ui.same_line();
        if ui.button(format!("Unselect all##{label}_all_unselect")) {
            app.unselect_all(kind);
        }

        ui.separator();

        let needle = query.to_lowercase();

        let mut any_pinned_shown = false;
        for &id in PINNED_STAT_IDS {
            if let Some(stat) = STAT_CATALOG.iter().find(|s| s.id == id)
                && (needle.is_empty() || stat.display_name.to_lowercase().contains(&needle))
            {
                any_pinned_shown = true;
                render_stat_icon(stat, &app.lock(), &cache_dir, ICON_SIZE, ui);
                let mut checked = app.lock().stat_list(kind).iter().any(|sid| sid == stat.id);
                if ui.checkbox(format!("{}##{label}_{}", stat.display_name, stat.id), &mut checked) {
                    app.toggle_stat(kind, stat.id);
                }
            }
        }
        if any_pinned_shown {
            ui.separator();
        }

        for (supercategory_name, subcategories) in SUPERCATEGORIES {
            if !ui.collapsing_header(format!("{supercategory_name}###{label}_{supercategory_name}"), TreeNodeFlags::DEFAULT_OPEN) {
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
                    .filter(|s| app.lock().stat_list(kind).iter().any(|id| id == s.id))
                    .count();
                // "###name_label" pins the widget's identity to the stable
                // category name plus which list is being edited, decoupled
                // from the displayed text - the displayed part includes
                // selected_count, which changes on every toggle. Without the
                // "###" split, ImGui derives identity from the whole label
                // by default, so a changing count would make it treat every
                // toggle as a brand new (closed) header instead of the same
                // one staying open; without the per-list suffix, opening
                // this header in one list's tab would also show it open in
                // every other list's tab, since they'd share one ImGui id.
                let header_label =
                    format!("{name} ({selected_count}/{} selected)###{label}_{name}", stats.len());

                if !ui.collapsing_header(header_label, TreeNodeFlags::empty()) {
                    continue;
                }
                ui.indent();

                let ids: Vec<&str> = stats.iter().map(|s| s.id).collect();
                if ui.button(format!("Select all##{label}_{name}_select")) {
                    app.select_ids(kind, &ids);
                }
                ui.same_line();
                if ui.button(format!("Unselect all##{label}_{name}_unselect")) {
                    app.unselect_ids(kind, &ids);
                }

                for stat in &stats {
                    render_stat_icon(stat, &app.lock(), &cache_dir, ICON_SIZE, ui);
                    let mut checked = app.lock().stat_list(kind).iter().any(|id| id == stat.id);
                    // "##label_name_id" disambiguates the widget id: the
                    // same stat can render in several categories (e.g. a
                    // currency in both "Currencies" and "WvW") and in
                    // several list tabs, and ImGui derives widget identity
                    // from the label text by default.
                    if ui.checkbox(format!("{}##{label}_{name}_{}", stat.display_name, stat.id), &mut checked) {
                        app.toggle_stat(kind, stat.id);
                    }
                }

                ui.unindent();
            }

            ui.unindent();
        }
    });
}
