table! {
    coding_activities (id) {
        id -> Int4,
        user_id -> Int4,
        start_time -> Timestamp,
        duration -> Int4,
        project_name -> Nullable<Varchar>,
        language -> Nullable<Varchar>,
        editor_name -> Nullable<Varchar>,
        hostname -> Nullable<Varchar>,
    }
}

table! {
    friend_relations (id) {
        id -> Int4,
        lesser_id -> Int4,
        greater_id -> Int4,
    }
}

table! {
    registered_users (id) {
        id -> Int4,
        auth_token -> Varchar,
        friend_code -> Varchar,
        username -> Varchar,
        password -> Bytea,
        salt -> Bytea,
        registration_time -> Timestamp,
    }
}

joinable!(coding_activities -> registered_users (user_id));

allow_tables_to_appear_in_same_query!(
    coding_activities,
    friend_relations,
    registered_users,
);
