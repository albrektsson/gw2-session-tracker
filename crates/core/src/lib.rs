pub mod api;
pub mod category;
pub mod config;
pub mod format;
pub mod map_context;
mod material_storage;
pub mod session;
pub mod stat_list;
pub mod stats;
pub mod sync;

#[cfg(test)]
mod smoke_test {
    #[test]
    fn workspace_builds_and_tests_run() {
        assert_eq!(2 + 2, 4);
    }
}
