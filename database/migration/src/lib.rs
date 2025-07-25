pub use sea_orm_migration::prelude::*;

mod m20250331_093145_init_crates_pro;
mod m20250401_100000_add_performance_indices;
mod m20250407_094050_alter_programs;
mod m20250409_085616_add_in_cratesio_column;
mod m20250409_090128_update_repo_id_type;
mod m20250418_081905_add_new_tables;
mod m20250424_092358_alter_programs;
mod m20250604_133200_init_repository_metadata;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250331_093145_init_crates_pro::Migration),
            Box::new(m20250401_100000_add_performance_indices::Migration),
            Box::new(m20250407_094050_alter_programs::Migration),
            Box::new(m20250409_085616_add_in_cratesio_column::Migration),
            Box::new(m20250409_090128_update_repo_id_type::Migration),
            Box::new(m20250418_081905_add_new_tables::Migration),
            Box::new(m20250424_092358_alter_programs::Migration),
            Box::new(m20250604_133200_init_repository_metadata::Migration),
        ]
    }
}
