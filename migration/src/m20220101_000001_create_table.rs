use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Platforms::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Platforms::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Platforms::Name)
                            .string_len(32)
                            .unique_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Platforms::Description)
                            .string_len(64)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        let insert = Query::insert()
            .into_table(Platforms::Table)
            .columns([Platforms::Name, Platforms::Description])
            .values_panic(["YouTube".into(), "YouTube".into()])
            .values_panic(["Reddit".into(), "Reddit".into()])
            .to_owned();

        manager.exec_stmt(insert).await?;

        manager
            .create_table(
                Table::create()
                    .table(Channels::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Channels::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Channels::Name).string_len(48).not_null())
                    .col(
                        ColumnDef::new(Channels::Description)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Channels::DiscordChannelId)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Channels::PlatformId)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .unique()
                    .table(Channels::Table)
                    .col(Channels::Name)
                    .col(Channels::DiscordChannelId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Channels::Table, Channels::PlatformId)
                    .to(Platforms::Table, Platforms::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Posts::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Posts::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Posts::Name)
                            .string_len(32)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Posts::TimeAdded).timestamp().not_null())
                    .col(ColumnDef::new(Posts::ChannelId).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Posts::Table, Posts::ChannelId)
                    .to(Channels::Table, Channels::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Posts::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Channels::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Platforms::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
pub enum Platforms {
    #[iden = "platforms"]
    Table,
    #[iden = "pl_id"]
    Id,
    #[iden = "pl_name"]
    Name,
    #[iden = "pl_description"]
    Description,
}

#[derive(Iden)]
pub enum Channels {
    #[iden = "channels"]
    Table,
    #[iden = "ch_id"]
    Id,
    #[iden = "ch_name"]
    Name,
    #[iden = "ch_description"]
    Description,
    #[iden = "ch_discord_channel_id"]
    DiscordChannelId,
    #[iden = "ch_pl_id"]
    PlatformId,
    #[iden = "ch_mention_flag"]
    MentionFlag,
    #[iden = "ch_role_mention_id"]
    RoleMentionId,
}

#[derive(Iden)]
pub enum Posts {
    #[iden = "posts"]
    Table,
    #[iden = "po_id"]
    Id,
    #[iden = "po_name"]
    Name,
    #[iden = "po_time_added"]
    TimeAdded,
    #[iden = "po_ch_id"]
    ChannelId,
}
