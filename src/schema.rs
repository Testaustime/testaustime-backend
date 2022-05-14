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
    leaderboard_members (id) {
        id -> Int4,
        leaderboard_id -> Int4,
        user_id -> Int4,
        admin -> Bool,
    }
}

table! {
    leaderboards (id) {
        id -> Int4,
        name -> Varchar,
        invite_code -> Varchar,
        creation_time -> Timestamp,
    }
}

table! {
    testausid_users (id) {
        id -> Int4,
        user_id -> Int4,
        service_id -> Int4,
        identity -> Int4,
    }
}

table! {
    testaustime_users (id) {
        id -> Int4,
        password -> Bytea,
        salt -> Bytea,
        identity -> Int4,
    }
}

table! {
    user_identities (id) {
        id -> Int4,
        username -> Varchar,
        auth_token -> Bpchar,
        friend_code -> Bpchar,
        registration_time -> Timestamp,
    }
}

joinable!(coding_activities -> user_identities (user_id));
joinable!(leaderboard_members -> leaderboards (leaderboard_id));
joinable!(leaderboard_members -> user_identities (user_id));
joinable!(testausid_users -> user_identities (identity));
joinable!(testaustime_users -> user_identities (identity));

allow_tables_to_appear_in_same_query!(
    coding_activities,
    friend_relations,
    leaderboard_members,
    leaderboards,
    testausid_users,
    testaustime_users,
    user_identities,
);
