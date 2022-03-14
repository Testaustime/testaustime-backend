#![allow(non_snake_case)]

table! {
    CodingActivities (id) {
        id -> Integer,
        user_id -> Integer,
        start_time -> Datetime,
        duration -> Integer,
        project_name -> Nullable<Text>,
        language -> Nullable<Text>,
        editor_name -> Nullable<Text>,
        hostname -> Nullable<Text>,
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
        auth_token -> Text,
        user_name -> Text,
        friend_code -> Nullable<Varchar>,
        password -> Binary,
        salt -> Binary,
        registration_time -> Datetime,
    }
}

joinable!(CodingActivities -> RegisteredUsers (user_id));

allow_tables_to_appear_in_same_query!(CodingActivities, FriendRelations, RegisteredUsers,);
