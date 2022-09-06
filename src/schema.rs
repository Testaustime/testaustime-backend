// @generated automatically by Diesel CLI.

diesel::table! {
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

diesel::table! {
    friend_relations (id) {
        id -> Int4,
        lesser_id -> Int4,
        greater_id -> Int4,
    }
}

diesel::table! {
    leaderboard_members (id) {
        id -> Int4,
        leaderboard_id -> Int4,
        user_id -> Int4,
        admin -> Bool,
    }
}

diesel::table! {
    leaderboards (id) {
        id -> Int4,
        name -> Varchar,
        invite_code -> Varchar,
        creation_time -> Timestamp,
    }
}

diesel::table! {
    testausid_users (id) {
        id -> Int4,
        user_id -> Text,
        service_id -> Text,
        identity -> Int4,
    }
}

diesel::table! {
    testaustime_users (id) {
        id -> Int4,
        password -> Bytea,
        salt -> Bytea,
        identity -> Int4,
    }
}

diesel::table! {
    user_identities (id) {
        id -> Int4,
        auth_token -> Bpchar,
        friend_code -> Bpchar,
        username -> Varchar,
        registration_time -> Timestamp,
        is_public -> Bool,
    }
}

diesel::joinable!(coding_activities -> user_identities (user_id));
diesel::joinable!(leaderboard_members -> leaderboards (leaderboard_id));
diesel::joinable!(leaderboard_members -> user_identities (user_id));
diesel::joinable!(testausid_users -> user_identities (identity));
diesel::joinable!(testaustime_users -> user_identities (identity));

diesel::allow_tables_to_appear_in_same_query!(
    coding_activities,
    friend_relations,
    leaderboard_members,
    leaderboards,
    testausid_users,
    testaustime_users,
    user_identities,
);
