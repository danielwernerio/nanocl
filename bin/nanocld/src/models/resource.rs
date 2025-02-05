use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use nanocl_stubs::resource::ResourceSpec;

use super::SpecDb;

use crate::schema::resources;

/// This structure represent a resource in the database.
/// A resource is a representation of a specification for internal nanocl services (controllers).
/// Custom `kind` can be added to the system.
/// We use the `spec_key` to link to the resource spec.
/// The `key` is used to identify the resource.
/// The `kind` is used to know which controller to use.
#[derive(
  Debug, Queryable, Identifiable, Insertable, Serialize, Deserialize,
)]
#[diesel(primary_key(key))]
#[diesel(table_name = resources)]
pub struct ResourceDb {
  /// The key of the resource
  pub key: String,
  /// The created at date
  pub created_at: chrono::NaiveDateTime,
  /// The kind of the resource
  pub kind: String,
  /// The spec key reference
  pub spec_key: uuid::Uuid,
}

/// This structure represent the update of a resource in the database.
#[derive(AsChangeset)]
#[diesel(table_name = resources)]
pub struct ResourceUpdateDb {
  /// The key of the resource
  pub key: Option<String>,
  /// The spec key reference
  pub spec_key: Option<uuid::Uuid>,
}

/// Helper to convert a `SpecDb` to a `ResourceSpec`
impl From<SpecDb> for ResourceSpec {
  fn from(db: SpecDb) -> Self {
    ResourceSpec {
      key: db.key,
      version: db.version,
      created_at: db.created_at,
      resource_key: db.kind_key,
      data: db.data,
      metadata: db.metadata,
    }
  }
}
