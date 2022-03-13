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
    RegisteredUsers (id) {
        id -> Integer,
        auth_token -> Text,
        user_name -> Text,
        password -> Binary,
        salt -> Binary,
        registration_time -> Datetime,
    }
}

joinable!(CodingActivities -> RegisteredUsers (user_id));

allow_tables_to_appear_in_same_query!(CodingActivities, RegisteredUsers,);
