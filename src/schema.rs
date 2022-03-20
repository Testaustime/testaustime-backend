#![allow(non_snake_case)]

table! {
    CodingActivities (id) {
        id -> Integer,
        user_id -> Integer,
        start_time -> Datetime,
        duration -> Integer,
        project_name -> Nullable<Varchar>,
        language -> Nullable<Varchar>,
        editor_name -> Nullable<Varchar>,
        hostname -> Nullable<Varchar>,
    }
}

table! {
    FriendRelations (id) {
        id -> Integer,
        lesser_id -> Integer,
        greater_id -> Integer,
    }
}

table! {
    RegisteredUsers (id) {
        id -> Integer,
        auth_token -> Varchar,
        user_name -> Varchar,
        friend_code -> Nullable<Varchar>,
        password -> Binary,
        salt -> Binary,
        registration_time -> Datetime,
    }
}

joinable!(CodingActivities -> RegisteredUsers (user_id));

allow_tables_to_appear_in_same_query!(CodingActivities, FriendRelations, RegisteredUsers,);
