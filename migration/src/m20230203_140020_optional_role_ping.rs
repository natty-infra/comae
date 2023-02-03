use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_table::Channels;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .add_column_if_not_exists(ColumnDef::new(Channels::RoleMentionId).big_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Channels::MentionFlag)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .drop_column(Channels::MentionFlag)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Channels::Table)
                    .drop_column(Channels::RoleMentionId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
