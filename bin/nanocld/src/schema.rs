// @generated automatically by Diesel CLI.

diesel::table! {
    cargoes (key) {
        key -> Varchar,
        created_at -> Timestamptz,
        name -> Varchar,
        spec_key -> Uuid,
        namespace_name -> Varchar,
    }
}

diesel::table! {
    jobs (key) {
        key -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        data -> Jsonb,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    metrics (key) {
        key -> Uuid,
        created_at -> Timestamptz,
        expire_at -> Timestamptz,
        node_name -> Varchar,
        kind -> Varchar,
        data -> Jsonb,
    }
}

diesel::table! {
    namespaces (name) {
        name -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    node_group_links (rowid) {
        node_name -> Varchar,
        node_group_name -> Varchar,
        rowid -> Int8,
    }
}

diesel::table! {
    node_groups (name) {
        name -> Varchar,
    }
}

diesel::table! {
    nodes (name) {
        name -> Varchar,
        ip_address -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    processes (key) {
        key -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        name -> Varchar,
        kind -> Varchar,
        data -> Jsonb,
        node_key -> Varchar,
        kind_key -> Varchar,
    }
}

diesel::table! {
    resource_kinds (name) {
        name -> Varchar,
        created_at -> Timestamptz,
        spec_key -> Uuid,
    }
}

diesel::table! {
    resources (key) {
        key -> Varchar,
        created_at -> Timestamptz,
        kind -> Varchar,
        spec_key -> Uuid,
    }
}

diesel::table! {
    secrets (key) {
        key -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        kind -> Varchar,
        immutable -> Bool,
        data -> Jsonb,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    specs (key) {
        key -> Uuid,
        created_at -> Timestamptz,
        kind_name -> Varchar,
        kind_key -> Varchar,
        version -> Varchar,
        data -> Jsonb,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    vm_images (name) {
        name -> Varchar,
        created_at -> Timestamptz,
        kind -> Varchar,
        path -> Varchar,
        format -> Varchar,
        size_actual -> Int8,
        size_virtual -> Int8,
        parent -> Nullable<Varchar>,
    }
}

diesel::table! {
    vms (key) {
        key -> Varchar,
        created_at -> Timestamptz,
        name -> Varchar,
        spec_key -> Uuid,
        namespace_name -> Varchar,
    }
}

diesel::joinable!(cargoes -> namespaces (namespace_name));
diesel::joinable!(cargoes -> specs (spec_key));
diesel::joinable!(node_group_links -> node_groups (node_group_name));
diesel::joinable!(node_group_links -> nodes (node_name));
diesel::joinable!(resource_kinds -> specs (spec_key));
diesel::joinable!(resources -> specs (spec_key));
diesel::joinable!(vms -> namespaces (namespace_name));
diesel::joinable!(vms -> specs (spec_key));

diesel::allow_tables_to_appear_in_same_query!(
  cargoes,
  jobs,
  metrics,
  namespaces,
  node_group_links,
  node_groups,
  nodes,
  processes,
  resource_kinds,
  resources,
  secrets,
  specs,
  vm_images,
  vms,
);
