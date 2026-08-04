# Category stays in the core crate

`Category`/`SUPERCATEGORIES` (the Select Stats picker's browsing taxonomy)
is conceptually UI-only — CONTEXT.md says as much — which makes it tempting
to move it into the `addon` crate, alongside the rest of the UI. We
considered that during an architecture review and rejected it: `CORE_STATS`'
`StatDef` literals embed `categories: &'static [Category]` directly in a
`core`-crate static table, and `core` can't depend on `addon` (the
dependency only runs the other way). Moving `Category` out would mean
decoupling categories from the catalog table first — a separate,
unrelated redesign, not a file move.

`Category` lives in its own module (`crates/core/src/category.rs`) so it's
organizationally distinct from the Stat Catalog it's referenced by, but it
stays in `core`.
