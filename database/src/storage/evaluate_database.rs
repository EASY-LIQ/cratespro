
use entity::metadata;
use futures::Stream;
use sea_orm::DatabaseConnection;
use sea_orm::{DbErr, EntityTrait, QueryOrder,};
use std::sync::Arc;

#[derive(Clone)]
pub struct EvaluateDatabase {
    pub connection: Arc<DatabaseConnection>,
}

impl EvaluateDatabase {
    /// 获取底层连接
    pub fn get_connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    pub async fn new(connection: Arc<DatabaseConnection>) -> Self {
        EvaluateDatabase { connection }
    }

    /// 获取 metadata 表的查询流
    pub async fn get_metadata_stream(
        &self,
    ) -> Result<impl Stream<Item = Result<metadata::Model, DbErr>> + Send + '_, DbErr> {
        metadata::Entity::find()
            .order_by_asc(metadata::Column::Id)
            .stream(self.get_connection())
            .await
    }

    // 后续可扩展 sea-orm 查询方法
}
