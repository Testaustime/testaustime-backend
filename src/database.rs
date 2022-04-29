use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use chrono::{prelude::*, Duration};
use diesel::{insert_into, pg::PgConnection, prelude::*, r2d2::ConnectionManager};
use r2d2::PooledConnection;

use crate::{
    error::TimeError,
    models::*,
    requests::{DataRequest, HeartBeat},
    utils::*,
};

fn user_exists(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    target_username: &str,
) -> Result<bool, TimeError> {
    use crate::schema::registered_users::dsl::*;
    Ok(registered_users
        .filter(username.eq(target_username))
        .first::<RegisteredUser>(conn)
        .optional()?
        .is_some())
}

pub fn get_user_by_name(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    target_username: &str,
) -> Result<RegisteredUser, TimeError> {
    use crate::schema::registered_users::dsl::*;
    Ok(registered_users
        .filter(username.eq(target_username))
        .first::<RegisteredUser>(conn)?)
}

pub fn delete_user(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    userid: i32,
) -> Result<bool, TimeError> {
    use crate::schema::registered_users::dsl::*;
    Ok(diesel::delete(registered_users.filter(id.eq(userid))).execute(conn)? > 0)
}

pub fn get_user_by_id(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    userid: i32,
) -> Result<RegisteredUser, TimeError> {
    use crate::schema::registered_users::dsl::*;
    Ok(registered_users
        .filter(id.eq(userid))
        .first::<RegisteredUser>(conn)?)
}

pub fn verify_user_password(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    username: &str,
    password: &str,
) -> Result<Option<RegisteredUser>, TimeError> {
    let user = get_user_by_name(conn, username)?;
    let argon2 = Argon2::default();
    let Ok(salt) = SaltString::new(std::str::from_utf8(&user.salt).unwrap()) else {
        return Ok(None); // The user has no password
    };
    let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
    if password_hash.hash.unwrap().as_bytes() == user.password {
        Ok(Some(user))
    } else {
        Ok(None)
    }
}

pub fn regenerate_token(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    userid: i32,
) -> Result<String, TimeError> {
    let token = crate::utils::generate_token();
    use crate::schema::registered_users::dsl::*;
    diesel::update(crate::schema::registered_users::table)
        .filter(id.eq(userid))
        .set(auth_token.eq(&token))
        .execute(conn)?;
    Ok(token)
}

pub fn new_user(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    username: &str,
    password: &str,
) -> Result<NewRegisteredUser, TimeError> {
    if user_exists(conn, username)? {
        return Err(TimeError::UserExists);
    }
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt).unwrap();
    let token = generate_token();
    let hash = password_hash.hash.unwrap();
    let new_user = NewRegisteredUser {
        auth_token: token,
        registration_time: chrono::Local::now().naive_local(),
        username: username.to_string(),
        friend_code: generate_friend_code(),
        password: hash.as_bytes().to_vec(),
        salt: salt.as_bytes().to_vec(),
    };
    diesel::insert_into(crate::schema::registered_users::table)
        .values(&new_user)
        .execute(conn)
        .map_err(|_| TimeError::UserExists)?;
    Ok(new_user)
}

pub fn change_username(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    user: i32,
    new_username: &str,
) -> Result<(), TimeError> {
    if user_exists(conn, new_username)? {
        return Err(TimeError::UserExists);
    }
    use crate::schema::registered_users::dsl::*;
    diesel::update(crate::schema::registered_users::table)
        .filter(id.eq(user))
        .set(username.eq(new_username))
        .execute(conn)
        .map_err(|_| TimeError::UserExists)?;
    Ok(())
}

pub fn change_password(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    user: i32,
    new_password: &str,
) -> Result<(), TimeError> {
    let new_salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(new_password.as_bytes(), &new_salt)
        .unwrap();
    let new_hash = password_hash.hash.unwrap();
    use crate::schema::registered_users::dsl::*;
    diesel::update(crate::schema::registered_users::table)
        .filter(id.eq(user))
        .set((
            password.eq(&new_hash.as_bytes()),
            salt.eq(new_salt.as_bytes()),
        ))
        .execute(conn)?;
    Ok(())
}

pub fn get_user_by_token(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    token: &str,
) -> Result<RegisteredUser, TimeError> {
    use crate::schema::registered_users::dsl::*;
    let user = registered_users
        .filter(auth_token.eq(token))
        .first::<RegisteredUser>(conn)?;
    Ok(user)
}

