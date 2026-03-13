use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "placement_profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub placement_id: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub profile_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::placements::Entity",
        from = "Column::PlacementId",
        to = "super::placements::Column::Id"
    )]
    Placement,
}

impl Related<super::placements::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Placement.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
