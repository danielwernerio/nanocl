use std::collections::HashMap;

use diesel::prelude::*;

use nanocl_error::io::IoResult;

use nanocl_stubs::generic::GenericFilter;

use crate::{
  gen_sql_multiple, gen_sql_order_by, gen_sql_query,
  models::{ColumnType, NodeDb, Pool, SystemState},
  schema::nodes,
};

use super::generic::*;

impl RepositoryBase for NodeDb {
  fn get_columns<'a>() -> HashMap<&'a str, (ColumnType, &'a str)> {
    HashMap::from([
      ("name", (ColumnType::Text, "nodes.name")),
      ("ip_address", (ColumnType::Text, "nodes.ip_address")),
      ("created_at", (ColumnType::Timestamptz, "nodes.created_at")),
    ])
  }
}

impl RepositoryCreate for NodeDb {}

impl RepositoryDelByPk for NodeDb {}

impl RepositoryReadBy for NodeDb {
  type Output = NodeDb;

  fn get_pk() -> &'static str {
    "name"
  }

  fn gen_read_query(
    filter: &GenericFilter,
    is_multiple: bool,
  ) -> impl diesel::query_dsl::methods::LoadQuery<
    'static,
    diesel::pg::PgConnection,
    Self::Output,
  > {
    let mut query = nodes::table.into_boxed();
    let columns = NodeDb::get_columns();
    query = gen_sql_query!(query, filter, columns);
    if let Some(orders) = &filter.order_by {
      query = gen_sql_order_by!(query, orders, columns);
    } else {
      query = query.order(nodes::created_at.desc());
    }
    if is_multiple {
      gen_sql_multiple!(query, nodes::created_at, filter);
    }
    query
  }
}

impl RepositoryCountBy for NodeDb {
  fn gen_count_query(
    filter: &GenericFilter,
  ) -> impl diesel::query_dsl::LoadQuery<'static, diesel::PgConnection, i64> {
    let mut query = nodes::table.into_boxed();
    let columns = NodeDb::get_columns();
    gen_sql_query!(query, filter, columns).count()
  }
}

impl NodeDb {
  pub async fn create_if_not_exists(
    node: &NodeDb,
    pool: &Pool,
  ) -> IoResult<NodeDb> {
    match NodeDb::read_by_pk(&node.name, pool).await {
      Err(_) => NodeDb::create_from(node.clone(), pool).await,
      Ok(node) => Ok(node),
    }
  }

  pub async fn register(state: &SystemState) -> IoResult<()> {
    let node = NodeDb {
      name: state.inner.config.hostname.clone(),
      ip_address: state.inner.config.gateway.clone(),
      created_at: chrono::Utc::now().naive_utc(),
    };
    NodeDb::create_if_not_exists(&node, &state.inner.pool).await?;
    Ok(())
  }
}