pub fn add_activity(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    updated_user_id: i32,
    heartbeat: HeartBeat,
    ctx_start_time: NaiveDateTime,
    ctx_duration: Duration,
) -> Result<(), TimeError> {
    use crate::schema::coding_activities::dsl::*;
    let activity = NewCodingActivity {
        user_id: updated_user_id,
        start_time: ctx_start_time,
        duration: ctx_duration.num_seconds() as i32,
        project_name: if heartbeat.project_name.is_some()
            && heartbeat.project_name.as_ref().unwrap().starts_with("tmp.")
        {
            Some(String::from("tmp"))
        } else {
            heartbeat.project_name
        },
        language: heartbeat.language,
        editor_name: heartbeat.editor_name,
        hostname: heartbeat.hostname,
    };
    diesel::insert_into(coding_activities)
        .values(activity)
        .execute(conn)?;
    Ok(())
}

pub fn get_activity(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    request: DataRequest,
    user: i32,
) -> Result<Vec<CodingActivity>, TimeError> {
    use crate::schema::coding_activities::dsl::*;
    let mut query = coding_activities.into_boxed().filter(user_id.eq(user));
    if let Some(from) = request.from {
        query = query.filter(start_time.ge(from.naive_local()));
    };
    if let Some(to) = request.to {
        query = query.filter(start_time.le(to.naive_local()));
    };
    if let Some(editor) = request.editor_name {
        query = query.filter(editor_name.eq(editor));
    };
    if let Some(project) = request.project_name {
        query = query.filter(project_name.eq(project));
    };
    if let Some(request_hostname) = request.hostname {
        query = query.filter(hostname.eq(request_hostname));
    };
    if let Some(request_language) = request.language {
        query = query.filter(language.eq(request_language));
    };
    if let Some(min_duration) = request.min_duration {
        query = query.filter(duration.ge(min_duration));
    };
    let res = query.load::<CodingActivity>(conn).unwrap();
    Ok(res)
}

pub fn add_friend(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    user: i32,
    friend: &str,
) -> Result<RegisteredUser, TimeError> {
    use crate::schema::registered_users::dsl::*;
    let Some(friend) = registered_users
    .filter(friend_code.eq(friend))
    .first::<RegisteredUser>(conn)
    .optional()? else {
        return Err(TimeError::UserNotFound)
    };

    let friend_id = friend.id;

    if friend_id == user {
        return Err(TimeError::CurrentUser);
    }

    let (lesser, greater) = if user < friend_id {
        (user, friend_id)
    } else {
        (friend_id, user)
    };

    insert_into(crate::schema::friend_relations::table)
        .values(crate::models::NewFriendRelation {
            lesser_id: lesser,
            greater_id: greater,
        })
        .execute(conn)?;
    Ok(friend)
}

pub fn get_friends(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    user: i32,
) -> Result<Vec<RegisteredUser>, TimeError> {
    use crate::schema::{
        friend_relations::dsl::{friend_relations, greater_id, lesser_id},
        registered_users::dsl::*,
    };
    let friends = friend_relations
        .filter(greater_id.eq(user).or(lesser_id.eq(user)))
        .load::<FriendRelation>(conn)?
        .iter()
        .map(
            |&FriendRelation {
                 lesser_id: other_lesser_id,
                 greater_id: other_greater_id,
                 ..
             }| {
                if other_lesser_id == user {
                    other_greater_id
                } else {
                    other_lesser_id
                }
            },
        )
        .filter_map(|cur_friend| {
            registered_users
                .filter(id.eq(cur_friend))
                .first::<RegisteredUser>(conn)
                .ok()
        })
        .collect();
    Ok(friends)
}

pub fn are_friends(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    user: i32,
    friend_id: i32,
) -> Result<bool, TimeError> {
    use crate::schema::friend_relations::dsl::*;
    let (lesser, greater) = if user < friend_id {
        (user, friend_id)
    } else {
        (friend_id, user)
    };
    Ok(friend_relations
        .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
        .first::<FriendRelation>(conn)
        .optional()?
        .is_some())
}

pub fn remove_friend(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    user: i32,
    friend_id: i32,
) -> Result<bool, TimeError> {
    use crate::schema::friend_relations::dsl::*;
    let (lesser, greater) = if user < friend_id {
        (user, friend_id)
    } else {
        (friend_id, user)
    };
    Ok(diesel::delete(friend_relations)
        .filter(lesser_id.eq(lesser).and(greater_id.eq(greater)))
        .execute(conn)?
        != 0)
}

