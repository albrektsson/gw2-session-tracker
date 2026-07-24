pub mod api;
pub mod config;
pub mod format;
mod material_storage;
pub mod session;
pub mod stats;
pub mod sync;

#[cfg(test)]
mod smoke_test {
    #[test]
    fn workspace_builds_and_tests_run() {
        assert_eq!(2 + 2, 4);
    }
}
