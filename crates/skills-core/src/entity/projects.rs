use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub path: String,
    pub name: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::placements::Entity")]
    Placements,
    #[sea_orm(has_many = "super::project_profiles::Entity")]
    ProjectProfiles,
    #[sea_orm(has_many = "super::project_linked_profiles::Entity")]
    ProjectLinkedProfiles,
    #[sea_orm(has_many = "super::project_agents::Entity")]
    ProjectAgents,
}

impl Related<super::placements::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Placements.def()
    }
}

impl Related<super::project_profiles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectProfiles.def()
    }
}

impl Related<super::project_linked_profiles::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectLinkedProfiles.def()
    }
}

impl Related<super::project_agents::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProjectAgents.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