pub fn regenerate_friend_code(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    userid: i32,
) -> Result<String, TimeError> {
    use crate::schema::registered_users::dsl::*;
    let code = crate::utils::generate_friend_code();
    diesel::update(crate::schema::registered_users::table)
        .filter(id.eq(userid))
        .set(friend_code.eq(&code))
        .execute(conn)?;
    Ok(code)
}

pub fn delete_activity(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    userid: i32,
    activity: i32,
) -> Result<bool, TimeError> {
    use crate::schema::coding_activities::dsl::*;
    let res = diesel::delete(crate::schema::coding_activities::table)
        .filter(id.eq(activity))
        .filter(user_id.eq(userid))
        .execute(conn)?;
    Ok(res != 0)
}

pub fn new_leaderboard(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    creator_id: i32,
    name: &str,
) -> Result<String, TimeError> {
    let code = crate::utils::generate_token();
    let board = NewLeaderboard {
        name: name.to_string(),
        creation_time: chrono::Local::now().naive_local(),
        invite_code: code.clone(),
    };
    let lid = {
        use crate::schema::leaderboards::dsl::*;
        insert_into(crate::schema::leaderboards::table)
            .values(&board)
            .returning(id)
            .get_results(conn)?[0]
    };

    let admin = NewLeaderboardMember {
        user_id: creator_id,
        admin: true,
        leaderboard_id: lid,
    };
    insert_into(crate::schema::leaderboard_members::table)
        .values(&admin)
        .execute(conn)?;
    Ok(code)
}

pub fn regenerate_leaderboard_invite(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    lid: i32,
) -> Result<String, TimeError> {
    let newinvite = crate::utils::generate_token();
    use crate::schema::leaderboards::dsl::*;
    diesel::update(crate::schema::leaderboards::table)
        .filter(id.eq(lid))
        .set(invite_code.eq(&newinvite))
        .execute(conn)?;
    Ok(newinvite)
}

pub fn delete_leaderboard(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    lname: &str,
) -> Result<bool, TimeError> {
    use crate::schema::leaderboards::dsl::*;
    let res = diesel::delete(crate::schema::leaderboards::table)
        .filter(name.eq(lname))
        .execute(conn)?;
    Ok(res != 0)
}

pub fn get_leaderboard_id_by_name(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    lname: &str,
) -> Result<i32, TimeError> {
    use crate::schema::leaderboards::dsl::*;
    Ok(leaderboards
        .filter(name.eq(lname))
        .select(id)
        .first::<i32>(conn)?)
}

pub fn get_leaderboard(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    lname: &str,
) -> Result<PrivateLeaderboard, TimeError> {
    let board = {
        use crate::schema::leaderboards::dsl::*;
        leaderboards
            .filter(name.eq(lname))
            .first::<Leaderboard>(conn)?
    };
    let members = {
        use crate::schema::leaderboard_members::dsl::*;
        leaderboard_members
            .filter(leaderboard_id.eq(board.id))
            .load::<LeaderboardMember>(conn)?
    };
    let mut fullmembers = Vec::new();
    let aweekago = NaiveDateTime::new(
        chrono::Local::today().naive_local() - chrono::Duration::weeks(1),
        chrono::NaiveTime::from_num_seconds_from_midnight(0, 0),
    );
    for m in members {
        if let Ok(user) = get_user_by_id(conn, m.user_id) {
            fullmembers.push(PrivateLeaderboardMember {
                username: user.username,
                admin: m.admin,
                time_coded: get_user_coding_time_since(conn, m.user_id, aweekago).unwrap_or(0),
            });
        }
    }
    Ok(PrivateLeaderboard {
        name: board.name,
        invite: board.invite_code,
        creation_time: board.creation_time,
        members: fullmembers,
    })
}

pub fn add_user_to_leaderboard(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    uid: i32,
    invite: &str,
) -> Result<String, TimeError> {
    let (lid, name) = {
        use crate::schema::leaderboards::dsl::*;
        leaderboards
            .filter(invite_code.eq(invite))
            .select((id, name))
            .first::<(i32, String)>(conn)?
    };
    let user = NewLeaderboardMember {
        user_id: uid,
        leaderboard_id: lid,
        admin: false,
    };
    insert_into(crate::schema::leaderboard_members::table)
        .values(&user)
        .execute(conn)?;
    Ok(name)
}

