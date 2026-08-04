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
