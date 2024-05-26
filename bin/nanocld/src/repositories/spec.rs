use diesel::prelude::*;

use nanocl_error::io::IoResult;

use nanocl_stubs::{
  generic::{GenericFilter, GenericClause},
  cargo_spec::{CargoSpecPartial, CargoSpec},
  vm_spec::{VmSpec, VmSpecPartial},
};

use crate::{
  gen_sql_multiple, gen_sql_where4uuid, gen_sql_where4string,
  models::{Pool, SpecDb},
  schema::specs,
};

use super::generic::*;

impl RepositoryBase for SpecDb {}

impl RepositoryCreate for SpecDb {}

impl RepositoryDelBy for SpecDb {
  fn gen_del_query(
    filter: &GenericFilter,
  ) -> diesel::query_builder::BoxedDeleteStatement<
    'static,
    diesel::pg::Pg,
    <Self as diesel::associations::HasTable>::Table,
  >
  where
    Self: diesel::associations::HasTable,
  {
    let condition = filter.r#where.clone().unwrap_or_default();
    let r#where = condition.conditions;
    let mut query = diesel::delete(specs::table).into_boxed();
    if let Some(value) = r#where.get("kind_key") {
      gen_sql_where4string!(query, specs::kind_key, value);
    }
    if let Some(value) = r#where.get("version") {
      gen_sql_where4string!(query, specs::version, value);
    }
    query
  }
}

impl RepositoryReadBy for SpecDb {
  type Output = SpecDb;

  fn get_pk() -> &'static str {
    "key"
  }

  fn gen_read_query(
    filter: &GenericFilter,
    is_multiple: bool,
  ) -> impl diesel::query_dsl::methods::LoadQuery<
    'static,
    diesel::pg::PgConnection,
    Self::Output,
  > {
    let condition = filter.r#where.clone().unwrap_or_default();
    let r#where = condition.conditions;
    let mut query = specs::table.into_boxed();
    if let Some(key) = r#where.get("key") {
      gen_sql_where4uuid!(query, specs::key, key);
    }
    if let Some(kind_key) = r#where.get("kind_key") {
      gen_sql_where4string!(query, specs::kind_key, kind_key);
    }
    if let Some(version) = r#where.get("version") {
      gen_sql_where4string!(query, specs::version, version);
    }
    if is_multiple {
      gen_sql_multiple!(query, specs::created_at, filter);
    }
    query
  }
}

impl SpecDb {
  pub async fn del_by_kind_key(key: &str, pool: &Pool) -> IoResult<()> {
    let filter = GenericFilter::new()
      .r#where("kind_key", GenericClause::Eq(key.to_owned()));
    SpecDb::del_by(&filter, pool).await
  }

  pub async fn get_version(
    name: &str,
    version: &str,
    pool: &Pool,
  ) -> IoResult<SpecDb> {
    let filter = GenericFilter::new()
      .r#where("kind_key", GenericClause::Eq(name.to_owned()))
      .r#where("version", GenericClause::Eq(version.to_owned()));
    SpecDb::read_one_by(&filter, pool).await
  }

  pub async fn read_by_kind_key(
    key: &str,
    pool: &Pool,
  ) -> IoResult<Vec<SpecDb>> {
    let filter = GenericFilter::new()
      .r#where("kind_key", GenericClause::Eq(key.to_owned()));
    SpecDb::read_by(&filter, pool).await
  }

  pub fn try_from_cargo_partial(
    key: &str,
    version: &str,
    item: &CargoSpecPartial,
  ) -> IoResult<Self> {
    Ok(Self {
      key: uuid::Uuid::new_v4(),
      created_at: chrono::Utc::now().naive_utc(),
      kind_name: "Cargo".to_owned(),
      kind_key: key.to_owned(),
      version: version.to_owned(),
      data: serde_json::to_value(item)?,
      metadata: item.metadata.clone(),
    })
  }

  pub fn try_from_vm_partial(
    key: &str,
    version: &str,
    item: &VmSpecPartial,
  ) -> IoResult<Self> {
    Ok(Self {
      key: uuid::Uuid::new_v4(),
      created_at: chrono::Utc::now().naive_utc(),
      kind_name: "Vm".to_owned(),
      kind_key: key.to_owned(),
      version: version.to_owned(),
      data: serde_json::to_value(item)?,
      metadata: item.metadata.clone(),
    })
  }

  pub fn try_to_cargo_spec(&self) -> IoResult<CargoSpec> {
    let p = serde_json::from_value::<CargoSpecPartial>(self.data.clone())?;
    let spec = CargoSpec {
      key: self.key,
      cargo_key: self.kind_key.clone(),
      version: self.version.clone(),
      created_at: self.created_at,
      name: p.name,
      metadata: self.metadata.clone(),
      init_container: p.init_container,
      secrets: p.secrets,
      container: p.container,
      replication: p.replication,
      image_pull_secret: p.image_pull_secret,
      image_pull_policy: p.image_pull_policy,
    };
    Ok(spec)
  }

  pub fn try_to_vm_spec(&self) -> IoResult<VmSpec> {
    let p = serde_json::from_value::<VmSpecPartial>(self.data.clone())?;
    let spec = VmSpec {
      key: self.key,
      vm_key: self.kind_key.clone(),
      version: self.version.clone(),
      created_at: self.created_at,
      name: p.name,
      metadata: self.metadata.clone(),
      hostname: p.hostname,
      password: p.password,
      disk: p.disk,
      host_config: p.host_config.unwrap_or_default(),
      ssh_key: p.ssh_key,
      user: p.user,
      mac_address: p.mac_address,
      labels: p.labels,
    };
    Ok(spec)
  }
}