pub fn remove_user_from_leaderboard(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    lid: i32,
    uid: i32,
) -> Result<bool, TimeError> {
    use crate::schema::leaderboard_members::dsl::*;
    let res = diesel::delete(crate::schema::leaderboard_members::table)
        .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
        .execute(conn)?;
    Ok(res != 0)
}

pub fn promote_user_to_leaderboard_admin(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    lid: i32,
    uid: i32,
) -> Result<bool, TimeError> {
    use crate::schema::leaderboard_members::dsl::*;
    let res = diesel::update(crate::schema::leaderboard_members::table)
        .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
        .set(admin.eq(true))
        .execute(conn)?;
    Ok(res != 0)
}

pub fn demote_user_to_leaderboard_member(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    lid: i32,
    uid: i32,
) -> Result<bool, TimeError> {
    use crate::schema::leaderboard_members::dsl::*;
    let res = diesel::update(crate::schema::leaderboard_members::table)
        .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
        .set(admin.eq(false))
        .execute(conn)?;
    Ok(res != 0)
}

pub fn is_leaderboard_member(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    uid: i32,
    lid: i32,
) -> Result<bool, TimeError> {
    use crate::schema::leaderboard_members::dsl::*;
    Ok(leaderboard_members
        .filter(user_id.eq(uid).and(leaderboard_id.eq(lid)))
        .select(id)
        .first::<i32>(conn)
        .optional()?
        .is_some())
}

pub fn get_user_coding_time_since(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    uid: i32,
    since: chrono::NaiveDateTime,
) -> Result<i32, TimeError> {
    use crate::schema::coding_activities::dsl::*;
    Ok(coding_activities
        .filter(user_id.eq(uid).and(start_time.ge(since)))
        .select(diesel::dsl::sum(duration))
        .first::<Option<i64>>(conn)?
        .unwrap_or(0) as i32)
}

pub fn is_leaderboard_admin(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    uid: i32,
    lid: i32,
) -> Result<bool, TimeError> {
    use crate::schema::leaderboard_members::dsl::*;
    Ok(leaderboard_members
        .filter(leaderboard_id.eq(lid).and(user_id.eq(uid)))
        .select(admin)
        .first::<bool>(conn)
        .optional()?
        .unwrap_or(false))
}

pub fn get_user_leaderboards(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    uid: i32,
) -> Result<Vec<crate::api::users::ListLeaderboard>, TimeError> {
    let ids = {
        use crate::schema::leaderboard_members::dsl::*;
        leaderboard_members
            .filter(user_id.eq(uid))
            .select(leaderboard_id)
            .order_by(leaderboard_id.asc())
            .load::<i32>(conn)?
    };
    let (names, memcount) = {
        let n = {
            use crate::schema::leaderboards::dsl::*;
            leaderboards
                .filter(id.eq_any(&ids))
                .order_by(id.asc())
                .select(name)
                .load::<String>(conn)?
        };
        let mut c = Vec::new();
        // FIXME: Do this in the query
        for i in ids {
            c.push({
                use crate::schema::leaderboard_members::dsl::*;
                leaderboard_members
                    .filter(leaderboard_id.eq(i))
                    .select(diesel::dsl::count(user_id))
                    .first::<i64>(conn)? as i32
            })
        }
        (n, c)
    };
    let mut ret = Vec::new();
    for (n, c) in names.iter().zip(memcount) {
        ret.push(crate::api::users::ListLeaderboard {
            name: n.to_string(),
            member_count: c,
        });
    }
    Ok(ret)
}

pub fn get_coding_time_steps(
    conn: &PooledConnection<ConnectionManager<PgConnection>>,
    uid: i32,
) -> CodingTimeSteps {
    CodingTimeSteps {
        all_time: get_user_coding_time_since(
            conn,
            uid,
            chrono::NaiveDateTime::from_timestamp(0, 0),
        )
        .unwrap_or(0),
        past_month: get_user_coding_time_since(
            conn,
            uid,
            chrono::Local::now().naive_local() - chrono::Duration::days(30),
        )
        .unwrap_or(0),
        past_week: get_user_coding_time_since(
            conn,
            uid,
            chrono::Local::now().naive_local() - chrono::Duration::days(7),
        )
        .unwrap_or(0),
    }
}
